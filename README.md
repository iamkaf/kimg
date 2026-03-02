# kimg

[![CI](https://github.com/iamkaf/kimg/actions/workflows/ci.yml/badge.svg)](https://github.com/iamkaf/kimg/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![WASM Size](https://img.shields.io/badge/wasm-271KB-green.svg)]()

A Rust+WASM image compositing engine. Think of it as a headless Photoshop you can `import` — layers, blend modes, filters, masks, and multi-format I/O, all running in a ~270KB WASM binary.

It works the same way in Node.js and the browser. No native dependencies, no Canvas API, no DOM.

## Why this exists

Most image libraries treat images as single buffers — apply a filter, resize, encode, done. If you need *layers* composited together with blend modes, scoped filters, and a render pipeline, your options are either browser-only (Photopea), commercial (IMG.LY), or huge (magick-wasm at 7MB+).

kimg fills that gap. Originally extracted from the [Spriteform](https://spriteform.com) compositor (which was pure JS and slow), it now runs 5-15x faster and doesn't need Node.js or Electron.

## Install

```bash
npm install kimg
```

## Quick start

### Browser

```js
import init, { Document } from 'kimg';

await init();

const doc = new Document(128, 128);
const layerId = doc.add_image_layer('sprite', rgbaPixels, 128, 128, 0, 0);
doc.set_opacity(layerId, 0.8);

const png = doc.export_png();
```

### Node.js

```js
import { readFileSync } from 'fs';
import { initSync, Document } from 'kimg';

initSync(readFileSync(new URL('kimg_wasm_bg.wasm', import.meta.resolve('kimg'))));

const doc = new Document(64, 64);
// same API from here on
```

## What it can do

**Layers** — Image, Paint, Filter, Group, SolidColor, Gradient. Nested groups with scoped filter application.

**16 blend modes** — Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity.

**Masks** — Grayscale layer masks and clipping masks (`clip_to_below`).

**Filters** — HSL adjustments, brightness/contrast, temperature/tint, sharpen. Invert, posterize, threshold, levels, gradient map. Box blur, Gaussian blur, edge detect, emboss (all as convolution kernels).

**Transforms** — Resize (nearest-neighbor, bilinear, Lanczos3), arbitrary-angle rotation, crop, trim alpha.

**Sprite tools** — Sprite sheet packer (shelf bin-packing), contact sheet grids, pixel-art upscale, color quantization, batch render pipeline.

**Format support** — PNG, JPEG, WebP, GIF (animated frames → layers), PSD (layer import). Auto-detection via magic bytes.

**Serialization** — Save/load full documents as `.kimg` files (JSON metadata + binary pixel data).

## Subpath exports

```js
// Base64 RGBA helpers — pure JS, no WASM init needed
import { rgbaToBase64, base64ToRgba } from 'kimg/base64';

// Pick readable text color for a background (needs WASM init first)
import { readableTextColor } from 'kimg/color-utils';
readableTextColor('#1a1a2e'); // '#ffffff'
readableTextColor('#f0f0f0'); // '#000000'
```

## Color utilities

These are free functions, not tied to a document:

```js
import { hex_to_rgb, rgb_to_hex, relative_luminance, contrast_ratio, dominant_rgb_from_rgba } from 'kimg';

hex_to_rgb('#ff8000');                    // Uint8Array [255, 128, 0]
rgb_to_hex(255, 128, 0);                  // '#ff8000'
relative_luminance('#3b82f6');            // 0.2355 (WCAG 2.x)
contrast_ratio('#ffffff', '#000000');     // 21.0
dominant_rgb_from_rgba(pixels, 128, 128); // Uint8Array [r, g, b]
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
│   │   │   ├── codec.rs       # PNG, JPEG, WebP, GIF, PSD decode/encode
│   │   │   ├── color.rs       # RGB/HSL conversion, luminance, contrast
│   │   │   ├── convolution.rs # Blur, sharpen, edge detect, emboss kernels
│   │   │   ├── document.rs    # Document struct, layer tree, render pipeline
│   │   │   ├── filter.rs      # HSL filters, invert, posterize, threshold, levels
│   │   │   ├── layer.rs       # Layer types and common properties
│   │   │   ├── serialize.rs   # Document save/load
│   │   │   ├── sprite.rs      # Sprite sheet packing, contact sheets, quantization
│   │   │   └── transform.rs   # Resize, rotate, crop, trim
│   │   └── benches/           # Criterion.rs benchmarks
│   └── kimg-wasm/     # wasm-bindgen API surface
├── pkg/               # Built output (JS + WASM + TypeScript types)
├── demo/              # Browser demo page
└── scripts/           # Build scripts
```

## Building from source

You need Rust, `wasm32-unknown-unknown` target, and `wasm-bindgen-cli`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

./scripts/build.sh
```

Output goes to `pkg/`. The demo page at `demo/index.html` loads from there.

## Running tests

```bash
cargo test -p kimg-core
```

117 tests covering blend modes, compositing, filters, transforms, codecs, serialization, sprites, and color utilities.

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
| `document` | Full render pipeline at 1–10 layers with and without filter layers |
| `codec` | PNG / JPEG / WebP encode and decode of a 512×512 buffer |
| `sprite` | Sprite sheet packing, palette extraction, quantization, pixel-art scale |

## WASM binary size

The full binary with all codecs (PNG, JPEG, WebP, GIF, PSD) is ~271KB uncompressed. Gzipped it sits around 120KB.

## License

MIT
