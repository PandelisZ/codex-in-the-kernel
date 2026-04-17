# Cilux Architecture

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
- `cilux-mcp`
  - unprivileged stdio MCP server launched by Codex on demand
  - forwards tool calls and resource reads to `cilux-brokerd`
  - exposes Cilux snapshot/event/trace controls plus curated kernel-adjacent
    guest reads such as `dmesg`, `/proc/modules`, `/proc/vmstat`, and
    `/proc/zoneinfo`
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

## Safety boundaries

- No arbitrary shell is exposed through the broker.
- `system_read` only permits curated selectors such as `dmesg` and selected
  `/proc` snapshots.
- Kernel mutation is limited to the Cilux trace mask and event-buffer clear.
