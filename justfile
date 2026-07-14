# Prefer bun, fall back to npm
pkgr := `command -v bun >/dev/null 2>&1 && printf bun || printf npm`

# Show available recipes
default:
    @just --list

# Install frontend dependencies
install:
    cd frontend && {{pkgr}} install

# Build frontend assets → frontend/dist/index.html (single inlined file)
build-frontend:
    cd frontend && {{pkgr}} install && {{pkgr}} run build

# Build the Rust binary (requires frontend/dist/index.html to exist)
build:
    cargo build --release

# Build everything: frontend then Rust
build-all: build-frontend
    cargo build --release

# Build for CI with a specific target
build-ci target: build-frontend
    cargo build --release --target {{target}}

# Type-check Svelte components
check-frontend:
    cd frontend && {{pkgr}} run check

# Run the server
dev *args:
    cargo run -- {{args}}

# Run Rust tests
test:
    cargo test

# Clean everything
clean:
    rm -rf frontend/dist frontend/node_modules
    cargo clean
