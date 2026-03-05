<script>
let { rgba, width, height, maxDisplay = 220, allowDownscale = false, label = "" } = $props();

  let canvasEl = $state(null);

  const scale = $derived.by(() => {
    const maxSide = Math.max(width, height);
    const raw = maxDisplay / maxSide;
    if (allowDownscale && raw < 1) return raw;
    return Math.max(1, Math.floor(raw));
  });

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
