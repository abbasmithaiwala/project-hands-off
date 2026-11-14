# syntax=docker/dockerfile:1.7
#
# Hands-Off in one image: the web app and the server. Model weights are not part of the image;
# the server downloads them on first start (see HANDS_OFF_RFDETR_SEG_URL).

ARG RUST_VERSION=1.94.0
ARG NODE_VERSION=22

# --- web app ---------------------------------------------------------------------------------
FROM node:${NODE_VERSION}-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# --- server ----------------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-slim-bookworm AS server
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml clippy.toml ./
COPY crates crates
COPY prompts prompts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p hands-off-api --bin hands-off \
    && cp target/release/hands-off /hands-off

# --- runtime ---------------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libstdc++6 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin handsoff \
    && mkdir -p /data/models /app \
    && chown -R handsoff:handsoff /data
COPY --from=server /hands-off /usr/local/bin/hands-off
COPY --from=web /src/web/dist /app/web

# Where the RF-DETR-Seg model is downloaded from on first start, and its checksum.
ARG MODEL_URL=""
ARG MODEL_SHA256=""
ENV HANDS_OFF_BIND=0.0.0.0:8080 \
    HANDS_OFF_DATA_DIR=/data \
    HANDS_OFF_MODEL_DIR=/data/models \
    HANDS_OFF_WEB_DIR=/app/web \
    HANDS_OFF_RFDETR_SEG_URL=${MODEL_URL} \
    HANDS_OFF_RFDETR_SEG_SHA256=${MODEL_SHA256}

USER handsoff
WORKDIR /data
VOLUME /data
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["hands-off", "healthcheck"]
ENTRYPOINT ["hands-off"]
