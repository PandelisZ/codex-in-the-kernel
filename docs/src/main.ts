import "./styles.css";

import {
  controlPlane,
  currentSurface,
  heroStats,
  hostIntegration,
  limitations,
  navItems,
  possibilities,
  repoPointers,
  repoUrl,
  safetyBoundaries,
  thesisCards,
  validatedFindings,
  whyCards,
} from "./content";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing #app root");
}

const renderCardGrid = (
  cards: Array<{ title: string; body: string }>,
  className = "",
): string =>
  `<div class="card-grid ${className}">${cards
    .map(
      (card) => `
        <article class="card reveal">
          <h3>${card.title}</h3>
          <p>${card.body}</p>
        </article>
      `,
    )
    .join("")}</div>`;

const renderList = (items: string[], className = "list"): string =>
  `<ul class="${className}">${items.map((item) => `<li>${item}</li>`).join("")}</ul>`;

const renderArchitectureFigure = (): string => `
  <figure class="architecture-figure reveal">
    <svg viewBox="0 0 1080 620" role="img" aria-labelledby="arch-title arch-desc">
      <title id="arch-title">Codex-in-the-kernel architecture diagram</title>
      <desc id="arch-desc">
        The host boots a disposable guest VM. Inside the guest, Codex can inspect through shell access
        and through a structured MCP path that reaches a broker and then the kernel module.
      </desc>
      <defs>
        <linearGradient id="surface" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#fff8f0" />
          <stop offset="100%" stop-color="#f0e0cc" />
        </linearGradient>
        <linearGradient id="guest" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#fef4df" />
          <stop offset="100%" stop-color="#f7d7b4" />
        </linearGradient>
        <linearGradient id="kernel" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#d4ebe1" />
          <stop offset="100%" stop-color="#9bcab9" />
        </linearGradient>
        <marker id="arrow" markerWidth="12" markerHeight="12" refX="9" refY="6" orient="auto">
          <path d="M0,0 L12,6 L0,12 z" fill="#195a67" />
        </marker>
      </defs>

      <rect x="24" y="32" width="1032" height="556" rx="28" fill="url(#surface)" stroke="#ceb28f" />

      <rect x="64" y="76" width="952" height="148" rx="24" fill="#f6ecdf" stroke="#d4b894" />
      <text x="96" y="118" class="svg-title">Host machine</text>
      <text x="96" y="156" class="svg-body">QEMU/HVF boots the guest, forwards port 8765,</text>
      <text x="96" y="184" class="svg-body">and exports the repo over virtio 9p.</text>
      <rect x="708" y="112" width="252" height="72" rx="20" fill="#ffffff" stroke="#b99a71" />
      <text x="734" y="146" class="svg-label">/workspace-ro over 9p</text>
      <text x="734" y="170" class="svg-small">overlay scratch creates writable /workspace</text>

      <rect x="64" y="258" width="952" height="288" rx="28" fill="url(#guest)" stroke="#d09862" />
      <text x="96" y="304" class="svg-title">Guest VM</text>
      <text x="96" y="338" class="svg-body">Codex stays in userspace and sees two complementary access paths.</text>

      <rect x="110" y="372" width="244" height="106" rx="24" fill="#fffaf4" stroke="#bf8b55" />
      <text x="138" y="408" class="svg-label">Root shell access</text>
      <text x="138" y="432" class="svg-small">/proc, /sys, debugfs, dmesg</text>
      <text x="138" y="456" class="svg-small">normal systems investigation</text>

      <rect x="420" y="372" width="244" height="106" rx="24" fill="#fffaf4" stroke="#bf8b55" />
      <text x="448" y="408" class="svg-label">cilux-mcp</text>
      <text x="448" y="432" class="svg-small">structured tools and resources</text>
      <text x="448" y="456" class="svg-small">health, snapshot, events, reads, writes</text>

      <rect x="730" y="372" width="232" height="106" rx="24" fill="#fffaf4" stroke="#bf8b55" />
      <text x="758" y="408" class="svg-label">cilux-brokerd</text>
      <text x="758" y="432" class="svg-small">root daemon with audit trail</text>
      <text x="758" y="456" class="svg-small">debugfs + netlink boundary</text>

      <rect x="420" y="494" width="542" height="74" rx="20" fill="url(#kernel)" stroke="#4f8f7c" />
      <text x="448" y="528" class="svg-label">Kernel target</text>
      <text x="448" y="552" class="svg-small">rust_cilux.ko, debugfs state, Generic Netlink controls, bounded event ring</text>

      <path d="M354 425 H400" stroke="#195a67" stroke-width="4" marker-end="url(#arrow)" />
      <path d="M664 425 H710" stroke="#195a67" stroke-width="4" marker-end="url(#arrow)" />
      <path d="M846 478 V496" stroke="#195a67" stroke-width="4" marker-end="url(#arrow)" />
      <path d="M542 478 V496" stroke="#195a67" stroke-width="4" marker-end="url(#arrow)" />

      <text x="370" y="398" class="svg-small">same Codex thread</text>
      <text x="688" y="398" class="svg-small">structured broker path</text>
    </svg>
    <figcaption>
      The point of Cilux is not to move Codex into the kernel. It is to give a userspace agent a clearer,
      more auditable way to ask the kernel questions and perform a few carefully chosen control operations.
    </figcaption>
  </figure>
`;

