export type NavItem = {
  id: string;
  label: string;
  eyebrow: string;
};

export type Stat = {
  value: string;
  label: string;
};

export type HighlightCard = {
  title: string;
  body: string;
};

export type StackItem = {
  name: string;
  detail: string;
};

export type RepoPointer = {
  title: string;
  href: string;
  description: string;
};

const defaultRepoUrl = "https://github.com/PandelisZ/codex-in-the-kernel";

export const repoUrl = import.meta.env.VITE_REPO_URL ?? defaultRepoUrl;

export const navItems: NavItem[] = [
  { id: "why", label: "Why", eyebrow: "Problem" },
  { id: "architecture", label: "Architecture", eyebrow: "System" },
  { id: "validated", label: "What Works", eyebrow: "Findings" },
  { id: "safety", label: "Boundaries", eyebrow: "Guardrails" },
  { id: "limitations", label: "Limitations", eyebrow: "Reality" },
  { id: "possible", label: "What May Be Possible", eyebrow: "Research" },
  { id: "repo", label: "Repo", eyebrow: "Pointers" },
];

export const heroStats: Stat[] = [
  { value: "1 VM", label: "Disposable ARM64 guest running the whole experiment" },
  { value: "2 paths", label: "Shell access and broker-backed MCP in the same session" },
  { value: "6 tools", label: "Current structured MCP controls and kernel-adjacent reads" },
  { value: "0 kernel agents", label: "Codex stays in userspace; the kernel remains an observed target" },
];

export const thesisCards: HighlightCard[] = [
  {
    title: "Research thesis",
    body:
      "A local coding agent can become materially more useful for systems work if it sees live kernel state and a narrow control plane, without requiring the agent itself to execute in kernel space.",
  },
  {
    title: "Current claim",
    body:
      "The harness already demonstrates that a single in-guest Codex thread can combine root shell inspection with explicit broker-mediated kernel reads and limited writes.",
  },
];

export const whyCards: HighlightCard[] = [
  {
    title: "Shell-only is too loose",
    body:
      "Raw shell access is powerful, but it makes kernel-facing operations implicit, ad hoc, and hard to audit. The interesting question is not whether shell works. It is whether the kernel surface can become legible to the agent.",
  },
  {
    title: "Tool-only is too narrow",
    body:
      "A purely tool-mediated system can be safer, but it strips away the normal debugging context that systems work depends on. Kernel research still benefits from `/proc`, `dmesg`, and direct process-level inspection.",
  },
  {
    title: "Brokered dual-access is the experiment",
    body:
      "Cilux is testing the combination: keep the shell for normal investigation, but route the deliberate kernel-facing operations through a constrained broker that can be reasoned about, audited, and expanded carefully.",
  },
];

export const controlPlane: StackItem[] = [
  {
    name: "rust_cilux.ko",
    detail:
      "Records selected kernel events into a bounded ring buffer, exposes debugfs state, and offers Generic Netlink operations for trace-mask updates and buffer clears.",
  },
  {
    name: "cilux-brokerd",
    detail:
      "Runs as root inside the guest, reads debugfs, performs constrained netlink writes, serves newline-delimited JSON RPC, and records an audit trail.",
  },
  {
    name: "cilux-mcp",
    detail:
      "Runs unprivileged as a stdio MCP server launched by Codex, exposing structured tools and resources backed by the broker.",
  },
  {
    name: "codex app-server",
    detail:
      "Runs inside the VM on port 8765 and is configured for danger-full-access so the in-guest agent can combine MCP with root shell access without approval stalls.",
  },
];

export const hostIntegration: string[] = [
  "QEMU/HVF boots the custom ARM64 kernel and an external initramfs.",
  "The host repo is exported into the guest over virtio 9p as `/workspace-ro`.",
  "A scratch tmpfs upperdir overlays that export to produce writable `/workspace` inside the guest.",
  "Host-side tests talk to the guest app-server through forwarded port `8765`.",
];

export const currentSurface: string[] = [
  "Structured tools: `cilux_health`, `cilux_kernel_snapshot`, `cilux_events_tail`, `cilux_trace_configure`, `cilux_buffer_clear`, and `cilux_system_read`.",
  "Curated reads: `dmesg`, `/proc/modules`, `/proc/meminfo`, `/proc/vmstat`, `/proc/buddyinfo`, `/proc/zoneinfo`, and other selected `/proc` snapshots.",
  "Resources: broker-backed `cilux://caps`, `cilux://state`, `cilux://events`, `cilux://health`, plus templated events and system selectors.",
];

