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
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
./scripts/build.sh
```

This builds the consumable JS/WASM package into `dist/`.

## Quick start

### Browser

```js
import { Composition } from '@iamkaf/kimg';

const doc = await Composition.create({ width: 128, height: 128 });
const layerId = doc.add_image_layer('sprite', rgbaPixels, 128, 128, 0, 0);
doc.set_opacity(layerId, 0.8);

const png = doc.export_png();
```

### Node.js

```js
import { Composition } from '@iamkaf/kimg';

const doc = await Composition.create({ width: 64, height: 64 });
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
import { rgbaToBase64, base64ToRgba } from './dist/base64.js';

// Pick readable text color for a background
import { readableTextColor } from '@iamkaf/kimg/color-utils';
readableTextColor('#1a1a2e'); // '#ffffff'
readableTextColor('#f0f0f0'); // '#000000'

// Low-level wasm-bound API
import initRaw, { Composition as RawComposition } from '@iamkaf/kimg/raw';

await initRaw();
const raw = new RawComposition(128, 128);
```

## Color utilities

These are free functions, not tied to a document:

```js
import { hexToRgb, rgbToHex, relativeLuminance, contrastRatio, dominantRgbFromRgba } from '@iamkaf/kimg';

await hexToRgb('#ff8000');                     // Uint8Array [255, 128, 0]
await rgbToHex(255, 128, 0);                   // '#ff8000'
await relativeLuminance('#3b82f6');            // 0.2355 (WCAG 2.x)
await contrastRatio('#ffffff', '#000000');     // 21.0
await dominantRgbFromRgba(pixels, 128, 128);   // Uint8Array [r, g, b]
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
├── js/                # Tracked JS/TS package sources copied into dist/
├── dist/              # Built output (JS + WASM + TypeScript types)
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

Output goes to `dist/`. The demo page at `demo/index.html` loads from there.

The build emits two wasm binaries:

- `kimg_wasm_bg.wasm` for the baseline target
- `kimg_wasm_simd_bg.wasm` for runtimes with `wasm32` SIMD (`simd128`)

## Running tests

```bash
cargo test -p kimg-core
```

121 tests covering blend modes, compositing, filters, transforms, codecs, serialization, sprites, and color utilities.

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

Notes on the harnesses:

- Very expensive resize cases use reduced flat-sampled Criterion groups so `cargo bench -p kimg-core` stays practical while still reporting worst-case medians.
- RGBA bilinear and Lanczos3 resize paths use `fast_image_resize`, so native builds pick up host SIMD and the browser `Composition.create()` path can load the separate `simd128` wasm artifact.
- Codec benchmarks use a deterministic textured 512×512 image instead of a flat fill, which avoids unrealistically optimistic compression timings.

Representative medians from a recent local run on March 2, 2026. These are hardware-dependent and should be treated as a baseline example, not a guarantee:

| Operation | Median |
|------|------:|
| `render/single_image/512` | `5.01 ms` |
| `render/10_layers/512` | `42.69 ms` |
| `render/10_layers_with_filter/512` | `48.61 ms` |
| `encode_png/512` | `1.25 ms` |
| `decode_png/512` | `1.24 ms` |
| `encode_jpeg/512` | `2.18 ms` |
| `decode_jpeg/512` | `1.21 ms` |
| `encode_webp/512` | `1.41 ms` |
| `decode_webp/512` | `2.65 ms` |
| `extract_palette/512/16colors` | `20.45 ms` |
| `resize_nearest/512→1024` | `1.63 ms` |
| `resize_bilinear/512→1024` | `1.01 ms` |
| `resize_lanczos3/512→1024` | `1.59 ms` |
| `resize_lanczos3/2048→4096` | `52.69 ms` |

## WASM binary size

Current local release build sizes:

- `dist/kimg_wasm_bg.wasm`: `934 KB` uncompressed, `346,582` bytes gzipped
- `dist/kimg_wasm_simd_bg.wasm`: `1.1 MB` uncompressed, `385,388` bytes gzipped

These vary slightly with toolchain and optimization settings.

## License

MIT
