# kimg — Headless JavaScript Photoshop

**Product Requirements Document v1.0**

---

## 1. Executive Summary

kimg is an open-source, Rust-compiled-to-WASM image compositing engine that exposes a programmatic JavaScript API for both Node.js and the browser. It provides the core rendering pipeline of a layer-based image editor — compositing, blending, filters, transformations, and pixel manipulation — without any UI, enabling developers to build image editors, game asset pipelines, automated design tools, and server-side rendering services on a single high-performance core.

**Tagline:** *Headless JavaScript Photoshop.*

**One-liner:** A Rust+WASM layer compositing engine with a JavaScript API that runs identically in Node.js and the browser.

---

## 2. Problem Statement

### The gap in the market

There is no open-source library that combines:

1. **Layer-based compositing** (not just single-image-in, single-image-out)
2. **Near-native performance** via Rust/WASM
3. **Universal runtime** — identical API in Node.js and browsers
4. **Headless/programmatic operation** — no DOM, no Canvas element required

The existing landscape is fragmented:

| Library | Layers | WASM/Native Speed | Browser | Node | Headless | Open Source |
|---------|--------|-------------------|---------|------|----------|-------------|
| Sharp | No | Yes (C) | No | Yes | Yes | Yes |
| Jimp | No | No (pure JS) | Yes | Yes | Yes | Yes |
| Photon (dead) | No | Yes (Rust) | Yes | Yes | Partial | Yes |
| @imagemagick/magick-wasm | No | Yes (C) | Yes | Yes | Yes | Yes |
| Photopea | Yes | No (JS+WebGL) | Yes | No | No | No |
| IMG.LY CE.SDK | Yes | Partial | Yes | Partial | Yes | No (paid) |
| **kimg** | **Yes** | **Yes (Rust)** | **Yes** | **Yes** | **Yes** | **Yes** |

### Our specific need

The Spriteform Compositor currently implements its entire rendering pipeline in JavaScript (pngjs), including:

- Layer tree traversal and compositing with alpha blending
- Affine transformations (rotation at 90° intervals, flip X/Y, anchor modes)
- HSL color adjustments, brightness/contrast, temperature/tint, sharpen kernels
- Paint layers with raw RGBA buffer management
- Smart layer variant scoping with isolated filter application
- Multi-level LRU caching and worker pool parallelism
- Turbo generation (combinatorial variant explosion)

This is correct and ships today, but it's slow for large composites and cannot run in a web-only context (it's tied to Node.js via pngjs and the filesystem). kimg extracts and generalizes this rendering core into a standalone, high-performance library.

---

## 3. Goals

### Primary Goals

1. **Extract the Spriteform Compositor's rendering engine** into a reusable library, decoupled from Electron/Node.js specifics.
2. **Achieve 5-15x performance improvement** over the current pure-JS pipeline for pixel operations (blitting, filtering, blending).
3. **Run identically in Node.js and browsers** — one WASM binary, one JS API surface.
4. **Keep the WASM binary under 500KB gzipped** (vs. wasm-vips at 4.6MB, magick-wasm at 7MB+).
5. **Provide a compositing model, not just image processing** — layers, groups, blend modes, filter stacks, and scene-graph rendering.

### Non-Goals (v1)

- GUI or visual editor components
- GPU/WebGL acceleration (future consideration)
- Video or animation processing
- RAW photo format support (e.g., CR2, NEF)
- AI/ML-based features (upscaling, inpainting, background removal)

---

## 4. Architecture

### 4.1 System Overview

