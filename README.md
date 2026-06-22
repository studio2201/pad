# RustPad

A stupid simple, no auth (unless you want it!), modern collaborative notepad application with auto-save functionality, fuzzy search, and multi-theme support. Built with Rust (Axum/Tokio backend and Yew/Trunk WebAssembly frontend).

---

## Overview

RustPad is a lightweight, self-hosted web application that provides real-time collaborative notepad editing. It is engineered from the ground up for minimal resource usage, zero external JS library bloat, and maximum load speeds. It utilizes Operational Transformation (OT) for conflict-free concurrent editing and native browser APIs (such as LocalStorage and DOM-based sanitization) to guarantee safety and high performance.

---

## Features

*   🤝 **Real-Time Collaboration**: Concurrent editing synchronization using Operational Transformation (OT) over WebSockets with peer cursor position tracking.
*   📶 **Robust Connection Hook**: Exponential back-off reconnection loop in Yew that queues offline operations and replays them upon recovery.
*   🎨 **Rich Theme Customization**: Pre-built Dracula, Sepia, Nord, Light, and Dark modes with instant matching SVG iconography toggles.
*   🔒 **Optional Access PIN**: Secure pads using a 4-10 digit PIN with IP-based rate-limiting lockout protection to prevent brute-force attacks.
*   🔍 **Fuzzy In-Memory Search**: Search titles and text content using an optimal zero-allocation subsequence scoring algorithm.
*   ⌨️ **Keyboard Accessibility**: Fast shortcuts (e.g. `Ctrl+K` for search, `?` for help) to navigate the editor completely keyboard-only.
*   🗺️ **Multi-Language Header Dropdown**: Switch between 8 primary internet developer languages (English, Chinese, Spanish, German, Japanese, French, Portuguese, Russian) straight from the header.

---

## Prerequisites & Environment Variables

### System Requirements
*   **Operating System**: Linux, macOS, or Windows (via WSL recommended for local build)
*   **Rust Toolchain**: Stable `rustc` and `cargo` (v1.75+)
*   **WebAssembly Toolchain**: `wasm32-unknown-unknown` target component
*   **Asset Bundler**: `trunk` CLI (v0.18+)

### Configuration Environment Variables
Create a `.env` file in the root directory to configure the application runtime:

| Variable | Description | Default | Environment |
| :--- | :--- | :--- | :--- |
| `PORT` | The port address the backend web server listens on | `4402` | All |
| `BASE_URL` | Application base URL (must start with http/https) | `http://localhost:4402` | All |
| `RUSTPAD_PIN` | Optional 4-10 digit authentication PIN (digit-only) | None | All |
| `SITE_TITLE` | The title shown in the browser and PWA manifest | `RustPad` | All |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout | `5` | Production/Dev |
| `LOCKOUT_TIME` | Bruteforce lockout duration in minutes | `15` | Production/Dev |
| `TRUST_PROXY` | Set true if deploying behind reverse proxy (Nginx, Cloudflare) | `false` | Production |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted proxy CIDRs/IPs | None | Production |
 
---
 
## Quick Start
 
Get RustPad up and running locally in under 2 minutes:
 
```bash
# 1. Clone the repository
git clone https://github.com/UberMetroid/RustPad.git
cd RustPad
 
# 2. Add WASM target and install Trunk
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
 
# 3. Build static frontend assets
cd frontend && trunk build --release && cd ..
 
# 4. Compile and start the backend server
cargo run --release
```
 
Open your browser to `http://localhost:4402` and start editing!
 
---
 
## Docker & Docker Compose Configurations
 
### Standalone Docker Run
To deploy the precompiled container image in one command:
```bash
docker run -d \
  -p 4402:4402 \
  -v ./data:/app/data \
  -e SITE_TITLE="My Notepad" \
  -e RUSTPAD_PIN="1234" \
  ghcr.io/ubermetroid/rustpad:latest
```
 
### Docker Compose
Create a `docker-compose.yml` file for persistent deployment:
```yaml
version: '3.8'
 
services:
  rustpad:
    image: ghcr.io/ubermetroid/rustpad:latest
    container_name: rustpad
    restart: unless-stopped
    ports:
      - "4402:4402"
    volumes:
      - ./data:/app/data
    environment:
      SITE_TITLE: "CompanyPad"
      RUSTPAD_PIN: "5678"
      BASE_URL: "http://localhost:4402"
      TRUST_PROXY: "false"
Run `docker compose up -d` to launch the services.

### Nix Layered Container Building (Alternative)

For maximum isolation, reproducibility, and minimal footprints (no terminal tools, no shell, running strictly as `USER nobody`), you can compile and package the server using the provided Nix flake:

```bash
# 1. Build the layered Docker image tarball via Nix flake
nix build .#dockerImage

