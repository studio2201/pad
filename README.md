# Pad - Real-Time Collaborative Notepad

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/pad/main/frontend/Assets/favicon.png?v=3.0.1" alt="Pad Logo" width="128" height="128">
</p>

Pad is a collaborative real-time notepad and text editor designed for minimal resource usage, zero external JS library bloat, and fast load speeds. Built with a high-performance Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## ⚡ Key Features

*   **Dynamic Themes**: Dynamic theme options.
*   **Access PIN Security**: Lock down the interface with an optional numerical PIN for absolute privacy.
*   **Internationalization**: Built-in multilingual translation selector support.
*   **Print Optimization**: Customized print stylesheet layout and print header action button.
*   **Performance First**: Tiny resource footprint, zero external JS engine dependencies, and rapid page load speeds.
*   **Real-Time Sync**: Collaborative typing synchronization across users via WebSockets.
*   **Rich Text Editing**: Document markup format capabilities and auto-saving.

---

## 📦 Container Registry

The Docker image is built with **Nix** (no Alpine, fully reproducible) and published to Docker Hub:

*   **Docker Hub**: [ubermetroid/pad](https://hub.docker.com/r/ubermetroid/pad)

---

## 🐳 Container Installation



1. Create a `docker-compose.yml` file:

```yaml
version: '3'
services:
  pad:
    image: ubermetroid/pad:latest
    container_name: pad
    restart: unless-stopped
    ports:
      - 4402:4402
    volumes:
      - ./data:/app/data
    environment:
      - PORT=4402
      - SITE_TITLE=Pad
      - BASE_URL=http://localhost:4402
      - ALLOWED_ORIGINS=*
      - PAD_PIN=1234
      - TZ=UTC
      - ENABLE_TRANSLATION=false
      - ENABLE_THEMES=true
      - ENABLE_PRINT=false
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4402`.

### Building the Image Locally

To build the Docker container locally from the source files using Nix:

```bash
nix build .#dockerImage
docker load < result
docker tag pad-nix:latest ubermetroid/pad:latest
```

The image is Nix-built (no Alpine, no Docker daemon dependency for the build).
For development iteration, use the devShell:

```bash
nix develop
```

## 📋 Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4402` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. *(Supports fallback `PAD_TITLE` / `PAD_SITE_TITLE`)* | `Pad` |
| `BASE_URL` | Application base URL. Essential when deploying behind reverse proxies to ensure redirect and websocket links are resolved correctly. | `http://localhost:4402` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `PAD_PIN` | Optional 4–10 digit PIN (numerical only) to lock access to the interface. Leave empty for public mode. *(Supports fallback `PIN`)* | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `false` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header (true/false). | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header (true/false). | `false` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `LOCKOUT_TIME` | Bruteforce lockout duration in minutes. | `15` |
| `TRUST_PROXY` | Set true if deploying behind reverse proxy (Nginx, Cloudflare). | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted proxy CIDRs/IPs. | None |



---

*Note: This repository was forked from [DumbPad](https://github.com/DumbWareio/DumbPad).*
