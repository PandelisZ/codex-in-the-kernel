#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

TARBALL="$DOWNLOAD_DIR/codex-aarch64-unknown-linux-musl.tar.gz"
EXTRACTED="$DOWNLOAD_DIR/codex-aarch64-unknown-linux-musl"

if [ ! -f "$TARBALL" ]; then
  curl -fL "$CODEX_RELEASE_URL" -o "$TARBALL"
fi

rm -rf "$EXTRACTED"
mkdir -p "$EXTRACTED"
tar -xzf "$TARBALL" -C "$EXTRACTED"

find "$EXTRACTED" -type f -name 'codex*' | head -n 1
