# codex-in-the-kernel

This repository packages the Cilux research harness for experimenting with a
kernel-aware Codex environment inside a disposable ARM64 VM.

The main experiment code lives in `cilux/`, while `codex/` and `linux/` are
checked out as submodules pinned to the versions used by the harness.

## Clone

Clone with submodules:

```sh
git clone --recurse-submodules git@github.com:PandelisZ/codex-in-the-kernel.git
cd codex-in-the-kernel
```

If you already cloned without submodules:

```sh
git submodule update --init --recursive
```

## Repository Shape

- `cilux/`
  - the research harness itself
- `codex/`
  - Git submodule pinned to OpenAI Codex
- `linux/`
  - Git submodule pinned to the Linux tree used by the Cilux kernel sample

## What This Is

`cilux/` is a disposable ARM64 Linux VM harness for experimenting with a
kernel-aware Codex environment.

The project combines:

- a custom Linux Rust sample module (`rust_cilux.ko`)
- a small privileged broker inside the guest (`cilux-brokerd`)
- a guest-side MCP server (`cilux-mcp`)
- stock `codex app-server` running inside the VM
- a QEMU/HVF host harness that boots the guest, forwards the app-server port,
  and exposes the host repo into the guest over virtio 9p

The current guest Codex session is intentionally configured for
`danger-full-access` inside the VM so the in-guest agent can use both root
shell access and the curated kernel-facing MCP tools without interactive
approval prompts.

## Research Goal

The core question behind this project is:

Can we give a local coding agent meaningful kernel visibility and a constrained
kernel control plane without moving the agent itself into kernel space?

The current design answers that by keeping Codex in guest userspace while
bridging into kernel state through a narrow, auditable broker.

More concretely, the experiment is trying to validate whether this setup is
useful for:

- inspecting live kernel state from an agent session
- reading structured kernel telemetry instead of scraping ad-hoc logs
- exposing explicit kernel control operations as tools
- combining normal shell access with safer structured kernel operations
- using the same agent session for both software engineering tasks and
  low-level systems investigation

## Non-Goals

This harness is not trying to:

- run Codex inside the kernel
- expose arbitrary broker-side shell execution
- provide a hardened production isolation boundary
- support every possible kernel debug/control surface
- replace normal kernel development workflows

This is a research harness, not a production security product.

## High-Level Design

### Guest control plane

- `rust_cilux.ko`
  - records selected kernel events into a bounded ring buffer
  - exposes `caps.json`, `state.json`, and `events.ndjson` in debugfs
  - exposes a Generic Netlink family for trace-mask changes, buffer clears, and
    basic status queries
- `cilux-brokerd`
  - runs as root inside the guest
  - binds `/run/cilux-broker.sock`
  - serves newline-delimited JSON RPC
  - reads Cilux debugfs state
  - performs constrained netlink control operations
  - records an audit trail for every request
- `cilux-mcp`
  - runs as an unprivileged stdio MCP server launched by Codex on demand
  - exposes structured tools/resources backed by `cilux-brokerd`
- `codex app-server`
  - runs inside the guest on port `8765`
  - is authenticated with a capability token file
  - is configured for `danger-full-access` in this VM

### Host integration

- QEMU/HVF boots a custom ARM64 kernel and external initramfs.
- The host source tree is exported read-only into the guest over virtio 9p.
- The guest mounts that export at `/workspace-ro`.
- The guest overlays a scratch tmpfs upperdir over `/workspace-ro` to produce a
  writable `/workspace`.
- Host tests talk to `ws://127.0.0.1:8765` through QEMU port forwarding.

## Why The Broker Exists

The broker is the most important design decision in the project.

Instead of letting the agent poke arbitrary kernel interfaces directly through a
bag of shell commands, the broker gives the experiment:

- a narrow list of supported kernel operations
- a place to log and audit those operations
- a structured JSON boundary between the agent and kernel-facing code
- a way to expose kernel state as MCP tools/resources instead of raw text only

This lets the experiment compare two access styles in the same guest:

- unrestricted in-guest shell access
- explicit structured kernel operations through the broker

## `cilux/` Layout

- `cilux/vm/`
  - host-side build, fetch, assemble, and launch scripts
- `cilux/guest/`
  - static guest utilities:
    - `cilux-brokerd`
    - `cilux-mcp`
    - `ciluxctl`
- `cilux/tests/`
  - host-side websocket app-server checks
- `cilux/docs/`
  - architecture and interface notes

## Current Guest Surface

### Full-access shell

Inside the guest, Codex can execute normal shell commands as `root` in
`danger-full-access` mode.

That means the agent can directly inspect:

- `/proc`
- `/sys`
- `/sys/kernel/debug/cilux`
- mounted workspace state
- kernel logs via `dmesg`
- loaded modules via `/proc/modules`

### MCP tools

The current Cilux MCP server exposes:

- `cilux_health`
- `cilux_kernel_snapshot`
- `cilux_events_tail`
- `cilux_trace_configure`
- `cilux_buffer_clear`
- `cilux_system_read`

