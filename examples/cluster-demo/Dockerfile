FROM rust:1.81-slim-bookworm AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && mkdir tests && touch tests/integration_tests.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release 2>/dev/null || true
RUN rm -rf src tests
COPY src src
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/* && \
    groupadd -r primusdb && useradd -r -g primusdb primusdb
COPY --from=builder /build/target/release/primusdb-server /usr/local/bin/
COPY --from=builder /build/target/release/primusdb-cli /usr/local/bin/
USER primusdb
EXPOSE 8080 9090
ENTRYPOINT ["primusdb-server"]
