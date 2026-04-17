#!/bin/sh
set -eu

usage() {
  cat >&2 <<'EOF'
usage: one-shot.sh [--user USER]

Run the Ubuntu desktop bootstrap from inside the guest in one command.
This script:
  1. mounts the UTM VirtioFS share at /mnt/utm-share if needed
  2. locates the staged desktop payload built on the host
  3. runs the standard installer with that payload

Build the payload on the host first:
  make desktop-payload
EOF
  exit 1
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "one-shot.sh must run as root" >&2
    exit 1
  fi
}

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../../.." && pwd)
MOUNT_POINT=${CILUX_UTM_SHARE_MOUNT:-/mnt/utm-share}
PAYLOAD_DIR=
USER_ARG=

while [ $# -gt 0 ]; do
  case "$1" in
    --user)
      [ $# -ge 2 ] || usage
      USER_ARG="--user $2"
      shift 2
      ;;
    --help|-h)
      usage
      ;;
    *)
      usage
      ;;
  esac
done

require_root

mkdir -p "$MOUNT_POINT"
if ! mountpoint -q "$MOUNT_POINT"; then
  mount -t virtiofs share "$MOUNT_POINT"
fi

if [ -d "$MOUNT_POINT/cilux/artifacts/desktop-payload" ]; then
  PAYLOAD_DIR="$MOUNT_POINT/cilux/artifacts/desktop-payload"
elif [ -d "$REPO_DIR/artifacts/desktop-payload" ]; then
  PAYLOAD_DIR="$REPO_DIR/artifacts/desktop-payload"
fi

if [ -z "$PAYLOAD_DIR" ]; then
  echo "desktop payload not found in the shared repo" >&2
  echo "run 'make desktop-payload' on the host first" >&2
  exit 1
fi

if [ -n "$USER_ARG" ]; then
  # shellcheck disable=SC2086
  exec "$SCRIPT_DIR/install.sh" --payload "$PAYLOAD_DIR" $USER_ARG
fi

exec "$SCRIPT_DIR/install.sh" --payload "$PAYLOAD_DIR"
