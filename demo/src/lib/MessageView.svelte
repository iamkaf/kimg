<script>
  let { text } = $props();
</script>

<div class="message-shell">
  <p>{text}</p>
</div>

<style>
  .message-shell {
    padding: 16px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.6;
  }
</style>
