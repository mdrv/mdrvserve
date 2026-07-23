use anyhow::{Context, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path as AxumPath, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    net::{Ipv4Addr, Ipv6Addr},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, OnceLock},
    time::SystemTime,
};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc, Mutex},
};
use tower_http::cors::CorsLayer;
use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_svg::{render_to_svg, SvgOptions};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;
use tracing::{debug, error, info, trace, warn};

const APP_HTML: &str = include_str!("../frontend/dist/index.html");
const MERMAID_JS: &str = include_str!("../static/js/mermaid.min.js");
const MERMAID_ETAG: &str = concat!("\"", env!("CARGO_PKG_VERSION"), "\"");
const MAX_PORT_ATTEMPTS: u16 = 10;

/// Opt-in diagram rendering engines.
#[derive(Clone, Copy, Default)]
pub(crate) struct DiagramOpts {
    /// Render Mermaid diagrams client-side (lazy-loads bundled JS).
    pub mermaid: bool,
    /// Render D2 diagrams server-side by shelling out to the `d2` binary.
    pub d2: bool,
    /// Render LaTeX math (inline `$...$` and block `$$...$$`) to SVG via RaTeX.
    pub latex: bool,
    /// Render ```typst fenced blocks to SVG via the `typst` binary (server-side).
    pub typst: bool,

    /// Render GitHub-flavored alert callouts (`> [!NOTE]`, ...) server-side.
    pub gfm: bool,
}

type SharedMarkdownState = Arc<Mutex<MarkdownState>>;


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
enum ServerMessage {
    Reload,
}

pub(crate) fn scan_markdown_files(
    dir: &Path,
    recursive: bool,
    include_html: bool,
    include_typst: bool,
) -> Result<Vec<PathBuf>> {
    let mut md_files = Vec::new();

    if recursive {
        collect_markdown_recursive(dir, &mut md_files, include_html, include_typst)?;
    } else {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && is_tracked_file(&path, include_html, include_typst) {
                md_files.push(path);
            }
        }
    }

    md_files.sort();

    Ok(md_files)
}

fn collect_markdown_recursive(
    dir: &Path,
    md_files: &mut Vec<PathBuf>,
    include_html: bool,
    include_typst: bool,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_markdown_recursive(&path, md_files, include_html, include_typst)?;
        } else if path.is_file() && is_tracked_file(&path, include_html, include_typst) {
            md_files.push(path);
        }
    }
    Ok(())
}

/// Compute a file's key relative to the served base directory, using
/// forward-slash separators so it matches the URL path. Files directly in
/// the base directory collapse to just their filename.
fn relative_key(path: &Path, base_dir: &Path) -> String {
    let rel = path.strip_prefix(base_dir).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
        .unwrap_or(false)
}

fn is_html_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm")
        })
        .unwrap_or(false)
}

/// Returns true if the file should be tracked. Markdown files are always
/// tracked; HTML files are tracked when `include_html` is set; Typst files
/// are tracked when `include_typst` is set.
fn is_tracked_file(path: &Path, include_html: bool, include_typst: bool) -> bool {
    is_markdown_file(path)
        || (include_html && is_html_file(path))
        || (include_typst && is_typst_file(path))
}

fn is_typst_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("typ"))
        .unwrap_or(false)
}

struct TrackedFile {
    path: PathBuf,
    last_modified: SystemTime,
    html: Option<String>,
    html_freeflow: Option<String>,
    source: String,
}

struct MarkdownState {
    base_dir: PathBuf,
    tracked_files: HashMap<String, TrackedFile>,
    is_directory_mode: bool,
    mermaid_enabled: bool,
    d2_enabled: bool,
    latex_enabled: bool,
    typst_enabled: bool,
    gfm_enabled: bool,
    include_html: bool,
    include_typst: bool,
    change_tx: broadcast::Sender<ServerMessage>,
}

impl MarkdownState {
    fn new(
        base_dir: PathBuf,
        file_paths: Vec<PathBuf>,
        is_directory_mode: bool,
        opts: DiagramOpts,
        include_html: bool,
        include_typst: bool,
    ) -> Result<Self> {
        let (change_tx, _) = broadcast::channel::<ServerMessage>(16);

        let mut tracked_files = HashMap::new();
        for file_path in file_paths {
            let metadata = fs::metadata(&file_path)?;
            let last_modified = metadata.modified()?;
            let content = fs::read_to_string(&file_path)?;
            let key = relative_key(&file_path, &base_dir);
            tracked_files.insert(
                key,
                TrackedFile {
                    path: file_path,
                    last_modified,
                    html: None,
                    html_freeflow: None,
                    source: content,
                },
            );
        }

        Ok(MarkdownState {
            base_dir,
            tracked_files,
            is_directory_mode,
            mermaid_enabled: opts.mermaid,
            d2_enabled: opts.d2,
            latex_enabled: opts.latex,
            typst_enabled: opts.typst,
            gfm_enabled: opts.gfm,
            include_html,
            include_typst,
            change_tx,
        })
    }

    fn show_navigation(&self) -> bool {
        self.is_directory_mode
    }

    fn get_sorted_filenames(&self) -> Vec<String> {
        let mut filenames: Vec<_> = self.tracked_files.keys().cloned().collect();
        filenames.sort();
        filenames
    }

    fn refresh_file(&mut self, filename: &str) -> Result<()> {
        if let Some(tracked) = self.tracked_files.get_mut(filename) {
            let content = fs::read_to_string(&tracked.path)?;
            tracked.source = content;
            tracked.html = None;
            tracked.html_freeflow = None;
            tracked.last_modified = fs::metadata(&tracked.path)?.modified()?;
            debug!(file = filename, "file changed; invalidated cached render");
        }
        Ok(())
    }

    fn add_tracked_file(&mut self, file_path: PathBuf) -> Result<()> {
        let key = relative_key(&file_path, &self.base_dir);

        if self.tracked_files.contains_key(&key) {
            return Ok(());
        }

        let metadata = fs::metadata(&file_path)?;
        let content = fs::read_to_string(&file_path)?;
        info!(file = %key, "tracking new file");
        self.tracked_files.insert(
            key,
            TrackedFile {
                path: file_path,
                last_modified: metadata.modified()?,
                html: None,
                html_freeflow: None,
                source: content,
            },
        );

        Ok(())
    }

    /// Render a file's source into (html, html_freeflow). Markdown and HTML
    /// are infallible; Typst compilation falls back to a source listing on
    /// any error so one bad file never breaks serving.
    fn render_file_content(
        content: &str,
        path: &Path,
        opts: DiagramOpts,
    ) -> (String, Option<String>) {
        let is_typst = is_typst_file(path);
        let html = if is_typst {
            render_typst_document(content).unwrap_or_default()
        } else if is_html_file(path) {
            Self::render_html_content(content, opts).unwrap_or_else(|_| content.to_string())
        } else {
            Self::markdown_to_html(content, opts).unwrap_or_default()
        };
        let html_freeflow = if is_typst {
            Some(render_typst_document_freeflow(content).unwrap_or_default())
        } else {
            None
        };
        (html, html_freeflow)
    }

    /// Lazily render `filename` on first access; no-op once cached. Rendering
    /// runs under the state lock — for a single-user preview server the
    /// per-file cost is acceptable and matches the existing watcher path.
    fn ensure_rendered(&mut self, filename: &str) {
        let opts = self.opts();
        let Some(tracked) = self.tracked_files.get_mut(filename) else {
            return;
        };
        if tracked.html.is_some() {
            trace!(file = filename, "render cache hit");
            return;
        }
        let (html, html_freeflow) =
            Self::render_file_content(&tracked.source, &tracked.path, opts);
        tracked.html = Some(html);
        tracked.html_freeflow = html_freeflow;
        debug!(file = filename, "rendered file");
    }

    fn opts(&self) -> DiagramOpts {
        DiagramOpts {
            mermaid: self.mermaid_enabled,
            d2: self.d2_enabled,
            latex: self.latex_enabled,
            typst: self.typst_enabled,
            gfm: self.gfm_enabled,
        }
    }

    fn markdown_to_html(content: &str, opts: DiagramOpts) -> Result<String> {
        let mut options = markdown::Options::gfm();
        options.compile.allow_dangerous_html = true;
        options.parse.constructs.frontmatter = true;

        if opts.latex {
            options.parse.constructs.math_flow = true;
            options.parse.constructs.math_text = true;
            options.parse.math_text_single_dollar = true;
        }

        let mut html_body = markdown::to_html_with_options(content, &options)
            .unwrap_or_else(|_| "Error parsing markdown".to_string());

        if opts.d2 {
            html_body = render_d2_blocks(&html_body);
        }

        if opts.latex {
            html_body = render_latex_blocks(&html_body);
        }

        if opts.typst {
            html_body = render_typst_blocks(&html_body);
        }

        if opts.gfm {
            html_body = render_gfm_alerts(&html_body);
        }

        Ok(html_body)
    }

    /// Process raw HTML content with D2 and LaTeX post-processing (no markdown
    /// conversion). Used for `.html`/`.htm` files served via `--include-html`.
    fn render_html_content(content: &str, opts: DiagramOpts) -> Result<String> {
        let mut html = content.to_string();

        if opts.d2 {
            html = render_d2_blocks(&html);
        }

        if opts.latex {
            html = render_latex_blocks(&html);
        }

        if opts.typst {
            html = render_typst_blocks(&html);
        }

        if opts.gfm {
            html = render_gfm_alerts(&html);
        }

        Ok(html)
    }
}

async fn handle_markdown_file_change(path: &Path, state: &SharedMarkdownState) {
    let mut state_guard = state.lock().await;
    if !is_tracked_file(path, state_guard.include_html, state_guard.include_typst) {
        return;
    }

    // Identify the file by its path relative to the served base directory.
    let key = relative_key(path, &state_guard.base_dir);

    // If file is already tracked, refresh its content
    if state_guard.tracked_files.contains_key(&key) {
        if state_guard.refresh_file(&key).is_ok() {
            let _ = state_guard.change_tx.send(ServerMessage::Reload);
        }
    } else if state_guard.is_directory_mode {
        // New file in directory mode - add and reload
        if state_guard.add_tracked_file(path.to_path_buf()).is_ok() {
            let _ = state_guard.change_tx.send(ServerMessage::Reload);
        }
    }
}

