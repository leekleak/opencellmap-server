FROM rust:1-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
 && cargo build --release --locked \
 && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/opencellmap-server /usr/local/bin/opencellmap-server
EXPOSE 3000
USER 65534:65534
CMD ["opencellmap-server"]
