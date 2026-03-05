<script>
  import CanvasView from "./CanvasView.svelte";
  import SwatchView from "./SwatchView.svelte";
  import CodeView from "./CodeView.svelte";
  import MessageView from "./MessageView.svelte";
  import { downloadCardImage, slugify } from "../helpers/canvas.js";

  let { test } = $props();

  function chooseMaxDisplay(view) {
    if (view.width > view.height * 1.25) return 320;
    if (view.width >= 160 && view.height >= 160) return 280;
    if (view.width <= 96 && view.height <= 96) return 200;
    return 220;
  }

  function handleDownload() {
    if (test.result) {
      downloadCardImage({
        title: test.title,
        expectation: test.expectation,
        result: test.result,
        elapsedMs: test.elapsed ?? 0,
      });
    }
  }

  const isWide = (view) =>
    view.kind === "code" ||
    view.kind === "message" ||
    view.wide === true ||
    (view.kind === "rgba" && view.width > view.height * 1.2);
</script>

<article
  class="card"
  class:is-featured={test.featured}
  class:is-full={test.fullSpan}
  class:is-pass={test.status === "pass"}
  class:is-fail={test.status === "fail"}
  class:is-experimental={test.status === "experimental"}
  id="test-{test.id}"
>
  <!-- Header -->
  <header class="card-header">
    <div class="card-header-row">
      <h3 class="card-title">{test.title}</h3>
      <div class="card-actions">
        <button
          class="btn"
          onclick={handleDownload}
          disabled={!test.result}
        >Download PNG</button>
        <span class="badge {test.status}">
          <span class="dot {test.status}"></span>
          {#if test.status === "pending"}
            Pending
          {:else if test.status === "running"}
            Running
          {:else if test.status === "pass"}
            {test.result?.assertions ?? 0} checks
          {:else if test.status === "fail"}
            Failed
          {:else if test.status === "experimental"}
            Experimental
          {/if}
        </span>
      </div>
    </div>
    <p class="card-expectation">
      <span class="look-for">Look for:</span>
      {test.expectation}
    </p>
  </header>

  <!-- Error message -->
  {#if test.status === "fail" && test.error}
    <div class="card-error">
      <span class="error-label">Error:</span>
      {test.error}
    </div>
  {/if}

  <!-- Views -->
  {#if test.result?.views?.length}
    <div
      class="view-grid"
      style={test.previewMin ? `--preview-min:${test.previewMin}px` : test.featured ? "--preview-min:210px" : ""}
    >
      {#each test.result.views as view}
        <figure class="view-figure" class:is-wide={isWide(view)}>
          {#if view.kind === "rgba"}
            <CanvasView
              rgba={view.rgba}
              width={view.width}
              height={view.height}
              maxDisplay={view.maxDisplay ?? chooseMaxDisplay(view)}
            />
          {:else if view.kind === "swatches"}
            <SwatchView palette={view.palette} />
          {:else if view.kind === "code"}
            <CodeView text={view.text} />
          {:else if view.kind === "message"}
            <MessageView text={view.text} />
          {/if}
          <figcaption class="view-caption">{view.label}</figcaption>
        </figure>
      {/each}
    </div>
  {/if}

  <!-- Metrics -->
  {#if test.result?.metrics?.length}
    <ul class="meta-list">
      {#each test.result.metrics as [label, value]}
        <li class="meta-item">
          <span class="meta-label">{label}</span>
          <span class="meta-value">{value}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <!-- Layer list -->
  {#if test.result?.layers?.length}
    <ul class="layer-list">
      {#each test.result.layers as layer}
        <li class="layer-item">
          <span>
            <span class="layer-kind">{layer.kind}</span>
            <span class="layer-name">{layer.name}</span>
          </span>
          <span class="layer-pos">{layer.x},{layer.y}{layer.parentId != null ? ` · parent ${layer.parentId}` : ""}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <!-- Note -->
  {#if test.result?.note}
    <p class="card-note"><strong>Note:</strong> {test.result.note}</p>
  {/if}

  <!-- Footer -->
  <footer class="card-footer">
    {#if test.elapsed != null}
      {test.result?.assertions ?? 0} checks in {Math.round(test.elapsed)} ms
    {:else if test.status === "pending"}
      Waiting to run
    {:else if test.status === "running"}
      Running…
    {/if}
  </footer>
</article>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 0;
    transition: border-color var(--transition), box-shadow var(--transition);
    overflow: hidden;
  }
  .card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 0 0 1px rgba(97, 123, 255, 0.06), 0 8px 32px rgba(0, 0, 0, 0.3);
  }
  .card.is-pass { border-color: rgba(61, 214, 140, 0.12); }
  .card.is-fail { border-color: rgba(240, 82, 82, 0.2); }
  .card.is-experimental { border-color: rgba(233, 177, 14, 0.12); }
  .card.is-featured { grid-column: span 2; }
  .card.is-full { grid-column: 1 / -1; }

  /* Header */
  .card-header {
    padding: 14px 16px 10px;
    border-bottom: 1px solid var(--border);
  }
  .card-header-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 6px;
  }
  .card-title {
    flex: 1;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    line-height: 1.4;
  }
  .card-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .card-expectation {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .look-for {
    color: var(--text-dim);
    font-weight: 600;
    margin-right: 4px;
  }

  /* Error */
  .card-error {
    padding: 10px 16px;
    background: rgba(240, 82, 82, 0.08);
    border-bottom: 1px solid rgba(240, 82, 82, 0.15);
    font-size: 11px;
    color: var(--red);
    word-break: break-word;
  }
  .error-label { font-weight: 700; margin-right: 4px; }

  /* View grid */
  .view-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 12px 16px;
    align-items: flex-start;
  }
  .view-figure {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .view-figure.is-wide { flex: 1 1 100%; }
  .view-caption {
    font-size: 10px;
    color: var(--text-muted);
    text-align: center;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  /* Metrics */
  .meta-list {
    list-style: none;
    border-top: 1px solid var(--border);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 0;
  }
  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 16px;
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .meta-label {
    font-size: 10px;
    color: var(--text-muted);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .meta-value {
    font-size: 12px;
    color: var(--text);
    font-weight: 600;
  }

  /* Layer list */
  .layer-list {
    list-style: none;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
  }
  .layer-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 5px 16px;
    border-bottom: 1px solid var(--border);
  }
  .layer-kind {
    font-size: 10px;
    color: var(--blue);
    font-weight: 700;
    letter-spacing: 0.04em;
    margin-right: 6px;
    text-transform: uppercase;
  }
  .layer-name { color: var(--text); }
  .layer-pos { color: var(--text-dim); font-size: 10px; flex-shrink: 0; }

  /* Note */
  .card-note {
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Footer */
  .card-footer {
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    font-size: 10px;
    color: var(--text-dim);
    margin-top: auto;
  }
</style>
