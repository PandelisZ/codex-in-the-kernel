#!/bin/sh
set -eu

if [ "$(id -u)" -ne 0 ]; then
  echo "smoke.sh should run as root so it can read broker health directly" >&2
  exit 1
fi

failures=0

check() {
  label=$1
  shift
  if "$@"; then
    printf 'ok: %s\n' "$label"
  else
    printf 'fail: %s\n' "$label" >&2
    failures=$((failures + 1))
  fi
}

check_sh() {
  label=$1
  command=$2
  if sh -lc "$command"; then
    printf 'ok: %s\n' "$label"
  else
    printf 'fail: %s\n' "$label" >&2
    failures=$((failures + 1))
  fi
}

check "utm share mount active" mountpoint -q /mnt/utm-share
check "workspace bind mount active" mountpoint -q /workspace
check "workspace path writable" test -w /workspace
check "broker service active" systemctl is-active --quiet cilux-brokerd.service
check "broker socket present" test -S /run/cilux-broker.sock
check_sh "health reports stock-kernel desktop mode" '/opt/cilux/bin/ciluxctl health | grep -q "\"guest_mode\": \"desktop_stock_kernel\""'
check_sh "health reports system_read capability" '/opt/cilux/bin/ciluxctl health | grep -q "\"system_read\": true"'
check_sh "health hides kernel snapshot capability" '/opt/cilux/bin/ciluxctl health | grep -q "\"kernel_snapshot\": false"'
check "proc_modules system read works" sh -lc '/opt/cilux/bin/ciluxctl system-read --selector proc_modules >/dev/null'

if /opt/cilux/bin/cilux-auth-check; then
  check "codex app-server active" systemctl is-active --quiet cilux-codex-app-server.service
else
  printf 'skip: codex app-server active check (guest auth missing)\n'
fi

if [ "$failures" -ne 0 ]; then
  exit 1
fi
