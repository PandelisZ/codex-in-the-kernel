import "./architecture.css";
import "./styles.css";

import { renderArchitectureFigure } from "./architecture";

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
            <a class="button button-secondary" href="./research/">View research log</a>
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
