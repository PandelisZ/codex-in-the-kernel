#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

INDEX_JSON="$DOWNLOAD_DIR/zig-index.json"
if [ ! -f "$INDEX_JSON" ]; then
  curl -L https://ziglang.org/download/index.json -o "$INDEX_JSON"
fi

ZIG_VERSION=${ZIG_VERSION:-$(jq -r 'to_entries | map(select(.key != "master"))[0].key' "$INDEX_JSON")}
ZIG_TARBALL_URL=$(jq -r --arg version "$ZIG_VERSION" '.[$version]["aarch64-macos"].tarball' "$INDEX_JSON")
ZIG_ARCHIVE="$DOWNLOAD_DIR/zig-$ZIG_VERSION-aarch64-macos.tar.xz"
ZIG_DIR="$TOOLS_DIR/zig"

if [ ! -f "$ZIG_ARCHIVE" ]; then
  curl -L "$ZIG_TARBALL_URL" -o "$ZIG_ARCHIVE"
fi

rm -rf "$TOOLS_DIR"/zig-*
tar -xJf "$ZIG_ARCHIVE" -C "$TOOLS_DIR"
EXTRACTED=$(find "$TOOLS_DIR" -maxdepth 1 -type d -name "zig-*" | head -n 1)
rm -rf "$ZIG_DIR"
mv "$EXTRACTED" "$ZIG_DIR"

echo "$ZIG_DIR"
