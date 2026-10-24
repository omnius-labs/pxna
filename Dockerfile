# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.96.0
FROM --platform=$BUILDPLATFORM node:24-bookworm-slim AS web
WORKDIR /build/ui-web
COPY ui-web/package.json ui-web/package-lock.json ./
RUN npm ci
COPY ui-web/ ./
RUN npm run build

FROM rust:${RUST_VERSION}-trixie AS daemon
ARG TARGETARCH
ARG PXNA_BUILD_VERSION=0.1.0
WORKDIR /build/daemon
RUN apt-get update && apt-get install -y --no-install-recommends libclang-dev pkg-config cmake && rm -rf /var/lib/apt/lists/*
COPY daemon/ ./
RUN test -f refs/core-rs/Cargo.toml || { echo "The core-rs submodule is empty. Run ./start.sh, or 'git submodule update --init' first." >&2; exit 1; }
RUN --mount=type=cache,id=pxna-registry-${TARGETARCH},target=/usr/local/cargo/registry \
    --mount=type=cache,id=pxna-target-${TARGETARCH},target=/build/daemon/target \
    RUSTC_WRAPPER= cargo build --locked --release -p omnius-pxna-daemon && \
    cp target/release/omnius-pxna-daemon /usr/local/bin/pxna-daemon

# Debian's ffmpeg (7.1 on trixie) reads only one tile of a tiled HEIC or AVIF, and ffprobe before 9.0 does not report the rotation
# of a tiled image, so use a static ffmpeg 9.0 build.
# It is the GPL build because transcoding uses libx264. Pin a month-end autobuild, which BtbN keeps longer than the daily ones.
FROM debian:trixie-slim AS ffmpeg
ARG TARGETARCH
ARG FFMPEG_RELEASE=autobuild-2026-09-30-13-08
ARG FFMPEG_BUILD=ffmpeg-n9.0.2-17-g2a571b6068
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl xz-utils && rm -rf /var/lib/apt/lists/*
RUN case "${TARGETARCH}" in \
        amd64) arch=linux64; sha256=68ee646831adaae2495618346f3bba94ff207ff83bbd34d643e7004730d66269 ;; \
        arm64) arch=linuxarm64; sha256=91afcd7695d4bbbb9e0057aa10baaf9875ec4e1a97303f9059ec971336a2ab46 ;; \
        *) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac && \
    curl -fsSL -o /tmp/ffmpeg.tar.xz "https://github.com/BtbN/FFmpeg-Builds/releases/download/${FFMPEG_RELEASE}/${FFMPEG_BUILD}-${arch}-gpl-9.0.tar.xz" && \
    echo "${sha256}  /tmp/ffmpeg.tar.xz" | sha256sum -c - && \
    mkdir -p /opt/ffmpeg && tar -xJf /tmp/ffmpeg.tar.xz -C /opt/ffmpeg --strip-components=1 && \
    install -m 0755 /opt/ffmpeg/bin/ffmpeg /opt/ffmpeg/bin/ffprobe /usr/local/bin/ && \
    rm -rf /tmp/ffmpeg.tar.xz /opt/ffmpeg

FROM debian:trixie-slim
# libheif decodes HEIC, including tiled images and their rotation. ffmpeg reads it when libheif cannot.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl libheif-examples libheif-plugin-libde265 && rm -rf /var/lib/apt/lists/* && \
    groupadd --gid 10001 pxna && useradd --uid 10001 --gid pxna --no-create-home --shell /usr/sbin/nologin pxna && \
    mkdir -p /config /state /libraries && chown pxna:pxna /config /state /libraries
COPY --from=ffmpeg /usr/local/bin/ffmpeg /usr/local/bin/ffprobe /usr/local/bin/
COPY --from=daemon /usr/local/bin/pxna-daemon /usr/local/bin/pxna-daemon
COPY --from=web /build/ui-web/dist/ /opt/pxna/web/
COPY --chown=pxna:pxna config/pxna.toml.example /config/pxna.toml
ENV PXNA_DAEMON_CONFIG_DIR=/config
USER 10001:10001
EXPOSE 6050
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:6050/api/v1/health || exit 1
ENTRYPOINT ["/usr/local/bin/pxna-daemon"]
CMD ["start"]