```
┌─────────────────────────────────────────────────────┐
│  JavaScript API  (@kimg/core)                     │
│  ┌───────────────┐ ┌────────────┐ ┌──────────────┐ │
│  │  Document      │ │  Layer     │ │  Export      │ │
│  │  Management    │ │  Operations│ │  Pipeline    │ │
│  └───────┬───────┘ └─────┬──────┘ └──────┬───────┘ │
│          │               │               │          │
│  ┌───────┴───────────────┴───────────────┴───────┐  │
│  │          wasm-bindgen bridge                   │  │
│  └───────────────────────┬───────────────────────┘  │
├──────────────────────────┼──────────────────────────┤
│  Rust Core (kimg)      │                          │
│  ┌───────────────────────┴───────────────────────┐  │
│  │  Scene Graph                                   │  │
│  │  ┌─────────┐ ┌─────────┐ ┌──────────────────┐ │  │
│  │  │ Document│ │ Layer   │ │ Render Pipeline  │ │  │
│  │  │ (canvas)│ │ Tree    │ │ (compositor)     │ │  │
│  │  └─────────┘ └─────────┘ └──────────────────┘ │  │
│  ├───────────────────────────────────────────────┤  │
│  │  Pixel Engine                                  │  │
│  │  ┌────────┐ ┌────────┐ ┌───────┐ ┌─────────┐ │  │
│  │  │ Blend  │ │ Filter │ │ Blit  │ │ Color   │ │  │
│  │  │ Modes  │ │ Stack  │ │ Xform │ │ Space   │ │  │
│  │  └────────┘ └────────┘ └───────┘ └─────────┘ │  │
│  ├───────────────────────────────────────────────┤  │
│  │  I/O                                           │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐             │  │
│  │  │  PNG   │ │  JPEG  │ │  RGBA  │             │  │
│  │  │  codec │ │  codec │ │  raw   │             │  │
│  │  └────────┘ └────────┘ └────────┘             │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 4.2 Crate Structure

```
kimg/
├── Cargo.toml              # workspace root
├── crates/
│   ├── kimg-core/        # Pure Rust pixel engine (no WASM deps)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pixel.rs        # RGBA pixel type, arithmetic
│   │   │   ├── buffer.rs       # ImageBuffer<RGBA8> with owned data
│   │   │   ├── blend.rs        # Blend modes (Normal, Multiply, Screen, etc.)
│   │   │   ├── blit.rs         # Transformed blit (position, flip, rotation, opacity)
│   │   │   ├── filter.rs       # Filter pipeline (HSL, brightness, contrast, etc.)
│   │   │   ├── color.rs        # Color space conversions (RGB, HSL, HSV, Lab)
│   │   │   ├── transform.rs    # Affine transforms, nearest-neighbor sampling
│   │   │   ├── composite.rs    # Layer tree, scene graph, render traversal
│   │   │   ├── mask.rs         # Alpha masks, luminance masks, clipping
│   │   │   ├── resize.rs       # Nearest-neighbor, bilinear, Lanczos
│   │   │   ├── quantize.rs     # Color quantization, palette generation
│   │   │   ├── analyze.rs      # Dominant color, histogram, statistics
│   │   │   └── codec/
│   │   │       ├── png.rs
│   │   │       ├── jpeg.rs
│   │   │       └── raw.rs      # Raw RGBA import/export
│   │   └── Cargo.toml
│   │
│   └── kimg-wasm/        # WASM bindings via wasm-bindgen
│       ├── src/
│       │   ├── lib.rs          # #[wasm_bindgen] API surface
│       │   ├── document.rs     # Document/canvas management
│       │   ├── layer.rs        # Layer CRUD + property access
│       │   ├── filter.rs       # Filter creation/application
│       │   └── export.rs       # Render + encode to bytes
│       └── Cargo.toml
│
├── packages/
│   └── kimg/             # npm package (JS wrapper + .wasm)
│       ├── src/
│       │   ├── index.ts        # Main entrypoint, async init
│       │   ├── document.ts     # Document class (thin wrapper)
│       │   ├── layer.ts        # Layer class hierarchy
│       │   ├── filter.ts       # Filter builder API
│       │   └── types.ts        # TypeScript type definitions
│       ├── package.json
│       └── tsconfig.json
│
├── tests/                  # Integration tests
├── benches/                # Criterion benchmarks
└── examples/
    ├── node-composite/     # Node.js compositing example
    ├── browser-editor/     # Minimal browser editor
    └── spritesheet/        # Sprite sheet generation
```

### 4.3 Memory Model

All image data lives in WASM linear memory. The JavaScript API never copies full pixel buffers across the WASM boundary during compositing — only on explicit import/export.

```
JS Heap                    WASM Linear Memory
┌──────────┐               ┌──────────────────────┐
│ Document │──── handle ───►│ Document struct       │
│ Layer    │──── handle ───►│ Layer struct          │
│          │               │ ┌──────────────────┐  │
│          │               │ │ ImageBuffer      │  │
│          │               │ │ [RGBA8 pixels]   │  │
│          │               │ └──────────────────┘  │
└──────────┘               └──────────────────────┘

Pixel data crosses the boundary only on:
  → importRgba(Uint8Array)    JS ──copy──► WASM
  ← exportPng(): Uint8Array   WASM ──copy──► JS
  ← toRgba(): Uint8Array      WASM ──view──► JS (zero-copy view)
```

---

## 5. Core API Design

### 5.1 Initialization

```typescript
import { initKimg } from '@kimg/core';

