---
name: cilux-kernel-lab
description: Explore the Cilux guest kernel from the in-VM Codex session. Use when inspecting kernel state, trace buffers, debugfs snapshots, dmesg, /proc counters, or the guest's kernel-facing MCP tools in the Cilux research harness.
---

# Cilux Kernel Lab

Use this skill inside the Cilux guest when the task is to inspect or experiment with the guest kernel and broker surface.

## Available Surface

- Full-access shell runs inside the guest as `root`.
- Cilux MCP tools:
  - `cilux_health`
  - `cilux_kernel_snapshot`
  - `cilux_events_tail`
  - `cilux_trace_configure`
  - `cilux_trace_status`
  - `cilux_trace_enable`
  - `cilux_trace_disable`
  - `cilux_trace_reset_default`
  - `cilux_buffer_clear`
  - `cilux_system_read`
- Cilux MCP resources:
  - `cilux://caps`
  - `cilux://state`
  - `cilux://events`
  - `cilux://events/{limit}`
  - `cilux://health`
  - `cilux://system/{selector}`

## Preferred Workflow

1. Start with readiness

- Call `cilux_health` first.
- Confirm `debugfs_ready` and `netlink_ready` before drawing conclusions.

2. Inspect Cilux state

- Use `cilux_kernel_snapshot` for the current `trace_mask`, counters, and capabilities.
- Use `cilux_trace_status` when you want the current enabled/supported trace categories without parsing the raw snapshot state.
- Use `cilux_events_tail` with an explicit limit such as `8`, `16`, or `32`.

3. Inspect broader kernel state

- Use `cilux_system_read` with one of:
  - `dmesg`
  - `proc_cmdline`
  - `proc_modules`
  - `proc_version`
  - `proc_meminfo`
  - `proc_loadavg`
  - `proc_uptime`
  - `proc_cpuinfo`
  - `proc_interrupts`
  - `proc_softirqs`
  - `proc_vmstat`
  - `proc_buddyinfo`
  - `proc_zoneinfo`
  - `proc_iomem`
  - `proc_ioports`
  - `proc_slabinfo`

4. Mutate carefully

- `cilux_trace_configure` changes the active trace mask.
- `cilux_trace_enable` and `cilux_trace_disable` change named trace categories without editing the raw mask directly.
- `cilux_trace_reset_default` restores the default supported trace categories.
- `cilux_buffer_clear` clears the event ring.
- When mutating, state the intended effect before the call and verify it afterward with `cilux_trace_status`, `cilux_kernel_snapshot`, or `cilux_events_tail`.

5. Prefer structured reads before ad-hoc shell

- Use MCP tools/resources for Cilux-specific state first.
- Use shell for deeper kernel experiments under `/proc`, `/sys`, debugfs, module inspection, and one-off validation.

## Reporting

- Prefer short, exact summaries with the current trace mask, event count, and relevant kernel indicators.
- Quote selector names and tool names exactly so follow-up prompts can reuse them.
