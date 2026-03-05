<script>
  import { splitPalette } from "../helpers/views.js";

  let { palette } = $props();
  const colors = $derived(splitPalette(palette));
</script>

<div class="swatch-grid">
  {#each colors as color}
    <div
      class="swatch"
      style="background: rgba({color[0]}, {color[1]}, {color[2]}, {color[3] / 255})"
      title="rgba({color[0]}, {color[1]}, {color[2]}, {color[3]})"
    ></div>
  {/each}
</div>

<style>
  .swatch-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 8px;
  }
  .swatch {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: 1px solid var(--border-strong);
    flex-shrink: 0;
  }
</style>