async fn handle_file_event(event: Event, state: &SharedMarkdownState) {
    trace!(kind = ?event.kind, paths = ?event.paths, "fs watcher event");
    match event.kind {
        notify::EventKind::Modify(notify::event::ModifyKind::Name(rename_mode)) => {
            use notify::event::RenameMode;
            match rename_mode {
                RenameMode::Both => {
                    // Linux/Windows: Both old and new paths provided in single event
                    if event.paths.len() == 2 {
                        let new_path = &event.paths[1];
                        handle_markdown_file_change(new_path, state).await;
                    }
                }
                RenameMode::From => {
                    // File being renamed away - ignore
                }
                RenameMode::To => {
                    // File renamed to this location
                    if let Some(path) = event.paths.first() {
                        handle_markdown_file_change(path, state).await;
                    }
                }
                RenameMode::Any => {
                    // macOS: Sends separate events for old and new paths
                    // Use file existence to distinguish old (doesn't exist) from new (exists)
                    if let Some(path) = event.paths.first() {
                        if path.exists() {
                            handle_markdown_file_change(path, state).await;
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {
            for path in &event.paths {
                if is_markdown_file(path) || is_html_file(path) || is_typst_file(path) {
                    match event.kind {
                        notify::EventKind::Create(_)
                        | notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                            handle_markdown_file_change(path, state).await;
                        }
                        notify::EventKind::Remove(_) => {
                            // Don't remove files from tracking. Editors like neovim save by
                            // renaming the file to a backup, then creating a new one. If we
                            // removed the file here, HTTP requests during that window would
                            // see empty tracked_files and return 404.
                        }
                        _ => {}
                    }
                } else if path.is_file() && is_image_file(path.to_str().unwrap_or("")) {
                    match event.kind {
                        notify::EventKind::Modify(_)
                        | notify::EventKind::Create(_)
                        | notify::EventKind::Remove(_) => {
                            let state_guard = state.lock().await;
                            let _ = state_guard.change_tx.send(ServerMessage::Reload);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn new_router(
    base_dir: PathBuf,
    tracked_files: Vec<PathBuf>,
    is_directory_mode: bool,
    is_recursive: bool,
    opts: DiagramOpts,
    include_html: bool,
    include_typst: bool,
) -> Result<Router> {
    let base_dir = base_dir.canonicalize()?;

    let state = Arc::new(Mutex::new(MarkdownState::new(
        base_dir.clone(),
        tracked_files,
        is_directory_mode,
        opts,
        include_html,
        include_typst,
    )?));

    let watcher_state = state.clone();
    let (tx, mut rx) = mpsc::channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        },
        Config::default(),
    )?;

    let watch_mode = if is_recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher.watch(&base_dir, watch_mode)?;

    tokio::spawn(async move {
        let _watcher = watcher;
        while let Some(event) = rx.recv().await {
            handle_file_event(event, &watcher_state).await;
        }
    });

    let router = Router::new()
        .route("/", get(serve_html_root))
        .route("/ws", get(websocket_handler))
        .route("/mermaid.min.js", get(serve_mermaid_js))
        .route("/{*filename}", get(serve_file))
        .layer(CorsLayer::permissive())
        .with_state(state);

    Ok(router)
}

async fn bind_with_retry(hostname: &str, port: u16) -> Result<(TcpListener, u16)> {
    let mut last_err = None;
    for offset in 0..MAX_PORT_ATTEMPTS {
        let try_port = match port.checked_add(offset) {
            Some(p) => p,
            None => break,
        };
        match TcpListener::bind((hostname, try_port)).await {
            Ok(listener) => return Ok((listener, try_port)),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => last_err = Some(e),
            Err(e) => return Err(e.into()),
        }
    }
    Err(last_err
        .map(|e| anyhow::anyhow!(e))
        .unwrap_or_else(|| anyhow::anyhow!("no valid port in range"))
        .context(format!(
            "could not bind to ports {}--{}",
            port,
            port.saturating_add(MAX_PORT_ATTEMPTS - 1)
        )))
}

pub(crate) async fn serve_markdown(
    base_dir: PathBuf,
    tracked_files: Vec<PathBuf>,
    is_directory_mode: bool,
    is_recursive: bool,
    hostname: impl AsRef<str>,
    port: u16,
    open: bool,
    opts: DiagramOpts,
    include_html: bool,
    include_typst: bool,
) -> Result<()> {
    let hostname = hostname.as_ref();

    if opts.d2 && !d2_available() {
        warn!(
            "--with-d2 set but the `d2` binary was not found on PATH; \
             D2 code blocks will render as plain source (https://d2lang.com)"
        );
    }

    if (opts.typst || include_typst) && !typst_available() {
        warn!(
            "--with-typst/--include-typst set but the `typst` binary was not found on PATH; \
             typst content will render as plain source (https://github.com/typst/typst)"
        );
    }

    if (opts.typst || include_typst) && typst_available() && !typst_html_available() {
        warn!(
            "`typst` is installed but lacks the HTML export feature; \
             typst free-flow will be disabled (paged rendering still works). \
             Install a build compiled with `--features html` (0.13+)"
        );
    }

    let first_file = tracked_files.first().cloned();
    let file_count = tracked_files.len();
    let router = new_router(
        base_dir.clone(),
        tracked_files,
        is_directory_mode,
        is_recursive,
        opts,
        include_html,
        include_typst,
    )?;

    let (listener, actual_port) = bind_with_retry(hostname, port).await?;

    if actual_port != port {
        warn!(requested = port, actual = actual_port, "requested port in use; using alternative");
    }

    let listen_addr = format_host(hostname, actual_port);

    if is_directory_mode {
        info!(dir = %base_dir.display(), files = file_count, "serving markdown directory");
    } else if let Some(file_path) = first_file {
        info!(file = %file_path.display(), "serving markdown file");
    }

    info!(url = format!("http://{listen_addr}"), "server listening; live reload enabled (Ctrl+C to stop)");

    if open {
        let browse_addr = format_host(&browsable_host(hostname), actual_port);
        open_browser(&format!("http://{browse_addr}"))?;
    }

    axum::serve(listener, router).await?;

    Ok(())
}

/// Format the host address (hostname + port) for printing.
fn format_host(hostname: &str, port: u16) -> String {
    if hostname.parse::<Ipv6Addr>().is_ok() {
        format!("[{hostname}]:{port}")
    } else {
        format!("{hostname}:{port}")
    }
}

/// Map wildcard bind addresses to loopback so the browser gets a
/// reachable URL.
fn browsable_host(hostname: &str) -> String {
    if hostname
        .parse::<Ipv4Addr>()
        .ok()
        .is_some_and(|ip| ip.is_unspecified())
    {
        "127.0.0.1".into()
    } else if hostname
        .parse::<Ipv6Addr>()
        .ok()
        .is_some_and(|ip| ip.is_unspecified())
    {
        "::1".into()
    } else {
        hostname.into()
    }
}

/// Open a URL in the default browser using platform commands.
///
/// Fails immediately if the command cannot be spawned (e.g. not
/// installed). Exit status is monitored in a background thread
/// since opener commands may block until their handler process
/// returns.
fn open_browser(url: &str) -> Result<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        anyhow::bail!("--open is not supported on this platform");
    };

    let mut child = std::process::Command::new(program)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("failed to run {program}"))?;

    std::thread::spawn(move || match child.wait() {
        Ok(status) if !status.success() => {
            warn!(%program, ?status, "browser command exited non-zero");
        }
        Err(e) => error!(%program, error = %e, "failed waiting on browser"),
        _ => {}
    });

    Ok(())
}

async fn serve_html_root(State(state): State<SharedMarkdownState>) -> impl IntoResponse {
    let mut state = state.lock().await;

    let filename = match state.get_sorted_filenames().into_iter().next() {
        Some(name) => name,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("No files available to serve".to_string()),
            );
        }
    };

    debug!(file = %filename, "serving root (index)");
    state.ensure_rendered(&filename);
    render_markdown(&state, &filename).await
}

async fn serve_file(
    AxumPath(filename): AxumPath<String>,
    State(state): State<SharedMarkdownState>,
) -> axum::response::Response {
    debug!(file = %filename, "serving request");
    if filename.ends_with(".md")
        || filename.ends_with(".markdown")
        || filename.ends_with(".html")
        || filename.ends_with(".htm")
        || filename.ends_with(".typ")
    {
        let mut state = state.lock().await;

        if !state.tracked_files.contains_key(&filename) {
            return (StatusCode::NOT_FOUND, Html("File not found".to_string())).into_response();
        }

        state.ensure_rendered(&filename);
        let (status, html) = render_markdown(&state, &filename).await;
        (status, html).into_response()
    } else if is_image_file(&filename) {
        serve_static_file_inner(filename, state).await
    } else {
        (StatusCode::NOT_FOUND, Html("File not found".to_string())).into_response()
    }
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
enum NavNode {
    File {
        name: String,
        #[serde(rename = "fullPath")]
        full_path: String,
        abbr: String,
    },
    Dir {
        name: String,
        children: Vec<NavNode>,
    },
}

fn build_nav_data(sorted_keys: &[String]) -> Vec<NavNode> {
    let mut root: Vec<NavNode> = Vec::new();
    for key in sorted_keys {
        let parts: Vec<&str> = key.split('/').collect();
        nav_insert(&mut root, &parts, key);
    }
    root
}

fn nav_insert(nodes: &mut Vec<NavNode>, parts: &[&str], full_key: &str) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        nodes.push(NavNode::File {
            name: parts[0].to_string(),
            full_path: full_key.to_string(),
            abbr: file_abbr(parts[0]),
        });
        return;
    }

    let dir_name = parts[0].to_string();
    let idx = nodes
        .iter()
        .position(|n| matches!(n, NavNode::Dir { name, .. } if name == &dir_name))
        .unwrap_or_else(|| {
            nodes.push(NavNode::Dir {
                name: dir_name.clone(),
                children: Vec::new(),
            });
            nodes.len() - 1
        });

    if let NavNode::Dir { children, .. } = &mut nodes[idx] {
        nav_insert(children, &parts[1..], full_key);
    }
}

/// Generate a 1-2 character abbreviation for a file name, suitable for the
/// collapsed sidebar circle buttons. Prefers uppercase initials (handles
/// PascalCase / camelCase names), falls back to the first two characters.
fn file_abbr(name: &str) -> String {
    let stem = name
        .trim_end_matches(".md")
        .trim_end_matches(".markdown");
    let caps: String = stem.chars().filter(|c| c.is_uppercase()).collect();
    match caps.len() {
        0 => stem.chars().take(2).collect(),
        _ => caps.chars().take(2).collect(),
    }
}

/// Whether the `d2` binary is available on PATH. Cached for the process.
fn d2_available() -> bool {
    static AVAIL: OnceLock<bool> = OnceLock::new();
    *AVAIL.get_or_init(|| {
        Command::new("d2")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Replace every ` ```d2 ` code block in `html` with an inline SVG rendered by
/// the `d2` binary. Blocks that fail to render (or when `d2` is absent) are
/// left untouched so the source remains visible.
fn render_d2_blocks(html: &str) -> String {
    if !d2_available() {
        return html.to_string();
    }

    let re = Regex::new(r#"(?s)<pre><code class="language-d2">(.*?)</code></pre>"#)
        .expect("static regex");

    let mut result = String::with_capacity(html.len());
    let mut last_end = 0;
    let mut index = 0u32;

    for caps in re.captures_iter(html) {
        let m = caps.get(0).unwrap();
        result.push_str(&html[last_end..m.start()]);

        let source = unescape_html(&caps[1]);
        match render_one_d2(&source, index) {
            Ok(svg) => {
                result.push_str(r#"<div class="d2-diagram">"#);
                result.push_str(&svg);
                result.push_str("</div>");
            }
            Err(_) => {
                // Leave the original block intact so the source is visible.
                result.push_str(m.as_str());
            }
        }

        last_end = m.end();
        index += 1;
    }
    result.push_str(&html[last_end..]);
    result
}

/// Render a single D2 source snippet to SVG via `d2 -` (stdin → stdout).
fn render_one_d2(source: &str, index: u32) -> Result<String> {
    let mut child = Command::new("d2")
        .args([
            "--no-xml-tag",
            &format!("--salt=mdrv{}", index),
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn `d2`")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(source.as_bytes())?;
    }
    // Drop stdin to signal EOF.
    child.stdin.take();

    let output = child
        .wait_with_output()
        .context("failed to wait on `d2`")?;

    if !output.status.success() {
        anyhow::bail!(
            "d2 exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Reverse the HTML entity encoding that markdown-rs applies to code block
/// bodies. `&amp;` is handled last so it does not corrupt the other entities.
fn unescape_html(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Escape `&`, `<`, `>` for safe display inside a `<pre><code>` fallback block.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Whether the `typst` binary is available on PATH. Cached for the process.
fn typst_available() -> bool {
    static AVAIL: OnceLock<bool> = OnceLock::new();
    *AVAIL.get_or_init(|| {
        Command::new("typst")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Whether the installed `typst` supports HTML export (`--features html`).
/// Probed once by compiling a trivial document and cached for the process.
fn typst_html_available() -> bool {
    static AVAIL: OnceLock<bool> = OnceLock::new();
    *AVAIL.get_or_init(|| {
        let Ok(mut child) = Command::new("typst")
            .args(["compile", "-", "-", "--format", "html", "--features", "html"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(b"x\n");
        }
        child
            .wait_with_output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout).contains("<body")
            })
            .unwrap_or(false)
    })
}

/// Replace every ```typst code block in `html` with an inline SVG rendered by
/// the `typst` binary. Blocks that fail to render (or when `typst` is absent)
/// are left untouched so the source remains visible.
fn render_typst_blocks(html: &str) -> String {
    if !typst_available() {
        return html.to_string();
    }

    let re = Regex::new(r#"(?s)<pre><code class="language-typst">(.*?)</code></pre>"#)
        .expect("static regex");

    let mut result = String::with_capacity(html.len());
    let mut last_end = 0;

    for caps in re.captures_iter(html) {
        let m = caps.get(0).unwrap();
        result.push_str(&html[last_end..m.start()]);

        let source = unescape_html(&caps[1]);
        match render_one_typst(&source) {
            Ok(svg) => {
                result.push_str(r#"<div class="typst-doc"><div class="typst-doc-page">"#);
                result.push_str(&svg);
                result.push_str("</div></div>");
            }
            Err(_) => {
                // Leave the original block intact so the source is visible.
                result.push_str(m.as_str());
            }
        }

        last_end = m.end();
    }
    result.push_str(&html[last_end..]);
    result
}

/// Render a single Typst source snippet to SVG via `typst compile - -`
/// (stdin → stdout). Single-page only; multi-page snippets fall back to source.
fn render_one_typst(source: &str) -> Result<String> {
    let mut child = Command::new("typst")
        .args(["compile", "-", "-", "--format", "svg"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn `typst`")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(source.as_bytes())?;
    }
    // Drop stdin to signal EOF.
    child.stdin.take();

    let output = child
        .wait_with_output()
        .context("failed to wait on `typst`")?;

    if !output.status.success() {
        anyhow::bail!(
            "typst exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Compile Typst `source` to a list of per-page SVG strings using a temp
/// directory and a page-number template (supports multi-page output).
/// Returns an empty vec when `typst` is absent or compilation fails, so the
/// caller can fall back to a source listing. IO/spawn errors propagate.
fn compile_typst_pages(source: &str) -> Result<Vec<String>> {
    if !typst_available() {
        return Ok(Vec::new());
    }

    let dir = tempfile::TempDir::new()?;
    let out_pattern = dir.path().join("page-{0p}-of-{t}.svg");
    let mut child = Command::new("typst")
        .args([
            "compile",
            "-",
            &out_pattern.to_string_lossy(),
            "--format",
            "svg",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn `typst`")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(source.as_bytes())?;
    }
    child.stdin.take();

    let output = child
        .wait_with_output()
        .context("failed to wait on `typst`")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let mut pages: Vec<PathBuf> = fs::read_dir(dir.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("svg"))
        .collect();
    pages.sort();

    pages
        .into_iter()
        .map(|p| fs::read_to_string(p).map_err(anyhow::Error::from))
        .collect()
}

/// Wrap a list of per-page SVG strings into the `.typst-doc` container
/// (paged layout: one `.typst-doc-page` per page, separated by a page break).
fn wrap_typst_pages(pages: &[String]) -> String {
    let mut html = String::new();
    html.push_str(r#"<div class="typst-doc">"#);
    for (i, svg) in pages.iter().enumerate() {
        if i > 0 {
            html.push_str(r#"<hr class="typst-page-break">"#);
        }
        html.push_str(r#"<div class="typst-doc-page">"#);
        html.push_str(svg);
        html.push_str("</div>");
    }
    html.push_str("</div>");
    html
}

/// Compile a standalone `.typ` document to inlined SVG page(s), preserving
/// Typst's paged model. Falls back to a plain source listing when `typst` is
/// absent or compilation fails.
fn render_typst_document(content: &str) -> Result<String> {
    let source_fallback = || {
        format!(
            r#"<pre><code class="language-typst">{}</code></pre>"#,
            escape_html(content)
        )
    };

    let pages = compile_typst_pages(content)?;
    if pages.is_empty() {
        return Ok(source_fallback());
    }

    Ok(wrap_typst_pages(&pages))
}

/// Compile a `.typ` document in "free-flow" mode using Typst's HTML export
/// (`--format html --features html`). The output is reflowable semantic HTML
/// with native MathML; Typst injects only MathML-alignment CSS, so the result
/// inherits mdrvserve's theme colors (dark mode included). Returns an empty
/// string when the feature is unavailable or compilation fails, which the
/// frontend uses to disable the free-flow toggle.
fn render_typst_document_freeflow(content: &str) -> Result<String> {
    if !typst_html_available() {
        return Ok(String::new());
    }

    let mut child = Command::new("typst")
        .args([
            "compile", "-", "-", "--format", "html", "--features", "html",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn `typst`")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())?;
    }
    child.stdin.take();

    let output = child
        .wait_with_output()
        .context("failed to wait on `typst`")?;

    if !output.status.success() {
        return Ok(String::new());
    }

    let html = String::from_utf8_lossy(&output.stdout);
    let style_block = Regex::new(r#"(?s)<style[^>]*>.*?</style>"#)
        .expect("static regex")
        .find(&html)
        .map(|m| m.as_str())
        .unwrap_or("");
    let body_inner = Regex::new(r#"(?s)<body[^>]*>(.*?)</body>"#)
        .expect("static regex")
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    if body_inner.trim().is_empty() {
        return Ok(String::new());
    }

    Ok(format!(
        r#"<div class="typst-doc typst-doc-freeflow">{style_block}{body_inner}</div>"#
    ))
}

/// Replace every math element (inline `$...$` and block `$$...$$`) in `html`
/// Convert GitHub-flavored Markdown alert blockquotes into styled callouts.
///
/// markdown-rs does not implement GFM alerts, so `> [!NOTE]` (and TIP,
/// IMPORTANT, WARNING, CAUTION) render as a normal blockquote whose first
/// paragraph begins with the literal `[!TYPE]` marker. This post-processor
/// detects such blockquotes and rewraps them as titled callout boxes
/// (`<div class="markdown-alert markdown-alert-{type}">`). Non-alert
/// blockquotes are left untouched.
fn render_gfm_alerts(html: &str) -> String {
    let Ok(block_re) = Regex::new(r"(?s)<blockquote>(.*?)</blockquote>") else {
        return html.to_string();
    };
    let Ok(alert_re) = Regex::new(r"(?s)^\s*<p>\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*(.*?)</p>(.*)$") else {
        return html.to_string();
    };

    block_re
        .replace_all(html, |caps: &regex::Captures| {
            let inner = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let Some(a) = alert_re.captures(inner) else {
                return caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default();
            };
            let typ = a.get(1).map(|m| m.as_str()).unwrap_or("NOTE");
            let first_rest = a.get(2).map(|m| m.as_str()).unwrap_or("");
            let remainder = a.get(3).map(|m| m.as_str()).unwrap_or("");
            let (kind, title) = match typ {
                "TIP" => ("tip", "Tip"),
                "IMPORTANT" => ("important", "Important"),
                "WARNING" => ("warning", "Warning"),
                "CAUTION" => ("caution", "Caution"),
                _ => ("note", "Note"),
            };
            let mut body = String::new();
            let first_trimmed = first_rest.trim();
            if !first_trimmed.is_empty() {
                body.push_str("<p>");
                body.push_str(first_trimmed);
                body.push_str("</p>");
            }
            body.push_str(remainder.trim_start());
            let mut out = String::new();
            out.push_str("<div class='markdown-alert markdown-alert-");
            out.push_str(kind);
            out.push_str("'><p class='markdown-alert-title'>");
            out.push_str(title);
            out.push_str("</p>");
            out.push_str(&body);
            out.push_str("</div>");
            out
        })
        .into_owned()
}

/// with an inline SVG rendered by RaTeX. Elements that fail to parse are
/// left untouched so the source remains visible.
fn render_latex_blocks(html: &str) -> String {
    // Display math: <pre><code class="language-math math-display">CONTENT</code></pre>
    let display_re = Regex::new(
        r#"(?s)<pre><code class="language-math math-display">(.*?)</code></pre>"#,
    )
    .expect("static regex");
    let mut result = display_re
        .replace_all(html, |caps: &regex::Captures| {
            let source = unescape_html(&caps[1]);
            match render_one_latex(&source, false) {
                Ok(svg) => format!(r#"<div class="latex-display">{svg}</div>"#),
                Err(_) => caps[0].to_string(),
            }
        })
        .into_owned();

    // Inline math: <code class="language-math math-inline">CONTENT</code>
    let inline_re = Regex::new(
        r#"(?s)<code class="language-math math-inline">(.*?)</code>"#,
    )
    .expect("static regex");
    result = inline_re
        .replace_all(&result, |caps: &regex::Captures| {
            let source = unescape_html(&caps[1]);
            match render_one_latex(&source, true) {
                Ok(svg) => format!(r#"<span class="latex-inline">{svg}</span>"#),
                Err(_) => caps[0].to_string(),
            }
        })
        .into_owned();

    // VJudge-style display math: <span/div class='... math math-display ...'>$$CONTENT$$</span/div>
    // Handles single/double quotes and multi-line class attributes.
    let vjudge_display_re = Regex::new(
        r#"(?s)<(?:span|div)\s[^>]*class\s*=\s*[\'"]\s*[^\'"]*\bmath\s+math-display\b[^\'"]*[\'"][^>]*>\s*\$\$(.*?)\$\$\s*</(?:span|div)>"#,
    )
    .expect("static regex");
    result = vjudge_display_re
        .replace_all(&result, |caps: &regex::Captures| {
            let source = unescape_html(caps[1].trim());
            match render_one_latex(&source, false) {
                Ok(svg) => format!(r#"<div class="latex-display">{svg}</div>"#),
                Err(_) => caps[0].to_string(),
            }
        })
        .into_owned();

    // VJudge-style inline math: <span class='... math math-inline ...'>$CONTENT$</span>
    let vjudge_inline_re = Regex::new(
        r#"(?s)<(?:span|code)\s[^>]*class\s*=\s*[\'"]\s*[^\'"]*\bmath\s+math-inline\b[^\'"]*[\'"][^>]*>\s*\$(.*?)\$\s*</(?:span|code)>"#,
    )
    .expect("static regex");
    result = vjudge_inline_re
        .replace_all(&result, |caps: &regex::Captures| {
            let source = unescape_html(caps[1].trim());
            match render_one_latex(&source, true) {
                Ok(svg) => format!(r#"<span class="latex-inline">{svg}</span>"#),
                Err(_) => caps[0].to_string(),
            }
        })
        .into_owned();

    result
}

/// Render a single LaTeX math expression to an SVG string via RaTeX.
/// `inline` controls the math style (Text vs Display).
fn render_one_latex(source: &str, inline: bool) -> Result<String> {
    let ast = parse(source).map_err(|e| anyhow::anyhow!("LaTeX parse error: {e}"))?;
    let style = if inline {
        MathStyle::Text
    } else {
        MathStyle::Display
    };
    let layout_opts = LayoutOptions::default()
        .with_style(style)
        .with_color(Color::BLACK);
    let lbox = layout(&ast, &layout_opts);
    let display_list = to_display_list(&lbox);
    let font_size = if inline { 20.0 } else { 32.0 };
    let svg_opts = SvgOptions {
        font_size,
        padding: 0.0,
        stroke_width: 1.0,
        embed_glyphs: true,
        font_dir: String::new(),
    };
    let svg = render_to_svg(&display_list, &svg_opts);
    Ok(scale_svg_to_em(&svg, font_size))
}

fn scale_svg_to_em(svg: &str, font_size: f64) -> String {
    let viewbox_re = regex::Regex::new(r#"viewBox="[\d.\-]+ [\d.\-]+ [\d.\-]+ ([\d.\-]+)""#).unwrap();
    let svg_tag_end = svg.find('>').unwrap_or(svg.len());
    let (head, rest) = svg.split_at(svg_tag_end);
    let dim_re = regex::Regex::new(r#"\s(?:width|height)=\"[^\"]*\""#).unwrap();
    let stripped_head = dim_re.replace_all(head, "");
    let stripped = format!("{stripped_head}{rest}");
    match viewbox_re.captures(&stripped) {
        Some(caps) => {
            let vh: f64 = caps[1].parse().unwrap_or(font_size);
            let em = vh / font_size;
            stripped.replacen("<svg", &format!("<svg style=\"height:{em}em;width:auto\""), 1).to_string()
        }
        None => stripped.to_string(),
    }
}

async fn render_markdown(state: &MarkdownState, current_file: &str) -> (StatusCode, Html<String>) {
    let tracked = match state.tracked_files.get(current_file) {
        Some(t) => t,
        None => return (StatusCode::NOT_FOUND, Html("File not found".to_string())),
    };
    let content = tracked.html.as_deref().unwrap_or("");
    let source = tracked.source.as_str();
    let is_typst = is_typst_file(std::path::Path::new(current_file));
    let content_freeflow = tracked.html_freeflow.as_deref().unwrap_or("");

    let has_mermaid = state.mermaid_enabled && content.contains(r#"class="language-mermaid""#);

    let page_title = std::path::Path::new(current_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(current_file);

    let show_nav = state.show_navigation();
    let nav_items = if show_nav {
        build_nav_data(&state.get_sorted_filenames())
    } else {
        Vec::new()
    };

    let data = serde_json::json!({
        "content": content,
        "sourceContent": source,
        "navItems": nav_items,
        "pageTitle": page_title,
        "showNavigation": show_nav,
        "mermaidEnabled": has_mermaid,
        "isTypst": is_typst,
        "contentFreeflow": content_freeflow,
    });

    let mut json = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());
    // Escape < to prevent </script> breaking the HTML parser
    json = json.replace('<', "\\u003c");

    let html = APP_HTML.replace("__MDRV_DATA_PLACEHOLDER__", &json);

    (StatusCode::OK, Html(html))
}

async fn serve_mermaid_js(headers: HeaderMap) -> impl IntoResponse {
    if is_etag_match(&headers) {
        return mermaid_response(StatusCode::NOT_MODIFIED, None);
    }

    mermaid_response(StatusCode::OK, Some(MERMAID_JS))
}

fn is_etag_match(headers: &HeaderMap) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|etags| etags.split(',').any(|tag| tag.trim() == MERMAID_ETAG))
}

fn mermaid_response(status: StatusCode, body: Option<&'static str>) -> impl IntoResponse {
    // Use no-cache to force revalidation on each request. This ensures clients
    // get updated content when mdserve is rebuilt with a new Mermaid version,
    // while still benefiting from 304 responses via ETag matching.
    let headers = [
        (header::CONTENT_TYPE, "application/javascript"),
        (header::ETAG, MERMAID_ETAG),
        (header::CACHE_CONTROL, "public, no-cache"),
    ];

    match body {
        Some(content) => (status, headers, content).into_response(),
        None => (status, headers).into_response(),
    }
}

async fn serve_static_file_inner(
    filename: String,
    state: SharedMarkdownState,
) -> axum::response::Response {
    let state = state.lock().await;

    let full_path = state.base_dir.join(&filename);

    match full_path.canonicalize() {
        Ok(canonical_path) => {
            if !canonical_path.starts_with(&state.base_dir) {
                return (
                    StatusCode::FORBIDDEN,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "Access denied".to_string(),
                )
                    .into_response();
            }

            match fs::read(&canonical_path) {
                Ok(contents) => {
                    let content_type = guess_image_content_type(&filename);
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, content_type.as_str())],
                        contents,
                    )
                        .into_response()
                }
                Err(_) => (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "File not found".to_string(),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            "File not found".to_string(),
        )
            .into_response(),
    }
}

fn is_image_file(file_path: &str) -> bool {
    guess_image_content_type(file_path).starts_with("image/")
}

fn guess_image_content_type(file_path: &str) -> String {
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
    .to_string()
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedMarkdownState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: SharedMarkdownState) {
    let (mut sender, mut receiver) = socket.split();

    let mut change_rx = {
        let state = state.lock().await;
        state.change_tx.subscribe()
    };

    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(_)) => {}
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Ok(reload_msg) = change_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&reload_msg) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Path::new("test.md")));
        assert!(is_markdown_file(Path::new("/path/to/file.md")));

        assert!(is_markdown_file(Path::new("test.markdown")));
        assert!(is_markdown_file(Path::new("/path/to/file.markdown")));

        assert!(is_markdown_file(Path::new("test.MD")));
        assert!(is_markdown_file(Path::new("test.Md")));
        assert!(is_markdown_file(Path::new("test.MARKDOWN")));
        assert!(is_markdown_file(Path::new("test.MarkDown")));

        assert!(!is_markdown_file(Path::new("test.txt")));
        assert!(!is_markdown_file(Path::new("test.rs")));
        assert!(!is_markdown_file(Path::new("test.html")));
        assert!(!is_markdown_file(Path::new("test")));
        assert!(!is_markdown_file(Path::new("README")));
    }

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("test.png"));
        assert!(is_image_file("test.jpg"));
        assert!(is_image_file("test.jpeg"));
        assert!(is_image_file("test.gif"));
        assert!(is_image_file("test.svg"));
        assert!(is_image_file("test.webp"));
        assert!(is_image_file("test.bmp"));
        assert!(is_image_file("test.ico"));

        assert!(is_image_file("test.PNG"));
        assert!(is_image_file("test.JPG"));
        assert!(is_image_file("test.JPEG"));

        assert!(is_image_file("/path/to/image.png"));
        assert!(is_image_file("./images/photo.jpg"));

        assert!(!is_image_file("test.txt"));
        assert!(!is_image_file("test.md"));
        assert!(!is_image_file("test.rs"));
        assert!(!is_image_file("test"));
    }

    #[test]
    fn test_guess_image_content_type() {
        assert_eq!(guess_image_content_type("test.png"), "image/png");
        assert_eq!(guess_image_content_type("test.jpg"), "image/jpeg");
        assert_eq!(guess_image_content_type("test.jpeg"), "image/jpeg");
        assert_eq!(guess_image_content_type("test.gif"), "image/gif");
        assert_eq!(guess_image_content_type("test.svg"), "image/svg+xml");
        assert_eq!(guess_image_content_type("test.webp"), "image/webp");
        assert_eq!(guess_image_content_type("test.bmp"), "image/bmp");
        assert_eq!(guess_image_content_type("test.ico"), "image/x-icon");

        assert_eq!(guess_image_content_type("test.PNG"), "image/png");
        assert_eq!(guess_image_content_type("test.JPG"), "image/jpeg");

        assert_eq!(
            guess_image_content_type("test.xyz"),
            "application/octet-stream"
        );
        assert_eq!(guess_image_content_type("test"), "application/octet-stream");
    }

    #[test]
    fn test_scan_markdown_files_empty_directory() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        let result = scan_markdown_files(temp_dir.path(), false, false, false).expect("Failed to scan");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scan_markdown_files_with_markdown_files() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        fs::write(temp_dir.path().join("test1.md"), "# Test 1").expect("Failed to write");
        fs::write(temp_dir.path().join("test2.markdown"), "# Test 2").expect("Failed to write");
        fs::write(temp_dir.path().join("test3.md"), "# Test 3").expect("Failed to write");

        fs::write(temp_dir.path().join("test.txt"), "text").expect("Failed to write");
        fs::write(temp_dir.path().join("README"), "readme").expect("Failed to write");

        let result = scan_markdown_files(temp_dir.path(), false, false, false).expect("Failed to scan");

        assert_eq!(result.len(), 3);

        let filenames: Vec<_> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(filenames, vec!["test1.md", "test2.markdown", "test3.md"]);
    }

    #[test]
    fn test_scan_markdown_files_ignores_subdirectories() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        fs::write(temp_dir.path().join("root.md"), "# Root").expect("Failed to write");

        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdir");
        fs::write(sub_dir.join("nested.md"), "# Nested").expect("Failed to write");

        let result = scan_markdown_files(temp_dir.path(), false, false, false).expect("Failed to scan");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_name().unwrap().to_str().unwrap(), "root.md");
    }

    #[test]
    fn test_scan_markdown_files_case_insensitive() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        fs::write(temp_dir.path().join("test1.md"), "# Test 1").expect("Failed to write");
        fs::write(temp_dir.path().join("test2.MD"), "# Test 2").expect("Failed to write");
        fs::write(temp_dir.path().join("test3.Md"), "# Test 3").expect("Failed to write");
        fs::write(temp_dir.path().join("test4.MARKDOWN"), "# Test 4").expect("Failed to write");

        let result = scan_markdown_files(temp_dir.path(), false, false, false).expect("Failed to scan");

        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_scan_markdown_files_recursive() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        fs::write(temp_dir.path().join("root.md"), "# Root").expect("Failed to write");

        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdir");
        fs::write(sub_dir.join("nested.md"), "# Nested").expect("Failed to write");

        let nested_dir = sub_dir.join("deep");
        fs::create_dir(&nested_dir).expect("Failed to create nested dir");
        fs::write(nested_dir.join("deeper.md"), "# Deeper").expect("Failed to write");

        // Non-recursive still ignores nested files
        let flat = scan_markdown_files(temp_dir.path(), false, false, false).expect("Failed to scan");
        assert_eq!(flat.len(), 1);

        // Recursive picks up nested files
        let result = scan_markdown_files(temp_dir.path(), true, false, false).expect("Failed to scan");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_format_host() {
        assert_eq!(format_host("127.0.0.1", 3000), "127.0.0.1:3000");
        assert_eq!(format_host("192.168.1.1", 8080), "192.168.1.1:8080");

        assert_eq!(format_host("localhost", 3000), "localhost:3000");
        assert_eq!(format_host("example.com", 80), "example.com:80");

        assert_eq!(format_host("::1", 3000), "[::1]:3000");
        assert_eq!(format_host("2001:db8::1", 8080), "[2001:db8::1]:8080");
    }

    #[test]
    fn test_browsable_host() {
        assert_eq!(browsable_host("0.0.0.0"), "127.0.0.1");
        assert_eq!(browsable_host("::"), "::1");
        assert_eq!(browsable_host("127.0.0.1"), "127.0.0.1");
        assert_eq!(browsable_host("::1"), "::1");
        assert_eq!(browsable_host("192.168.1.1"), "192.168.1.1");
        assert_eq!(browsable_host("localhost"), "localhost");
        assert_eq!(browsable_host("example.com"), "example.com");
    }

    #[tokio::test]
    async fn test_bind_retries_on_addr_in_use() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let blocked_port = listener.local_addr().unwrap().port();

        let (retry_listener, actual_port) =
            bind_with_retry("127.0.0.1", blocked_port).await.unwrap();

        assert!(
            actual_port > blocked_port,
            "Should bind to a higher port when requested port is in use"
        );

        drop(retry_listener);
        drop(listener);
    }

    use axum_test::TestServer;
    use std::time::Duration;
    use tempfile::{Builder, NamedTempFile, TempDir};

    const FILE_WATCH_DELAY_MS: u64 = 100;
    const WEBSOCKET_TIMEOUT_SECS: u64 = 5;

    const TEST_FILE_1_CONTENT: &str = "# Test 1\n\nContent of test1";
    const TEST_FILE_2_CONTENT: &str = "# Test 2\n\nContent of test2";
    const TEST_FILE_3_CONTENT: &str = "# Test 3\n\nContent of test3";
    const YAML_FRONTMATTER_CONTENT: &str =
        "---\ntitle: Test Post\nauthor: Name\n---\n\n# Test Post\n";
    const TOML_FRONTMATTER_CONTENT: &str = "+++\ntitle = \"Test Post\"\n+++\n\n# Test Post\n";

    fn extract_mdrv_data(body: &str) -> serde_json::Value {
        let marker = r#"id="__mdrv-data">"#;
        match body.find(marker) {
            Some(start) => {
                let rest = &body[start + marker.len()..];
                match rest.find("</script>") {
                    Some(end) => {
                        serde_json::from_str(&rest[..end]).unwrap_or(serde_json::Value::Null)
                    }
                    None => serde_json::Value::Null,
                }
            }
            None => serde_json::Value::Null,
        }
    }

    fn extract_content(body: &str) -> String {
        extract_mdrv_data(body)["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }

    fn extract_nav_json(body: &str) -> String {
        serde_json::to_string(&extract_mdrv_data(body)["navItems"]).unwrap_or_default()
    }

    fn create_test_server_impl(
        content: &str,
        use_http: bool,
        opts: DiagramOpts,
    ) -> (TestServer, NamedTempFile) {
        let temp_file = Builder::new()
            .suffix(".md")
            .tempfile()
            .expect("Failed to create temp file");
        fs::write(&temp_file, content).expect("Failed to write temp file");

        let canonical_path = temp_file
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_file.path().to_path_buf());

        let base_dir = canonical_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        let tracked_files = vec![canonical_path];
        let is_directory_mode = false;

        let router = new_router(base_dir, tracked_files, is_directory_mode, false, opts, false, false)
            .expect("Failed to create router");

        let server = if use_http {
            TestServer::builder()
                .http_transport()
                .build(router)
        } else {
            TestServer::new(router)
        };

        (server, temp_file)
    }

    async fn create_test_server(content: &str) -> (TestServer, NamedTempFile) {
        create_test_server_impl(content, false, DiagramOpts::default())
    }

    async fn create_test_server_with_http(content: &str) -> (TestServer, NamedTempFile) {
        create_test_server_impl(content, true, DiagramOpts::default())
    }

    async fn create_test_server_with_mermaid(content: &str) -> (TestServer, NamedTempFile) {
        create_test_server_impl(
            content,
            false,
            DiagramOpts {
                mermaid: true,
                d2: false,
                latex: false,
                typst: false,
                gfm: false,
            },
        )
    }

    fn create_directory_server_impl(use_http: bool) -> (TestServer, TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        fs::write(temp_dir.path().join("test1.md"), TEST_FILE_1_CONTENT)
            .expect("Failed to write test1.md");
        fs::write(temp_dir.path().join("test2.markdown"), TEST_FILE_2_CONTENT)
            .expect("Failed to write test2.markdown");
        fs::write(temp_dir.path().join("test3.md"), TEST_FILE_3_CONTENT)
            .expect("Failed to write test3.md");

        let base_dir = temp_dir.path().to_path_buf();
        let tracked_files =
            scan_markdown_files(&base_dir, false, false, false).expect("Failed to scan markdown files");
        let is_directory_mode = true;

        let router = new_router(
            base_dir,
            tracked_files,
            is_directory_mode,
            false,
            DiagramOpts::default(),
            false,
            false,
        )
        .expect("Failed to create router");

        let server = if use_http {
            TestServer::builder()
                .http_transport()
                .build(router)
        } else {
            TestServer::new(router)
        };

        (server, temp_dir)
    }

    async fn create_directory_server() -> (TestServer, TempDir) {
        create_directory_server_impl(false)
    }

    async fn create_directory_server_with_http() -> (TestServer, TempDir) {
        create_directory_server_impl(true)
    }

    async fn create_recursive_directory_server() -> (TestServer, TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        fs::write(temp_dir.path().join("index.md"), TEST_FILE_1_CONTENT)
            .expect("Failed to write index.md");

        let sub = temp_dir.path().join("guide");
        fs::create_dir(&sub).expect("Failed to create guide dir");
        fs::write(sub.join("intro.md"), "# Intro\n\nNested content")
            .expect("Failed to write guide/intro.md");

        let base_dir = temp_dir.path().to_path_buf();
        let tracked_files = scan_markdown_files(&base_dir, true, false, false).expect("Failed to scan");
        let is_directory_mode = true;

        let router = new_router(
            base_dir,
            tracked_files,
            is_directory_mode,
            true,
            DiagramOpts::default(),
            false,
            false,
        )
        .expect("Failed to create router");
        let server = TestServer::new(router);

        (server, temp_dir)
    }

    #[tokio::test]
    async fn test_server_starts_and_serves_basic_markdown() {
        let (server, _temp_file) =
            create_test_server("# Hello World\n\nThis is **bold** text.").await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);

        assert!(content.contains("<h1>Hello World</h1>"));
        assert!(content.contains("<strong>bold</strong>"));
        assert!(body.contains("theme-toggle"));
        assert!(body.contains("--bg-color"));
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        let (server, _temp_file) = create_test_server_with_http("# WebSocket Test").await;

        let response = server.get_websocket("/ws").await;
        response.assert_status_switching_protocols();
    }

    #[tokio::test]
    async fn test_file_modification_updates_via_websocket() {
        let (server, temp_file) = create_test_server_with_http("# Original Content").await;

        let mut websocket = server.get_websocket("/ws").await.into_websocket().await;

        fs::write(&temp_file, "# Modified Content").expect("Failed to modify file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let update_result = tokio::time::timeout(
            Duration::from_secs(WEBSOCKET_TIMEOUT_SECS),
            websocket.receive_json::<ServerMessage>(),
        )
        .await;

        update_result.expect("Timeout waiting for WebSocket update after file modification");
    }

    #[tokio::test]
    async fn test_server_handles_gfm_features() {
        let markdown_content = r#"# GFM Test

## Table
| Name | Age |
|------|-----|
| John | 30  |
| Jane | 25  |

## Strikethrough
~~deleted text~~

## Code block
```rust
fn main() {
    println!("Hello!");
}
```
"#;

        let (server, _temp_file) = create_test_server(markdown_content).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);

        assert!(content.contains("<table>"));
        assert!(content.contains("<th>Name</th>"));
        assert!(content.contains("<td>John</td>"));
        assert!(content.contains("<del>deleted text</del>"));
        assert!(content.contains("<pre>"));
        assert!(content.contains("fn main()"));
    }

    #[tokio::test]
    async fn test_404_for_unknown_routes() {
        let (server, _temp_file) = create_test_server("# 404 Test").await;

        let response = server.get("/unknown-route").await;

        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    async fn test_image_serving() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        let md_content =
            "# Test with Image\n\n![Test Image](test.png)\n\nThis markdown references an image.";
        let md_path = temp_dir.path().join("test.md");
        fs::write(&md_path, md_content).expect("Failed to write markdown file");

        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0x0F, 0x00, 0x00, 0x01, 0x00, 0x01, 0x5C, 0xDD, 0x8D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let img_path = temp_dir.path().join("test.png");
        fs::write(&img_path, png_data).expect("Failed to write image file");

        let base_dir = temp_dir.path().to_path_buf();
        let tracked_files = vec![md_path];
        let is_directory_mode = false;
        let router = new_router(
            base_dir,
            tracked_files,
            is_directory_mode,
            false,
            DiagramOpts::default(),
            false,
            false,
        )
        .expect("Failed to create router");
        let server = TestServer::new(router);

        let response = server.get("/").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);
        assert!(content.contains("<img src=\"test.png\" alt=\"Test Image\""));

        let img_response = server.get("/test.png").await;
        assert_eq!(img_response.status_code(), 200);
        assert_eq!(img_response.header("content-type"), "image/png");
        assert!(!img_response.as_bytes().is_empty());
    }

    #[tokio::test]
    async fn test_non_image_files_not_served() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        let md_content = "# Test";
        let md_path = temp_dir.path().join("test.md");
        fs::write(&md_path, md_content).expect("Failed to write markdown file");

        let txt_path = temp_dir.path().join("secret.txt");
        fs::write(&txt_path, "secret content").expect("Failed to write txt file");

        let base_dir = temp_dir.path().to_path_buf();
        let tracked_files = vec![md_path];
        let is_directory_mode = false;
        let router = new_router(
            base_dir,
            tracked_files,
            is_directory_mode,
            false,
            DiagramOpts::default(),
            false,
            false,
        )
        .expect("Failed to create router");
        let server = TestServer::new(router);

        let response = server.get("/secret.txt").await;
        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    async fn test_html_tags_in_markdown_are_rendered() {
        let markdown_content = r#"# HTML Test

This markdown contains HTML tags:

<div class="highlight">
    <p>This should be rendered as HTML, not escaped</p>
    <span style="color: red;">Red text</span>
</div>

Regular **markdown** still works.
"#;

        let (server, _temp_file) = create_test_server(markdown_content).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);

        assert!(content.contains(r#"<div class="highlight">"#));
        assert!(content.contains(r#"<span style="color: red;">"#));
        assert!(content.contains("<p>This should be rendered as HTML, not escaped</p>"));
        assert!(!content.contains("&lt;div"));
        assert!(!content.contains("&gt;"));
        assert!(content.contains("<strong>markdown</strong>"));
    }

    #[tokio::test]
    async fn test_mermaid_diagram_detection_and_script_injection() {
        let markdown_content = r#"# Mermaid Test

Regular content here.

```mermaid
graph TD
    A[Start] --> B{Decision}
    B -->|Yes| C[End]
    B -->|No| D[Continue]
```

More regular content.

```javascript
// This is a regular code block, not mermaid
console.log("Hello World");
```
"#;

        let (server, _temp_file) = create_test_server_with_mermaid(markdown_content).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);
        let data = extract_mdrv_data(&body);

        assert!(content.contains(r#"class="language-mermaid""#));
        assert!(content.contains("graph TD"));

        let has_raw_content = content.contains("A[Start] --> B{Decision}");
        let has_encoded_content = content.contains("A[Start] --&gt; B{Decision}");
        assert!(
            has_raw_content || has_encoded_content,
            "Expected mermaid content not found"
        );

        assert_eq!(data["mermaidEnabled"], true);
        assert!(content.contains(r#"class="language-javascript""#));
        assert!(content.contains("console.log"));
    }

    #[tokio::test]
    async fn test_no_mermaid_script_injection_without_mermaid_blocks() {
        let markdown_content = r#"# No Mermaid Test

This content has no mermaid diagrams.

```javascript
console.log("Hello World");
```

```bash
echo "Regular code block"
```

Just regular markdown content.
"#;

        let (server, _temp_file) = create_test_server_with_mermaid(markdown_content).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);
        let data = extract_mdrv_data(&body);

        assert_eq!(data["mermaidEnabled"], false);
        assert!(content.contains(r#"class="language-javascript""#));
        assert!(content.contains(r#"class="language-bash""#));
    }

    #[tokio::test]
    async fn test_multiple_mermaid_diagrams() {
        let markdown_content = r#"# Multiple Mermaid Diagrams

## Flowchart
```mermaid
graph LR
    A --> B
```

## Sequence Diagram
```mermaid
sequenceDiagram
    Alice->>Bob: Hello
    Bob-->>Alice: Hi
```

## Class Diagram
```mermaid
classDiagram
    Animal <|-- Duck
```
"#;

        let (server, _temp_file) = create_test_server_with_mermaid(markdown_content).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);
        let data = extract_mdrv_data(&body);

        let mermaid_occurrences = content.matches(r#"class="language-mermaid""#).count();
        assert_eq!(mermaid_occurrences, 3);

        assert!(content.contains("graph LR"));
        assert!(content.contains("sequenceDiagram"));
        assert!(content.contains("classDiagram"));

        assert!(content.contains("A --&gt; B") || content.contains("A --> B"));
        assert!(content.contains("Alice-&gt;&gt;Bob") || content.contains("Alice->>Bob"));
        assert!(content.contains("Animal &lt;|-- Duck") || content.contains("Animal <|-- Duck"));

        assert_eq!(data["mermaidEnabled"], true);
    }

    #[tokio::test]
    async fn test_diagrams_disabled_by_default() {
        // Without --with-mermaid, a mermaid block must not trigger the flag.
        let (server, _temp_file) = create_test_server("# H\n\n```mermaid\nA --> B\n```\n").await;
        let body = server.get("/").await.text();
        assert_eq!(extract_mdrv_data(&body)["mermaidEnabled"], false);
    }

    #[test]
    fn test_gfm_alert_renders_callout() {
        let md = "> [!NOTE]\n> This is a note.\n";
        let html = MarkdownState::markdown_to_html(
            md,
            DiagramOpts { gfm: true, ..Default::default() },
        )
        .unwrap();
        assert!(
            html.contains("markdown-alert-note"),
            "note callout missing: {html}"
        );
        assert!(
            html.contains("markdown-alert-title"),
            "title missing: {html}"
        );
        assert!(html.contains("This is a note."), "body missing: {html}");
    }

    #[test]
    fn test_gfm_alert_leaves_plain_blockquote() {
        let md = "> Just a regular quote.\n";
        let html = MarkdownState::markdown_to_html(
            md,
            DiagramOpts { gfm: true, ..Default::default() },
        )
        .unwrap();
        assert!(html.contains("<blockquote>"));
        assert!(
            !html.contains("markdown-alert"),
            "plain blockquote became an alert: {html}"
        );
    }

    #[tokio::test]
    async fn test_d2_block_degrades_gracefully_without_binary() {
        // A d2 block renders either as inline SVG (d2 installed) or as a plain
        // code block (d2 absent). Both outcomes are acceptable.
        let markdown_content = "# D2 Test\n\n```d2\nx -> y\n```\n";
        let (server, _temp_file) = create_test_server_impl(
            markdown_content,
            false,
            DiagramOpts {
                mermaid: false,
                d2: true,
                latex: false,
                typst: false,
                gfm: false,
            },
        );

        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("<svg") || content.contains(r#"class="language-d2""#),
            "d2 block should produce SVG or fall back to a code block"
        );
    }

    #[tokio::test]
    async fn test_typst_block_renders_svg_or_degrades() {
        // A ```typst block renders either as inline SVG (typst installed) or as
        // a plain code block (typst absent). Both outcomes are acceptable.
        let markdown_content = "# Typst Test\n\n```typst\nHello.\n```\n";
        let (server, _temp_file) = create_test_server_impl(
            markdown_content,
            false,
            DiagramOpts {
                typst: true,
                ..Default::default()
            },
        );

        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("<svg") || content.contains(r#"class="language-typst""#),
            "typst block should produce SVG or fall back to a code block"
        );
    }

    #[tokio::test]
    async fn test_typst_document_renders_svg_or_degrades() {
        // A `.typ` file served via --include-typst renders either as inlined
        // SVG page(s) (typst installed) or as a plain source listing (typst
        // absent). Both outcomes are acceptable.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let typ_content = "#set page(width: 200pt)\nHello from Typst.\n";
        fs::write(temp_dir.path().join("doc.typ"), typ_content)
            .expect("Failed to write doc.typ");

        let base_dir = temp_dir.path().to_path_buf();
        let tracked_files = scan_markdown_files(&base_dir, false, false, true)
            .expect("Failed to scan");
        assert_eq!(
            tracked_files.len(),
            1,
            ".typ file should be tracked with --include-typst"
        );

        let router = new_router(
            base_dir,
            tracked_files,
            true,
            false,
            DiagramOpts::default(),
            false,
            true,
        )
        .expect("Failed to create router");
        let server = TestServer::new(router);

        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("<svg") || content.contains(r#"class="language-typst""#),
            "typst document should produce SVG pages or fall back to a source listing"
        );
    }

    #[test]
    fn test_typst_document_freeflow_renders_or_degrades() {
        // Free-flow uses typst's HTML export. When the feature is available
        // the output is reflowable HTML wrapped in `.typst-doc-freeflow`; when it
        // is unavailable (or compilation fails) the result is empty and the
        // frontend disables the toggle. Both are valid.
        let out = render_typst_document_freeflow("#set page(width: 200pt)\nFlow.\n")
            .expect("freeflow render should not error");
        assert!(
            out.contains(r#"typst-doc-freeflow"#) || out.is_empty(),
            "freeflow output should be themed HTML or empty when unavailable"
        );
    }

    #[tokio::test]
    async fn test_typst_document_exposes_freeflow_in_data() {
        // A served `.typ` file must signal `isTypst` and expose the
        // `contentFreeflow` field (populated when HTML export is available).
        let temp_dir = tempdir().expect("Failed to create temp dir");
        fs::write(temp_dir.path().join("doc.typ"), "Hello Typst.\n")
            .expect("Failed to write doc.typ");
        let base_dir = temp_dir.path().to_path_buf();
        let tracked_files = scan_markdown_files(&base_dir, false, false, true)
            .expect("Failed to scan");
        let router = new_router(
            base_dir,
            tracked_files,
            true,
            false,
            DiagramOpts::default(),
            false,
            true,
        )
        .expect("Failed to create router");
        let server = TestServer::new(router);

        let data = extract_mdrv_data(&server.get("/").await.text());
        assert_eq!(data["isTypst"], true, ".typ file should set isTypst");
        let freeflow = data["contentFreeflow"].as_str().unwrap_or("");
        if typst_html_available() {
            assert!(
                !freeflow.is_empty(),
                "contentFreeflow should be populated when typst HTML export is available"
            );
        } else {
            assert!(
                freeflow.is_empty(),
                "contentFreeflow should be empty when typst HTML export is unavailable"
            );
        }
    }

    #[tokio::test]
    async fn test_latex_inline_renders_svg() {
        let markdown_content = "Inline math: $x^2 + y^2$\n";
        let (server, _temp_file) = create_test_server_impl(
            markdown_content,
            false,
            DiagramOpts {
                mermaid: false,
                d2: false,
                latex: true,
                typst: false,
                gfm: false,
            },
        );

        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("<svg"),
            "inline math should produce an SVG"
        );
        assert!(
            content.contains("latex-inline"),
            "inline math should be wrapped in a latex-inline span"
        );
    }

    #[tokio::test]
    async fn test_latex_display_renders_svg() {
        let markdown_content = "Block math:\n\n$$\n\\sum_{i=1}^n i = \\frac{n(n+1)}{2}
$$\n";
        let (server, _temp_file) = create_test_server_impl(
            markdown_content,
            false,
            DiagramOpts {
                mermaid: false,
                d2: false,
                latex: true,
                typst: false,
                gfm: false,
            },
        );

        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("<svg"),
            "display math should produce an SVG"
        );
        assert!(
            content.contains("latex-display"),
            "display math should be wrapped in a latex-display div"
        );
    }

    #[tokio::test]
    async fn test_latex_not_rendered_without_flag() {
        // Without --with-latex, $...$ delimiters appear as literal text.
        let markdown_content = "Price is $5 and $10 each\n";
        let (server, _temp_file) = create_test_server(markdown_content).await;

        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            !content.contains("latex-inline"),
            "no latex-inline should appear without --with-latex"
        );
        assert!(
            !content.contains("<svg"),
            "no SVG should appear without --with-latex"
        );
    }

    fn create_html_test_server(content: &str, opts: DiagramOpts) -> (TestServer, NamedTempFile) {
        let temp_file = Builder::new()
            .suffix(".html")
            .tempfile()
            .expect("Failed to create temp file");
        fs::write(&temp_file, content).expect("Failed to write temp file");

        let canonical_path = temp_file
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_file.path().to_path_buf());

        let base_dir = canonical_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        let tracked_files = vec![canonical_path];

        let router = new_router(base_dir, tracked_files, false, false, opts, true, false)
            .expect("Failed to create router");
        let server = TestServer::new(router);
        (server, temp_file)
    }

    #[tokio::test]
    async fn test_html_file_served_with_content() {
        let html = "<h1>Hello HTML</h1><p>Raw HTML file</p>";
        let (server, _temp) = create_html_test_server(html, DiagramOpts::default());
        // .html single-file: the filename is the temp file's name
        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("<h1>Hello HTML</h1>"),
            "raw HTML should be served: {content}"
        );
    }

    #[tokio::test]
    async fn test_html_file_latex_vjudge_inline() {
        let html = "<p>Width <span class='math math-inline'>$ n \\times n $</span></p>";
        let opts = DiagramOpts { latex: true, ..Default::default() };
        let (server, _temp) = create_html_test_server(html, opts);
        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("latex-inline"),
            "VJudge inline math should render: {content}"
        );
        assert!(
            content.contains("<svg"),
            "SVG should be generated: {content}"
        );
    }

    #[tokio::test]
    async fn test_html_file_latex_multiline_class() {
        // VJudge sometimes splits the class attribute across multiple lines
        let html = "<span class='\n  math\n  math-inline\n'>$ x $</span>";
        let opts = DiagramOpts { latex: true, ..Default::default() };
        let (server, _temp) = create_html_test_server(html, opts);
        let body = server.get("/").await.text();
        let content = extract_content(&body);
        assert!(
            content.contains("latex-inline"),
            "multi-line class math should render: {content}"
        );
    }


    #[tokio::test]
    async fn test_mermaid_js_etag_caching() {
        let (server, _temp_file) = create_test_server("# Test").await;

        let response = server.get("/mermaid.min.js").await;
        assert_eq!(response.status_code(), 200);

        let etag = response.header("etag");
        assert!(!etag.is_empty(), "ETag header should be present");

        let cache_control = response.header("cache-control");
        let cache_control_str = cache_control.to_str().unwrap();
        assert!(cache_control_str.contains("public"));
        assert!(cache_control_str.contains("no-cache"));

        let content_type = response.header("content-type");
        assert_eq!(content_type, "application/javascript");

        assert!(!response.as_bytes().is_empty());

        let response_304 = server
            .get("/mermaid.min.js")
            .add_header(
                axum::http::header::IF_NONE_MATCH,
                axum::http::HeaderValue::from_str(etag.to_str().unwrap()).unwrap(),
            )
            .await;

        assert_eq!(response_304.status_code(), 304);
        assert_eq!(response_304.header("etag"), etag);
        assert!(response_304.as_bytes().is_empty());

        let response_200 = server
            .get("/mermaid.min.js")
            .add_header(
                axum::http::header::IF_NONE_MATCH,
                axum::http::HeaderValue::from_static("\"different-etag\""),
            )
            .await;

        assert_eq!(response_200.status_code(), 200);
        assert!(!response_200.as_bytes().is_empty());
    }

    #[tokio::test]
    async fn test_directory_mode_serves_multiple_files() {
        let (server, _temp_dir) = create_directory_server().await;

        let response1 = server.get("/test1.md").await;
        assert_eq!(response1.status_code(), 200);
        let body1 = response1.text();
        let content1 = extract_content(&body1);
        assert!(content1.contains("<h1>Test 1</h1>"));
        assert!(content1.contains("Content of test1"));

        let response2 = server.get("/test2.markdown").await;
        assert_eq!(response2.status_code(), 200);
        let body2 = response2.text();
        let content2 = extract_content(&body2);
        assert!(content2.contains("<h1>Test 2</h1>"));
        assert!(content2.contains("Content of test2"));

        let response3 = server.get("/test3.md").await;
        assert_eq!(response3.status_code(), 200);
        let body3 = response3.text();
        let content3 = extract_content(&body3);
        assert!(content3.contains("<h1>Test 3</h1>"));
        assert!(content3.contains("Content of test3"));
    }

    #[tokio::test]
    async fn test_directory_mode_file_not_found() {
        let (server, _temp_dir) = create_directory_server().await;

        let response = server.get("/nonexistent.md").await;
        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    async fn test_directory_mode_has_navigation_sidebar() {
        let (server, _temp_dir) = create_directory_server().await;

        let response = server.get("/test1.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();

        assert!(body.contains(r#"<nav class="sidebar">"#));
        assert!(body.contains(r#"<ul class="file-list">"#));
        assert!(body.contains("test1.md"));
        assert!(body.contains("test2.markdown"));
        assert!(body.contains("test3.md"));
    }

    #[tokio::test]
    async fn test_single_file_mode_no_navigation_sidebar() {
        let (server, _temp_file) = create_test_server("# Single File Test").await;

        let response = server.get("/").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let data = extract_mdrv_data(&body);

        assert_eq!(data["showNavigation"], false);
        assert_eq!(data["navItems"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_directory_mode_active_file_highlighting() {
        let (server, _temp_dir) = create_directory_server().await;

        let response1 = server.get("/test1.md").await;
        assert_eq!(response1.status_code(), 200);
        let body1 = response1.text();
        let nav1 = extract_nav_json(&body1);

        assert!(nav1.contains("test1.md"), "test1.md should be in nav");

        let response2 = server.get("/test2.markdown").await;
        assert_eq!(response2.status_code(), 200);
        let body2 = response2.text();
        let nav2 = extract_nav_json(&body2);

        assert!(nav2.contains("test2.markdown"), "test2.markdown should be in nav");
    }

    #[tokio::test]
    async fn test_directory_mode_file_order() {
        let (server, _temp_dir) = create_directory_server().await;

        let response = server.get("/test1.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();

        let test1_pos = body.find("test1.md").expect("test1.md not found");
        let test2_pos = body
            .find("test2.markdown")
            .expect("test2.markdown not found");
        let test3_pos = body.find("test3.md").expect("test3.md not found");

        assert!(
            test1_pos < test2_pos,
            "test1.md should appear before test2.markdown"
        );
        assert!(
            test2_pos < test3_pos,
            "test2.markdown should appear before test3.md"
        );
    }

    #[tokio::test]
    async fn test_recursive_mode_serves_nested_file() {
        let (server, _temp_dir) = create_recursive_directory_server().await;

        let response = server.get("/guide/intro.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);
        assert!(content.contains("<h1>Intro</h1>"));
        assert!(content.contains("Nested content"));
    }

    #[tokio::test]
    async fn test_recursive_mode_root_file_still_served() {
        let (server, _temp_dir) = create_recursive_directory_server().await;

        let response = server.get("/index.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);
        assert!(content.contains("<h1>Test 1</h1>"));
    }

    #[tokio::test]
    async fn test_recursive_mode_sidebar_shows_tree() {
        let (server, _temp_dir) = create_recursive_directory_server().await;

        let response = server.get("/index.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let nav = extract_nav_json(&body);

        assert!(
            nav.contains("\"type\":\"dir\""),
            "nav should have directory groups"
        );
        assert!(nav.contains("guide"), "nav should contain guide dir");
        assert!(nav.contains("intro.md"), "nav should contain intro.md");
        assert!(nav.contains("guide/intro.md"), "nav should have guide/intro.md path");
        assert!(nav.contains("index.md"), "nav should have index.md path");
    }

    #[tokio::test]
    async fn test_recursive_mode_active_highlight_nested() {
        let (server, _temp_dir) = create_recursive_directory_server().await;

        let response = server.get("/guide/intro.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let nav = extract_nav_json(&body);

        assert!(nav.contains("guide/intro.md"), "guide/intro.md should be in nav");
    }

    #[tokio::test]
    async fn test_directory_mode_websocket_file_modification() {
        let (server, temp_dir) = create_directory_server_with_http().await;

        let mut websocket = server.get_websocket("/ws").await.into_websocket().await;

        let test_file = temp_dir.path().join("test1.md");
        fs::write(&test_file, "# Modified Test 1\n\nContent has changed")
            .expect("Failed to modify file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let update_result = tokio::time::timeout(
            Duration::from_secs(WEBSOCKET_TIMEOUT_SECS),
            websocket.receive_json::<ServerMessage>(),
        )
        .await;

        update_result.expect("Timeout waiting for WebSocket update after file modification");
    }

    #[tokio::test]
    async fn test_directory_mode_new_file_triggers_reload() {
        let (server, temp_dir) = create_directory_server_with_http().await;

        let mut websocket = server.get_websocket("/ws").await.into_websocket().await;

        let new_file = temp_dir.path().join("test4.md");
        fs::write(&new_file, "# Test 4\n\nThis is a new file").expect("Failed to create new file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let update_result = tokio::time::timeout(
            Duration::from_secs(WEBSOCKET_TIMEOUT_SECS),
            websocket.receive_json::<ServerMessage>(),
        )
        .await;

        update_result.expect("Timeout waiting for WebSocket update after new file creation");

        let response = server.get("/test1.md").await;
        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let nav = extract_nav_json(&body);

        assert!(
            nav.contains("test4.md"),
            "New file should appear in navigation"
        );

        let new_file_response = server.get("/test4.md").await;
        assert_eq!(new_file_response.status_code(), 200);
        let new_file_body = new_file_response.text();
        let new_content = extract_content(&new_file_body);
        assert!(new_content.contains("<h1>Test 4</h1>"));
        assert!(new_content.contains("This is a new file"));
    }

    #[tokio::test]
    async fn test_editor_save_simulation_single_file_mode() {
        let (server, temp_file) =
            create_test_server_with_http("# Original\n\nOriginal content").await;

        let file_path = temp_file.path().to_path_buf();
        let backup_path = file_path.with_extension("md~");

        let initial_response = server.get("/").await;
        assert_eq!(initial_response.status_code(), 200);
        assert!(initial_response.text().contains("Original content"));

        fs::rename(&file_path, &backup_path).expect("Failed to rename to backup");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let during_save_response = server.get("/").await;
        assert_eq!(
            during_save_response.status_code(),
            200,
            "File should not return 404 during editor save"
        );

        fs::write(&file_path, "# Updated\n\nUpdated content").expect("Failed to write new file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let final_response = server.get("/").await;
        assert_eq!(final_response.status_code(), 200);
        let final_body = final_response.text();
        assert!(
            final_body.contains("Updated content"),
            "Should serve updated content after save"
        );
        assert!(
            !final_body.contains("Original content"),
            "Should not serve old content"
        );

        let _ = fs::remove_file(&backup_path);
    }

    #[tokio::test]
    async fn test_editor_save_simulation_directory_mode() {
        let (server, temp_dir) = create_directory_server_with_http().await;

        let file_path = temp_dir.path().join("test1.md");
        let backup_path = temp_dir.path().join("test1.md~");

        let initial_response = server.get("/test1.md").await;
        assert_eq!(initial_response.status_code(), 200);
        assert!(initial_response.text().contains("Content of test1"));

        fs::rename(&file_path, &backup_path).expect("Failed to rename to backup");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let during_save_response = server.get("/test1.md").await;
        assert_eq!(
            during_save_response.status_code(),
            200,
            "File should not return 404 during editor save in directory mode"
        );

        fs::write(&file_path, "# Test 1 Updated\n\nUpdated content")
            .expect("Failed to write new file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let final_response = server.get("/test1.md").await;
        assert_eq!(final_response.status_code(), 200);
        let final_body = final_response.text();
        assert!(
            final_body.contains("Updated content"),
            "Should serve updated content after save"
        );

        let _ = fs::remove_file(&backup_path);
    }

    #[tokio::test]
    async fn test_no_404_during_editor_save_sequence() {
        let (server, temp_dir) = create_directory_server_with_http().await;
        let mut websocket = server.get_websocket("/ws").await.into_websocket().await;

        let file_path = temp_dir.path().join("test1.md");
        let backup_path = temp_dir.path().join("test1.md~");

        fs::rename(&file_path, &backup_path).expect("Failed to rename to backup");
        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let response_after_rename = server.get("/test1.md").await;
        assert_eq!(
            response_after_rename.status_code(),
            200,
            "Should not get 404 after rename to backup"
        );

        fs::write(&file_path, "# Test 1 Updated\n\nNew content").expect("Failed to write new file");
        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let response_after_create = server.get("/test1.md").await;
        assert_eq!(
            response_after_create.status_code(),
            200,
            "Should successfully serve after new file created"
        );
        assert!(response_after_create.text().contains("New content"));

        let update_result = tokio::time::timeout(
            Duration::from_secs(WEBSOCKET_TIMEOUT_SECS),
            websocket.receive_json::<ServerMessage>(),
        )
        .await;

        assert!(update_result.is_ok(), "Should receive reload after save");

        let _ = fs::remove_file(&backup_path);
    }

    #[tokio::test]
    async fn test_yaml_frontmatter_is_stripped() {
        let (server, _temp_file) = create_test_server(YAML_FRONTMATTER_CONTENT).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);

        assert!(!content.contains("title: Test Post"));
        assert!(!content.contains("author: Name"));
        assert!(content.contains("<h1>Test Post</h1>"));
    }

    #[tokio::test]
    async fn test_toml_frontmatter_is_stripped() {
        let (server, _temp_file) = create_test_server(TOML_FRONTMATTER_CONTENT).await;

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
        let body = response.text();
        let content = extract_content(&body);

        assert!(!content.contains("title = \"Test Post\""));
        assert!(content.contains("<h1>Test Post</h1>"));
    }

    #[tokio::test]
    async fn test_temp_file_rename_triggers_reload_single_file_mode() {
        let (server, temp_file) =
            create_test_server_with_http("# Original\n\nOriginal content").await;

        let mut websocket = server.get_websocket("/ws").await.into_websocket().await;

        let file_path = temp_file.path().to_path_buf();
        let temp_write_path = file_path.with_extension("md.tmp.12345");

        let initial_response = server.get("/").await;
        assert_eq!(initial_response.status_code(), 200);
        assert!(
            initial_response.text().contains("Original content"),
            "File should be tracked and serving content before edit"
        );

        fs::write(
            &temp_write_path,
            "# Updated\n\nUpdated content via temp file",
        )
        .expect("Failed to write temp file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        fs::rename(&temp_write_path, &file_path).expect("Failed to rename temp file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let update_result = tokio::time::timeout(
            Duration::from_secs(WEBSOCKET_TIMEOUT_SECS),
            websocket.receive_json::<ServerMessage>(),
        )
        .await;

        update_result.expect("Timeout waiting for WebSocket update after temp file rename");

        let final_response = server.get("/").await;
        assert_eq!(final_response.status_code(), 200);
        let final_body = final_response.text();
        assert!(
            final_body.contains("Updated content via temp file"),
            "Should serve updated content after temp file rename"
        );
        assert!(
            !final_body.contains("Original content"),
            "Should not serve old content"
        );
    }

    #[tokio::test]
    async fn test_temp_file_rename_triggers_reload_directory_mode() {
        let (server, temp_dir) = create_directory_server_with_http().await;

        let mut websocket = server.get_websocket("/ws").await.into_websocket().await;

        let file_path = temp_dir.path().join("test1.md");
        let temp_write_path = temp_dir.path().join("test1.md.tmp.67890");

        let initial_response = server.get("/test1.md").await;
        assert_eq!(initial_response.status_code(), 200);
        assert!(
            initial_response.text().contains("Content of test1"),
            "File should be tracked and serving content before edit"
        );

        fs::write(
            &temp_write_path,
            "# Test 1 Updated\n\nUpdated via temp file rename",
        )
        .expect("Failed to write temp file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        fs::rename(&temp_write_path, &file_path).expect("Failed to rename temp file");

        tokio::time::sleep(Duration::from_millis(FILE_WATCH_DELAY_MS)).await;

        let update_result = tokio::time::timeout(
            Duration::from_secs(WEBSOCKET_TIMEOUT_SECS),
            websocket.receive_json::<ServerMessage>(),
        )
        .await;

        update_result.expect(
            "Timeout waiting for WebSocket update after temp file rename in directory mode",
        );

        let final_response = server.get("/test1.md").await;
        assert_eq!(final_response.status_code(), 200);
        let final_body = final_response.text();
        assert!(
            final_body.contains("Updated via temp file rename"),
            "Should serve updated content after temp file rename"
        );
        assert!(
            !final_body.contains("Content of test1"),
            "Should not serve old content"
        );
    }
}
