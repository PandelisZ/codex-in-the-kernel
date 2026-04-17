#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

ARCHIVE="$DOWNLOAD_DIR/$ALPINE_BASENAME"
if [ ! -f "$ARCHIVE" ]; then
  curl -L "$ALPINE_URL" -o "$ARCHIVE"
fi

echo "$ARCHIVE"
