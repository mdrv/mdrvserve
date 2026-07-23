use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod app;

use app::{scan_markdown_files, serve_markdown, DiagramOpts};

#[derive(Parser)]
#[command(name = "mdrvserve")]
#[command(about = "A simple HTTP server for markdown preview")]
#[command(version)]
struct Args {
    /// Path to markdown file or directory to serve
    path: PathBuf,

    /// Hostname (domain or IP address) to listen on
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    hostname: String,

    /// Port to serve on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Open the preview in the default browser
    #[arg(short, long)]
    open: bool,

    /// Watch subdirectories recursively (directory mode only)
    #[arg(short, long)]
    recursive: bool,

    /// Render Mermaid diagrams client-side (lazy-loads the bundled JS)
    #[arg(long = "with-mermaid")]
    with_mermaid: bool,

    /// Render D2 diagrams server-side via the `d2` binary (inlines SVG)
    #[arg(long = "with-d2")]
    with_d2: bool,

    /// Render LaTeX math (inline `$...$` and block `$$...$$`) to SVG via RaTeX
    #[arg(long = "with-latex")]
    with_latex: bool,

    /// Render typst fenced blocks server-side via the `typst` binary (inlines SVG)
    #[arg(long = "with-typst")]
    with_typst: bool,

    /// Also serve `.html`/`.htm` files alongside markdown (directory mode)
    #[arg(long = "include-html")]
    include_html: bool,

    /// Also serve `.typ` files alongside markdown, compiled to SVG via `typst` (directory mode)
    #[arg(long = "include-typst")]
    include_typst: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let absolute_path = args.path.canonicalize().unwrap_or(args.path);

    let (base_dir, tracked_files, is_directory_mode) = if absolute_path.is_file() {
        // Single-file mode: derive parent directory
        let base_dir = absolute_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        let tracked_files = vec![absolute_path];
        (base_dir, tracked_files, false)
    } else if absolute_path.is_dir() {
        // Directory mode: scan directory for markdown files
        let tracked_files = scan_markdown_files(&absolute_path, args.recursive, args.include_html, args.include_typst)?;
        if tracked_files.is_empty() {
            anyhow::bail!("No markdown files found in directory");
        }
        (absolute_path, tracked_files, true)
    } else {
        anyhow::bail!("Path must be a file or directory");
    };

    // Single unified serve function
    serve_markdown(
        base_dir,
        tracked_files,
        is_directory_mode,
        args.recursive,
        args.hostname,
        args.port,
        args.open,
        DiagramOpts {
            mermaid: args.with_mermaid,
            d2: args.with_d2,
            latex: args.with_latex,
            typst: args.with_typst,
        },
        args.include_html,
        args.include_typst,
    )
    .await?;

    Ok(())
}
