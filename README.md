# RustPad - Real-Time Collaborative Notepad

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
| `PORT` | The port address the backend web server listens on. | `4402` |
| `BASE_URL` | Application base URL. | `http://localhost:4402` |
| `RUSTPAD_PIN` | Optional 4-10 digit authentication PIN. | None |
| `SITE_TITLE` | The title shown in the browser and PWA manifest. | `RustPad` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `LOCKOUT_TIME` | Bruteforce lockout duration in minutes. | `15` |
| `TRUST_PROXY` | Set true if deploying behind reverse proxy (Nginx, Cloudflare). | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted proxy CIDRs/IPs. | None |
