import "./styles.css";
import "./research.css";

import { repoUrl } from "./content";
import {
  researchEntries,
  researchGuardrails,
  researchHeroStats,
  researchPrinciples,
  researchSources,
  researchStatusLabel,
  researchTracks,
  researchWorkflow,
  type ResearchStatus,
} from "./research-content";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing #app root");
}

const navItems = [
  { id: "tracks", label: "Tracks", eyebrow: "Focus" },
  { id: "log", label: "Experiment Log", eyebrow: "Entries" },
  { id: "workflow", label: "Publishing Flow", eyebrow: "Process" },
  { id: "sources", label: "Sources", eyebrow: "Inputs" },
];

const renderList = (items: string[], className = "list"): string =>
  `<ul class="${className}">${items.map((item) => `<li>${item}</li>`).join("")}</ul>`;

const renderStatusChip = (status: ResearchStatus): string =>
  `<span class="status-chip status-chip--${status}">${researchStatusLabel[status]}</span>`;

app.innerHTML = `
  <div class="page-shell">
    <div class="ambient ambient-top"></div>
    <div class="ambient ambient-bottom"></div>

    <header class="hero">
      <p class="eyebrow">Research log</p>
      <div class="hero-layout">
        <div class="hero-copy">
          <a class="hero-kicker" href="../">codex-in-the-kernel / research</a>
          <h1>Experiments, findings, and the next kernel questions.</h1>
          <p class="hero-summary">
            This page is the structured companion to the homepage: a place to track what the harness has
            actually validated, what is still being explored, and what evidence backs each claim.
          </p>
          <p class="hero-detail">
            Future research updates should land here first so the public narrative stays honest. Stable,
            repo-backed claims can then graduate to the main docs overview.
          </p>
          <div class="hero-actions">
            <a class="button button-primary" href="../">Back to overview</a>
            <a class="button button-secondary" href="${repoUrl}">Open the repo</a>
          </div>
        </div>

        <aside class="hero-panel reveal research-summary-card">
          <div>
            <p class="panel-label">Editorial stance</p>
            <div class="hero-panel-grid">
              ${researchPrinciples
                .map(
                  (principle) => `
                    <article class="mini-card">
                      <h2>${principle.title}</h2>
                      <p>${principle.body}</p>
                    </article>
                  `,
                )
                .join("")}
            </div>
          </div>
        </aside>
      </div>

      <div class="stats-grid research-stats reveal">
        ${researchHeroStats
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
      <aside class="section-nav research-nav">
        <p class="panel-label">Research map</p>
        <nav aria-label="Research section navigation">
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

      <main class="main-content research-main">
        <section id="tracks" class="section" data-section="tracks">
          <div class="section-heading reveal">
            <p class="eyebrow">Current tracks</p>
            <h2>The research is strongest when each line of inquiry stays explicit.</h2>
            <p>
              These tracks organize the current public state of the harness so new experiments extend a known
              thread instead of becoming disconnected notes.
            </p>
          </div>
          <div class="card-grid track-grid">
            ${researchTracks
              .map(
                (track) => `
                  <article class="card reveal track-card">
                    <header>
                      <h3>${track.title}</h3>
                      ${renderStatusChip(track.status)}
                    </header>
                    <p>${track.summary}</p>
                    <div class="detail-grid detail-grid--two">
                      <section class="detail-panel">
                        <h4>Evidence</h4>
                        ${renderList(track.evidence, "list compact-list")}
                      </section>
                      <section class="detail-panel">
                        <h4>Next</h4>
                        ${renderList(track.next, "list compact-list")}
                      </section>
                    </div>
                  </article>
                `,
              )
              .join("")}
          </div>
        </section>

        <section id="log" class="section" data-section="log">
          <div class="section-heading reveal">
            <p class="eyebrow">Experiment log</p>
            <h2>Seed the page with repo-backed summaries, then extend it as the harness evolves.</h2>
            <p>
              Each entry should say what changed, what supports the claim, and what the next unresolved question is.
            </p>
          </div>
          <div class="research-stack">
            ${researchEntries
              .map(
                (entry) => `
                  <article class="panel reveal entry-card">
                    <p class="entry-label">${entry.label}</p>
                    <header>
                      <h3>${entry.title}</h3>
                      ${renderStatusChip(entry.status)}
                    </header>
                    <p>${entry.summary}</p>
                    <div class="detail-grid">
                      <section class="detail-panel">
                        <h4>Evidence</h4>
                        ${renderList(entry.evidence, "list compact-list")}
                      </section>
                      <section class="detail-panel">
                        <h4>Implications</h4>
                        ${renderList(entry.implications, "list compact-list")}
                      </section>
                      <section class="detail-panel">
                        <h4>Next</h4>
                        ${renderList(entry.next, "list compact-list")}
                      </section>
                    </div>
                  </article>
                `,
              )
              .join("")}
          </div>
        </section>

        <section id="workflow" class="section" data-section="workflow">
          <div class="section-heading reveal">
            <p class="eyebrow">Publishing flow</p>
            <h2>Use the docs site as a structured research surface, not a dumping ground.</h2>
            <p>
              The goal is to keep the public story precise while experiments are still moving quickly inside the repo.
            </p>
          </div>
          <div class="two-column workflow-grid">
            <article class="panel reveal">
              <p class="panel-label">Update workflow</p>
              ${renderList(researchWorkflow)}
            </article>
            <article class="panel reveal">
              <p class="panel-label">Guardrails</p>
              ${renderList(researchGuardrails)}
            </article>
          </div>
        </section>

        <section id="sources" class="section" data-section="sources">
          <div class="section-heading reveal">
            <p class="eyebrow">Source material</p>
            <h2>Keep claims tied to evidence that already exists in the repo.</h2>
            <p>
              These are the main places to read before publishing or revising a research summary on this page.
            </p>
          </div>
          <div class="card-grid source-grid">
            ${researchSources
              .map(
                (source) => `
                  <article class="card reveal">
                    <h3>${source.title}</h3>
                    <p>${source.detail}</p>
                    <a class="inline-link" href="${source.href}">${source.href.replace("https://github.com/", "")}</a>
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
