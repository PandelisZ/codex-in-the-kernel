#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require qemu-system-aarch64
require clang
require ld.lld
require llvm-objcopy
require bindgen
require cargo
require jq
require curl
require npm
require iasl
require openssl
require cpio
require python3
require cargo-zigbuild
require rustup
require "${MAKE_BIN##*/}"

if ! command -v zig >/dev/null 2>&1; then
  ZIG_BIN_DIR=$("$SCRIPT_DIR/fetch-zig.sh")
  export PATH="$ZIG_BIN_DIR:$PATH"
fi

require zig
require docker

if [ -z "${OPENAI_API_KEY:-}" ] && [ ! -f "$HOME/.codex/auth.json" ]; then
  echo "either OPENAI_API_KEY or $HOME/.codex/auth.json must be available for guest Codex app-server auth" >&2
  exit 1
fi

rustup target add aarch64-unknown-linux-musl >/dev/null
rustup component add rust-src >/dev/null
echo "preflight ok"