// Browser: fetches .wasm from CDN or bundled
// Node.js: reads .wasm from node_modules
const kaf = await initKimg();
```

The `initKimg()` function returns the library singleton. All constructors hang off this object to ensure the WASM module is initialized before use.

### 5.2 Document

A Document is the top-level container — analogous to a Photoshop document or a Spriteform Composite. It owns a canvas size and a layer tree.

```typescript
const doc = kaf.createDocument({ width: 64, height: 64 });
doc.resize(128, 128);     // resize canvas (not content)
doc.width;                 // 128
doc.height;                // 128
```

### 5.3 Layers

kimg supports a discriminated layer type hierarchy matching and extending the Spriteform Compositor's model:

| Layer Type | Description | Spriteform Equivalent |
|-----------|-------------|----------------------|
| `ImageLayer` | Raster image with position, transform, opacity | `SpritePartLayer` |
| `PaintLayer` | Editable RGBA buffer for drawing | `PaintLayer` |
| `FilterLayer` | Non-destructive adjustment (HSL, B/C, etc.) | `FilterLayer` |
| `GroupLayer` | Folder containing child layers | `FolderLayer` |
| `SolidColorLayer` | Flat color fill | *new* |
| `GradientLayer` | Linear/radial gradient fill | *new* |

```typescript
// Add an image layer from PNG bytes
const layer = doc.addImageLayer({
  name: 'Base Sprite',
  data: pngBytes,           // Uint8Array (PNG, JPEG, or raw RGBA)
  format: 'png',            // 'png' | 'jpeg' | 'rgba'
  x: 0,
  y: 0,
});

// Or from raw RGBA
const paint = doc.addPaintLayer({
  name: 'Paint',
  width: 64,
  height: 64,
});

// Nest layers in groups
const group = doc.addGroupLayer({ name: 'Head Parts' });
group.addImageLayer({ name: 'Hair', data: hairPng, format: 'png' });
group.addImageLayer({ name: 'Eyes', data: eyesPng, format: 'png' });
```

#### Layer Properties

Every layer (except FilterLayer) exposes spatial and blending properties:

```typescript
layer.x = 10;                         // position
layer.y = -5;
layer.opacity = 0.75;                 // 0.0 – 1.0
layer.visible = false;                // skip during render
layer.blendMode = 'multiply';         // see Blend Modes section
layer.anchor = 'center';              // 'topleft' | 'center'
layer.flipX = true;                   // horizontal mirror
layer.flipY = false;                  // vertical mirror
layer.rotation = 90;                  // 0 | 90 | 180 | 270
layer.name = 'Renamed';
```

#### Layer Tree Operations

```typescript
doc.moveLayer(layerId, { before: otherLayerId });
doc.moveLayer(layerId, { into: groupId, index: 0 });
doc.removeLayer(layerId);
doc.duplicateLayer(layerId);           // deep clone with new IDs
doc.flattenLayer(groupId);            // merge group into single image layer

const layer = doc.getLayerById(id);
const layers = doc.getLayers();        // top-level layers
const all = doc.getLayersFlat();       // all layers, depth-first
```

### 5.4 Blend Modes

kimg implements standard Photoshop-compatible blend modes at the pixel level:

**Normal:**
- `normal` — standard alpha compositing (Porter-Duff source-over)

**Darken group:**
- `darken` — min(src, dst) per channel
- `multiply` — src × dst
- `colorBurn` — inverted divide
- `linearBurn` — src + dst - 1

**Lighten group:**
- `lighten` — max(src, dst) per channel
- `screen` — 1 - (1-src)(1-dst)
- `colorDodge` — dst / (1-src)
- `linearDodge` — src + dst (add)

**Contrast group:**
- `overlay` — multiply/screen hybrid based on dst
- `softLight` — gentle contrast
- `hardLight` — multiply/screen hybrid based on src

**Inversion group:**
- `difference` — |src - dst|
- `exclusion` — src + dst - 2(src)(dst)

**Component group (HSL-based):**
- `hue` — hue from src, sat+lum from dst
- `saturation` — saturation from src
- `color` — hue+sat from src, lum from dst
- `luminosity` — lum from src, hue+sat from dst

### 5.5 Filters

Filters are non-destructive adjustment layers that modify pixels beneath them during rendering. They do not alter source data.

```typescript
// Add a filter layer to the document
const filter = doc.addFilterLayer({
  name: 'Color Shift',
  kind: 'hsl',                         // filter UI grouping
});

// HSL adjustments
filter.hue = 30;                       // degrees, -180 to 180
filter.saturation = 0.2;               // -1.0 to 1.0
filter.lightness = -0.1;               // -1.0 to 1.0

// Tone adjustments
filter.brightness = 0.15;              // -1.0 to 1.0
filter.contrast = 0.3;                 // -1.0 to 1.0

