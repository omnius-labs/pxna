# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.96.0
FROM --platform=$BUILDPLATFORM node:24-bookworm-slim AS web
WORKDIR /build/ui-web
COPY ui-web/package.json ui-web/package-lock.json ./
RUN npm ci
COPY ui-web/ ./
RUN npm run build

FROM rust:${RUST_VERSION}-bookworm AS daemon
ARG TARGETARCH
ARG PXNA_BUILD_VERSION=0.1.0
WORKDIR /build/daemon
RUN apt-get update && apt-get install -y --no-install-recommends libclang-dev pkg-config cmake && rm -rf /var/lib/apt/lists/*
COPY daemon/ ./
RUN --mount=type=cache,id=pxna-registry-${TARGETARCH},target=/usr/local/cargo/registry \
    --mount=type=cache,id=pxna-target-${TARGETARCH},target=/build/daemon/target \
    RUSTC_WRAPPER= cargo build --locked --release -p omnius-pxna-daemon && \
    cp target/release/omnius-pxna-daemon /usr/local/bin/pxna-daemon

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates ffmpeg curl && rm -rf /var/lib/apt/lists/* && \
    groupadd --gid 10001 pxna && useradd --uid 10001 --gid pxna --no-create-home --shell /usr/sbin/nologin pxna && \
    mkdir -p /config /state /libraries && chown pxna:pxna /config /state /libraries
COPY --from=daemon /usr/local/bin/pxna-daemon /usr/local/bin/pxna-daemon
COPY --from=web /build/ui-web/dist/ /opt/pxna/web/
COPY --chown=pxna:pxna docker/pxna.toml /config/pxna.toml
ENV PXNA_DAEMON_CONFIG_DIR=/config
USER 10001:10001
EXPOSE 6050
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:6050/api/v1/health || exit 1
ENTRYPOINT ["/usr/local/bin/pxna-daemon"]
CMD ["start"]
