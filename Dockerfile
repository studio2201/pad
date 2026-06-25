# --- Stage 1: Build the Rust Backend and Frontend ---
FROM rust:alpine AS rust-builder
RUN apk add --no-cache musl-dev wget gzip brotli

# Install WebAssembly target and Trunk builder
RUN rustup target add wasm32-unknown-unknown
RUN wget -qO- "https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-musl.tar.gz" | tar -xzf- -C /usr/local/bin

WORKDIR /app

# Copy cargo configuration and dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY frontend/Cargo.toml ./frontend/
COPY backend/Cargo.toml ./backend/

# Cache backend dependencies by building a dummy binary
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs
RUN mkdir -p frontend/src && echo "fn main() {}" > frontend/src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/log*
RUN cargo build --release --target wasm32-unknown-unknown -p log-frontend

# Copy actual source code and compile
COPY backend/src ./backend/src
COPY frontend/src ./frontend/src
COPY frontend/index.html ./frontend/
COPY frontend/service-worker.js ./frontend/
COPY frontend/Assets ./frontend/Assets

RUN cd frontend && trunk build --release && \
    find dist -type f \( -name "*.js" -o -name "*.wasm" -o -name "*.css" -o -name "*.html" -o -name "*.svg" -o -name "*.json" \) -exec gzip -k -9 {} \; -exec brotli -k -Z {} \;
RUN cargo build --release --bin log

# --- Stage 2: Final Runtime Container ---
FROM alpine:latest
LABEL org.opencontainers.image.source="https://github.com/UberMetroid/Log"

WORKDIR /app

ENV PORT=4402
ENV NODE_ENV=production
ENV LOG_DIR=/app/log

# Copy compiled Rust binary
COPY --from=rust-builder /app/target/release/log /app/log

# Copy compiled frontend assets
COPY --from=rust-builder /app/frontend/dist /app/frontend/dist

# Setup data and asset directories with correct ownership
RUN mkdir -p /app/data /app/frontend/dist/Assets && \
    chown -R 99:100 /app

# Run as Unraid nobody:users
USER 99:100

# Mount data volume
VOLUME /app/data

EXPOSE 4402

CMD ["/app/log"]