app.innerHTML = `
  <div class="page-shell">
    <div class="ambient ambient-top"></div>
    <div class="ambient ambient-bottom"></div>

    <header class="hero">
      <p class="eyebrow">Kernel-aware agent research</p>
      <div class="hero-layout">
        <div class="hero-copy">
          <a class="hero-kicker" href="${repoUrl}">codex-in-the-kernel</a>
          <h1>Codex in the kernel.</h1>
          <p class="hero-summary">
            Cilux is a disposable ARM64 VM harness exploring whether a local coding agent becomes meaningfully
            better at systems work when it can inspect live kernel state and use a narrow, explicit control plane.
          </p>
          <p class="hero-detail">
            The current design keeps Codex in guest userspace, adds a privileged broker for the kernel-facing edge,
            and compares structured MCP operations against direct shell investigation in the same session.
          </p>
          <div class="hero-actions">
            <a class="button button-primary" href="#architecture">Read the architecture</a>
            <a class="button button-secondary" href="${repoUrl}">Open the repo</a>
          </div>
        </div>

        <div class="hero-panel reveal">
          <p class="panel-label">Why this matters</p>
          <div class="hero-panel-grid">
            ${thesisCards
              .map(
                (card) => `
                  <article class="mini-card">
                    <h2>${card.title}</h2>
                    <p>${card.body}</p>
                  </article>
                `,
              )
              .join("")}
          </div>
        </div>
      </div>

      <div class="stats-grid reveal">
        ${heroStats
          .map(
            (stat) => `
              <article class="stat-card">
                <strong>${stat.value}</strong>
                <span>${stat.label}</span>
              </article>
            `,
          )
          .join("")}
      </div>
    </header>

    <div class="content-shell">
      <aside class="section-nav">
        <p class="panel-label">Research map</p>
        <nav aria-label="Section navigation">
          <ul>
            ${navItems
              .map(
                (item) => `
                  <li>
                    <a href="#${item.id}" data-nav-link="${item.id}">
                      <span>${item.eyebrow}</span>
                      <strong>${item.label}</strong>
                    </a>
                  </li>
                `,
              )
              .join("")}
          </ul>
        </nav>
      </aside>

      <main class="main-content">
        <section id="why" class="section" data-section="why">
          <div class="section-heading reveal">
            <p class="eyebrow">Why this project exists</p>
            <h2>The experiment is about legibility, not just privilege.</h2>
            <p>
              Cilux exists because neither extreme is satisfying. Shell-only access is powerful but opaque.
              Tool-only access is safer but too thin for real systems work. The brokered design is testing whether
              a local agent can have both context and restraint.
            </p>
          </div>
          ${renderCardGrid(whyCards)}
        </section>

        <section id="architecture" class="section" data-section="architecture">
          <div class="section-heading reveal">
            <p class="eyebrow">System architecture</p>
            <h2>A disposable guest with a narrow kernel-facing broker.</h2>
            <p>
              The guest control plane is intentionally layered so the structured kernel edge stays explicit even
              when the in-guest Codex session itself has full shell access.
            </p>
          </div>
          ${renderArchitectureFigure()}
          <div class="two-column">
            <article class="panel reveal">
              <p class="panel-label">Guest control plane</p>
              <div class="stack-list">
                ${controlPlane
                  .map(
                    (item) => `
                      <section class="stack-item">
                        <h3>${item.name}</h3>
                        <p>${item.detail}</p>
                      </section>
                    `,
                  )
                  .join("")}
              </div>
            </article>
            <article class="panel reveal">
              <p class="panel-label">Host integration</p>
              ${renderList(hostIntegration)}
              <div class="surface-block">
                <p class="panel-label">Current guest surface</p>
                ${renderList(currentSurface, "list compact-list")}
              </div>
            </article>
          </div>
        </section>

        <section id="validated" class="section" data-section="validated">
          <div class="section-heading reveal">
            <p class="eyebrow">What works today</p>
            <h2>The harness has already crossed the “interesting research artifact” line.</h2>
            <p>
              These results are validated in the current repository state and come directly from the working
              end-to-end setup rather than from speculative design notes.
            </p>
          </div>
          ${renderCardGrid(validatedFindings, "findings-grid")}
        </section>

        <section id="safety" class="section" data-section="safety">
          <div class="section-heading reveal">
            <p class="eyebrow">Safety boundaries and non-goals</p>
            <h2>The broker still matters even in a full-access guest.</h2>
            <p>
              The presence of shell access does not make the broker redundant. It makes the broker more valuable as
              the place where kernel-facing operations become first-class, constrained, and auditable.
            </p>
          </div>
          <article class="panel reveal">
            ${renderList(safetyBoundaries)}
          </article>
        </section>

        <section id="limitations" class="section" data-section="limitations">
          <div class="section-heading reveal">
            <p class="eyebrow">Known limitations</p>
            <h2>The research result is real, and so are the gaps.</h2>
            <p>
              The site is intentionally candid about what remains unfinished. The harness is useful now, but it is
              not pretending to be deeper, safer, or more complete than the repo demonstrates today.
            </p>
          </div>
          ${renderCardGrid(limitations)}
          <pre class="trace-block reveal"><code>rust_cilux: cilux: trace init unavailable (errno -38), continuing without live trace subscriptions</code></pre>
        </section>

        <section id="possible" class="section" data-section="possible">
          <div class="section-heading reveal">
            <p class="eyebrow">What may be possible</p>
            <h2>The next step is not more power. It is better structure.</h2>
            <p>
              The promising path forward is to expose more kernel context and control only when it can be justified
              as explicit, typed, and inspectable. That keeps the research honest and makes comparisons meaningful.
            </p>
          </div>
          ${renderCardGrid(possibilities)}
        </section>

        <section id="repo" class="section" data-section="repo">
          <div class="section-heading reveal">
            <p class="eyebrow">Repo pointers</p>
            <h2>Where to go next in the codebase.</h2>
            <p>
              This site is a compact public narrative. The repository remains the canonical technical source for the
              exact interfaces, implementation details, and current experimental state.
            </p>
          </div>
          <div class="card-grid repo-grid">
            ${repoPointers
              .map(
                (pointer) => `
                  <article class="card reveal">
                    <h3>${pointer.title}</h3>
                    <p>${pointer.description}</p>
                    <a class="inline-link" href="${pointer.href}">${pointer.href.replace("https://github.com/", "")}</a>
                  </article>
                `,
              )
              .join("")}
          </div>
        </section>
      </main>
    </div>
  </div>
`;

const revealObserver = new IntersectionObserver(
  (entries, observer) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) {
        return;
      }

      entry.target.classList.add("is-visible");
      observer.unobserve(entry.target);
    });
  },
  {
    threshold: 0.18,
  },
);

document.querySelectorAll<HTMLElement>(".reveal").forEach((element) => {
  revealObserver.observe(element);
});

const navLinkById = new Map(
  [...document.querySelectorAll<HTMLAnchorElement>("[data-nav-link]")].map((link) => [
    link.dataset.navLink ?? "",
    link,
  ]),
);

const setActiveSection = (id: string) => {
  navLinkById.forEach((link, linkId) => {
    link.classList.toggle("is-active", linkId === id);
  });
};

const sections = [...document.querySelectorAll<HTMLElement>("[data-section]")];

const updateActiveSection = () => {
  const offset = 160;
  let activeSection = sections[0];

  sections.forEach((section) => {
    if (section.getBoundingClientRect().top - offset <= 0) {
      activeSection = section;
    }
  });

  if (activeSection) {
    setActiveSection(activeSection.id);
  }
};

window.addEventListener("scroll", updateActiveSection, { passive: true });
window.addEventListener("resize", updateActiveSection);

updateActiveSection();
