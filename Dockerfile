# 1. Use the official Rust image
FROM rust:1.85 as builder

# 2. Create a new empty project to cache dependencies
WORKDIR /app
RUN cargo init --lib

# 3. Copy your dependency declarations first
COPY Cargo.toml Cargo.lock ./

# 4. Pre-build dependencies (this layer gets cached)
RUN cargo build --release
RUN rm src/*.rs

# 5. Copy actual source code (only invalidates if changed)
COPY src ./src
RUN cargo build --release

# 6. Final minimal image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/slack_socket_bridge /usr/local/bin/slack_socket_bridge
ENTRYPOINT ["/usr/local/bin/slack_socket_bridge"]
