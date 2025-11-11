FROM rust:1.91-slim-bookworm

RUN \
    apt-get update \
    && apt-get install -y ca-certificates curl libssl-dev pkg-config \
    && install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}") stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

RUN rustup component add clippy rustfmt

WORKDIR /app
COPY Cargo.lock Cargo.toml .
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build && rm -rf src

COPY src src

CMD ["cargo", "run"]
