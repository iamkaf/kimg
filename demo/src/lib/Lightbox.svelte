<script>
  import CanvasView from "./CanvasView.svelte";

  let { preview = null, onClose = () => {} } = $props();

  let innerWidth = $state(0);
  let innerHeight = $state(0);

  const maxDisplay = $derived(
    Math.max(120, Math.min(Math.floor(innerWidth * 0.86), Math.floor(innerHeight * 0.76))),
  );

  function close() {
    onClose?.();
  }

  function handleBackdropClick(event) {
    if (event.currentTarget === event.target) close();
  }

  function handleBackdropKeydown(event) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      close();
    }
  }

  function handleKeydown(event) {
    if (preview && event.key === "Escape") {
      event.preventDefault();
      close();
    }
  }

  $effect(() => {
    if (!preview) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = previousOverflow;
    };
  });
</script>

<svelte:window bind:innerWidth bind:innerHeight onkeydown={handleKeydown} />

{#if preview}
  <div
    class="lightbox"
    role="dialog"
    tabindex="0"
    aria-modal="true"
    aria-label={`Composition preview: ${preview.testTitle}`}
    onclick={handleBackdropClick}
    onkeydown={handleBackdropKeydown}
  >
    <div class="panel">
      <header class="panel-header">
        <div class="meta">
          <p class="title">{preview.testTitle}</p>
          <p class="subtitle">{preview.viewLabel} · {preview.width}×{preview.height}</p>
        </div>
        <button class="btn panel-close" type="button" onclick={close}>Close</button>
      </header>

      <div class="panel-grid">
        <aside class="info-rail">
          <div class="info-block">
            <p class="info-label">Test</p>
            <p class="info-value">{preview.testTitle}</p>
          </div>
          <div class="info-block">
            <p class="info-label">Composition</p>
            <p class="info-value">{preview.viewLabel}</p>
          </div>
          <div class="info-block">
            <p class="info-label">Resolution</p>
            <p class="info-value">{preview.width}×{preview.height}</p>
          </div>
          <div class="info-block">
            <p class="info-label">Controls</p>
            <p class="info-value">Click backdrop or press Escape to close.</p>
          </div>
        </aside>

        <div class="panel-body checker">
          <CanvasView
            rgba={preview.rgba}
            width={preview.width}
            height={preview.height}
            maxDisplay={maxDisplay}
            allowDownscale={true}
          />
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(4, 4, 8, 0.82);
    backdrop-filter: blur(3px);
  }
  .panel {
    width: min(100%, 1320px);
    max-height: calc(100vh - 48px);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 22px 70px rgba(0, 0, 0, 0.55);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .meta {
    min-width: 0;
  }
  .title {
    color: var(--text);
    font-size: 13px;
    font-weight: 700;
    line-height: 1.4;
  }
  .subtitle {
    color: var(--text-muted);
    font-size: 11px;
    margin-top: 2px;
  }
  .panel-close {
    flex-shrink: 0;
  }
  .panel-grid {
    display: grid;
    grid-template-columns: 1fr;
    min-height: 0;
    overflow: hidden;
  }
  .info-rail {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0));
  }
  .info-block {
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    min-width: 0;
  }
  .info-label {
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
    font-weight: 700;
  }
  .info-value {
    margin-top: 4px;
    font-size: 11px;
    color: var(--text);
    line-height: 1.45;
    word-break: break-word;
  }
  .panel-body {
    padding: 18px;
    overflow: auto;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  @media (min-width: 1100px) {
    .panel-grid {
      grid-template-columns: 280px minmax(0, 1fr);
      align-items: stretch;
      flex: 1;
    }
    .info-rail {
      display: flex;
      flex-direction: column;
      gap: 8px;
      padding: 14px;
      border-right: 1px solid var(--border);
      border-bottom: none;
      overflow: auto;
    }
  }
</style>
