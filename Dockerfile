############################
# Stage 1: Build the binary
############################
FROM rust:latest AS builder

# Build dependencies for Rust + Tauri
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire repo into the image
COPY . .

# Tauri's build scripts expect /app/target; link it to src-tauri/target
RUN ln -s src-tauri/target target 2>/dev/null || true

# Build the headless proxy binary
RUN cd src-tauri && \
    cargo build --release --bin antigravity_proxy_server

############################
# Stage 2: Runtime image
############################
FROM debian:bookworm-slim

# Install runtime dependencies only (inside container, not on host)
RUN apt-get update && apt-get install -y \
    libgtk-3-0 \
    libwebkit2gtk-4.1-0 \
    libayatana-appindicator3-1 \
    librsvg2-2 \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN useradd -m antigravity

USER antigravity
WORKDIR /home/antigravity

# Copy built binary from builder stage
COPY --from=builder /app/src-tauri/target/release/antigravity_proxy_server \
    /usr/local/bin/antigravity_proxy_server

# Antigravity config + accounts directory
VOLUME ["/home/antigravity/.antigravity_tools"]

EXPOSE 8045

ENTRYPOINT ["/usr/local/bin/antigravity_proxy_server"]