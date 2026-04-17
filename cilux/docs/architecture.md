# Cilux Architecture

## Supported guest modes

- Research kernel VM
  - QEMU/HVF booting the custom ARM64 kernel plus Alpine initramfs
  - full Cilux kernel surface: debugfs snapshots, event ring, and Generic
    Netlink trace control
- Ubuntu desktop VM
  - stock Ubuntu 24.04.x Desktop ARM64 running in UTM on the Apple
    virtualization backend
  - keeps the broker, MCP server, CLI, and Codex app-server
  - relies on `cilux_health` capability reporting and a dynamic MCP catalog to
    hide trace/debugfs-backed surfaces when the custom kernel integration is
    absent

## Guest control plane

- `rust_cilux.ko`
  - records selected kernel events into a bounded ring buffer
  - exposes `caps.json`, `state.json`, and `events.ndjson` in debugfs
  - exposes a Generic Netlink family for trace-mask changes, buffer clears, and
    basic status queries
- `cilux-brokerd`
  - root-owned daemon bound to `/run/cilux-broker.sock`
  - serves newline-delimited JSON RPC
  - reads debugfs snapshots and uses Generic Netlink for control operations
  - records an audit trail for every request
  - reports `guest_mode` plus per-feature availability so higher layers can
    distinguish a research-kernel guest from a stock desktop guest
- `cilux-mcp`
  - unprivileged stdio MCP server launched by Codex on demand
  - forwards tool calls and resource reads to `cilux-brokerd`
  - exposes Cilux snapshot/event/trace controls, named trace enable/disable
    helpers, and curated kernel-adjacent guest reads such as `dmesg`,
    `/proc/softirqs`, `/proc/vmstat`, and `/proc/slabinfo`
  - dynamically trims its catalog when snapshot, event-tail, or trace-control
    features are unavailable on the current guest
- `codex app-server`
  - unprivileged websocket server on guest port `8765`
  - authenticated with a capability token file
  - configured for `danger-full-access` so in-guest Codex can use MCP tools and
    root shell access without interactive approval prompts

## Host integration

- QEMU/HVF boots a custom ARM64 kernel and external initramfs.
- The host source tree is exported read-only into the guest over virtio 9p.
- The guest overlays a scratch tmpfs upperdir over `/workspace-ro` to produce
  `/workspace`.
- Host tests connect to `ws://127.0.0.1:8765` through QEMU port forwarding.
- The UTM desktop path stages a repo-built payload into a stock Ubuntu Desktop
  guest, mounts the host repo over VirtioFS at `/mnt/utm-share`, and remaps it
  to `/workspace` through bindfs for the selected desktop user.

## Safety boundaries

- No arbitrary shell is exposed through the broker.
- `system_read` only permits curated selectors such as `dmesg` and selected
  `/proc` snapshots.
- Kernel mutation is limited to trace-control operations over the Cilux trace
  mask plus event-buffer clear.
