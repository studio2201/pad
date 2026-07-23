<h1 align="center">
  <img src="assets/icon.png?v=1.0.31" width="48" height="48" valign="middle"> Pad
</h1>

<p align="center">
  <b>Real-time collaborative scratchpad web application written in Rust with WebSocket Operational Transformation.</b>
</p>

---

### Instant One-Line Install (Docker Container)

Run the official zero-dependency container on port 4402:

```bash
docker run -d --name pad -p 4402:4402 -v /mnt/user/appdata/pad:/config ghcr.io/studio2201/pad:latest
```

Open your browser to `http://localhost:4402` to start creating collaborative scratchpads immediately.

---

### One-Line Install (Native Package Manager)

On Debian, Ubuntu, Fedora, or RHEL:

```bash
curl -fsSL https://studio2201.github.io/packages/install.sh | sudo bash
```

---

### Unraid NAS Deployment

Deploy via the official Unraid Template:

1. Copy [`pad.xml`](pad.xml) to your Unraid flash drive under `/boot/config/plugins/dockerMan/templates-user/`.
2. Open **Docker** -> **Add Container** -> Select **pad** from the template dropdown.
3. Click **Apply**.

---

### Environment Configuration

The backend service can be customized using the following environment variables:

| Variable | Description | Default |
| :--- | :--- | :---: |
| `PORT` | Network port the web server binds to | `4402` |
| `PAD_PIN` | Security PIN required for application access | *(Disabled)* |
| `PAD_DATA_DIR` | Directory path for persistent data and notepads | `/config` |
| `PAD_ALLOWED_ORIGINS` | CORS allowed origins list (comma-separated) | `*` |
| `TRUST_PROXY` | Honor reverse proxy headers (`X-Forwarded-For`) | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated CIDR list of trusted reverse proxies | *(None)* |
| `LOG_LEVEL` | Tracing filter (`error`, `warn`, `info`, `debug`) | `info` |

---

### Administration CLI & TUI Dashboard

Every container and package includes a built-in administration utility (`pad`).

Launch interactive TUI dashboard:
```bash
docker exec -it pad pad tui
```

System diagnostics and self-healing check:
```bash
docker exec -it pad pad doctor
```

CLI Command Reference:
- `pad tui` — Interactive terminal user interface.
- `pad doctor` — Diagnoses storage permissions, ports, and database health.
- `pad status` — Displays network configuration and security parameters.
- `pad data stats` — Shows storage utilization and entry metrics.
- `pad data list` — Lists notepads and active document entries.

---

### Architecture & Security

- **Axum Web Backend**: High-concurrency async streaming runtime built on Tokio with WebSocket RFC 6455 framing.
- **Yew WebAssembly Frontend**: Type-safe client bundle running natively in browser WASM runtime.
- **Operational Transformation (OT)**: Concurrent document editing without lock contention.
- **Strict Input & Path Sanitization**: Path canonicalization guards preventing directory traversal escapes.

---

### License

Distributed under the Apache 2.0 License. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <a href="https://github.com/studio2201/pad">
    <img src="assets/corgi-footer.jpg" alt="studio2201 banner" width="100%">
  </a>
</p>
