FROM rust:1.91-bookworm AS builder

WORKDIR /build

# Cache dependency builds: copy manifests first
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    mkdir -p .sqlx && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy full source (including .sqlx offline cache)
COPY . .
# Touch main.rs so cargo sees it as changed
RUN touch src/main.rs
ENV SQLX_OFFLINE=true
RUN cargo build --release

# --- Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/poiesisd /usr/local/bin/poiesisd

WORKDIR /app

EXPOSE 8080

ENTRYPOINT ["poiesisd"]
