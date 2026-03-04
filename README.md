# kimg

[![CI](https://github.com/iamkaf/kimg/actions/workflows/ci.yml/badge.svg)](https://github.com/iamkaf/kimg/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust+WASM image compositing engine. Think of it as a headless Photoshop you can `import` — layers, blend modes, filters, masks, and multi-format I/O, all running in release-built WASM binaries.

It works the same way in Node.js and the browser. No native dependencies, no Canvas API, no DOM.

## Why this exists

Most image libraries treat images as single buffers — apply a filter, resize, encode, done. If you need *layers* composited together with blend modes, scoped filters, and a render pipeline, your options are either browser-only (Photopea), commercial (IMG.LY), or huge (magick-wasm at 7MB+).

kimg fills that gap. Originally extracted from the [Spriteform](https://spriteform.com) compositor (which was pure JS and slow), it now runs 5-15x faster and doesn't need Node.js or Electron.

## Install

```bash
npm install @iamkaf/kimg
```

For local development from this repo:

```bash
npm install
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
./scripts/build.sh
```

This builds the consumable JS/WASM package into `dist/` using `tsgo` for the tracked TypeScript wrapper layer. The main `Composition` API loads the text-enabled renderer automatically, lazy-loads the SVG-enabled renderer in the browser when needed, and uses the full text+SVG renderer eagerly on Node. Pure utility APIs and the low-level `raw` entrypoint can still use the leaner non-text builds.

## Quick start

### Browser

```js
import { Composition } from '@iamkaf/kimg';

const doc = await Composition.create({ width: 128, height: 128 });
const layerId = doc.addImageLayer({
  name: 'sprite',
  rgba: rgbaPixels,
  width: 128,
  height: 128,
  x: 0,
  y: 0,
});
doc.updateLayer(layerId, {
  opacity: 0.8,
  anchor: 'center',
  rotation: 22.5,
  scaleX: 1.25,
  scaleY: 0.75,
});

const png = doc.exportPng();
```

### Node.js

```js
import { Composition } from '@iamkaf/kimg';

const doc = await Composition.create({ width: 64, height: 64 });
// same API from here on
```

## What it can do

**Layers** — Raster, Filter, Group, Fill, Shape, Text, SVG. Nested groups with scoped filter application.
Shape layers cover rectangles with optional corner radius, ellipses, lines, and polygons with fill/stroke styling. Text layers support real font rendering with weight, style, wrapping, alignment, transforms, runtime font registration, and browser Google Fonts loading. SVG layers keep source SVG data around so logos and icons stay crisp under normal scaling until you explicitly rasterize them.

**16 blend modes** — Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity.

**Masks** — Grayscale layer masks and clipping masks (`setLayerClipToBelow()` in the JS facade).

**Filters** — HSL adjustments, brightness/contrast, temperature/tint, sharpen. Invert, posterize, threshold, levels, gradient map. Box blur, Gaussian blur, edge detect, emboss (all as convolution kernels).

**Transforms** — Non-destructive per-layer translate / scale / rotate / flip for raster, shape, text, and SVG layers, plus destructive resize (nearest-neighbor, bilinear, Lanczos3), crop, trim alpha.

**Paint tools** — Raster brush strokes with round and grain tips, size, opacity, flow, hardness, spacing, simple or modeler-backed smoothing, pressure-driven size/opacity, tilt-shaped dabs, eraser mode, and streamed stroke sessions, plus bucket fill with contiguous/non-contiguous modes and alpha-aware RGBA tolerance matching.

**Sprite tools** — Sprite sheet packer (shelf bin-packing), contact sheet grids, pixel-art upscale, color quantization, batch render pipeline.

**Format support** — PNG, JPEG, WebP, GIF (animated frames → layers), retained SVG layers, and experimental PSD layer import. Auto-detection via magic bytes for raster imports.

**Serialization** — Save/load full documents as `.kimg` files (versioned binary metadata + raw pixel data).

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

- SVG layers are retained scalable assets, not editable path geometry.
- Static SVG is the target. Scripts, animation elements, and external image references are rejected.
- SVGs containing `<text>` render best when the required fonts are registered first.
- Browser `Composition` only loads the heavier SVG-capable wasm when you first add an SVG layer or deserialize a `.kimg` document that already contains one. Node uses the text+SVG renderer eagerly.

### Text notes

- `Composition.create()` and `Composition.deserialize()` use the text-enabled wasm renderer so text works out of the box from the main package.
- `loadGoogleFont()` is browser-only. On Node, use `registerFont()` with raw font bytes.
- `.kimg` documents serialize text content and style metadata, but they do not embed font binaries yet. Re-register the same fonts before rendering deserialized documents.
- Registered fonts live in a module-global wasm registry for the current runtime session.
- Text is currently plain string content with block-level layout. Rich text editing, selection, and inline spans are still out of scope.

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

### Bucket fill

Coordinates are layer-local pixel coordinates. Tolerance is alpha-aware and
checked per channel across RGBA.

```js
doc.bucketFillLayer(layerId, {
  x: 12,
  y: 18,
  color: [0, 255, 0, 255],
  contiguous: true,
  tolerance: 0,
});
```

### Brush strokes

Brush coordinates are layer-local pixel coordinates.

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
doc.pushBrushPoints(strokeId, [
  { x: 88, y: 74, pressure: 1.0, tiltX: 0.7, tiltY: 0.1, timeMs: 80 },
]);
doc.endBrushStroke(strokeId);
```

## Subpath exports

```js
// Base64 RGBA helpers — pure JS, no WASM init needed
import { rgbaToBase64, base64ToRgba } from '@iamkaf/kimg/base64';

// Pick readable text color for a background
import { readableTextColor } from '@iamkaf/kimg/color-utils';
readableTextColor('#1a1a2e'); // '#ffffff'
readableTextColor('#f0f0f0'); // '#000000'

// Low-level wasm-bound API (browser)
import initRaw, { Composition as RawComposition } from '@iamkaf/kimg/raw';

await initRaw();
const raw = new RawComposition(128, 128);

// Low-level wasm-bound API (Node.js)
import { readFileSync } from 'node:fs';
import { initSync } from '@iamkaf/kimg/raw';

const wasm = readFileSync(new URL('./kimg_wasm_bg.wasm', import.meta.url));
initSync({ module: wasm });
```

## Color utilities

These are free functions, not tied to a document:

```js
import { hexToRgb, rgbToHex, relativeLuminance, contrastRatio, dominantRgbFromRgba } from '@iamkaf/kimg';

await hexToRgb('#ff8000');                     // Uint8Array [255, 128, 0]
await rgbToHex(255, 128, 0);                   // '#ff8000'
await relativeLuminance('#3b82f6');            // 0.2355 (WCAG 2.x)
await contrastRatio('#ffffff', '#000000');     // 21.0
await dominantRgbFromRgba(pixels, { width: 128, height: 128 }); // Uint8Array [r, g, b]
```

## Project structure

```
kimg/
├── crates/
│   ├── kimg-core/     # Pure Rust pixel engine (no WASM deps)
│   │   ├── src/
│   │   │   ├── blend.rs       # 16 blend modes
│   │   │   ├── blit.rs        # Transformed blit (position, flip, rotation, opacity)
│   │   │   ├── buffer.rs      # ImageBuffer with RGBA pixel data
│   │   │   ├── codec.rs       # PNG, JPEG, WebP, GIF, experimental PSD import
│   │   │   ├── color.rs       # RGB/HSL conversion, luminance, contrast
│   │   │   ├── convolution.rs # Blur, sharpen, edge detect, emboss kernels
│   │   │   ├── document.rs    # Document struct, layer tree, render pipeline
│   │   │   ├── brush.rs       # Raster brush engine for paint / erase strokes
│   │   │   ├── fill.rs        # Bucket fill for raster layers
│   │   │   ├── filter.rs      # HSL filters, invert, posterize, threshold, levels
│   │   │   ├── layer.rs       # Layer types and common properties
│   │   │   ├── serialize.rs   # Document save/load
│   │   │   ├── sprite.rs      # Sprite sheet packing, contact sheets, quantization
│   │   │   └── transform.rs   # Resize, rotate, crop, trim
│   │   └── benches/           # Criterion.rs benchmarks
│   └── kimg-wasm/     # wasm-bindgen API surface
├── js/                # Tracked JS/TS package sources compiled into dist/
├── dist/              # Built output (JS + WASM + TypeScript types)
├── demo/              # Browser demo page
└── scripts/           # Build scripts
```

## Building from source

You need Node.js/npm, Rust, the `wasm32-unknown-unknown` target, and `wasm-bindgen-cli`:

```bash
npm install
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

./scripts/build.sh
```

Output goes to `dist/`. The demo page at `demo/index.html` loads from there.

The build emits eight wasm binaries:

- `kimg_wasm_bg.wasm` for the baseline target
- `kimg_wasm_simd_bg.wasm` for runtimes with `wasm32` SIMD (`simd128`)
- `kimg_wasm_svg_bg.wasm` for the SVG-enabled baseline target
- `kimg_wasm_svg_simd_bg.wasm` for SVG-enabled runtimes with `wasm32` SIMD (`simd128`)
- `kimg_wasm_text_bg.wasm` for the text-enabled baseline target
- `kimg_wasm_text_simd_bg.wasm` for text-enabled runtimes with `wasm32` SIMD (`simd128`)
- `kimg_wasm_text_svg_bg.wasm` for the combined text+SVG baseline target
- `kimg_wasm_text_svg_simd_bg.wasm` for the combined text+SVG SIMD target

The main package uses the text-enabled variants for `Composition`, upgrades to the SVG-capable variants lazily in the browser, and uses the combined text+SVG variants eagerly on Node. The baseline variants remain useful for the `raw` entrypoint and utility-only scenarios.

## Running tests

```bash
cargo test -p kimg-core
npm run fmt:js:check
npm run test:js
npm run test:demo
npm run test:pack
npm run test:all
```

165 core Rust tests covering blend modes, compositing, filters, transforms, codecs, serialization, sprites, color utilities, shape layers, text layers, SVG layers, bucket fill, brush strokes, and shared per-layer transforms.

The package layer also has a small Vitest suite that exercises the built JS/WASM facade, subpath exports, and Node-side initialization behavior.

`npm run test:all` is the convenience entrypoint for the full Rust + package-layer test pass.

`npm run fmt:js` and `npm run fmt:js:check` use `oxfmt` for the tracked TypeScript sources and tests.

`npm run test:demo` serves `/demo/` locally, loads the full visual suite in a headless browser, and fails if the page reports runtime failures, diagnostics, or an incomplete card set.

`npm run test:pack` packs the repo, installs the tarball into temporary Node/browser projects, and smoke-tests the published package shape instead of the local source tree.

## Benchmarks

Criterion.rs benchmarks cover all performance-sensitive operations. Run the full suite:

```bash
cargo bench -p kimg-core
```

Run a single bench file:

```bash
cargo bench -p kimg-core --bench transform
```

Smoke-test compilation without collecting statistics:

```bash
cargo bench -p kimg-core -- --test
```

HTML reports with timing history are written to `target/criterion/` after a full run.

The benchmarks cover:

| File | What's measured |
|------|----------------|
| `blend` | Porter-Duff source-over and 3 blend modes at 64×64 / 512×512 / 2048×2048 |
| `transform` | Nearest, bilinear, and Lanczos3 resize; crop; trim; arbitrary rotation |
| `convolution` | 3×3 and 5×5 kernels; box blur; Gaussian blur |
| `filter` | HSL pipeline, invert, levels, posterize, gradient map |
| `document` | Full render pipeline at 1–10 layers, shape-heavy scenes, clipping/masking overhead, non-destructive transform render costs, and text cold/cached render cost |
| `codec` | PNG / JPEG / WebP encode and decode of a 512×512 buffer |
| `sprite` | Sprite sheet packing, palette extraction, quantization, pixel-art scale |
| `fill` | Contiguous and non-contiguous bucket fill, plus alpha-aware tolerance matching |
| `brush` | Hard/soft raster brush strokes, erase mode, batched and streamed pressure strokes, textured tilt/modeler strokes, repeated short strokes |
| `shape` | Standalone shape rasterization cost for rectangle and polygon primitives |

Notes on the harnesses:

- Very expensive resize cases use reduced flat-sampled Criterion groups so `cargo bench -p kimg-core` stays practical while still reporting worst-case medians.
- RGBA bilinear and Lanczos3 resize paths use `fast_image_resize`, so native builds pick up host SIMD and the browser `Composition.create()` path can load the separate `simd128` wasm artifact.
- The full suite runs with default features. The text medians below were refreshed separately with `cargo bench -p kimg-core --bench document --features cosmic-text-backend -- 'render/(text|repeated_text)'` so they reflect the shipped text renderer instead of the lean fallback path.
- Codec benchmarks use a deterministic textured 512×512 image instead of a flat fill, which avoids unrealistically optimistic compression timings.
- `render/repeated_transformed_layer/512` performs two back-to-back renders of the same transformed document in one iteration to measure transform-cache wins directly.
- Brush benchmarks operate directly on raster buffers and cover hard/soft strokes, erase mode, batched and streamed long pressure strokes, a textured tilt/modeler path, and repeated short-stroke workloads.
- Standalone shape benches instantiate a fresh shape per sample so they continue to measure rasterization work instead of the document-level layer cache.

Representative medians from recent local runs on March 4, 2026. These are hardware-dependent and should be treated as a baseline example, not a guarantee:

| Operation | Median |
|------|------:|
| `render/single_image/512` | `966.44 µs` |
| `render/10_layers/512` | `9.63 ms` |
| `render/10_normal_layers/512` | `19.04 ms` |
| `render/10_layers_with_filter/512` | `15.21 ms` |
| `render/single_shape/512` | `973.94 µs` |
| `render/10_shapes/512` | `9.60 ms` |
| `render/10_shapes_with_filter/512` | `17.06 ms` |
| `render/group_of_5/512` | `5.16 ms` |
| `render/clipped_layer_stack/512` | `18.39 ms` |
| `render/masked_layer_stack/512` | `10.32 ms` |
| `render/transformed_image/512` | `996.86 µs` |
| `render/transformed_paint/512` | `1.16 ms` |
| `render/transformed_shape/512` | `1.13 ms` |
| `render/10_layers_with_transforms/512` | `10.30 ms` |
| `render/repeated_transformed_layer/512` | `2.00 ms` |
| `render/text_registered_cold/320x168` | `20.51 ms` |
| `render/text_registered_cached/320x168` | `228.84 µs` |
| `render/text_styles_cold/320x176` | `31.22 ms` |
| `render/text_styles_cached/320x176` | `195.31 µs` |
| `render/repeated_text_styles/320x176` | `392.28 µs` |
| `serialize_deserialize/10_layers` | `775.90 µs` |
| `apply_hsl_filter/512` | `5.09 ms` |
| `bucket_fill/contiguous/512` | `685.52 µs` |
| `bucket_fill/non_contiguous/512` | `280.61 µs` |
| `bucket_fill/tolerance/512` | `390.72 µs` |
| `brush/round_hard_small/256` | `68.79 µs` |
| `brush/round_soft_large/512` | `648.18 µs` |
| `brush/erase_soft/512` | `297.56 µs` |
| `brush/long_pressure_stroke/1024` | `1.36 ms` |
| `brush/streamed_long_pressure_stroke/1024` | `1.35 ms` |
| `brush/grain_tilt_modeler/512` | `713.14 µs` |
| `brush/repeated_short_strokes/512` | `78.33 µs` |
| `encode_png/512` | `1.26 ms` |
| `decode_png/512` | `1.27 ms` |
| `encode_jpeg/512` | `2.09 ms` |
| `decode_jpeg/512` | `1.21 ms` |
| `encode_webp/512` | `1.43 ms` |
| `decode_webp/512` | `2.70 ms` |
| `extract_palette/512/16colors` | `20.85 ms` |
| `shape/rasterize_rectangle/512` | `879.04 µs` |
| `shape/rasterize_polygon/512` | `12.02 ms` |
| `resize_nearest/512→1024` | `1.64 ms` |
| `resize_bilinear/512→1024` | `1.03 ms` |
| `resize_lanczos3/512→1024` | `1.61 ms` |
| `resize_lanczos3/2048→4096` | `53.55 ms` |

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

These vary slightly with toolchain and optimization settings.

## Roadmap

Tracked for later:

- Selection system
- Selection-aware painting and fill
- Richer brush tools: alpha lock, symmetry, scatter/jitter, smudge/wet tools, and selection-aware painting

Possible follow-up work if those areas become important:

- Keep PSD import experimental unless it becomes a priority again
- Improve text editing ergonomics once selection exists

## License

MIT
