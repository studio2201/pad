# Pad — Real-Time Collaborative Notepad <img src="https://raw.githubusercontent.com/etecoons/unraid-apps/main/icons/pad.png" width="48" height="48" alt="pad logo" align="right">

Pad is a collaborative real-time notepad and text editor designed for minimal resource usage, zero external JS library bloat, and fast load speeds. Built with a high-performance Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## 🏛️ Architecture & Stack
*   **Frontend**: Yew (WASM)
*   **Backend**: Axum (Rust) / Tokio (WebSockets)
*   **Deployment**: UBI container (Red Hat UBI9) on Docker Hub / Unraid / Podman / Docker Compose

---

## 🟢 Key Features
*   **Real-Time Sync**: Collaborative typing synchronization across users via WebSockets.
*   **Rich Text Editing**: Document markup format capabilities and auto-saving.
*   **Access PIN Security**: Lock down the interface with an optional numerical PIN for absolute privacy.
*   **Dynamic Themes**: Super Metroid UI themes (Crateria, Brinstar, Norfair, Wrecked Ship, Maridia, Tourian).
*   **Internationalization**: Built-in multilingual translation selector support.
*   **Print Optimization**: Customized print stylesheet layout and print header action button.
*   **Performance First**: Tiny resource footprint, zero external JS engine dependencies, and rapid page load speeds.

---

## 💾 Deployment & Installation

### Container images (Docker Hub)

Images are **UBI9-minimal** based (Red Hat Universal Base Image). Tags:

| Tag | Meaning |
| :--- | :--- |
| `latest` | Current recommended build |
| `ubi` | Explicit UBI image (same lineage as `latest`) |
| `3.0.20` | Immutable release pin |

```bash
# Pull examples
podman pull docker.io/etecoons/pad:latest
podman pull docker.io/etecoons/pad:ubi
podman pull docker.io/etecoons/pad:3.0.20
```

Hub: [https://hub.docker.com/r/etecoons/pad](https://hub.docker.com/r/etecoons/pad)

### Docker Compose
Create a `docker-compose.yml` file with the following service definition:

```yaml
services:
  pad:
    image: etecoons/pad:latest
    container_name: pad
    restart: unless-stopped
    ports:
      - ${PORT:-4402}:4402
    volumes:
      - ${PAD_DATA_PATH:-./data}:/app/data
    environment:
      PORT: 4402
      SITE_TITLE: ${PAD_SITE_TITLE:-Pad}
      PAD_PIN: ${PAD_PIN:-}
      BASE_URL: ${PAD_BASE_URL:-http://localhost:4402}
      ALLOWED_ORIGINS: ${PAD_ALLOWED_ORIGINS:-*}
      TZ: ${TZ:-UTC}
      ENABLE_TRANSLATION: ${ENABLE_TRANSLATION:-false}
      ENABLE_THEMES: ${ENABLE_THEMES:-true}
      ENABLE_PRINT: ${ENABLE_PRINT:-true}
      MAX_ATTEMPTS: ${MAX_ATTEMPTS:-5}
```

### Build the UBI image locally

Requires [Podman](https://podman.io/) (or Docker) and network access to pull base images and crates.

```bash
# From the repository root
podman build --format docker -f Containerfile.ubi \
  -t docker.io/etecoons/pad:3.0.20 \
  -t docker.io/etecoons/pad:latest \
  -t docker.io/etecoons/pad:ubi \
  .

# Optional: push all three tags
podman push docker.io/etecoons/pad:3.0.20
podman push docker.io/etecoons/pad:latest
podman push docker.io/etecoons/pad:ubi
```

---

## ⚙️ Configuration Options

| Environment Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4402` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. | `Pad` |
| `BASE_URL` | Application base URL. Essential when deploying behind reverse proxies. | `http://localhost:4402` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). | `*` |
| `PAD_PIN` | Optional 4–10 digit numerical PIN to lock access to the interface. | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header. | `false` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header. | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header. | `true` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `LOCKOUT_TIME_MINUTES` | Bruteforce lockout duration in minutes. | `15` |
| `COOKIE_MAX_AGE_HOURS` | Duration in hours that the user's PIN session cookie remains valid. | `24` |
| `SHUTDOWN_DRAIN_SECONDS` | Seconds to wait for active connections to finish before shutting down. | `5` |
| `SHOW_VERSION` | Display the application version number in the footer. | `true` |
| `SHOW_GITHUB` | Display the GitHub repository link in the footer. | `true` |
| `PAGE_HISTORY_COOKIE_AGE` | Duration in days to preserve local undo history in client cookies. | `365` |
| `TRUST_PROXY` | Set true if deploying behind reverse proxy (Nginx, Cloudflare). | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted proxy CIDRs/IPs. | None |

---

## 🛠️ Local Development

Ensure you have the Rust toolchain and Trunk installed.

```bash
# 1. Run workspace tests
cargo test

# 2. Run clippy workspace checks
cargo clippy --workspace --all-targets

# 3. Start frontend Yew dev server (from frontend/)
cd frontend && trunk serve

# 4. Start backend Axum server (from backend/)
cd backend && cargo run
```

---

## 📄 License
Licensed under the [Apache License, Version 2.0](LICENSE). Copyright 2026 etecoons.
