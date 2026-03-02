# kimg Plan

This is the only planning document. It tracks the delta from the current repo state
to the public API we actually want to ship and support.

## Current Baseline

Already in repo:

- Rust core compositor with layers, blend modes, masks, filters, transforms, sprite tools, codecs, and document serialization
- `wasm-bindgen` surface exposed through `kimg-wasm`
- Tracked JS package sources in `js/` and generated build output in `dist/`
- Dual wasm builds: baseline + `simd128`, with runtime SIMD detection in JS
- Fuzz targets for codec/deserialization paths
- `cargo audit` / `cargo deny` security policy
- Benchmark coverage and published local baseline numbers in the README

Current weakness:

- The shipped JS API is still mostly a direct wasm-bound surface
- Method naming is Rust-style `snake_case`
- Layer manipulation is functional but not ergonomic from JS

## Product Direction

Target a thin, stable, JavaScript-first API over the current engine.

Design rules:

- No singleton `initKimg()` object
- No deep layer-handle class hierarchy in v1
- Keep numeric layer ids in v1 for simplicity and low wrapper overhead
- Use `camelCase` and object parameters at the package boundary
- Keep the raw wasm-bound API available under a separate subpath for power users/tests
- Treat copied `Uint8Array` results as the stable contract; do not promise zero-copy views

## Target Public API

Main package entrypoint:

```js
import { Composition, preload, simdSupported } from "@iamkaf/kimg";

await preload(); // optional eager warm-up
const comp = await Composition.create({ width: 128, height: 128 });
```

Composition creation and common operations:

```js
const comp = await Composition.create({ width: 256, height: 256 });

const groupId = comp.addGroupLayer({ name: "Character" });

const layerId = comp.addImageLayer({
  name: "Hair",
  rgba,
  width: 64,
  height: 64,
  x: 0,
  y: 0,
  parentId: groupId,
});

comp.updateLayer(layerId, {
  opacity: 0.8,
  visible: true,
  blendMode: "multiply",
  anchor: "center",
  flipX: false,
  flipY: false,
  rotation: 90,
  clipToBelow: false,
});

comp.setLayerMask(layerId, {
  rgba: mask,
  width: 64,
  height: 64,
  inverted: false,
});

const rendered = comp.renderRgba();
const png = comp.exportPng();
```

Format and utility surface:

```js
import {
  decodeImage,
  detectFormat,
  hexToRgb,
  rgbToHex,
  relativeLuminance,
  contrastRatio,
  dominantRgbFromRgba,
} from "@iamkaf/kimg";

const format = await detectFormat(bytes);
```

Raw compatibility surface:

```js
import init, { Composition as RawComposition } from "@iamkaf/kimg/raw";

await init();
const comp = new RawComposition(128, 128);
```

## Stable v1 Scope

Must be true before calling the API stable:

- `kimg` main entrypoint hides wasm filenames and internal glue naming
- `kimg/raw` exposes the existing low-level wasm-bound surface
- `@iamkaf/kimg` is the published package name
- Main entrypoint uses `camelCase` names and object arguments
- `Composition` supports creation, render/export, import/decode, and common layer creation
- Common layer updates go through one stable patch method instead of many narrow setters in JS
- Layer tree queries exist in JS-friendly form: `getLayer`, `listLayers`
- Basic structural mutation exists: `removeLayer`, `moveLayer`
- Package can be published under a real npm name without conflicting with an unrelated package

## Delta Plan

### 1. Package Surface

- [x] Rename the main package entrypoint to `js/index.js` / `dist/index.js`
- [x] Replace main-package explicit init with lazy async entrypoints (`Composition.create`, async utilities, optional `preload`)
- [x] Move the current direct wasm wrapper behind `kimg/raw`
- [x] Stop exposing `kimg_wasm` filenames as the primary user-facing API
- [x] Decide the final npm package name and publication path (`@iamkaf/kimg`)

### 2. JS Facade

- [x] Add a stable JS `Composition` wrapper with object-based constructor and methods
- [x] Convert public API names to `camelCase`
- [x] Add wrapper methods for:
  - `addImageLayer`
  - `addPaintLayer`
  - `addFilterLayer`
  - `addGroupLayer`
  - `addSolidColorLayer`
  - `addGradientLayer`
  - `renderRgba`
  - `exportPng`
  - `exportJpeg`
  - `exportWebp`