// FX adjustments
filter.temperature = -0.2;             // -1.0 to 1.0 (cool ↔ warm)
filter.tint = 0.1;                     // -1.0 to 1.0 (green ↔ magenta)
filter.sharpen = 0.5;                  // 0.0 to 1.0 (unsharp mask strength)

// Alpha adjustment
filter.alpha = -0.3;                   // -1.0 to 1.0 (alpha delta)
```

#### Additional Filter Types (beyond Spriteform)

```typescript
// Convolution kernels
const blur = doc.addFilterLayer({ name: 'Blur', kind: 'convolve' });
blur.kernel = kaf.kernels.gaussianBlur(3);    // 3×3 Gaussian
blur.strength = 0.8;

// Levels / Curves
const levels = doc.addFilterLayer({ name: 'Levels', kind: 'levels' });
levels.inputBlack = 20;
levels.inputWhite = 235;
levels.gamma = 1.2;
levels.outputBlack = 0;
levels.outputWhite = 255;

// Posterize
const poster = doc.addFilterLayer({ name: 'Posterize', kind: 'posterize' });
poster.levels = 4;                     // number of color levels per channel

// Color Ramp / Gradient Map
const ramp = doc.addFilterLayer({ name: 'Gradient Map', kind: 'gradientMap' });
ramp.stops = [
  { position: 0.0, color: '#000000' },
  { position: 0.5, color: '#ff6600' },
  { position: 1.0, color: '#ffffff' },
];

// Threshold
const thresh = doc.addFilterLayer({ name: 'Threshold', kind: 'threshold' });
thresh.level = 128;                    // 0-255

// Invert
const inv = doc.addFilterLayer({ name: 'Invert', kind: 'invert' });
```

### 5.6 Masks

Every layer can have an optional alpha mask that restricts its visible area:

```typescript
// Set a mask from grayscale data (white = visible, black = hidden)
layer.setMask(maskBytes, { format: 'grayscale', width: 64, height: 64 });

// Or from another layer's alpha channel
layer.setMaskFromAlpha(otherLayerId);

// Clipping mask: layer is clipped to the non-transparent area of the layer below
layer.clippingMask = true;

// Disable without deleting
layer.maskEnabled = false;

// Invert mask
layer.maskInverted = true;
```

### 5.7 Rendering & Export

```typescript
// Render the full document to a flat RGBA buffer (in WASM memory)
const rendered = doc.render();
rendered.width;                        // number
rendered.height;                       // number
rendered.toRgba();                     // Uint8Array (zero-copy view into WASM memory)
rendered.toRgbaCopy();                 // Uint8Array (copied to JS heap, safe to keep)

// Encode to format
const pngBytes = doc.exportPng();                         // Uint8Array
const jpegBytes = doc.exportJpeg({ quality: 85 });        // Uint8Array

// Render a specific layer subtree (isolate a group or smart layer)
const subtree = doc.renderLayer(groupId);
subtree.exportPng();

// Render at a different scale (nearest-neighbor for pixel art)
const scaled = doc.render({ scale: 4, interpolation: 'nearest' });
```

### 5.8 Pixel-Level Access

For paint tools, procedural generation, or custom effects:

```typescript
// Direct pixel buffer access on a PaintLayer
const buf = paint.getPixels();         // RgbaBuffer (view into WASM memory)
buf.getPixel(x, y);                    // { r, g, b, a }
buf.setPixel(x, y, { r: 255, g: 0, b: 0, a: 255 });

// Bulk operations
buf.fill({ r: 0, g: 0, b: 0, a: 0 });                   // clear
buf.fillRect(x, y, w, h, { r: 255, g: 255, b: 255, a: 128 });
buf.blit(srcBuffer, dx, dy);                               // copy region
buf.blitTransformed(srcBuffer, {                           // Spriteform's blitTransformed
  x: 10, y: 10, anchor: 'center',
  flipX: true, rotation: 90, opacity: 0.8,
});

// Apply a function to every pixel
buf.mapPixels((r, g, b, a, x, y) => {
  return { r: 255 - r, g: 255 - g, b: 255 - b, a };     // invert
});
```

### 5.9 Color Utilities

```typescript
// Color space conversions
kaf.color.rgbToHsl(255, 128, 0);       // { h, s, l }
kaf.color.hslToRgb(30, 1.0, 0.5);     // { r, g, b }
kaf.color.rgbToHsv(255, 128, 0);       // { h, s, v }
kaf.color.rgbToLab(255, 128, 0);       // { l, a, b } (CIE L*a*b*)
kaf.color.hexToRgb('#ff8000');         // { r, g, b }
kaf.color.rgbToHex(255, 128, 0);       // '#ff8000'