### Curated `cilux_system_read` selectors

- `dmesg`
- `proc_modules`
- `proc_meminfo`
- `proc_loadavg`
- `proc_uptime`
- `proc_cpuinfo`
- `proc_interrupts`
- `proc_vmstat`
- `proc_buddyinfo`
- `proc_zoneinfo`

### MCP resources

- `cilux://caps`
- `cilux://state`
- `cilux://events`
- `cilux://health`
- `cilux://system/dmesg`
- `cilux://system/proc_modules`
- `cilux://system/proc_meminfo`
- `cilux://system/proc_loadavg`
- `cilux://system/proc_uptime`
- `cilux://system/proc_cpuinfo`
- `cilux://system/proc_interrupts`
- `cilux://system/proc_vmstat`
- `cilux://system/proc_buddyinfo`
- `cilux://system/proc_zoneinfo`

### MCP resource templates

- `cilux://events/{limit}`
- `cilux://system/{selector}`

### Guest skill

The repo includes a guest-facing Codex skill at:

- `.codex/skills/cilux-kernel-lab/SKILL.md`

This skill tells the in-guest agent how to approach the kernel/broker surface:

- start with `cilux_health`
- inspect state with `cilux_kernel_snapshot`
- use `cilux_system_read` for broader kernel-adjacent context
- treat `cilux_trace_configure` and `cilux_buffer_clear` as explicit writes

## Auth Model

The guest app-server currently supports two ways to authenticate Codex:

- `OPENAI_API_KEY`
- a copied host `~/.codex/auth.json`

During initramfs assembly:

- `~/.codex/auth.json` is copied into the guest as `/root/.codex/auth.json`
  when present
- `OPENAI_API_KEY` is written into `/etc/cilux/openai_api_key` when present

The VM bootstrap accepts either one.

## Prerequisites

`cilux/vm/scripts/preflight.sh` currently requires:

- `qemu-system-aarch64`
- `clang`
- `ld.lld`
- `llvm-objcopy`
- `bindgen`
- `cargo`
- `jq`
- `curl`
- `npm`
- `iasl`
- `openssl`
- `cpio`
- `python3`
- `cargo-zigbuild`
- `rustup`
- `make` or `gmake`
- `zig`
- `docker`

It also requires one of:

- `OPENAI_API_KEY` in the environment
- `~/.codex/auth.json` on the host

## Build And Run Flow

### One-shot end-to-end run

The simplest path is:

```sh
cilux/vm/scripts/run-e2e.sh
```

That script runs:

1. `preflight.sh`
2. `build-guest.sh`
3. `build-kernel.sh`
4. `assemble-initramfs.sh`
5. `launch.sh`
6. `wait-ready.sh`
7. `python3 cilux/tests/app_server_e2e.py`

### Incremental workflow

If you want to drive the harness manually:

```sh
cilux/vm/scripts/preflight.sh
cilux/vm/scripts/build-guest.sh
cilux/vm/scripts/build-kernel.sh
cilux/vm/scripts/assemble-initramfs.sh
cilux/vm/scripts/launch.sh
cilux/vm/scripts/wait-ready.sh
python3 cilux/tests/app_server_e2e.py
```

Stop the VM with:

```sh
cilux/vm/scripts/stop.sh
```

## Artifacts

By default the harness writes:

- kernel build output:
  - `cilux/artifacts/build/linux-arm64`
- rootfs staging:
  - `cilux/artifacts/rootfs`
- initramfs:
  - `cilux/artifacts/initramfs.cpio.gz`
- downloaded dependencies:
  - `cilux/artifacts/downloads`
- runtime logs:
  - `cilux/run/serial.log`
  - `cilux/run/qemu.log`

## What The Harness Boots Today

The current host launch path uses:

- `qemu-system-aarch64`
- `-machine virt,accel=hvf`
- `-cpu max`
- `-smp 4`
- `-m 4096`
- forwarded guest port `8765`
- virtio-net
- virtio 9p host export
- serial log captured to `cilux/run/serial.log`

The guest kernel and initramfs are loaded explicitly with:

- `-kernel <build>/arch/arm64/boot/Image`
- `-initrd <artifacts>/initramfs.cpio.gz`

## Current Verification

The websocket e2e currently checks three main behaviors:

1. Positive tool use

- Codex starts a thread in `danger-full-access`
- uses `cilux_kernel_snapshot`
- uses `cilux_events_tail`
- returns a coherent three-line summary

2. Negative write failure surfacing

- Codex calls `cilux_trace_configure` with an invalid mask
- the exact failure is surfaced back to the user

3. Health plus curated system reads

- Codex uses `cilux_health`
- Codex uses `cilux_system_read` with `proc_modules`
- confirms `debugfs_ready`
- confirms `netlink_ready`
- confirms `rust_cilux` appears in `/proc/modules`

## What We Found So Far

### 1. Full access mode really works

This is not just a config flag on paper.

