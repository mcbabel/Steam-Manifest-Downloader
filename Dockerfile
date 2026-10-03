FROM rust:1-alpine AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src-core src-core
COPY src-tui src-tui
COPY src-tauri/tauri.conf.json src-tauri/tauri.conf.json
COPY public/locales public/locales
ARG SMD_BUILD_CHANNEL=source
ARG SMD_GIT_SHA=
RUN SMD_BUILD_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ) cargo build --release --locked -p smd-tui \
 && cp target/release/smd /smd

FROM alpine:3 AS base
RUN adduser -D -H -h /data -u 1000 smd \
 && mkdir -p /data /games \
 && chown smd:smd /data /games
ENV SMD_DATA_DIR=/data \
    SMD_OUTPUT_DIR=/games \
    HOME=/data
WORKDIR /games
VOLUME ["/data", "/games"]
USER smd
ENTRYPOINT ["smd"]
CMD ["--help"]

FROM base AS prebuilt
COPY --chmod=755 smd /usr/local/bin/smd

FROM base AS source
COPY --from=build /smd /usr/local/bin/smd
