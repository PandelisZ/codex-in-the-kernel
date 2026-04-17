import { repoUrl } from "./content";

export type ResearchStatus = "validated" | "active" | "blocked" | "planned";

export type ResearchTrack = {
  title: string;
  status: ResearchStatus;
  summary: string;
  evidence: string[];
  next: string[];
};

export type ResearchEntry = {
  label: string;
  title: string;
  status: ResearchStatus;
  summary: string;
  evidence: string[];
  implications: string[];
  next: string[];
};

export type ResearchSource = {
  title: string;
  href: string;
  detail: string;
};

export const researchStatusLabel: Record<ResearchStatus, string> = {
  validated: "Validated",
  active: "Active",
  blocked: "Blocked",
  planned: "Planned",
};

export const researchHeroStats = [
  { value: "4 tracks", label: "The current public research areas tracked in the docs site" },
  { value: "4 entries", label: "Seeded experiment summaries grounded in the repo’s current state" },
  { value: "1 source", label: "Update docs/src/research-content.ts to publish new research notes" },
];

export const researchPrinciples = [
  {
    title: "Separate proof from speculation",
    body:
      "Validated findings should stay distinct from open questions so the site does not over-claim what the harness currently demonstrates.",
  },
  {
    title: "Keep experiments legible",
    body:
      "Each entry should explain the evidence, the implication for the agent workflow, and the next question to answer.",
  },
];

export const researchTracks: ResearchTrack[] = [
  {
    title: "Mixed shell and broker access",
    status: "validated",
    summary:
      "The core result holds: one in-guest Codex thread can inspect the guest as root and use structured broker-backed MCP tools in the same session.",
    evidence: [
      "The guest runs Codex as root inside the VM.",
      "The MCP surface exposes broker-backed kernel-adjacent reads and limited writes.",
      "The README treats the mixed-access model as the main research result so far.",
    ],
    next: [
      "Measure where structured tools outperform shell-only investigation.",
      "Add more side-by-side experiment notes that compare speed, clarity, and failure modes.",
    ],
  },
  {
    title: "Workspace and auth fidelity",
    status: "validated",
    summary:
      "The guest now uses the real host repository via 9p plus overlay, and the auth bootstrap makes the in-guest app-server usable without a manual login flow.",
    evidence: [
      "The mounted workspace path is visible in the README’s current status section.",
      "Writable overlay staging happens on top of /workspace-ro rather than a copied tree.",
      "Host Codex auth material can be injected into the guest bootstrap path.",
    ],
    next: [
      "Keep recording any guest bootstrap regressions that affect repeatability.",
      "Promote especially important bootstrap findings to the homepage when they stabilize.",
    ],
  },
  {
    title: "Structured kernel-facing surface",
    status: "active",
    summary:
      "The broker already exposes useful reads and a narrow write surface, but the research question is how far that surface should grow before it stops being explicit and auditable.",
    evidence: [
      "Broker-backed reads already cover Cilux health, snapshot state, event tails, and selected /proc or dmesg views.",
      "Operational writes currently include buffer clears and trace-mask updates.",
      "The repo treats deliberate, typed operations as the main benefit of the broker boundary.",
    ],
    next: [
      "Expand reads when they can stay explicit and reusable.",
      "Add new writes only when they are justified as first-class supported operations.",
    ],
  },
  {
    title: "Trace initialization gap",
    status: "blocked",
    summary:
      "The strongest known kernel-side gap is still the trace initialization path that reports errno -38, which limits the richer trace experiment the harness wants.",
    evidence: [
      "The README calls out trace initialization as the clearest remaining kernel-side issue.",
      "The public site already mentions the trace init unavailable message as a live limitation.",
    ],
    next: [
      "Investigate the trace init path in rust_cilux.ko.",
      "Record exactly what changed, what was tried, and whether the broker or module surface improved.",
    ],
  },
];

