#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"

MINIMAL_CONFIG="$REPO_DIR/cilux/vm/config/kernel-arm64-cilux-minimal.config"
rm -rf "$KERNEL_BUILD_DIR"
mkdir -p "$KERNEL_BUILD_DIR"

docker build -t "$KERNEL_BUILDER_IMAGE" -f "$REPO_DIR/cilux/vm/docker/kernel-builder.Dockerfile" "$REPO_DIR/cilux/vm/docker"

docker run --rm --platform linux/arm64 \
  -v "$REPO_DIR:/workspace" \
  -w /workspace \
  "$KERNEL_BUILDER_IMAGE" \
  bash -lc "
    set -eu
    ulimit -n 65536 || true
    CONTAINER_BUILD_DIR=/workspace/cilux/artifacts/build/linux-arm64
    CONTAINER_MINIMAL_CONFIG=/workspace/cilux/vm/config/kernel-arm64-cilux-minimal.config
    mkdir -p \"\$CONTAINER_BUILD_DIR\"
    make -C /workspace/linux O=\"\$CONTAINER_BUILD_DIR\" ARCH=arm64 LLVM=1 KCONFIG_ALLCONFIG=\"\$CONTAINER_MINIMAL_CONFIG\" allnoconfig
    # Force the guest workspace mount stack on after allnoconfig so menu-gated
    # dependencies do not silently drop 9p and tmpfs support.
    /workspace/linux/scripts/config --file \"\$CONTAINER_BUILD_DIR/.config\" \
      -e NETWORK_FILESYSTEMS \
      -e SAMPLES \
      -e RUST \
      -e DEBUG_FS \
      -e TRACEPOINTS \
      -e NET \
      -e SHMEM \
      -e TMPFS \
      -e TMPFS_POSIX_ACL \
      -e NET_9P \
      -e NET_9P_VIRTIO \
      -e 9P_FS \
      -e 9P_FS_POSIX_ACL \
      -e TTY \
      -e VIRTIO_CONSOLE \
      -e SERIAL_AMBA_PL011 \
      -e SERIAL_AMBA_PL011_CONSOLE \
      -e SAMPLES_RUST \
      -m SAMPLE_RUST_MINIMAL \
      -m SAMPLE_RUST_CILUX
    make -C /workspace/linux O=\"\$CONTAINER_BUILD_DIR\" ARCH=arm64 LLVM=1 olddefconfig
    make -C /workspace/linux O=\"\$CONTAINER_BUILD_DIR\" ARCH=arm64 LLVM=1 -j$KERNEL_JOBS Image.gz
    make -C /workspace/linux O=\"\$CONTAINER_BUILD_DIR\" ARCH=arm64 LLVM=1 -j$KERNEL_JOBS modules
  "
