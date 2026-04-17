#!/bin/sh
set -eu

usage() {
  cat >&2 <<'EOF'
usage: install.sh [--user USER] [--payload DIR]

Install the staged Cilux Ubuntu desktop payload into the current guest.

Environment overrides:
  CILUX_DESKTOP_USER         Desktop user to target.
  CILUX_DESKTOP_PAYLOAD_DIR  Payload directory to install from.
  CILUX_OPENAI_API_KEY       Optional API key to write to /etc/cilux/openai_api_key.
EOF
  exit 1
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "install.sh must run as root" >&2
    exit 1
  fi
}

resolve_desktop_user() {
  if [ -n "${CILUX_DESKTOP_USER:-}" ]; then
    printf '%s\n' "$CILUX_DESKTOP_USER"
    return
  fi

  if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; then
    printf '%s\n' "$SUDO_USER"
    return
  fi

  awk -F: '
    $3 >= 1000 &&
    $3 < 60000 &&
    $6 ~ "^/home/" &&
    $7 !~ /(nologin|false)$/
      { print $1; exit }
  ' /etc/passwd
}

require_root

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../../.." && pwd)
PAYLOAD_DIR=${CILUX_DESKTOP_PAYLOAD_DIR:-"$REPO_DIR/cilux/artifacts/desktop-payload"}
DESKTOP_USER=${CILUX_DESKTOP_USER:-}

while [ $# -gt 0 ]; do
  case "$1" in
    --user)
      [ $# -ge 2 ] || usage
      DESKTOP_USER=$2
      shift 2
      ;;
    --payload)
      [ $# -ge 2 ] || usage
      PAYLOAD_DIR=$2
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

if [ -z "$DESKTOP_USER" ]; then
  DESKTOP_USER=$(resolve_desktop_user)
fi

if [ -z "$DESKTOP_USER" ]; then
  echo "failed to resolve a desktop user; pass --user or CILUX_DESKTOP_USER" >&2
  exit 1
fi

if ! id "$DESKTOP_USER" >/dev/null 2>&1; then
  echo "desktop user does not exist: $DESKTOP_USER" >&2
  exit 1
fi

if [ ! -d "$PAYLOAD_DIR/bin" ] || [ ! -d "$PAYLOAD_DIR/systemd" ]; then
  echo "desktop payload not found at $PAYLOAD_DIR" >&2
  echo "build it first with: make desktop-payload" >&2
  exit 1
fi

DESKTOP_HOME=$(getent passwd "$DESKTOP_USER" | cut -d: -f6)
DESKTOP_UID=$(id -u "$DESKTOP_USER")
DESKTOP_GID=$(id -g "$DESKTOP_USER")

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y bindfs openssl spice-vdagent

mkdir -p \
  /etc/cilux \
  /mnt/utm-share \
  /opt/cilux/bin \
  /root/.codex \
  /var/log \
  /workspace \
  "$DESKTOP_HOME/.codex"

install -m 0755 "$PAYLOAD_DIR/bin/"* /opt/cilux/bin/
install -m 0644 "$PAYLOAD_DIR/config/codex-config.toml" /etc/cilux/codex-config.toml
install -m 0644 "$PAYLOAD_DIR/systemd/"* /etc/systemd/system/
install -m 0644 /etc/cilux/codex-config.toml /root/.codex/config.toml
install -o "$DESKTOP_UID" -g "$DESKTOP_GID" -m 0644 \
  /etc/cilux/codex-config.toml \
  "$DESKTOP_HOME/.codex/config.toml"

if [ -n "${CILUX_OPENAI_API_KEY:-}" ]; then
  printf '%s\n' "$CILUX_OPENAI_API_KEY" > /etc/cilux/openai_api_key
  chmod 0600 /etc/cilux/openai_api_key
fi

if [ ! -f /root/.codex/auth.json ] && [ -f "$DESKTOP_HOME/.codex/auth.json" ]; then
  install -m 0600 "$DESKTOP_HOME/.codex/auth.json" /root/.codex/auth.json
fi

if [ ! -s /etc/cilux/ws-token ]; then
  umask 077
  openssl rand -hex 32 > /etc/cilux/ws-token
  chmod 0600 /etc/cilux/ws-token
fi

cat > /etc/cilux/desktop.env <<EOF
DESKTOP_USER=$DESKTOP_USER
DESKTOP_HOME=$DESKTOP_HOME
DESKTOP_UID=$DESKTOP_UID
DESKTOP_GID=$DESKTOP_GID
WORKSPACE_SOURCE=/mnt/utm-share
WORKSPACE_TARGET=/workspace
EOF

if systemctl list-unit-files | grep -q '^spice-vdagentd.service'; then
  systemctl enable --now spice-vdagentd.service >/dev/null 2>&1 || true
fi

systemctl daemon-reload
systemctl enable mnt-utm\\x2dshare.mount cilux-workspace.service cilux-brokerd.service cilux-codex-app-server.service >/dev/null
systemctl start mnt-utm\\x2dshare.mount
systemctl restart cilux-workspace.service
systemctl restart cilux-brokerd.service

if /opt/cilux/bin/cilux-auth-check; then
  systemctl restart cilux-codex-app-server.service
else
  echo "cilux: no guest Codex auth found; cilux-codex-app-server.service is enabled but not started" >&2
fi

echo "installed Cilux desktop payload for $DESKTOP_USER from $PAYLOAD_DIR"
