# Palpo Matrix Homeserver — Multi-stage Build
# Mirrors upstream Palpo's official build/docker/Dockerfile.palpo so we
# stay inside the CI-tested combination. Runtime uses debian:bookworm
# (full variant, not -slim) because upstream tests that tag and it
# historically pulls reliably where bookworm-slim sometimes 404s on
# Docker Hub's CDN.

FROM rust:bookworm AS builder
WORKDIR /work
RUN apt-get update && apt-get install -y --no-install-recommends \
    libclang-dev libpq-dev cmake \
    && rm -rf /var/lib/apt/lists/*
COPY ./repos/palpo .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/work/target \
    cargo build --release && cp target/release/palpo /usr/local/bin/palpo

FROM debian:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libpq-dev \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /var/palpo/media
COPY --from=builder /usr/local/bin/palpo /usr/local/bin/palpo
ENV PALPO_CONFIG=/var/palpo/palpo.toml
EXPOSE 8008
HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=15s \
    CMD curl -sf http://localhost:8008/_matrix/client/versions || exit 1
CMD ["/usr/local/bin/palpo"]
