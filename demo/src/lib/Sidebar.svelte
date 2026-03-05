<script>
  import DiagnosticsPanel from "./DiagnosticsPanel.svelte";
  import { suiteCounts, suiteStatus, sectionCounts, simd, runtimeStatusText } from "../stores/suite.svelte.js";
  import { SECTION_ORDER, SECTION_INFO } from "../constants.js";

  let { onRerun } = $props();

  function scrollTo(id) {
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
</script>

<aside class="sidebar">
  <!-- Project identity -->
  <div class="sidebar-brand">
    <span class="brand-name">kimg</span>
    <span class="brand-subtitle">visual test suite</span>
  </div>

  <!-- Runtime stats -->
  <div class="stats-block">
    <div class="stat-row">
      <span class="stat-label">Status</span>
      <span class="stat-value status-{$suiteStatus}">
        <span class="dot {$suiteStatus === 'running' ? 'running' : $suiteStatus === 'done' ? 'pass' : $suiteStatus === 'failed' ? 'fail' : 'pending'}"></span>
        {$runtimeStatusText}
      </span>
    </div>
    <div class="stat-row">
      <span class="stat-label">SIMD</span>
      <span class="stat-value">{$simd}</span>
    </div>
    <div class="stat-divider"></div>
    <div class="counts-grid">
      <div class="count-cell">
        <span class="count-num">{$suiteCounts.total}</span>
        <span class="count-label">total</span>
      </div>
      <div class="count-cell pass">
        <span class="count-num">{$suiteCounts.pass}</span>
        <span class="count-label">pass</span>
      </div>
      <div class="count-cell fail">
        <span class="count-num">{$suiteCounts.fail}</span>
        <span class="count-label">fail</span>
      </div>
      <div class="count-cell exp">
        <span class="count-num">{$suiteCounts.experimental}</span>
        <span class="count-label">exp</span>
      </div>
    </div>
  </div>

  <!-- Rerun button -->
  <div class="sidebar-actions">
    <button class="btn btn-accent rerun-btn" onclick={onRerun}>
      ↺ Rerun Suite
    </button>
  </div>

  <!-- Section nav -->
  <nav class="section-nav">
    <p class="nav-label">Sections</p>
    {#each SECTION_ORDER as key}
      {@const info = SECTION_INFO[key]}
      {@const counts = $sectionCounts[key] ?? { pass: 0, fail: 0, total: 0, running: 0, pending: 0 }}
      <button class="nav-item" onclick={() => scrollTo(key)}>
        <span class="nav-item-chip">{info?.chip ?? key}</span>
        <span class="nav-item-counts">
          {#if counts.running > 0}
            <span class="dot running"></span>
          {:else if counts.fail > 0}
            <span class="mini-count fail">{counts.fail}</span>
          {:else if counts.pass > 0 && counts.pass === counts.total && counts.total > 0}
            <span class="mini-check">✓</span>
          {:else if counts.pass > 0}
            <span class="mini-count">{counts.pass}/{counts.total}</span>
          {:else}
            <span class="mini-count muted">{counts.total}</span>
          {/if}
        </span>
      </button>
    {/each}
  </nav>

  <!-- Diagnostics -->
  <DiagnosticsPanel />
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    flex-shrink: 0;
    height: 100vh;
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    overflow-x: hidden;
  }

  /* Brand */
  .sidebar-brand {
    padding: 18px 16px 14px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .brand-name {
    font-size: 18px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -0.02em;
  }
  .brand-subtitle {
    font-size: 10px;
    color: var(--text-dim);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  /* Stats */
  .stats-block {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .stat-label {
    font-size: 10px;
    color: var(--text-dim);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .stat-value {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .stat-divider { height: 1px; background: var(--border); margin: 4px 0; }
  .counts-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }
  .count-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    padding: 6px 4px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }
  .count-num {
    font-size: 16px;
    font-weight: 700;
    color: var(--text);
    line-height: 1;
  }
  .count-label {
    font-size: 9px;
    color: var(--text-dim);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .count-cell.pass .count-num { color: var(--green); }
  .count-cell.fail .count-num { color: var(--red); }
  .count-cell.exp .count-num { color: var(--yellow); }

  /* Actions */
  .sidebar-actions {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }
  .rerun-btn { width: 100%; justify-content: center; }

  /* Section nav */
  .section-nav {
    flex: 1;
    padding: 10px 0 8px;
    overflow-y: auto;
  }
  .nav-label {
    padding: 0 16px 6px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-dim);
  }
  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 6px 16px;
    background: transparent;
    border: none;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
    text-align: left;
    transition: background var(--transition), color var(--transition);
  }
  .nav-item:hover { background: var(--surface-2); color: var(--text); }
  .nav-item-chip { font-weight: 600; }
  .nav-item-counts { display: flex; align-items: center; gap: 4px; }

  .mini-count {
    font-size: 10px;
    color: var(--text-dim);
    font-weight: 600;
  }
  .mini-count.fail { color: var(--red); }
  .mini-count.muted { color: var(--text-dim); }
  .mini-check {
    font-size: 10px;
    color: var(--green);
    font-weight: 700;
  }
</style>
