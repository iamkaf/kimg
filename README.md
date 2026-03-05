<div align="center">

# kimg

<strong>Layered image compositing in Rust + WASM.</strong><br />
<sub>One engine, same API shape in Node.js and the browser.</sub>

<br />

[![CI](https://img.shields.io/github/actions/workflow/status/iamkaf/kimg/ci.yml?branch=main&style=for-the-badge)](https://github.com/iamkaf/kimg/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/%40iamkaf%2Fkimg?style=for-the-badge)](https://www.npmjs.com/package/@iamkaf/kimg)
[![License: MIT](https://img.shields.io/badge/license-MIT-2f6feb?style=for-the-badge)](LICENSE)

<br />
<img src="demo/assets/kimg-hero.png" alt="kimg hero image generated with kimg itself" width="920" />

</div>

> kimg is a headless image compositor for layered workflows: masks, blend modes, filters, transforms, text, SVG, and format I/O. It does not depend on the DOM, Canvas API, or native addons.

## Quick links

- [Why this project exists](#why-this-project-exists)
- [Install](#install)
- [Quick start](#quick-start)
- [Features](#features)
- [API snippets](#api-snippets)
- [Subpath exports](#subpath-exports)
- [Project layout](#project-layout)
- [Build from source](#build-from-source)
- [Tests](#tests)
- [Benchmarks](#benchmarks)
- [WASM binary size](#wasm-binary-size)
- [Roadmap](#roadmap)

## Why this project exists

Most image libraries are single-buffer tools: decode, edit, encode.

That is fine for many tasks, but it is awkward once you need a real compositing pipeline with:

- multiple layers
- blend modes
- masks and clipping
- scoped filter passes

kimg was extracted from the Spriteform compositor and rebuilt in Rust so the same engine can run in both Node and browser environments.

## Install

```bash
npm install @iamkaf/kimg
```

Local development in this repo:

```bash
npm install
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
./scripts/build.sh
```

`./scripts/build.sh` writes the package output to `dist/`. The tracked wrapper layer is TypeScript in `js/`, compiled with `tsgo`.

## Quick start

### Browser

```js
import { Composition } from "@iamkaf/kimg";

const doc = await Composition.create({ width: 128, height: 128 });
const layerId = doc.addImageLayer({
  name: "sprite",
  rgba: rgbaPixels,
  width: 128,
  height: 128,
  x: 0,
  y: 0,
});

doc.updateLayer(layerId, {
  opacity: 0.8,
  anchor: "center",
  rotation: 22.5,
  scaleX: 1.25,
  scaleY: 0.75,
});

const png = doc.exportPng();
```

### Node.js

```js
import { Composition } from "@iamkaf/kimg";

const doc = await Composition.create({ width: 64, height: 64 });
// same API from here
```

## Features

| Area | What you get |
|------|--------------|
| Layer model | `Raster`, `Filter`, `Group`, `Fill`, `Shape`, `Text`, `Svg` |
| Blend modes | 16 modes, including `Normal`, `Multiply`, `Screen`, `Overlay`, and HSL-family modes |
| Masks & clipping | Grayscale layer masks and clip-to-below behavior |
| Filters | HSL pipeline, levels, threshold, posterize, gradient map, blur, sharpen, edge detect, emboss |
| Transforms | Non-destructive translate/scale/rotate/flip plus destructive resize/crop/trim |
| Paint tools | Brush engine (round/grain tips, pressure, tilt, modeler smoothing, eraser, alpha lock, streaming) + bucket fill |
| Sprite tools | Packing, contact sheets, pixel scale, quantization, batch rendering |
| I/O | PNG/JPEG/WebP/GIF, retained SVG layers, experimental PSD import, magic-byte detect |
| Serialization | `.kimg` documents with versioned binary metadata + raw pixels |

### Layer notes

- `Shape` supports rectangle (optional corner radius), ellipse, line, and polygon.
- `Text` supports runtime font registration, weight/style/wrap/alignment, transforms, and browser-side Google Fonts loading.
- `Svg` keeps source SVG data for clean scaling until you explicitly rasterize.

### Blend modes

`Normal`, `Multiply`, `Screen`, `Overlay`, `Darken`, `Lighten`, `ColorDodge`, `ColorBurn`, `HardLight`, `SoftLight`, `Difference`, `Exclusion`, `Hue`, `Saturation`, `Color`, `Luminosity`.

## API snippets

### Shape layers

```js
const badgeId = doc.addShapeLayer({
  name: "Badge",
  type: "rectangle",
  x: 24,
  y: 24,
  width: 96,
  height: 40,
  radius: 12,
  fill: [255, 0, 0, 255],
  stroke: { color: [255, 255, 255, 255], width: 2 },
});
```

### Text layers

```js
await registerFont({
  family: "Inter",
  bytes: interFontBytes,
  weight: 400,
  style: "normal",
});

const titleId = doc.addTextLayer({
  name: "Title",
  text: "HELLO\nKIMG",
  fontFamily: "Inter",
  color: [24, 77, 163, 255],
  fontSize: 24,
  lineHeight: 28,
  letterSpacing: 2,
  x: 24,
  y: 24,
});

doc.updateLayer(titleId, {
  anchor: "center",
  rotation: -12,
  textConfig: {
    text: "HELLO\nTEXT",
    color: [201, 73, 45, 255],
  },
});
```

Browser Google Fonts helper:

```js
await loadGoogleFont({
  family: "Inter",
  weights: [400, 700],
  text: "HELLOKIMGTEXT",
});
```

### SVG layers

```js
const logoId = doc.addSvgLayer({
  name: "Logo",
  svg: svgMarkup,
  width: 160,
  height: 160,
  x: 32,
  y: 24,
});

doc.updateLayer(logoId, {
  anchor: "center",
  rotation: -8,
  scaleX: 1.5,
  scaleY: 1.5,
});

doc.rasterizeSvgLayer(logoId);
```

Notes:

- SVG layers are scalable assets, not editable path geometry.
- Scripts, animation elements, and external image references are rejected.
- SVG with `<text>` works best when required fonts are registered first.

### Per-layer transforms

```js
doc.updateLayer(layerId, {
  x: 10,
  y: -4,
  anchor: "center",
  flipX: false,
  flipY: true,
  rotation: 30,
  scaleX: 1.25,
  scaleY: 0.75,
});
```

### Alignment helpers

```js
doc.alignLayers({
  layerIds: [titleId, badgeId, iconId],
  mode: "horizontalCenter",
  reference: "canvas", // or "selection"
});
```

### Bucket fill and alpha lock

Coordinates are layer-local. Tolerance is checked per RGBA channel.

```js
doc.bucketFillLayer(layerId, {
  x: 12,
  y: 18,
  color: [0, 255, 0, 255],
  contiguous: true,
  tolerance: 0,
});

doc.setLayerAlphaLocked(layerId, true);
```

### Brush strokes

```js
doc.paintStrokeLayer(layerId, {
  color: [201, 73, 45, 255],
  size: 12,
  hardness: 0.8,
  tip: "grain",
  flow: 0.75,
  spacing: 0.4,
  smoothing: 0.2,
  smoothingMode: "modeler",
  pressureSize: 1,
  pressureOpacity: 0.35,
  points: [
    { x: 12, y: 18, pressure: 0.3, tiltX: 0.1, tiltY: 0.8, timeMs: 0 },
    { x: 42, y: 26, pressure: 0.8, tiltX: 0.6, tiltY: 0.4, timeMs: 16 },
    { x: 88, y: 44, pressure: 1.0, tiltX: 1.0, tiltY: 0.0, timeMs: 32 },
  ],
});

doc.paintStrokeLayer(layerId, {
  tool: "erase",
  size: 10,
  spacing: 0.3,
  points: [
    { x: 48, y: 18, pressure: 1 },
    { x: 76, y: 48, pressure: 1 },
  ],
});

const strokeId = doc.beginBrushStroke(layerId, {
  color: [35, 79, 221, 255],
  size: 14,
  hardness: 0.3,
  flow: 0.7,
  smoothing: 0.2,
  smoothingMode: "modeler",
  spacing: 0.35,
});

doc.pushBrushPoints(strokeId, [
  { x: 16, y: 52, pressure: 0.2, tiltX: -0.2, tiltY: 0.7, timeMs: 48 },
  { x: 44, y: 60, pressure: 0.7, tiltX: 0.2, tiltY: 0.5, timeMs: 64 },
]);
doc.pushBrushPoints(strokeId, [{ x: 88, y: 74, pressure: 1.0, tiltX: 0.7, tiltY: 0.1, timeMs: 80 }]);
doc.endBrushStroke(strokeId);
```

## Subpath exports

```js
// Pure JS RGBA/base64 helpers
import { rgbaToBase64, base64ToRgba } from "@iamkaf/kimg/base64";

// Pure JS color contrast helper
import { readableTextColor } from "@iamkaf/kimg/color-utils";

// Low-level wasm API (browser)
import initRaw, { Composition as RawComposition } from "@iamkaf/kimg/raw";
await initRaw();
const raw = new RawComposition(128, 128);

// Low-level wasm API (Node)
import { readFileSync } from "node:fs";
import { initSync } from "@iamkaf/kimg/raw";
const wasm = readFileSync(new URL("./kimg_wasm_bg.wasm", import.meta.url));
initSync({ module: wasm });
```

## Color utilities

```js
import { hexToRgb, rgbToHex, relativeLuminance, contrastRatio, dominantRgbFromRgba } from "@iamkaf/kimg";

await hexToRgb("#ff8000");
await rgbToHex(255, 128, 0);
await relativeLuminance("#3b82f6");
await contrastRatio("#ffffff", "#000000");
await dominantRgbFromRgba(pixels, { width: 128, height: 128 });
```

## Project layout

```text
kimg/
├── crates/
│   ├── kimg-core/     # Pure Rust pixel engine
│   │   ├── src/
│   │   │   ├── blend.rs
│   │   │   ├── blit.rs
│   │   │   ├── buffer.rs
│   │   │   ├── brush.rs
│   │   │   ├── codec.rs
│   │   │   ├── color.rs
│   │   │   ├── convolution.rs
│   │   │   ├── document.rs
│   │   │   ├── fill.rs
│   │   │   ├── filter.rs
│   │   │   ├── layer.rs
│   │   │   ├── serialize.rs
│   │   │   ├── sprite.rs
│   │   │   └── transform.rs
│   │   └── benches/
│   └── kimg-wasm/     # wasm-bindgen API surface
├── js/                # Tracked TS sources
├── dist/              # Generated package output
├── demo/              # Visual test suite (Vite + Svelte)
└── scripts/
```

## Build from source

Requirements:

- Node.js + npm
- Rust
- `wasm32-unknown-unknown` target
- `wasm-bindgen-cli`

```bash
npm install
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
./scripts/build.sh
```

The build writes all artifacts to `dist/`.

### Generated wasm variants

`./scripts/build.sh` emits:

- `kimg_wasm_bg.wasm`
- `kimg_wasm_simd_bg.wasm`
- `kimg_wasm_svg_bg.wasm`
- `kimg_wasm_svg_simd_bg.wasm`
- `kimg_wasm_text_bg.wasm`
- `kimg_wasm_text_simd_bg.wasm`
- `kimg_wasm_text_svg_bg.wasm`
- `kimg_wasm_text_svg_simd_bg.wasm`

Runtime loading behavior:

- Browser `Composition` starts with text-enabled wasm and upgrades to SVG-capable wasm when needed.
- Node `Composition` uses text+SVG wasm eagerly.
- `raw` and utility-only usage can stay on leaner variants.

## Tests

Common commands:

```bash
cargo test -p kimg-core
npm run fmt:js:check
npm run test:js
npm run test:demo
npm run test:pack
npm run test:all
```

Current scope:

- `171` core Rust tests
- `57` wasm tests
- `26` package-layer Vitest tests

`npm run test:all` is the full Rust + JS pass.

`npm run test:demo` runs the visual suite in headless browser mode and fails on error diagnostics, incomplete cards, or failed assertions.

`npm run test:pack` validates the published package shape from a packed tarball, not from the local source tree.

## Visual test suite

The `demo/` directory is a Vite + Svelte app with 200+ visual cards covering the full API surface.

```bash
npm run demo:dev    # start dev server at localhost:5173
npm run demo:build  # build static output to demo/dist/
```

The suite runs all tests on load and displays pass/fail status, rendered canvas output, and per-test metrics in a sidebar-nav layout.

## Benchmarks

Run all:

```bash
cargo bench -p kimg-core
```

Run one bench file:

```bash
cargo bench -p kimg-core --bench transform
```

Compile-only smoke:

```bash
cargo bench -p kimg-core -- --test
```

Criterion reports are written to `target/criterion/`.

### Bench coverage

| File | What it measures |
|------|------------------|
| `blend` | Source-over and blend modes at multiple sizes |
| `transform` | resize/crop/trim/rotate |
| `convolution` | 3x3/5x5 kernels, box blur, Gaussian blur |
| `filter` | HSL pipeline and core destructive filters |
| `document` | full compositing pipeline, clipping/masking overhead, transform caching, text cold/cached paths |
| `codec` | PNG/JPEG/WebP encode+decode |
| `sprite` | packing, palette extraction, quantization, pixel scale |
| `fill` | contiguous/non-contiguous/tolerance fill |
| `brush` | hard/soft/erase/pressure/streamed/grain+tilt+modeler paths |
| `shape` | standalone rectangle and polygon rasterization |

Notes:

- Very expensive resize cases use reduced flat sampling so full runs stay practical.
- Resize paths use `fast_image_resize`.
- Text medians below were refreshed with `--features cosmic-text-backend` so they match shipped text rendering.
- Codec benches use a deterministic textured image.
- `render/repeated_transformed_layer/512` performs two back-to-back renders to expose cache wins.
- Shape benches instantiate fresh shape instances per sample.

### Performance snapshot

Representative medians from local runs on March 4, 2026:

| Hot path | Median |
|------|------:|
| `render/10_layers/512` | `9.61 ms` |
| `render/10_layers_with_filter/512` | `15.07 ms` |
| `render/10_layers_with_transforms/512` | `10.38 ms` |
| `render/text_styles_cold/320x176` | `37.72 ms` |
| `render/text_styles_cached/320x176` | `192.91 µs` |
| `bucket_fill/contiguous/512` | `703.87 µs` |
| `brush/round_soft_large/512` | `585.72 µs` |
| `brush/grain_tilt_modeler/512` | `726.67 µs` |
| `encode_png/512` | `1.26 ms` |
| `decode_webp/512` | `2.63 ms` |
| `resize_lanczos3/2048→4096` | `53.98 ms` |

<details>
<summary>Full benchmark baseline table</summary>

| Operation | Median |
|------|------:|
| `render/single_image/512` | `962.97 µs` |
| `render/10_layers/512` | `9.61 ms` |
| `render/10_normal_layers/512` | `19.14 ms` |
| `render/10_layers_with_filter/512` | `15.07 ms` |
| `render/single_shape/512` | `953.49 µs` |
| `render/10_shapes/512` | `9.40 ms` |
| `render/10_shapes_with_filter/512` | `16.84 ms` |
| `render/group_of_5/512` | `5.17 ms` |
| `render/clipped_layer_stack/512` | `18.28 ms` |
| `render/masked_layer_stack/512` | `10.27 ms` |
| `render/transformed_image/512` | `1.01 ms` |
| `render/transformed_paint/512` | `1.17 ms` |
| `render/transformed_shape/512` | `1.13 ms` |
| `render/10_layers_with_transforms/512` | `10.38 ms` |
| `render/repeated_transformed_layer/512` | `2.00 ms` |
| `render/text_registered_cold/320x168` | `24.90 ms` |
| `render/text_registered_cached/320x168` | `228.49 µs` |
| `render/text_styles_cold/320x176` | `37.72 ms` |
| `render/text_styles_cached/320x176` | `192.91 µs` |
| `render/repeated_text_styles/320x176` | `386.90 µs` |
| `serialize_deserialize/10_layers` | `713.38 µs` |
| `apply_hsl_filter/512` | `4.96 ms` |
| `bucket_fill/contiguous/512` | `703.87 µs` |
| `bucket_fill/non_contiguous/512` | `278.29 µs` |
| `bucket_fill/tolerance/512` | `391.37 µs` |
| `brush/round_hard_small/256` | `60.00 µs` |
| `brush/round_soft_large/512` | `585.72 µs` |
| `brush/erase_soft/512` | `284.01 µs` |
| `brush/long_pressure_stroke/1024` | `1.31 ms` |
| `brush/streamed_long_pressure_stroke/1024` | `1.30 ms` |
| `brush/grain_tilt_modeler/512` | `726.67 µs` |
| `brush/repeated_short_strokes/512` | `74.03 µs` |
| `encode_png/512` | `1.26 ms` |
| `decode_png/512` | `1.23 ms` |
| `encode_jpeg/512` | `2.09 ms` |
| `decode_jpeg/512` | `1.19 ms` |
| `encode_webp/512` | `1.42 ms` |
| `decode_webp/512` | `2.63 ms` |
| `extract_palette/512/16colors` | `20.55 ms` |
| `shape/rasterize_rectangle/512` | `886.16 µs` |
| `shape/rasterize_polygon/512` | `11.82 ms` |
| `resize_nearest/512→1024` | `1.63 ms` |
| `resize_bilinear/512→1024` | `1.00 ms` |
| `resize_lanczos3/512→1024` | `1.60 ms` |
| `resize_lanczos3/2048→4096` | `53.98 ms` |

</details>

## WASM binary size

Current local release build sizes:

- `dist/kimg_wasm_bg.wasm`: `1.0 MB` uncompressed, `370,869` bytes gzipped
- `dist/kimg_wasm_simd_bg.wasm`: `1.2 MB` uncompressed, `409,572` bytes gzipped
- `dist/kimg_wasm_text_bg.wasm`: `3.2 MB` uncompressed, `1,076,325` bytes gzipped
- `dist/kimg_wasm_text_simd_bg.wasm`: `3.5 MB` uncompressed, `1,178,561` bytes gzipped
- `dist/kimg_wasm_svg_bg.wasm`: `2.9 MB` uncompressed
- `dist/kimg_wasm_svg_simd_bg.wasm`: `3.1 MB` uncompressed
- `dist/kimg_wasm_text_svg_bg.wasm`: `4.9 MB` uncompressed
- `dist/kimg_wasm_text_svg_simd_bg.wasm`: `5.1 MB` uncompressed

Sizes vary a bit by toolchain and optimization settings.

## Roadmap

See the feature roadmap: [ROADMAP.md](ROADMAP.md).

Current top priority:

1. selection system
2. selection-aware operations
3. adjustment layers

## License

MIT
