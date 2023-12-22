#!/usr/bin/env bash

case "${TARGETPLATFORM}" in
    "linux/arm64") TARGET_BINARY="aarch64-unknown-linux-musl" ;;
    "linux/arm/v6") TARGET_BINARY="arm-unknown-linux-musleabihf" ;;
    "linux/arm/v7") TARGET_BINARY="armv7-unknown-linux-musleabihf" ;;
    "linux/amd64") TARGET_BINARY="x86_64-unknown-linux-musl" ;;
    "linux/386") TARGET_BINARY="i686-unknown-linux-musl" ;;
    *) exit 1 ;;
esac;

cp "./artifact-binaries/$TARGET_BINARY" /usr/local/bin/searproxy;