export const researchEntries: ResearchEntry[] = [
  {
    label: "Current baseline",
    title: "One guest thread can combine root shell inspection with MCP-backed kernel reads",
    status: "validated",
    summary:
      "This is the main public result so far. The agent is not restricted to a thin tools-only surface, but it also does not have to treat every kernel-facing action as an opaque shell command.",
    evidence: [
      "The in-guest session runs as root.",
      "The broker-backed MCP layer is live in the same session.",
      "The project now presents mixed shell plus broker access as the important research setup.",
    ],
    implications: [
      "The harness is strong enough to compare structure against raw shell access instead of arguing about those modes abstractly.",
      "Future experiments can ask whether explicit tools improve reliability or operator understanding.",
    ],
    next: [
      "Capture concrete comparison cases where shell and MCP produce different outcomes.",
    ],
  },
  {
    label: "Environment fidelity",
    title: "The guest now sees the host repo and auth state in a way that makes the environment repeatable",
    status: "validated",
    summary:
      "The harness stopped depending on a copied fallback tree and now behaves like a genuine working environment for iterative experiments.",
    evidence: [
      "The host repo mounts read-only over 9p as /workspace-ro.",
      "A writable overlay is layered at /workspace inside the guest.",
      "Guest auth bootstrap can surface the host’s Codex auth material.",
    ],
    implications: [
      "Research findings are easier to trust when the workspace path is the real repo rather than an incidental copy.",
      "The app-server and agent session can be reused without a manual auth dance on every run.",
    ],
    next: [
      "Keep documenting bootstrap failures that would make experiment results hard to reproduce.",
    ],
  },
  {
    label: "Broker surface",
    title: "Structured reads and constrained writes are already operational enough to shape the workflow",
    status: "active",
    summary:
      "The broker is no longer theoretical. It already offers a small but useful kernel-facing interface that can be observed, described, and extended deliberately.",
    evidence: [
      "Structured reads cover readiness, Cilux state, event tails, and selected kernel-adjacent system views.",
      "Structured writes currently include buffer clear and trace-mask configuration operations.",
      "The README reports successful live experiments against both operations.",
    ],
    implications: [
      "The docs site can talk about an explicit control plane instead of a planned one.",
      "Each added operation now raises a design question about auditability and scope.",
    ],
    next: [
      "Add new research entries as the read surface expands or new writes become justified.",
    ],
  },
  {
    label: "Known gap",
    title: "Trace initialization remains the clearest unresolved kernel-side blocker",
    status: "blocked",
    summary:
      "The harness is useful already, but the trace path remains incomplete and deserves explicit public tracking rather than being buried in implementation notes.",
    evidence: [
      "The current limitation is documented as trace init unavailable with errno -38.",
      "The rest of the broker and app-server surface remains operational despite that gap.",
    ],
    implications: [
      "The site can be candid about the difference between a valuable harness and a finished kernel instrumentation story.",
      "Future trace work should publish both failures and improvements here.",
    ],
    next: [
      "Attach future trace experiments to this thread of evidence so the docs show progress over time.",
    ],
  },
];

export const researchWorkflow = [
  "Treat README findings, guest test output, kernel logs, and docs under cilux/docs/ as the evidence base before writing a public claim.",
  "Update docs/src/research-content.ts first so the research page stays structured and easy to extend.",
  "Keep each new entry explicit about status: validated, active, blocked, or planned.",
  "Promote only stable, repo-backed claims into docs/src/content.ts so the homepage stays concise.",
  "Run pnpm run docs:build after changes to catch broken entrypoints or TypeScript errors.",
];

export const researchGuardrails = [
  "Do not turn a hypothesis into a validated finding unless the repo or live harness state supports it.",
  "Prefer concrete evidence over generic research language.",
  "Record the next unresolved question so the page stays useful between experiments.",
  "Keep the page additive: extend existing tracks when possible before inventing new categories.",
];

export const researchSources: ResearchSource[] = [
  {
    title: "Repository README",
    href: `${repoUrl}#readme`,
    detail:
      "The current findings, iteration notes, and known limitations are the main public evidence source for the seeded research page entries.",
  },
  {
    title: "Cilux architecture note",
    href: `${repoUrl}/blob/main/cilux/docs/architecture.md`,
    detail:
      "Use this when an experiment changes the control-plane story and the public research summary needs matching architecture context.",
  },
  {
    title: "Research publishing skill",
    href: `${repoUrl}/tree/main/.codex/skills/cilux-research-docs`,
    detail:
      "The repo-local skill explains how to turn experiment output into grounded docs updates on this page.",
  },
];
