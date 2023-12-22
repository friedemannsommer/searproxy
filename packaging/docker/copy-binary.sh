#!/usr/bin/env bash

set -xeuo pipefail

case "${TARGETPLATFORM}" in
    "linux/arm64") ARTIFACT_DIR="SearProxy_aarch64-unknown-linux-musl" ;;
    "linux/arm/v6") ARTIFACT_DIR="SearProxy_arm-unknown-linux-musleabihf" ;;
    "linux/arm/v7") ARTIFACT_DIR="SearProxy_armv7-unknown-linux-musleabihf" ;;
    "linux/amd64") ARTIFACT_DIR="SearProxy_x86_64-unknown-linux-musl" ;;
    "linux/386") ARTIFACT_DIR="SearProxy_i686-unknown-linux-musl" ;;
    *) exit 1 ;;
esac;

cp "/opt/searproxy/binaries/$ARTIFACT_DIR/searproxy" /usr/local/bin/searproxy;
chmod ugo=rx /usr/local/bin/searproxy;
