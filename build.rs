fn main() {
    let dist = std::path::Path::new("frontend/dist/index.html");
    if !dist.exists() {
        panic!(
            "frontend/dist/index.html not found.\n\
             Build the frontend first:\n  just build-frontend\n\
             Or manually:\n  cd frontend && bun install && bun run build"
        );
    }
    println!("cargo:rerun-if-changed=frontend/dist/index.html");
}
