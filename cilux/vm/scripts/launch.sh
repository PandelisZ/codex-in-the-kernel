#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

mkdir -p "$RUN_DIR"
rm -f "$QEMU_PID_FILE" "$SERIAL_LOG" "$QEMU_LOG"
HOST_EPOCH=$(date -u +%s)

qemu-system-aarch64 \
  -machine virt,accel=hvf \
  -cpu max \
  -smp 4 \
  -m 4096 \
  -display none \
  -rtc base=utc,clock=host \
  -kernel "$KERNEL_BUILD_DIR/arch/arm64/boot/Image" \
  -initrd "$INITRAMFS_PATH" \
  -append "console=ttyAMA0 rdinit=/init hostepoch=$HOST_EPOCH" \
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:8765-:8765 \
  -device virtio-net-device,netdev=net0 \
  -fsdev local,id=src,path="$REPO_DIR",security_model=none,readonly=on \
  -device virtio-9p-device,fsdev=src,mount_tag=src \
  -serial "file:$SERIAL_LOG" \
  -monitor none \
  -pidfile "$QEMU_PID_FILE" \
  -daemonize \
  >"$QEMU_LOG" 2>&1

echo "started qemu pid $(cat "$QEMU_PID_FILE")"