// Analysis
kaf.color.dominantColor(rgbaBuffer, width, height);        // { r, g, b }
kaf.color.histogram(rgbaBuffer, width, height);            // { r[], g[], b[], a[] }
kaf.color.relativeLuminance('#ff8000');                     // WCAG luminance
kaf.color.contrastRatio('#ffffff', '#000000');              // WCAG contrast ratio
kaf.color.readableTextColor('#336699');                     // '#ffffff' or '#000000'
```

### 5.10 Image I/O

```typescript
// Decode
const img = kaf.decode(bytes, 'png');    // RgbaBuffer
const img = kaf.decode(bytes, 'jpeg');
const img = kaf.decode(bytes, 'auto');   // detect format from magic bytes

// Encode
const png = kaf.encodePng(rgbaBuffer, width, height);
const jpeg = kaf.encodeJpeg(rgbaBuffer, width, height, { quality: 85 });

// Raw RGBA round-trip
const raw = kaf.fromRgba(uint8Array, width, height);
const out = raw.toRgba();

// Base64 helpers (for Spriteform compatibility)
const buf = kaf.decodeRgbaBase64(base64String, width, height);
const b64 = kaf.encodeRgbaBase64(rgbaBuffer);
```

### 5.11 Resize & Transform

```typescript
// Resize with interpolation control
const resized = kaf.resize(rgbaBuffer, {
  width: 128,
  height: 128,
  interpolation: 'nearest',            // 'nearest' | 'bilinear' | 'lanczos3'
});

// Pixel-art upscale (nearest-neighbor, integer scales only)
const upscaled = kaf.pixelScale(rgbaBuffer, 4);            // 4x scale

// Crop
const cropped = kaf.crop(rgbaBuffer, { x: 10, y: 10, width: 32, height: 32 });

// Trim transparent edges
const trimmed = kaf.trimAlpha(rgbaBuffer);                 // { buffer, x, y, width, height }

// Rotate (arbitrary angle, or fast 90° increments)
const rotated = kaf.rotate(rgbaBuffer, 90);                // 0, 90, 180, 270 (fast path)
const rotatedArb = kaf.rotate(rgbaBuffer, 45, {            // arbitrary (slower)
  interpolation: 'bilinear',
  background: { r: 0, g: 0, b: 0, a: 0 },
});

// Flip
const flippedH = kaf.flipHorizontal(rgbaBuffer);
const flippedV = kaf.flipVertical(rgbaBuffer);
```

### 5.12 Sprite Sheet Generation

A first-class feature for game development workflows:

```typescript
const sheet = kaf.createSpriteSheet({
  padding: 1,                          // px between sprites
  powerOfTwo: true,                    // constrain to power-of-two dimensions
  maxWidth: 2048,
  layout: 'auto',                      // 'auto' | 'row' | 'column' | 'grid'
});

sheet.addFrame('idle_0', rgbaBuffer0, 16, 16);
sheet.addFrame('idle_1', rgbaBuffer1, 16, 16);
sheet.addFrame('walk_0', rgbaBuffer2, 16, 16);
// ...

const result = sheet.pack();
result.image;                          // Uint8Array (PNG)
result.atlas;                          // { frames: { [name]: { x, y, w, h } }, width, height }
result.atlasJson;                      // JSON string (TexturePacker-compatible format)
```

### 5.13 Contact Sheet / Grid Layout

For marketing and documentation:

```typescript
const grid = kaf.createGrid({
  columns: 4,
  cellWidth: 64,
  cellHeight: 64,
  padding: 4,
  background: '#1a1a2e',
  scale: 2,                            // render each cell at 2x
  interpolation: 'nearest',
});

grid.addCell(rgbaBuffer, { label: 'Knight' });
grid.addCell(rgbaBuffer, { label: 'Mage' });
// ...

const image = grid.render();           // Uint8Array (PNG)
```

### 5.14 Batch / Pipeline API

For server-side automation, CI/CD asset pipelines, and Spriteform's turbo generation:

```typescript
// Compositing pipeline: render many variants from a single document
const doc = kaf.createDocument({ width: 32, height: 32 });
// ... set up layers ...

const variants = [
  { 'hair': 'red', 'eyes': 'blue' },
  { 'hair': 'blonde', 'eyes': 'green' },
  // ...
];

