FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt update
RUN apt install -y build-essential libssl-dev pkg-config protobuf-compiler libclang1 clang cmake libpq-dev libdw-dev \
    binutils \
    lld \
    libudev-dev
RUN rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt update && apt install -y libdw-dev ca-certificates git
COPY --from=builder /app/target/release/jayce /usr/local/bin
ENTRYPOINT ["/usr/local/bin/jayce"]
