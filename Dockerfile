# --- Stage 1: Build the Rust Backend and Frontend ---
FROM rust:alpine AS rust-builder
RUN apk add --no-cache musl-dev wget gzip brotli

# Install WebAssembly target and Trunk builder
RUN rustup target add wasm32-unknown-unknown
RUN case "$(uname -m)" in \
      x86_64) ARCH=x86_64 ;; \
      aarch64) ARCH=aarch64 ;; \
      *) echo "Unsupported architecture" && exit 1 ;; \
    esac && \
    wget -qO- "https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-${ARCH}-unknown-linux-musl.tar.gz" | tar -xzf- -C /usr/local/bin

WORKDIR /app

# Copy cargo configuration and dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY frontend/Cargo.toml ./frontend/

# Cache backend dependencies by building a dummy binary
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/rustpad*

# Cache frontend dependencies
RUN mkdir -p frontend/src && echo "fn main() {}" > frontend/src/main.rs
RUN cd frontend && trunk build --release

# Copy actual source code and compile
COPY src ./src
COPY frontend/src ./frontend/src
COPY frontend/index.html ./frontend/
COPY frontend/service-worker.js ./frontend/
COPY frontend/Assets ./frontend/Assets

RUN cd frontend && trunk build --release && \
    find dist -type f \( -name "*.js" -o -name "*.wasm" -o -name "*.css" -o -name "*.html" -o -name "*.svg" -o -name "*.json" \) -exec gzip -k -9 {} \; -exec brotli -k -Z {} \;
RUN cargo build --release --bin rustpad

# --- Stage 2: Final Runtime Container ---
FROM alpine:latest

WORKDIR /app

# Create a non-root user matching UID 1000
RUN addgroup -g 1000 rustpad && \
    adduser -u 1000 -G rustpad -s /bin/sh -D rustpad

# Copy compiled Rust binary
COPY --from=rust-builder /app/target/release/rustpad /app/rustpad

# Copy compiled frontend assets
COPY --from=rust-builder /app/frontend/dist /app/frontend/dist

# Setup data and asset directories with correct ownership
RUN mkdir -p /app/data /app/frontend/dist/Assets && \
    chown -R rustpad:rustpad /app

USER rustpad

# Mount data volume
VOLUME /app/data

EXPOSE 3000

CMD ["/app/rustpad"]
