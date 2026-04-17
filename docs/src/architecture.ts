export const renderArchitectureFigure = (): string => `
  <figure class="architecture-figure reveal">
    <div class="architecture-diagram">
      <section class="architecture-panel architecture-panel--host" aria-labelledby="architecture-host-title">
        <div class="architecture-panel__header">
          <div class="architecture-panel__copy">
            <p class="architecture-kicker">Host machine</p>
            <h3 id="architecture-host-title" class="architecture-panel__title">Boots the VM and exposes the workspace.</h3>
            <p class="architecture-panel__body">
              QEMU/HVF launches the disposable ARM64 guest, forwards the guest app-server port, and shares the repository
              through a 9p mount that becomes a writable overlay inside the VM.
            </p>
          </div>
        </div>

        <div class="architecture-chip-row" aria-label="Host integration details">
          <article class="architecture-chip">
            <strong>port 8765</strong>
            <span>guest app-server forwarded to the host</span>
          </article>
          <article class="architecture-chip">
            <strong>/workspace-ro over 9p</strong>
            <span>host repo mounted read-only inside the guest</span>
          </article>
          <article class="architecture-chip">
            <strong>overlay -&gt; /workspace</strong>
            <span>scratch tmpfs turns that mount into a writable workspace</span>
          </article>
        </div>
      </section>

      <section class="architecture-panel architecture-panel--guest" aria-labelledby="architecture-guest-title">
        <div class="architecture-panel__header architecture-panel__header--guest">
          <div class="architecture-panel__copy">
            <p class="architecture-kicker">Guest VM</p>
            <h3 id="architecture-guest-title" class="architecture-panel__title">Inside the guest, Codex gets two access styles.</h3>
            <p class="architecture-panel__body">
              Direct shell access stays available for fast systems investigation. Structured kernel-facing operations stay on
              a narrower broker path.
            </p>
          </div>

          <div class="architecture-thread-pill">One Codex session uses both paths</div>
        </div>

        <div class="architecture-lane-grid">
          <section class="architecture-track architecture-track--shell" aria-labelledby="architecture-shell-label">
            <p id="architecture-shell-label" class="architecture-track__label">Direct shell path</p>
            <article class="architecture-node architecture-node--shell">
              <h4>Root shell access</h4>
              <ul class="architecture-list">
                <li><code>/proc</code>, <code>/sys</code>, debugfs, and <code>dmesg</code></li>
                <li>Fast reads from the running guest</li>
                <li>Useful for ad-hoc systems debugging</li>
              </ul>
            </article>
          </section>

          <section class="architecture-track architecture-track--broker" aria-labelledby="architecture-broker-label">
            <p id="architecture-broker-label" class="architecture-track__label">Structured broker path</p>

            <div class="architecture-flow">
              <article class="architecture-node architecture-node--service">
                <p class="architecture-node__eyebrow">Userspace MCP</p>
                <h4>cilux-mcp</h4>
                <ul class="architecture-list">
                  <li>Health, snapshot, and event tools</li>
                  <li>Curated kernel-adjacent reads</li>
                  <li>Launched in the Codex session</li>
                </ul>
              </article>

              <div class="architecture-flow__arrow" aria-hidden="true">
                <span>&rarr;</span>
              </div>

              <article class="architecture-node architecture-node--service">
                <p class="architecture-node__eyebrow">Privileged broker</p>
                <h4>cilux-brokerd</h4>
                <ul class="architecture-list">
                  <li>Audit trail for each request</li>
                  <li>Constrained debugfs reads</li>
                  <li>Narrow Generic Netlink writes</li>
                </ul>
              </article>
            </div>

            <div class="architecture-bridge">
              <span>typed reads + limited writes</span>
            </div>

            <article class="architecture-node architecture-node--kernel">
              <p class="architecture-node__eyebrow">Kernel target</p>
              <h4>rust_cilux.ko</h4>
              <div class="architecture-tag-row" aria-label="Kernel capabilities">
                <span>debugfs state</span>
                <span>Generic Netlink controls</span>
                <span>bounded event ring</span>
              </div>
            </article>
          </section>
        </div>
      </section>
    </div>

    <figcaption>
      Cilux keeps Codex in guest userspace while making the kernel-facing surface clearer: shell for broad inspection,
      broker-backed MCP for explicit structured operations.
    </figcaption>
  </figure>
`;
