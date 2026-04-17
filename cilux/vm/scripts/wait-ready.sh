#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

ATTEMPTS=${ATTEMPTS:-120}
SLEEP_SECS=${SLEEP_SECS:-2}

i=0
while [ "$i" -lt "$ATTEMPTS" ]; do
  if curl -fsS http://127.0.0.1:8765/readyz >/dev/null 2>&1; then
    echo "app-server ready"
    exit 0
  fi
  i=$((i + 1))
  sleep "$SLEEP_SECS"
done

echo "timed out waiting for guest app-server readiness" >&2
exit 1