- [x] Add JS utility wrappers for current free functions:
  - `decodeImage`
  - `detectFormat`
  - `hexToRgb`
  - `rgbToHex`
  - `relativeLuminance`
  - `contrastRatio`
  - `dominantRgbFromRgba`
  - `extractPaletteFromRgba`
  - `histogramRgba`
  - `quantizeRgba`

### 3. Core/Binding Delta

- [ ] Add `removeLayer(id)` for both top-level and nested layers
- [ ] Add `moveLayer(id, target)` for reorder/reparent operations
- [ ] Add `getLayer(id)` metadata snapshot
- [ ] Add `listLayers({ parentId?, recursive? })`
- [ ] Add `resizeCanvas(width, height)` on `Composition`
- [ ] Add a single layer patch/update path that the JS wrapper can target cleanly
- [ ] Keep current raw methods for backwards compatibility until the facade is complete

### 4. Requested Feature Tracks

These are requested product features. They are tracked here, not scheduled for
immediate implementation unless reprioritized.

#### 4.1 Shape Layers

- [ ] Add Photoshop-style shape layers as first-class document layers
- [ ] Start with rasterized shape primitives:
  - rectangle
  - rounded rectangle
  - ellipse
  - line
  - polygon
- [ ] Support fill and optional stroke in v1 of the feature
- [ ] Ensure shape layers participate in the normal layer stack: opacity, blend mode, masks, clipping, groups, filters
- [ ] JS-facing API target:

```js
const id = comp.addShapeLayer({
  name: "Badge",
  type: "roundedRect",
  x: 24,
  y: 24,
  width: 96,
  height: 40,
  radius: 12,
  fill: [255, 0, 0, 255],
  stroke: { color: [255, 255, 255, 255], width: 2 },
  parentId,
});
```

#### 4.2 Per-Layer Transform Model

Current status:

- Position, flip, anchor, and snapped rotation already exist for image layers
- Destructive resize/crop/rotate layer helpers already exist
- What is missing is one stable, non-destructive transform model at the JS API boundary

- [ ] Expose per-layer translate / scale / rotate / flip in the stable JS API
- [ ] Support at least image, paint, and shape layers
- [ ] Decide whether group transforms are in scope for the first pass
- [ ] Prefer a patch-style API over many narrow setter methods
- [ ] JS-facing API target:

```js
comp.updateLayer(id, {
  x: 10,
  y: -4,
  scaleX: 1.25,
  scaleY: 0.75,
  rotation: 30,
  flipX: false,
  flipY: true,
  anchor: "center",
});
```

#### 4.3 Bucket Fill

- [ ] Add bucket fill for paint/image-style pixel layers
- [ ] Support `contiguous: true | false`
- [ ] Add tolerance control so fill can be strict or color-range based
- [ ] Decide alpha-aware matching behavior before implementation
- [ ] JS-facing API target:

```js
comp.bucketFillLayer(layerId, {
  x: 12,
  y: 18,
  color: [0, 255, 0, 255],
  contiguous: true,
  tolerance: 0,
});
```

### 5. Packaging and Release

- [ ] Verify `dist/` can be packed/published directly
- [ ] Add a local pack/install smoke test for Node.js and browser consumption
- [ ] Update README/examples to use the stable entrypoint, not implementation filenames
- [ ] Add CI coverage for:
  - `cargo test --workspace`
  - `cargo check --target wasm32-unknown-unknown -p kimg-wasm`
  - `./scripts/build.sh`
  - `cargo audit`
  - `cargo deny check`

## Roadmap

Tracked for later, not part of the current delivery target:

- [ ] Selection system
- [ ] Text
- [ ] Brush / brush engine

## Explicit Non-Goals For This Plan

Not part of the current target unless reprioritized:

- Reintroducing the old singleton `initKimg()` design
- Layer wrapper classes per layer kind
- Zero-copy RGBA view APIs across the wasm boundary
- Radial gradients
- Additional blend modes beyond the current 16-mode set
- Worker-pool or threaded rendering API
