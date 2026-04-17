#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

CODEX_BIN=$("$SCRIPT_DIR/fetch-codex.sh")
"$SCRIPT_DIR/build-guest.sh"

rm -rf "$DESKTOP_PAYLOAD_DIR"
mkdir -p \
  "$DESKTOP_PAYLOAD_DIR/bin" \
  "$DESKTOP_PAYLOAD_DIR/config" \
  "$DESKTOP_PAYLOAD_DIR/systemd"

cp "$GUEST_TARGET_DIR/cilux-brokerd" "$DESKTOP_PAYLOAD_DIR/bin/"
cp "$GUEST_TARGET_DIR/cilux-mcp" "$DESKTOP_PAYLOAD_DIR/bin/"
cp "$GUEST_TARGET_DIR/ciluxctl" "$DESKTOP_PAYLOAD_DIR/bin/"
cp "$CODEX_BIN" "$DESKTOP_PAYLOAD_DIR/bin/codex"
chmod +x "$DESKTOP_PAYLOAD_DIR/bin/"*

cp "$DESKTOP_ASSETS_DIR/codex-config.toml" "$DESKTOP_PAYLOAD_DIR/config/"
cp "$DESKTOP_ASSETS_DIR/bin/"* "$DESKTOP_PAYLOAD_DIR/bin/"
cp "$DESKTOP_ASSETS_DIR/systemd/"* "$DESKTOP_PAYLOAD_DIR/systemd/"
chmod +x "$DESKTOP_PAYLOAD_DIR/bin/"*

rm -f "$DESKTOP_PAYLOAD_TARBALL"
tar -czf "$DESKTOP_PAYLOAD_TARBALL" -C "$DESKTOP_PAYLOAD_DIR" .

echo "$DESKTOP_PAYLOAD_DIR"
