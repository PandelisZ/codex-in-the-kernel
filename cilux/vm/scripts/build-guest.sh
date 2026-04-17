#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

cargo zigbuild \
  --manifest-path "$REPO_DIR/cilux/guest/Cargo.toml" \
  --release \
  --target aarch64-unknown-linux-musl
