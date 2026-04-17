# codex-in-the-kernel

This repository packages the Cilux research harness for experimenting with a
kernel-aware Codex environment inside a disposable ARM64 VM.

The main project documentation lives in:

- [cilux/README.md](./cilux/README.md)

## Repository Shape

This repo expects:

- `cilux/` as the research harness code and documentation
- `codex/` as a Git submodule pinned to OpenAI Codex
- `linux/` as a Git submodule pinned to the Cilux Linux fork/branch used by the
  experiment

Clone with submodules:

```sh
git clone --recurse-submodules git@github.com:PandelisZ/codex-in-the-kernel.git
cd codex-in-the-kernel
```

If you already cloned without submodules:

```sh
git submodule update --init --recursive
```

## Quick Start

Once the submodules are present, the one-shot verification path is:

```sh
cilux/vm/scripts/run-e2e.sh
```

That flow builds the guest tools, builds the ARM64 kernel and modules,
assembles an initramfs, boots the VM, waits for guest `codex app-server`
readiness, and runs the websocket app-server checks.
