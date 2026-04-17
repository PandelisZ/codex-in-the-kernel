#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
HARNESS_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
REPO_DIR=$(CDPATH= cd -- "$HARNESS_DIR/.." && pwd)

if [ -d "$HOME/.cargo/bin" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

ARTIFACT_DIR=${ARTIFACT_DIR:-"$HARNESS_DIR/artifacts"}
DOWNLOAD_DIR=${DOWNLOAD_DIR:-"$ARTIFACT_DIR/downloads"}
BUILD_DIR=${BUILD_DIR:-"$ARTIFACT_DIR/build"}
ROOTFS_DIR=${ROOTFS_DIR:-"$ARTIFACT_DIR/rootfs"}
RUN_DIR=${RUN_DIR:-"$HARNESS_DIR/run"}
TOOLS_DIR=${TOOLS_DIR:-"$ARTIFACT_DIR/tools"}

if [ -f "$TOOLS_DIR/zig/zig" ]; then
  export PATH="$TOOLS_DIR/zig:$PATH"
  export CARGO_ZIGBUILD_ZIG_PATH="$TOOLS_DIR/zig/zig"
fi

KERNEL_BUILD_DIR=${KERNEL_BUILD_DIR:-"$BUILD_DIR/linux-arm64"}
GUEST_TARGET_DIR=${GUEST_TARGET_DIR:-"$REPO_DIR/cilux/guest/target/aarch64-unknown-linux-musl/release"}
INITRAMFS_PATH=${INITRAMFS_PATH:-"$ARTIFACT_DIR/initramfs.cpio.gz"}
QEMU_PID_FILE=${QEMU_PID_FILE:-"$RUN_DIR/qemu.pid"}
SERIAL_LOG=${SERIAL_LOG:-"$RUN_DIR/serial.log"}
QEMU_LOG=${QEMU_LOG:-"$RUN_DIR/qemu.log"}
KERNEL_JOBS=${KERNEL_JOBS:-8}
ALPINE_VERSION=${ALPINE_VERSION:-"3.21.3"}
ALPINE_FLAVOR=${ALPINE_FLAVOR:-"aarch64"}
ALPINE_BASENAME="alpine-minirootfs-$ALPINE_VERSION-$ALPINE_FLAVOR.tar.gz"
ALPINE_URL=${ALPINE_URL:-"https://dl-cdn.alpinelinux.org/alpine/v3.21/releases/$ALPINE_FLAVOR/$ALPINE_BASENAME"}
CODEX_PACKAGE=${CODEX_PACKAGE:-"@openai/codex-linux-arm64"}
CODEX_RELEASE_URL=${CODEX_RELEASE_URL:-"https://github.com/openai/codex/releases/latest/download/codex-aarch64-unknown-linux-musl.tar.gz"}
KERNEL_BUILDER_IMAGE=${KERNEL_BUILDER_IMAGE:-"cilux-kernel-builder:latest"}

LLVM_PREFIX=${LLVM_PREFIX:-$(brew --prefix llvm 2>/dev/null || true)}
LIBELF_PREFIX=${LIBELF_PREFIX:-$(brew --prefix libelf 2>/dev/null || true)}
MAKE_BIN=${MAKE_BIN:-$(command -v gmake 2>/dev/null || command -v make)}
if [ -n "${LLVM_PREFIX}" ] && [ -d "${LLVM_PREFIX}/bin" ]; then
  export PATH="${LLVM_PREFIX}/bin:$PATH"
  if [ -d "${LLVM_PREFIX}/lib" ]; then
    export LIBCLANG_PATH="${LLVM_PREFIX}/lib"
  fi
fi

HOSTCFLAGS_EXTRA=${HOSTCFLAGS_EXTRA:-}
HOSTLDFLAGS_EXTRA=${HOSTLDFLAGS_EXTRA:-}
HOSTCFLAGS_EXTRA="${HOSTCFLAGS_EXTRA} -I${REPO_DIR}/cilux/vm/host-include"
if [ -n "${LIBELF_PREFIX}" ] && [ -d "${LIBELF_PREFIX}/include" ]; then
  HOSTCFLAGS_EXTRA="${HOSTCFLAGS_EXTRA} -I${LIBELF_PREFIX}/include"
fi
if [ -n "${LIBELF_PREFIX}" ] && [ -d "${LIBELF_PREFIX}/lib" ]; then
  HOSTLDFLAGS_EXTRA="${HOSTLDFLAGS_EXTRA} -L${LIBELF_PREFIX}/lib"
fi

mkdir -p "$ARTIFACT_DIR" "$DOWNLOAD_DIR" "$BUILD_DIR" "$RUN_DIR" "$TOOLS_DIR"
