# syntax=docker/dockerfile:1

# Минор зафиксирован: сборка образа не должна меняться от выхода новой стабильной версии Rust.
# Поднимайте вместе с rust-toolchain.toml и проверкой `task dogfood`.
FROM rust:1.99-bookworm AS build
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
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /out/crane /usr/local/bin/crane
# Каталог создаётся с владельцем nonroot: новый именованный том наследует права образа и остаётся записываемым.
COPY --from=build --chown=65532:65532 /out/data /data
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["crane"]
CMD ["all"]
