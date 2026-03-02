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

- [x] Add `removeLayer(id)` for both top-level and nested layers
- [x] Add `moveLayer(id, target)` for reorder/reparent operations
- [x] Add `getLayer(id)` metadata snapshot
- [x] Add `listLayers({ parentId?, recursive? })`
- [x] Add `resizeCanvas(width, height)` on `Composition`
- [x] Add a single layer patch/update path that the JS wrapper can target cleanly
- [x] Keep current raw methods for backwards compatibility until the facade is complete

### 4. Requested Feature Tracks

These are requested product features. They are tracked here, not scheduled for
immediate implementation unless reprioritized.

#### 4.1 Shape Layers

- [x] Add Photoshop-style shape layers as first-class document layers
- [x] Start with rasterized shape primitives:
  - rectangle
  - rounded rectangle
  - ellipse
  - line
  - polygon
- [x] Support fill and optional stroke in v1 of the feature
- [x] Ensure shape layers participate in the normal layer stack: opacity, blend mode, masks, clipping, groups, filters
- [x] JS-facing API target:

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

- Stable JS patch-based transforms now exist for image, paint, and shape layers
- Destructive resize/crop/rotate layer helpers still exist alongside the non-destructive model

- [x] Expose per-layer translate / scale / rotate / flip in the stable JS API
- [x] Support at least image, paint, and shape layers
- [x] Decide whether group transforms are in scope for the first pass
- [x] Prefer a patch-style API over many narrow setter methods
- First pass excludes group transforms. Group-level transforms can be layered on later if needed.
- [x] JS-facing API target:

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

- [x] Add bucket fill for paint/image-style pixel layers
- [x] Support `contiguous: true | false`
- [x] Add tolerance control so fill can be strict or color-range based
- [x] Decide alpha-aware matching behavior before implementation
- Matching is alpha-aware: tolerance is applied per channel across RGBA, including alpha.
- API coordinates are layer-local pixel coordinates, not canvas/render coordinates.
- [x] JS-facing API target:

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

### 6. Architecture and Dependency Evaluation

These are targeted spikes to improve runtime performance, reduce custom code, and
trim long-term maintenance cost. Each spike must be evaluated against both native
and wasm builds before adoption.

#### 6.1 Serialization Spike: `postcard`

- [x] Prototype replacing the handwritten metadata parser in `kimg-core/src/serialize.rs` with a typed metadata struct encoded via `postcard`
- [x] Preserve the current overall file shape: structured metadata + raw pixel payload
- [x] Decide and document the backward-compatibility story:
  - migrate in place with versioning
  - or keep legacy decode support for old `.kimg` payloads
- Implemented with a versioned binary metadata header for new documents and legacy JSON decode fallback for older `.kimg` payloads.
- [x] Benchmark against the current `serialize_deserialize/10_layers` baseline
- [x] Validate native and wasm builds
- Success criteria:
  - less custom parsing code
  - equal or better deserialize performance
  - no regression in wasm package size or startup behavior
- Wasm status: supported and expected to be a good fit

#### 6.2 Shape Backend Spike: `tiny-skia`

- [x] Prototype shape rendering with `tiny-skia` behind a temporary feature flag or isolated branch
- Prototype is available behind the `tiny-skia-shapes` feature in both `kimg-core` and `kimg-wasm`.
- [x] Compare against the current custom rasterizer in `kimg-core/src/shape.rs`
- [x] Confirm coverage for current primitives:
  - rectangle
  - rounded rectangle
  - ellipse
  - line
  - polygon
- [x] Confirm fill, stroke, clipping, and transform behavior match current layer semantics closely enough
- Verified with core shape/document tests and wasm shape serialization/render tests under the feature flag.
- [x] Re-run shape render benchmarks:
  - `render/single_shape/512`
  - `render/10_shapes/512`
  - `render/10_shapes_with_filter/512`
- Current comparison vs. manual backend:
  - `render/single_shape/512`: `6.08 ms` vs `6.40 ms`
  - `render/10_shapes/512`: `52.35 ms` vs `55.66 ms`
  - `render/10_shapes_with_filter/512`: `64.53 ms` vs `67.91 ms`
- [x] Validate native and wasm builds
- Success criteria:
  - less shape/rasterization code to maintain
  - comparable or better render performance
  - clearer path to future shape features
- Wasm status: likely supported; verify in practice before adopting

#### 6.3 Quantization Spike: `imagequant`

- [ ] A/B benchmark `imagequant` against the current sprite/palette path in `kimg-core/src/sprite.rs`
- [ ] Compare output quality, runtime, and memory behavior on representative inputs:
  - flat UI art
  - textured images
  - pixel art / sprite sheets
- [ ] Re-run palette/quantization benchmarks:
  - `extract_palette/512/16colors`
  - `quantize/512/16colors`
- [ ] Validate native and wasm builds
- Success criteria:
  - clearly better palette/output quality, or meaningfully simpler code at similar quality
  - acceptable performance and memory cost
- Wasm status: supported, but disable threaded defaults for wasm targets

#### 6.4 Codec Spike: `zune-jpeg` and `zune-png`

- [ ] Prototype `zune-jpeg` as an alternative to `jpeg-decoder`
- [ ] Prototype `zune-png` as an alternative to `png`
- [ ] Re-run codec benchmarks with the current textured-image harness
- [ ] Compare native decode speed, wasm behavior, code complexity, and output compatibility
- [ ] Keep current encode paths unless a replacement clearly improves the package
- Success criteria:
  - materially better decode performance, or simpler maintenance at comparable performance
  - no packaging or wasm integration regressions
- Wasm status: supported; SIMD acceleration should gracefully fall back to portable paths on wasm

#### 6.5 Buffer Ergonomics Pass: `bytemuck` and optional `rgb`

- [ ] Audit RGBA byte/pixel conversion code in buffer, codec, and wasm glue layers
- [ ] Use `bytemuck` where it reduces manual casting or indexing noise without obscuring layout assumptions
- [ ] Evaluate `rgb` only if it makes pixel manipulation clearer without spreading new wrapper types everywhere
- [ ] Keep this pass incremental; do not block higher-value spikes on it
- Success criteria:
  - leaner byte/pixel glue code
  - no behavior changes
  - no measurable performance regressions
- Wasm status: supported

#### 6.6 Deferred / Only If Reprioritized

- [ ] PSD parser replacement spike with `rawpsd` only if PSD import becomes important again
- [ ] Text engine evaluation with `cosmic-text` only when the text roadmap item becomes active
- Notes:
  - `rawpsd` looks plausible for wasm, but should be treated as unverified until tested
  - `cosmic-text` is not a near-term wasm choice for this project; treat it as future work with extra integration risk

#### 6.7 Recommended Order

- [ ] 1. `postcard`
- [ ] 2. `tiny-skia`
- [ ] 3. `imagequant`
- [ ] 4. `zune-jpeg` / `zune-png`
- [ ] 5. `bytemuck`

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
