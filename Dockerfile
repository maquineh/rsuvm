# syntax=docker/dockerfile:1
# Build multi-stage do binário rsuvm. A imagem resultante roda como UID não-root.
# Isto não é um sandbox: para código não confiável, use isolamento externo.
FROM rust:1.94-slim-bookworm AS build

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY README.md LICENSE-MIT LICENSE-APACHE CHANGELOG.md ./
RUN cargo build --release --locked -p rvm-cli

FROM debian:bookworm-slim

COPY --from=build /src/target/release/rsuvm /usr/local/bin/rsuvm
USER 10001:10001
ENTRYPOINT ["rsuvm"]
