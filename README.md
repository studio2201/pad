# RustPad - Real-Time Collaborative Notepad

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/RustPad/main/frontend/Assets/rustpad.png" alt="RustPad Logo" width="128" height="128">
</p>

RustPad is a collaborative real-time notepad and text editor designed for minimal resource usage, zero external JS library bloat, and fast load speeds. Built with a Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## 🐳 Container Installation

### Option 1: Docker Compose (Recommended)

1. Create a `docker-compose.yml` file:

```yaml
version: '3'
services:
  rustpad:
    image: ubermetroid/rustpad:latest
    container_name: rustpad
    restart: unless-stopped
    ports:
      - 4402:4402
    volumes:
      - ./data:/app/data
    environment:
      - PORT=4402
      - BASE_URL=http://localhost:4402
      - RUSTPAD_PIN=1234
      - SITE_TITLE=RustPad
      - TRUST_PROXY=false
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4402`.

### Option 2: Docker CLI

Run the following command to start the container:

```bash
docker run -d \
  --name rustpad \
  --restart unless-stopped \
  -p 4402:4402 \
  -v $(pwd)/data:/app/data \
  -e RUSTPAD_PIN=1234 \
  ubermetroid/rustpad:latest
```

---

## 📋 Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4402` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. *(Supports fallback `RUSTRUSTPAD_TITLE`)* | `RustPad` |
| `BASE_URL` | Application base URL. Essential when deploying behind reverse proxies to ensure redirect and websocket links are resolved correctly. | `http://localhost:4402` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `RUSTPAD_PIN` | Optional 4–10 digit PIN (numerical only) to lock access to the interface. Leave empty for public mode. | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `LOCKOUT_TIME` | Bruteforce lockout duration in minutes. | `15` |
| `TRUST_PROXY` | Set true if deploying behind reverse proxy (Nginx, Cloudflare). | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted proxy CIDRs/IPs. | None |

## 📂 Repository Structure

```
.
├── backend/
│   ├── Cargo.toml
│   └── src
│       ├── config.rs
│       ├── main.rs
│       ├── migration.rs
│       ├── routes
│       │   ├── auth.rs
│       │   ├── mod.rs
│       │   ├── notepads_crud.rs
│       │   ├── notepads_io.rs
│       │   └── pages.rs
│       ├── search.rs
│       ├── state.rs
│       ├── tests.rs
│       ├── utils.rs
│       └── ws.rs
└── frontend/
    ├── Assets
    │   ├── app.css
    │   ├── asset-manifest.json
    │   ├── base.css
    │   ├── header.css
    │   ├── login.css
    │   ├── manifest.json
    │   ├── rustpad.png
    │   └── rustpad.svg
    ├── Cargo.toml
    ├── index.html
    ├── service-worker.js
    └── src
        ├── app.rs
        ├── collab.rs
        ├── collab_utils.rs
        ├── editor.rs
        ├── header.rs
        ├── i18n
        │   ├── de.rs
        │   ├── en.rs
        │   ├── es.rs
        │   ├── fr.rs
        │   ├── ja.rs
        │   ├── pt.rs
        │   ├── ru.rs
        │   └── zh.rs
        ├── i18n.rs
        ├── login.rs
        ├── main.rs
        ├── services.rs
        ├── storage.rs
        └── types.rs
```