export const validatedFindings: HighlightCard[] = [
  {
    title: "The mixed-access model is live",
    body:
      "The guest agent can use raw root shell inspection and structured broker-mediated MCP operations in the same thread, which is the core research capability this repo set out to validate.",
  },
  {
    title: "The workspace path is real",
    body:
      "The host repo is mounted read-only over 9p and overlaid into a writable guest workspace instead of silently falling back to a copied rootfs tree.",
  },
  {
    title: "Auth injection works",
    body:
      "The guest bootstrap can copy host Codex auth material or inject an API key, making the in-guest app-server viable without resorting to a one-off manual login flow.",
  },
  {
    title: "Structured reads are already useful",
    body:
      "The MCP layer can surface broker readiness, Cilux state, event tails, and curated kernel-adjacent reads, which means the agent can ask for structured kernel context instead of scraping every read from raw text.",
  },
  {
    title: "Structured writes are intentionally small but real",
    body:
      "The broker-backed write surface currently supports event-buffer clears and trace-mask updates. That is narrow by design, but it is already operational and auditable.",
  },
  {
    title: "The end-to-end path is green",
    body:
      "The guest app-server answers `/readyz`, websocket tests pass, and the harness is usable as a repeatable kernel-aware agent environment rather than a one-off demo.",
  },
];

export const safetyBoundaries: string[] = [
  "Codex stays in guest userspace. The project is not attempting to run the agent inside the kernel.",
  "The broker does not expose arbitrary shell execution or arbitrary file writes.",
  "Kernel mutation is deliberately constrained to trace-mask updates and event-buffer clears.",
  "Curated `system_read` selectors are explicit rather than open-ended, keeping the kernel-facing surface narrow enough to audit.",
  "This is a research harness, not a hardened production isolation boundary or general kernel management plane.",
];

export const limitations: HighlightCard[] = [
  {
    title: "Trace initialization is still incomplete",
    body:
      "The kernel sample still logs `trace init unavailable (errno -38)`, which means the trace subscription path the project wants is not fully wired up yet.",
  },
  {
    title: "The broker write surface is narrow on purpose",
    body:
      "That limitation is part of the design, but it also means deeper kernel control has not yet been justified, modelled, or exposed as first-class operations.",
  },
  {
    title: "The environment is deliberately permissive",
    body:
      "The in-guest session is configured for danger-full-access so the research can move quickly. That is valuable for experimentation, but it is not evidence of production-grade containment.",
  },
];

export const possibilities: HighlightCard[] = [
  {
    title: "Richer kernel telemetry",
    body:
      "The broker-backed read surface could expand into more targeted `/proc`, `/sys`, or Cilux-specific projections so the agent can ask for kernel state in higher-level, typed forms.",
  },
  {
    title: "Explicit, auditable control operations",
    body:
      "If deeper kernel control becomes worth it, the next move is to widen the broker API deliberately rather than letting shell access silently become the control plane.",
  },
  {
    title: "Shell versus tools as a research dimension",
    body:
      "This setup is strong because it lets the same agent compare raw shell investigation with explicit tool usage and reveal where structure actually improves speed, reliability, or safety.",
  },
  {
    title: "A more capable agent systems lab",
    body:
      "If the trace path and structured state keep improving, Cilux becomes more than a demo: it becomes a place to study what local agents can do when the kernel is observable, queryable, and partially steerable.",
  },
];

export const repoPointers: RepoPointer[] = [
  {
    title: "Repository overview",
    href: repoUrl,
    description:
      "The repo root explains the thesis, the current guest surface, the latest findings, and the end-to-end harness flow.",
  },
  {
    title: "Harness source",
    href: `${repoUrl}/tree/main/cilux`,
    description:
      "The `cilux/` directory holds the VM harness, guest utilities, tests, and internal documentation for the experiment.",
  },
  {
    title: "Internal architecture notes",
    href: `${repoUrl}/blob/main/cilux/docs/architecture.md`,
    description:
      "The engineering architecture note remains separate from this site and is still the concise internal reference for the broker and guest control plane.",
  },
];
