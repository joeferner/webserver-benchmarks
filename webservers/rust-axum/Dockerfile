FROM rust:1.91-slim-bookworm

RUN \
    apt-get update \
    && apt-get install -y libssl-dev pkg-config

RUN rustup component add clippy rustfmt

WORKDIR /app
COPY Cargo.lock Cargo.toml .
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY src src

RUN cargo build --release
CMD ["/app/target/release/rust-axum"]