In the running guest, Codex was observed issuing real shell commands as root.
For example, a direct probe executed:

- `/bin/sh -lc whoami`

and returned:

- `root`

That means in-guest Codex can use:

- direct shell access
- MCP tools/resources
- both within the same session

### 2. The workspace mount path now works

The host repo is now mounted into the guest as:

```text
src on /workspace-ro type 9p (ro,relatime,access=client,trans=virtio)
overlay on /workspace type overlay (rw,relatime,lowerdir=/workspace-ro,upperdir=/scratch/upper,workdir=/scratch/work,uuid=on)
```

That is a major result because earlier iterations were falling back to the
rootfs copy of `/workspace`.

### 3. The in-guest auth material is injected correctly

The guest currently sees:

- `/root/.codex/auth.json`

when the host has `~/.codex/auth.json`.

That makes the guest app-server viable without forcing API-key-only auth.

### 4. Structured MCP reads are useful

The new MCP surface now supports:

- broker readiness
- Cilux snapshot/state reads
- ring-buffer tail reads
- curated kernel-adjacent reads from `/proc` and `dmesg`

In practice, this let the agent read:

- `proc_modules`
- `proc_interrupts`
- `proc_vmstat`

through structured tool calls rather than shell-only scraping.

### 5. Structured MCP writes are useful too

The write surface is still intentionally small, but it is now operational:

- `cilux_buffer_clear`
- `cilux_trace_configure`

In live experiments:

- `cilux_buffer_clear` returned `ok: true`
- `cilux_events_tail` showed an empty list immediately afterward
- `cilux_trace_configure` successfully applied `trace_mask = 15`

### 6. Mixed shell + broker access is the interesting part

This is probably the most important research result so far.

The guest agent can now do both of these in the same thread:

- raw shell inspection as `root`
- structured broker-mediated kernel reads/writes through MCP

That is a much stronger research setup than either of these alone:

- shell-only access
- tool-only access

### 7. The broker boundary still matters

Even though the guest session is full access, the broker is still useful
because it keeps the kernel-facing surface explicit.

Today it still constrains:

- what kernel state is exposed as first-class tools
- what writes are treated as supported control operations
- how those operations are logged and audited

### 8. End-to-end verification is green

The current live status after the latest iterations is:

- `/readyz` returns `200 OK`
- `python3 cilux/tests/app_server_e2e.py` passes
- the guest app-server is reachable at `ws://127.0.0.1:8765`

## Important Iteration Findings

These were the main issues uncovered and fixed while bringing the harness up:

- MCP tool calls were stalling on approvals until the guest thread and guest
  Codex config were moved to `danger-full-access`.
- The 9p workspace path was not actually available until the kernel build was
  forced to keep:
  - `NETWORK_FILESYSTEMS`
  - `9P_FS`
  - `TMPFS`
  - `TTY`
  - PL011 console support
- The overlay workspace setup initially failed because `/scratch/upper` and
  `/scratch/work` were created before mounting tmpfs on `/scratch`.
- Recursive `chown -R /workspace` in guest init was walking the mounted repo and
  causing startup problems.
- Packaging `.ko` files from `linux/samples/rust/*.ko` was unsafe because those
  artifacts could be stale.
- The correct reliable packaging source is now:
  - `cilux/artifacts/build/linux-arm64/samples/rust/*.ko`

## Known Limitations

### `trace init unavailable`

The guest still logs:

```text
rust_cilux: cilux: trace init unavailable (errno -38), continuing without live trace subscriptions
```

This means the current kernel module is still not fully wired up for the trace
path the experiment wants.

The rest of the broker/tool surface still works, but this is the clearest next
kernel-side target.

### Broker write surface is intentionally narrow

Right now the broker can mutate:

- the trace mask
- the event ring buffer

That is by design, but if the research goal shifts toward deeper kernel control,
the next step is to deliberately extend the broker API rather than relying only
on raw shell access.

### This is not a hardened production sandbox

The guest Codex session is deliberately full access. That is useful for
research, but it is not a claim of production-hard isolation.

## Suggested Next Experiments

- Expand broker-backed reads for more kernel diagnostics under `/proc`, `/sys`,
  or Cilux-specific debugfs projections.
- Add more structured kernel writes only when they can be justified as explicit,
  auditable operations.
- Investigate the `trace init unavailable (errno -38)` path in `rust_cilux.ko`.
- Compare what the agent can do faster or more reliably with shell access versus
  structured MCP tools.
- Add more end-to-end assertions that combine shell, MCP reads, and MCP writes
  in the same session.

## Short Status Summary

Today the research harness successfully demonstrates:

- a full-access in-guest Codex session running as root
- a working 9p-mounted host workspace with writable overlay
- injected guest auth material for Codex
- a structured kernel-facing MCP surface for reads and limited writes
- passing websocket app-server end-to-end tests

That is enough to make the environment genuinely useful as a kernel-aware agent
research sandbox, even though the trace-init path is still incomplete.
