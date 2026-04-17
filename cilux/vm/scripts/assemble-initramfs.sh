#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

ALPINE_ARCHIVE=$("$SCRIPT_DIR/fetch-alpine.sh")
CODEX_BIN=$("$SCRIPT_DIR/fetch-codex.sh")
RUST_CILUX_KO=${RUST_CILUX_KO:-"$KERNEL_BUILD_DIR/samples/rust/rust_cilux.ko"}
RUST_MINIMAL_KO=${RUST_MINIMAL_KO:-"$KERNEL_BUILD_DIR/samples/rust/rust_minimal.ko"}
HOST_CODEX_AUTH=${HOST_CODEX_AUTH:-"$HOME/.codex/auth.json"}

rm -rf "$ROOTFS_DIR"
mkdir -p "$ROOTFS_DIR"
tar -xzf "$ALPINE_ARCHIVE" -C "$ROOTFS_DIR"

mkdir -p \
  "$ROOTFS_DIR/etc/cilux" \
  "$ROOTFS_DIR/usr/local/bin" \
  "$ROOTFS_DIR/usr/lib/cilux" \
  "$ROOTFS_DIR/usr/share/cilux" \
  "$ROOTFS_DIR/home/codex/.codex" \
  "$ROOTFS_DIR/root/.codex" \
  "$ROOTFS_DIR/var/log"

cp "$CODEX_BIN" "$ROOTFS_DIR/usr/local/bin/codex"
chmod +x "$ROOTFS_DIR/usr/local/bin/codex"

cp "$GUEST_TARGET_DIR/cilux-brokerd" "$ROOTFS_DIR/usr/local/bin/cilux-brokerd"
cp "$GUEST_TARGET_DIR/cilux-mcp" "$ROOTFS_DIR/usr/local/bin/cilux-mcp"
cp "$GUEST_TARGET_DIR/ciluxctl" "$ROOTFS_DIR/usr/local/bin/ciluxctl"
chmod +x \
  "$ROOTFS_DIR/usr/local/bin/cilux-brokerd" \
  "$ROOTFS_DIR/usr/local/bin/cilux-mcp" \
  "$ROOTFS_DIR/usr/local/bin/ciluxctl"

cp "$REPO_DIR/cilux/vm/scripts/init.sh" "$ROOTFS_DIR/init"
cp "$REPO_DIR/cilux/vm/scripts/codex-config.toml" \
  "$ROOTFS_DIR/usr/share/cilux/codex-config.toml"
chmod +x "$ROOTFS_DIR/init"

printf '%s\n' "${OPENAI_API_KEY:-}" > "$ROOTFS_DIR/etc/cilux/openai_api_key"
openssl rand -hex 32 > "$ROOTFS_DIR/etc/cilux/ws-token"
if [ -f "$HOST_CODEX_AUTH" ]; then
  cp "$HOST_CODEX_AUTH" "$ROOTFS_DIR/root/.codex/auth.json"
fi

cat >> "$ROOTFS_DIR/etc/passwd" <<'EOF'
codex:x:1000:1000:Codex:/home/codex:/bin/sh
EOF
cat >> "$ROOTFS_DIR/etc/group" <<'EOF'
codex:x:1000:
EOF

cp "$RUST_CILUX_KO" "$ROOTFS_DIR/usr/lib/cilux/"
cp "$RUST_MINIMAL_KO" "$ROOTFS_DIR/usr/lib/cilux/"

(cd "$ROOTFS_DIR" && find . -print | cpio -o -H newc | gzip -9) > "$INITRAMFS_PATH"
echo "$INITRAMFS_PATH"
