<script>
  let { text } = $props();
</script>

<div class="code-shell">
  <pre class="code-block">{text}</pre>
</div>

<style>
  .code-shell {
    width: 100%;
    overflow: auto;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
  }
  .code-block {
    padding: 12px 16px;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.6;
    color: var(--text-muted);
    white-space: pre;
    tab-size: 2;
    margin: 0;
  }
</style>
