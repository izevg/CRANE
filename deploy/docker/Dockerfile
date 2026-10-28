# syntax=docker/dockerfile:1

# Версия Rust и базовые образы закреплены тегом и digest мульти-арх индекса: сборка не меняется
# от выхода новой версии. Поднимайте так: новый тег и его digest из `docker buildx imagetools inspect <образ:тег>`,
# вместе с rust-toolchain.toml и проверкой `task dogfood`.
FROM rust:1.99.1-bookworm@sha256:48b7f435b8fc6378ce43b44f15e3a47fd8919b2825f835c39bd4bd3da50af5b8 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
# Кэш target вне слоя образа, поэтому бинарник копируется в /out внутри того же RUN.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p crane-server \
    && mkdir -p /out/data/blobs \
    && cp target/release/crane /out/crane

# distroless/cc: glibc и libgcc для rustls/aws-lc-rs, без shell. uid 65532 (nonroot).
# Healthcheck в compose — `crane healthcheck`, поэтому curl в образе не нужен.
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=build /out/crane /usr/local/bin/crane
# Каталог создаётся с владельцем nonroot: новый именованный том наследует права образа и остаётся записываемым.
COPY --from=build --chown=65532:65532 /out/data /data
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["crane"]
CMD ["all"]
