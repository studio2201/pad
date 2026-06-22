# RustPad

<p align="center">
  <img src="https://img.shields.io/github/v/tag/UberMetroid/RustPad?label=version" alt="GitHub tag" />
  <img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License" />
  <img src="https://img.shields.io/github/actions/workflow/status/UberMetroid/RustPad/docker-publish.yml" alt="GitHub Actions Workflow Status" />
</p>

A stupid simple, no auth (unless you want it!), modern collaborative notepad application with auto-save functionality, fuzzy search, and multi-theme support. Built with Rust (Axum/Tokio backend and Yew/Trunk WebAssembly frontend).

---

## Features

- 📝 **Live Auto-Save**: Small visual auto-save indicator (`"Saving..."` ➡️ `"Saved"`).
- 🎨 **Multi-Theme Support**: Light, Dark, Sepia, Nord, and Dracula modes with matching styled toggle icons.
- 🤝 **Real-Time Collaboration**: Peer cursor sync and live Operational Transformation edit synchronization via WebSockets.
- 📶 **Robust Connection**: Exponential back-off reconnection loop with offline edit queuing.
- 🔍 **Fuzzy Search**: Find notepads by title or content with highlighted search previews.
- 🔒 **PIN Security**: Lock down your pad with an optional 4-10 digit PIN and brute-force lockout protection.
- ⌨️ **Keyboard Shortcuts**: Fully keyboard-accessible controls with a shortcut help modal (`?`).
- 🛠️ **Markdown Toolbar**: Helper panel for Bold, Italic, Headers, Links, and Code formatting.

---

## Quick Start

### Docker (Recommended)

```bash
docker run -d -p 3000:3000 -v ./data:/app/data ghcr.io/ubermetroid/rustpad:latest
```

1. Go to `http://localhost:3000`
2. Start typing! Notes auto-save in `./data`.

### Docker Compose

Create a `docker-compose.yml` file:

```yaml
services:
  rustpad:
    image: ghcr.io/ubermetroid/rustpad:latest
    container_name: rustpad
    restart: unless-stopped
    ports:
      - 3000:3000
    volumes:
      - ./data:/app/data
    environment:
      SITE_TITLE: RustPad
      RUSTPAD_PIN: 1234 # Optional authentication PIN (leave empty to disable)
      BASE_URL: http://localhost:3000
```

Start the container:
```bash
docker compose up -d
```

---

## Configuration

RustPad can be configured via environment variables:

| Variable | Description | Default |
| --- | --- | --- |
| `PORT` | Port the web server listens on | `3000` |
| `BASE_URL` | Application base URL (must end with `/`) | `http://localhost:PORT/` |
| `RUSTPAD_PIN` | Optional 4-10 digit authentication PIN | None |
| `SITE_TITLE` | The title shown in the web interface | `RustPad` |
| `ALLOWED_ORIGINS` | Comma-separated list of origins allowed for CORS | All origins (`*`) |
| `LOCKOUT_TIME` | Pin brute-force lockout time in minutes | `15` |
| `MAX_ATTEMPTS` | Maximum pin entry attempts before lockout | `5` |
| `DISABLE_PRINT_EXPAND` | Disable auto-expanding detail blocks in print/PDF | `false` |

> [!NOTE]
> See [.env.example](file:///.env.example) for advanced configurations and reverse-proxy settings.

---

## Technical Details

- **Backend**: Rust (Axum + Tokio + WebSockets)
- **Frontend**: Rust (Yew + WebAssembly via Trunk)
- **Styling**: Vanilla CSS variables
- **Container**: Multi-stage lightweight Docker image

---

## Contributing & License

1. Fork the repo and create your feature branch.
2. Commit changes using Conventional Commits.
3. Open a Pull Request.

Distributed under the **GPL-3.0 License**. See [LICENSE](file:///LICENSE) for more information.
