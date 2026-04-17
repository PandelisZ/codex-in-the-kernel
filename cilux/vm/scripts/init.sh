#!/bin/sh
set -eu

export PATH=/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

mount -t devtmpfs devtmpfs /dev
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t debugfs debugfs /sys/kernel/debug

HOST_EPOCH=$(sed -n 's/.*hostepoch=\([0-9][0-9]*\).*/\1/p' /proc/cmdline)
if [ -n "${HOST_EPOCH:-}" ]; then
  date -u -s "@$HOST_EPOCH" >/dev/null 2>&1 || true
fi

mkdir -p /run /tmp /var/log /workspace-ro /workspace /scratch /home/codex/.codex
mkdir -p /root/.codex
hostname cilux

ip link set lo up || true
ip link set eth0 up || true
udhcpc -i eth0 -q -n || true

if mount -t 9p -o trans=virtio,version=9p2000.L,ro src /workspace-ro; then
  mount -t tmpfs tmpfs /scratch
  mkdir -p /scratch/upper /scratch/work
  mount -t overlay overlay -o lowerdir=/workspace-ro,upperdir=/scratch/upper,workdir=/scratch/work /workspace
else
  echo "cilux: 9p mount unavailable, using rootfs workspace fallback" >/dev/kmsg
fi

chown -R 1000:1000 /home/codex /scratch || true

insmod /usr/lib/cilux/rust_cilux.ko
insmod /usr/lib/cilux/rust_minimal.ko || true
rmmod rust_minimal || true

cp /usr/share/cilux/codex-config.toml /home/codex/.codex/config.toml
chown -R 1000:1000 /home/codex

/usr/local/bin/cilux-brokerd \
  --socket /run/cilux-broker.sock \
  --audit-log /var/log/cilux-broker.log \
  --debugfs-root /sys/kernel/debug/cilux \
  >/var/log/cilux-brokerd.log 2>&1 &
BROKER_PID=$!

cp /usr/share/cilux/codex-config.toml /root/.codex/config.toml
if [ -s /etc/cilux/openai_api_key ]; then
  OPENAI_API_KEY=$(cat /etc/cilux/openai_api_key)
  export OPENAI_API_KEY
fi
HOME=/root CODEX_HOME=/root/.codex /usr/local/bin/codex app-server --listen ws://0.0.0.0:8765 --ws-auth capability-token --ws-token-file /etc/cilux/ws-token \
  >/var/log/codex-app-server.log 2>&1 &
APP_PID=$!

while kill -0 "$BROKER_PID" 2>/dev/null && kill -0 "$APP_PID" 2>/dev/null; do
  sleep 5
done

echo "critical guest service exited" >/dev/kmsg
echo "--- cilux-brokerd.log ---"
cat /var/log/cilux-brokerd.log 2>/dev/null || true
echo "--- codex-app-server.log ---"
cat /var/log/codex-app-server.log 2>/dev/null || true
exec sh
