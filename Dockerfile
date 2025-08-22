# BUILD Stage
FROM rust:1.89 as builder
WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./

# Create a dummy main to cache deps efficiently
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

# Now copy real sources
COPY src ./src

# Build the actual binary
RUN cargo build --release

# RUN Stage
FROM debian:bookworm-slim

# Install minimal runtime deps (ca-certificates in case future HTTP clients are added)
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 10001 appuser

# Copy the compiled binary
COPY --from=builder /app/target/release/rust-observability /usr/local/bin/app

EXPOSE 8080
USER appuser

CMD ["/usr/local/bin/app"]