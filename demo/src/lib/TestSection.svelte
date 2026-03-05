<script>
  import { sectionCounts } from "../stores/suite.svelte.js";
  import { SECTION_INFO } from "../constants.js";

  let { sectionKey, children } = $props();

  const info = $derived(SECTION_INFO[sectionKey]);
  const counts = $derived($sectionCounts[sectionKey] ?? { pass: 0, fail: 0, total: 0, experimental: 0 });
</script>

<section class="section-block" id={sectionKey}>
  <div class="section-header">
    <div class="section-header-left">
      <span class="section-chip">{info?.chip ?? sectionKey}</span>
      <h2 class="section-title">{info?.title ?? sectionKey}</h2>
      <p class="section-desc">{info?.description ?? ""}</p>
    </div>
    <div class="section-summary">
      {#if counts.pass > 0}
        <span class="count-badge pass">{counts.pass} pass</span>
      {/if}
      {#if counts.fail > 0}
        <span class="count-badge fail">{counts.fail} fail</span>
      {/if}
      {#if counts.experimental > 0}
        <span class="count-badge exp">{counts.experimental} exp</span>
      {/if}
    </div>
  </div>

  <div class="section-grid">
    {@render children?.()}
  </div>
</section>

<style>
  .section-block {
    margin-bottom: 40px;
    scroll-margin-top: 20px;
  }
  .section-header {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 16px;
    padding: 16px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius);
  }
  .section-header-left { flex: 1; display: flex; flex-direction: column; gap: 4px; }
  .section-chip {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent);
  }
  .section-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    line-height: 1.3;
  }
  .section-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.5;
    margin-top: 2px;
  }
  .section-summary {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-end;
    flex-shrink: 0;
  }
  .count-badge {
    padding: 2px 8px;
    border-radius: 100px;
    font-size: 10px;
    font-weight: 700;
    white-space: nowrap;
  }
  .count-badge.pass { background: rgba(61, 214, 140, 0.12); color: var(--green); }
  .count-badge.fail { background: rgba(240, 82, 82, 0.12); color: var(--red); }
  .count-badge.exp { background: rgba(233, 177, 14, 0.1); color: var(--yellow); }

  .section-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 12px;
  }
</style>