const results = await kaf.batch(variants, async (selection) => {
  // Swap layer visibility or content based on selection
  doc.getLayerById(hairLayers[selection.hair]).visible = true;
  doc.getLayerById(eyeLayers[selection.eyes]).visible = true;

  const png = doc.exportPng();

  // Reset
  for (const l of Object.values(hairLayers)) doc.getLayerById(l).visible = false;
  for (const l of Object.values(eyeLayers)) doc.getLayerById(l).visible = false;

  return { key: `${selection.hair}_${selection.eyes}`, data: png };
});
```

### 5.15 Serialization

Documents can be serialized to a portable JSON+binary format for persistence:

```typescript
// Serialize document state
const serialized = doc.serialize();     // { json: string, buffers: Uint8Array[] }

// Deserialize
const restored = kaf.deserialize(serialized);

// Or just the layer tree as JSON (no pixel data)
const tree = doc.toJSON();
```

---

## 6. Performance Targets

All benchmarks measured on a 256×256 RGBA canvas with 8 layers, compared against the current Spriteform JS implementation (pngjs-based).

| Operation | JS Baseline (pngjs) | kimg Target | Speedup |
|-----------|---------------------|---------------|---------|
| Alpha composite (2 layers) | ~12ms | <1ms | 12x+ |
| 8-layer full render | ~85ms | <8ms | 10x+ |
| HSL filter (full canvas) | ~18ms | <2ms | 9x+ |
| Sharpen convolution (3×3) | ~22ms | <3ms | 7x+ |
| Transformed blit (flip+rotate+opacity) | ~15ms | <2ms | 7x+ |
| PNG encode (256×256) | ~25ms | <5ms | 5x+ |
| PNG decode (256×256) | ~20ms | <4ms | 5x+ |
| Batch: 100 variants | ~8.5s | <800ms | 10x+ |

### WASM Binary Size Target

| Component | Target (gzipped) |
|-----------|------------------|
| Core (compositing + filters + blend modes) | <200KB |
| + PNG codec | <280KB |
| + JPEG codec | <350KB |
| + All codecs + utilities | <500KB |

---

## 7. Platform Support

### Browser

- Chrome 91+ / Edge 91+
- Firefox 89+
- Safari 15+
- Requires `WebAssembly` support (99%+ of global browser traffic)
- No DOM or Canvas dependency — works in Web Workers
- ESM import with top-level await for initialization

### Node.js

- Node.js 18+ (LTS)
- Uses `@aspect-build/rules_js`-compatible WASM loading
- No native dependencies — pure WASM + JS
- Works in Cloudflare Workers, Deno, Bun

### Package Distribution

```
@kimg/core         # Main package (JS wrapper + WASM binary)
@kimg/core/node    # Node.js-specific entrypoint (sync WASM load)
@kimg/core/web     # Browser-specific entrypoint (async fetch)
```

---

## 8. Rust Crate Design Principles

### 8.1 Zero-copy where possible

- Layer pixel data is stored as contiguous `Vec<u8>` in RGBA8 format.
- Rendering composites into a shared output buffer using mutable slices — no intermediate allocations per layer.
- Filters operate in-place on the output buffer.

### 8.2 SIMD-ready inner loops

Structure critical pixel loops to be auto-vectorizable by LLVM:

```rust
// This pattern auto-vectorizes with wasm-simd
for i in (0..len).step_by(4) {
    let sa = src[i + 3] as f32 / 255.0 * opacity;
    let da = dst[i + 3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a > 0.0 {
        dst[i + 0] = ((src[i + 0] as f32 * sa + dst[i + 0] as f32 * da * (1.0 - sa)) / out_a) as u8;
        dst[i + 1] = ((src[i + 1] as f32 * sa + dst[i + 1] as f32 * da * (1.0 - sa)) / out_a) as u8;
        dst[i + 2] = ((src[i + 2] as f32 * sa + dst[i + 2] as f32 * da * (1.0 - sa)) / out_a) as u8;
        dst[i + 3] = (out_a * 255.0) as u8;
    }
}
```

Target `wasm32-unknown-unknown` with `simd128` feature for browsers that support it, with a scalar fallback.

### 8.3 No `std` dependency in the pixel engine

The core pixel operations (`kimg-core`) should be `#![no_std]` compatible using `alloc` only. This enables:
- Smaller WASM binary (no std bloat)
- Potential future use in embedded contexts
- Cleaner dependency tree

### 8.4 Minimal dependencies

```toml
[dependencies]
# Core pixel math: zero dependencies
# PNG: use a minimal decoder/encoder
png = "0.17"               # ~40KB contribution to WASM
# JPEG: optional feature
jpeg-decoder = { version = "0.3", optional = true }
jpeg-encoder = { version = "0.6", optional = true }

[dependencies.wasm-bindgen]
version = "0.2"
# Only in kimg-wasm, not kimg-core
```

---

## 9. Feature Roadmap

### v0.1 — Foundation (MVP)

Priority: Ship the core that can replace Spriteform's JS compositor.

- [x] `ImageBuffer<RGBA8>` with owned pixel data
- [x] Alpha blending (Porter-Duff source-over) — `normal` blend mode
- [x] Transformed blit (position, anchor, flipX/Y, rotation 0/90/180/270, opacity)
- [x] Filter: HSL adjustment (hue, saturation, lightness)
- [x] Filter: Alpha adjustment
- [x] Filter: Brightness, Contrast
- [x] Filter: Temperature, Tint
- [x] Filter: Sharpen (3×3 unsharp mask)
- [x] Layer tree: ImageLayer, PaintLayer, FilterLayer, GroupLayer
- [x] Layer visibility, opacity, position
- [x] Document rendering (back-to-front traversal, folder recursion)
- [x] Scoped filter application (filters within a group only affect that group)
- [x] PNG decode/encode
- [x] Raw RGBA import/export
- [x] Base64 RGBA encode/decode (Spriteform compat)
- [x] WASM bindings via wasm-bindgen
- [x] npm package with Node.js + Browser entrypoints
- [x] Color utilities: RGB↔HSL, hex conversion, dominant color, luminance, contrast ratio, readable text color

### v0.2 — Extended Blend Modes & Masks

- [x] All 16 blend modes (darken, multiply, screen, overlay, etc.)
- [x] Layer masks (grayscale alpha mask per layer)
- [x] Clipping masks
- [x] Mask inversion
- [x] Flatten group to single layer
- [x] SolidColorLayer, GradientLayer

### v0.3 — Advanced Filters & Transforms

- [x] Convolution kernels (blur, edge detect, emboss, custom)
- [x] Levels/Curves adjustment
- [x] Posterize, Threshold, Invert filters
- [x] Gradient Map filter
- [x] Arbitrary-angle rotation (bilinear interpolation)
- [x] Resize with bilinear and Lanczos3 interpolation
- [x] Crop, trim alpha

### v0.4 — Sprite & Game Dev Tools

- [x] Sprite sheet packer (bin-packing algorithm)
- [x] Contact sheet / grid layout generator
- [x] Pixel-art upscale (nearest-neighbor with integer scales)
- [x] Color quantization and palette generation
- [x] Histogram and image statistics
- [x] Batch rendering pipeline

### v0.5 — Format Support & Serialization

- [x] JPEG decode/encode
- [x] WebP decode/encode (via `webp` crate)
- [x] GIF decode (animated frames to layer sequence)
- [x] Document serialization (JSON + binary blobs)
- [x] PSD read support (layer tree import)

### v1.0 — Stable Release

- [x] API stability guarantee
- [x] Comprehensive documentation and examples
- [ ] Performance benchmarks published
- [ ] WASM-SIMD optimized builds
- [ ] Fuzz-tested codec paths
- [ ] Security audit of all decode paths

### Future Considerations (post v1.0)

- WebGPU compute shader acceleration for blend modes and filters
- Tiled rendering for large images (>4096px)
- Streaming/lazy decode for large files
- Animation timeline (frame sequences)
- Text rendering (via rasterized glyph atlases)
- SVG rasterization (via `resvg`)
- Plugin system for custom filters (user-provided WASM modules)

---

## 10. Testing Strategy

### Unit Tests (Rust)

- Every blend mode tested against reference Photoshop output (golden image comparison)
- Filter accuracy tests: apply known HSL/brightness/contrast values to a solid color, verify exact output
- Blit transform tests: all 8 combinations of flipX × flipY × rotation at each 90° increment
- Edge cases: zero-area layers, fully transparent layers, out-of-bounds blit positions
- Codec round-trip: encode → decode → pixel-exact match

### Integration Tests (JavaScript)

- Node.js: create document, add layers, render, verify PNG output against golden images
- Browser: same tests via Playwright in headless Chrome
- Performance regression tests: render benchmarks must stay within 10% of baseline

### Fuzz Testing

- PNG decoder: fuzz with `cargo-fuzz` to find panics/OOB
- JPEG decoder: same
- All public Rust APIs: property-based testing with `proptest`

---

## 11. Documentation Plan

- **README.md** — Quick start, installation, basic examples
- **API Reference** — Generated from TypeScript types + JSDoc (typedoc)
- **Rust Docs** — Generated with `cargo doc`, published to docs.rs
- **Cookbook** — Step-by-step recipes:
  - "Composite a character from parts" (Spriteform use case)
  - "Apply Instagram-style filters"
  - "Generate a sprite sheet from frames"
  - "Batch-render 1000 avatar variants"
  - "Build a browser-based pixel art editor"
- **Migration Guide** — For Spriteform: how to replace the JS compositor with kimg calls
- **Architecture Guide** — WASM memory model, performance tips, gotchas

---

## 12. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| WASM binary size (gzipped) | <500KB | CI measurement |
| Render speed vs JS baseline | >5x faster | Benchmark suite |
| npm weekly downloads (6mo) | >1,000 | npm stats |
| GitHub stars (6mo) | >500 | GitHub |
| Spriteform integration | Full replacement of JS compositor | Feature parity test suite |
| Browser compatibility | 99%+ global traffic | caniuse WebAssembly |
| API breaking changes after v1.0 | 0 | Semver adherence |

---

## 13. Open Questions

1. **WASM-SIMD feature detection:** Should we ship two WASM binaries (simd + scalar) and auto-detect at init, or target scalar-only for v1?
2. **Thread support:** Should we use `wasm-bindgen-rayon` for multi-threaded rendering in browsers (SharedArrayBuffer), or keep it single-threaded for simplicity? Spriteform currently uses a worker pool — do we want that built into kimg or left to the consumer?
3. **Streaming API:** For very large images (>4096px), should v1 support tiled/streaming rendering, or is it acceptable to require the full image in memory?
4. **WebGPU path:** Should we plan the API to be forward-compatible with a future WebGPU compute backend (e.g., abstract the blend/filter pipeline behind a trait)?

---

## 14. Appendix: Spriteform Compositor Feature Mapping

Exact mapping from Spriteform's current JS implementation to kimg API:

| Spriteform Feature | Spriteform Implementation | kimg Equivalent |
|---|---|---|
| `SpritePartLayer` | JS object with spriteId + transform props | `doc.addImageLayer({ data, x, y, flipX, flipY, rotation, opacity, anchor })` |
| `PaintLayer` | Base64-encoded RGBA buffer | `doc.addPaintLayer({ width, height })` + `layer.getPixels().setData(decoded)` |
| `FilterLayer` (HSVL) | `applyHslFilter()` with hueDeg/sat/light/alpha | `doc.addFilterLayer({ kind: 'hsl' })` + set `.hue/.saturation/.lightness/.alpha` |
| `FilterLayer` (Contrast) | `applyHslFilter()` with brightness/contrast | `doc.addFilterLayer({ kind: 'hsl' })` + set `.brightness/.contrast` |
| `FilterLayer` (FX) | `applyHslFilter()` with temp/tint/sharpen | `doc.addFilterLayer({ kind: 'hsl' })` + set `.temperature/.tint/.sharpen` |
| `FolderLayer` | Recursive renderLayerInto() | `doc.addGroupLayer()` — render traversal handles recursion |
| `SmartLayer` + variants | Stack ordering, variant selection, scoped render | Consumer manages variant selection; kimg renders the active layer tree. Scoped filter isolation via GroupLayer. |
| `blitTransformed()` | Per-pixel loop with rotation LUT, flip, anchor, opacity | `kimg-core::blit::blit_transformed()` — same algorithm in Rust |
| `applyHslFilter()` | RGB↔HSL conversion, per-pixel adjustment, sharpen kernel | `kimg-core::filter::apply_hsl_filter()` — same algorithm in Rust |
| `blendPng()` | Porter-Duff source-over per-pixel | `kimg-core::blend::blend_normal()` |
| `renderCompositePngFromPackFile()` | Back-to-front stack traversal, variant resolution, caching | Consumer calls `doc.render()` after setting up layers. Caching is consumer responsibility. |
| `renderSmartVariantScoped()` | Isolated buffer for variant, filters collected and applied after compositing | GroupLayer rendering + FilterLayer scoping within the group |
| `pngToDataUrl()` | PNG encode → base64 data URL | `doc.exportPng()` → consumer wraps in data URL if needed |
| `turboVariantKey()` / `fnv1a32()` | FNV-1a hash for cache keys | Consumer-side utility, not in kimg core. Can be re-exported as `kaf.util.fnv1a32()`. |
| LRU caching (3 caches) | Map-based with eviction | Consumer responsibility — kimg is stateless between renders |
| Worker pool (`compositorRenderPool`) | Node.js Worker threads | Consumer can run kimg in Web Workers or worker_threads. kimg is single-threaded per call. |
| `dominantRgbFromRgba()` | Grid-sampled quantized histogram | `kaf.color.dominantColor()` |
| `relativeLuminance()` / `contrastRatio()` | WCAG formulas | `kaf.color.relativeLuminance()` / `kaf.color.contrastRatio()` |

---

*kimg — because every pixel matters.*
