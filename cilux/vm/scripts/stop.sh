#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

if [ -f "$QEMU_PID_FILE" ]; then
  kill "$(cat "$QEMU_PID_FILE")" || true
  rm -f "$QEMU_PID_FILE"
fi
