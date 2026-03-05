<script>
let { rgba, width, height, maxDisplay = 220, label = "" } = $props();

  let canvasEl = $state(null);

  const scale = $derived(Math.max(1, Math.floor(maxDisplay / Math.max(width, height))));

  function render() {
    if (!canvasEl || !rgba) return;
    const ctx = canvasEl.getContext("2d");
    ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0);
  }

  $effect(() => {
    rgba; width; height;
    render();
  });
</script>

<div
  class="canvas-shell checker"
  style="width:{width * scale}px; height:{height * scale}px;"
>
  <canvas
    bind:this={canvasEl}
    {width}
    {height}
    style="width:{width * scale}px; height:{height * scale}px; image-rendering:pixelated;"
  ></canvas>
</div>

<style>
  .canvas-shell {
    display: inline-block;
    border-radius: var(--radius-sm);
    overflow: hidden;
    border: 1px solid var(--border);
  }
  canvas {
    display: block;
  }
</style>
