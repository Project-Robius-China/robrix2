# Palpo Matrix Homeserver — Federation Build (Alpine variant)
# --------------------------------------------------
# Alpine-based to avoid the Debian bookworm-slim base image which is
# not cached in some local registry mirrors. Final runtime image is
# ~25 MB (vs ~90 MB for Debian).
#
# Runtime image exposes 8008 (client-server) and 8448 (federation).

FROM rust:1.93-alpine AS builder
WORKDIR /work
RUN apk add --no-cache \
      build-base \
      clang-dev \
      cmake \
      git \
      musl-dev \
      openssl-dev \
      pkgconfig \
      postgresql-dev
# musl defaults to fully static linking; disable crt-static so libpq can
# be dynamically linked at runtime (full static linking of libpq is not
# practical due to its transitive OpenSSL/krb5 dependencies).
ENV RUSTFLAGS="-C target-feature=-crt-static"
COPY ./repos/palpo .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/work/target \
    cargo build --release && cp target/release/palpo /usr/local/bin/palpo

FROM alpine:3.21
# libgcc: provides libgcc_s.so.1 which the Rust-produced binary needs for
#         stack unwinding symbols (_Unwind_*). Required because -crt-static
#         is disabled in the builder stage (so we can dynamically link libpq).
RUN apk add --no-cache ca-certificates curl libgcc libpq \
    && mkdir -p /var/palpo/media /var/palpo/certs
COPY --from=builder /usr/local/bin/palpo /usr/local/bin/palpo
ENV PALPO_CONFIG=/var/palpo/palpo.toml
EXPOSE 8008 8448
HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=15s \
    CMD curl -sf http://localhost:8008/_matrix/client/versions || exit 1
CMD ["/usr/local/bin/palpo"]
