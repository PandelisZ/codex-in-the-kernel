#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

"$SCRIPT_DIR/preflight.sh"
"$SCRIPT_DIR/build-guest.sh"
"$SCRIPT_DIR/build-kernel.sh"
"$SCRIPT_DIR/assemble-initramfs.sh"
"$SCRIPT_DIR/launch.sh"
trap '"$SCRIPT_DIR/stop.sh"' EXIT INT TERM
"$SCRIPT_DIR/wait-ready.sh"
python3 "$REPO_DIR/cilux/tests/app_server_e2e.py"