# 2. Load the resulting tarball image directly into Docker
docker load < result

# 3. Execute the Nix-built container
docker run -d \
  --name rustpad-nix \
  -p 4402:4402 \
  -v ./data:/app/data \
  -e RUSTPAD_PIN=1234 \
  rustpad-nix:latest
```

---

## Technical Details

*   **Type-Safe Core**: Written in 100% safe Rust. No unsafe memory allocations or compiler warnings are present.
*   **Real-Time Sync**: Implements operational transformation (OT) mapping client document version sequences with server ACKs.
*   **Memory Efficiency**: Utilizes string caching of search keys during notepad creation to bypass heap-allocation bottlenecks during active search loops.
*   **Security Integrity**: Implements browser-native DOM-based HTML sanitization on parsed markdown outputs, completely neutralizing XSS risks.
*   **Fast Loading**: Front-end WASM package is optimized for size, compiling down to a 612KB package (236KB gzipped transfer payload).

---

## File Tree

```
RustPad/
├── Cargo.toml          # Workspace root manifest
├── Dockerfile          # Multi-stage optimized Docker builder
├── .github/            # GitHub Actions integration
│   └── workflows/
│       ├── ci.yml      # CI lint/compile check pipeline
│       └── docker-publish.yml # CD container publisher
├── data/               # Persistent text files and index databases
├── backend/            # Backend Crate (Axum API)
│   ├── Cargo.toml      # Backend dependency manifest
│   └── src/            # Backend Rust Axum source files
│       ├── main.rs     # Server initialization, config parsing, file watcher
│       ├── state.rs    # AppState and rate-limiting mappings
│       ├── search.rs   # In-memory search cache and sequence matching
│       ├── utils.rs    # IP extractor and cryptographic hashing
│       ├── ws.rs       # WebSocket communication handler
│       ├── migration.rs# Schema layout migrations
│       ├── tests.rs    # Backend unit test suite
│       └── routes/     # Router controllers
│           ├── mod.rs  # REST endpoint mappings and PWA manifest builder
│           ├── auth.rs # PIN authentication and session validations
│           ├── notepads_crud.rs # Notepad metadata routes
│           ├── notepads_io.rs   # Note saving, deleting, and loading IO
│           └── pages.rs# Static html page fallback templates
└── frontend/           # Frontend Rust Yew source files
    ├── Cargo.toml      # Frontend dependency manifest
    ├── index.html      # Trunk template HTML entrypoint
    ├── Assets/         # Static assets and stylesheets
    │   ├── app.css     # Main editor styling
    │   ├── base.css    # Variables and CSS resets
    │   ├── header.css  # Toolbar header layout styling
    │   ├── login.css   # PIN authentication panel layout styling
    │   ├── preview-styles.css # Markdown preview styling
    │   └── service-worker.js # Offline service worker caching assets
    └── src/            # Yew components
        ├── main.rs     # App entrypoint and locale mounting
        ├── app.rs      # Main coordinator Yew component
        ├── editor.rs   # Core textarea notepad workspace
        ├── login.rs    # PIN code entry panel
        ├── services.rs # API REST HTTP client
        ├── collab.rs   # WS sync loop hook
        ├── collab_utils.rs # Peer cursor calculation JS interface
        ├── header.rs   # Toolbar header and language selectors
        ├── storage.rs  # LocalStorage abstractions
        └── i18n.rs     # Translation Context provider and dictionary loaders
```

---

## Testing & Linting

### Unit and Integration Tests
Validate workspace rules and functional unit tests across all targets:
```bash
cargo test --workspace
```

### Checking Lints (Clippy)
Ensure the code conforms to standard clean architecture guidelines:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Checking Code Formatting
```bash
cargo fmt --all --check
```

---

## Contributing

1.  Fork the repository and create your feature branch (`git checkout -b feature/cool-idea`).
2.  Commit your updates using the Conventional Commits format (`git commit -m "feat: add syntax highlighting for rust"`).
3.  Ensure your code formatting passes formatting checks (`cargo fmt`).
4.  Open a Pull Request to the `main` branch.

---

## License

Distributed under the **GPL-3.0 License**. See [LICENSE](file:///LICENSE) for more details.
