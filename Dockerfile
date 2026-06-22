# --- Stage 1: Build the Rust Backend ---
FROM rust:alpine AS rust-builder
RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependencies by building a dummy project
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/dumbpad*

# Copy actual source code and build it
COPY src ./src
RUN cargo build --release

# --- Stage 2: Fetch Frontend Node Modules ---
FROM node:22-alpine AS node-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev

# --- Stage 3: Final Runtime Container ---
FROM alpine:latest

WORKDIR /app

# Create a non-root user matching UID 1000
RUN addgroup -g 1000 dumbpad && \
    adduser -u 1000 -G dumbpad -s /bin/sh -D dumbpad

# Copy compiled Rust binary
COPY --from=rust-builder /app/target/release/dumbpad /app/dumbpad

# Copy node modules (frontend dependencies)
COPY --from=node-builder /app/node_modules /app/node_modules

# Copy static frontend public assets
COPY public /app/public

# Setup data and asset directories with correct ownership
RUN mkdir -p /app/data /app/public/Assets && \
    chown -R dumbpad:dumbpad /app

USER dumbpad

# Mount data volume
VOLUME /app/data

EXPOSE 3000

CMD ["/app/dumbpad"]
