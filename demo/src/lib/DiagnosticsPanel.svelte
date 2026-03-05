<script>
  import { diagnostics } from "../stores/suite.svelte.js";

  let expanded = $state(false);
  const recent = $derived($diagnostics.slice(-20).reverse());
</script>

<div class="diag-panel">
  <button class="diag-toggle" onclick={() => (expanded = !expanded)}>
    <span class="diag-label">Diagnostics</span>
    {#if $diagnostics.length > 0}
      <span class="diag-count">{$diagnostics.length}</span>
    {/if}
    <span class="diag-chevron" class:open={expanded}>▾</span>
  </button>

  {#if expanded}
    <ul class="diag-list">
      {#if recent.length === 0}
        <li class="diag-empty">No errors captured.</li>
      {:else}
        {#each recent as item}
          <li class="diag-item {item.level}">{item.message}</li>
        {/each}
      {/if}
    </ul>
  {/if}
</div>

<style>
  .diag-panel {
    border-top: 1px solid var(--border);
    margin-top: auto;
  }
  .diag-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 10px 16px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
    text-align: left;
    transition: color var(--transition);
  }
  .diag-toggle:hover { color: var(--text); }
  .diag-label { flex: 1; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; }
  .diag-count {
    background: rgba(240, 82, 82, 0.2);
    color: var(--red);
    border-radius: 100px;
    padding: 1px 7px;
    font-size: 10px;
    font-weight: 700;
  }
  .diag-chevron { font-size: 12px; transition: transform var(--transition); }
  .diag-chevron.open { transform: rotate(180deg); }

  .diag-list {
    list-style: none;
    border-top: 1px solid var(--border);
    max-height: 200px;
    overflow-y: auto;
  }
  .diag-item {
    padding: 6px 16px;
    font-size: 11px;
    line-height: 1.5;
    border-bottom: 1px solid var(--border);
    word-break: break-word;
  }
  .diag-item.error { color: var(--red); }
  .diag-item.warn { color: var(--yellow); }
  .diag-empty {
    padding: 10px 16px;
    color: var(--text-dim);
    font-size: 11px;
    font-style: italic;
  }
</style>
