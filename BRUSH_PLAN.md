# Brush Engine Plan

## Goal

Add a real raster brush engine to `kimg` that feels native to the current document model:

- paints destructively into `Raster` layers
- works in native and `wasm`
- supports pressure-sensitive input
- is fast enough for interactive use
- keeps the public API simple

This is a painting engine, not a vector stroke editor.

## Product Scope

### Must Have

- round brush
- hard/soft edge control
- size
- opacity
- flow
- spacing
- color
- eraser mode
- pressure-driven size and opacity
- simple smoothing / stabilization
- stroke batching into raster layers
- good dirty-rect invalidation

### Nice Soon After

- tilt-driven ellipse / angle
- scatter / jitter
- textured tip
- wet-mix / smudge-like tools
- symmetry
- alpha lock

### Explicitly Out of Scope for V1

- vector/path editing
- layer selections and masked painting
- procedural brush scripting
- full MyPaint/Krita-style parameter explosion
- GPU brush rendering

## Current Fit In kimg

The current architecture is a good fit for a raster brush engine:

- painting should target `Raster` layers only
- raster layers already own mutable `ImageBuffer`s and cache invalidation paths
- the compositor already handles blending, opacity, grouping, masking, clipping, and transforms after the layer has been painted
- bucket fill already proves destructive raster editing is a valid document operation

The missing piece is stroke application, not a new document model.

## Recommendation

Build the core brush engine ourselves in `kimg-core`.

Do not depend on a third-party brush engine for the main implementation.

Use external crates only as focused building blocks where they clearly help.

## Research Summary

### 1. `libmypaint`

Status:
- technically proven brush engine
- not a good direct dependency for `kimg`

Why it is interesting:
- it is literally a production brush engine
- it has a long history and a rich parameter model

Why it is a bad fit:
- C codebase
- large parameter surface
- more complexity than `kimg` needs for a first real brush engine
- awkward story for `wasm`
- integration would fight the current Rust-native codebase

Use it as:
- design reference
- performance/reference behavior benchmark

Do not use it as:
- runtime dependency

Relevant source:
- https://github.com/mypaint/libmypaint

### 2. `perfect_freehand` / `freedraw`

Status:
- useful conceptually
- not the right core for a paint engine

What they do:
- turn sampled input points into smooth stroke outlines
- ideal for “ink stroke as polygon” style rendering

Why they are not enough:
- they generate stroke geometry, not a dab-based painting engine
- they are better for signature/whiteboard/vector-ish strokes than Photoshop-style raster brushes
- they do not give us flow accumulation, soft circular tips, eraser behavior, or textured dabs

Potential use:
- optional future brush-preview path
- optional future vector-ink tool

Not recommended for:
- the core paint brush

Relevant sources:
- https://docs.rs/perfect_freehand/latest/perfect_freehand/
- https://docs.rs/freedraw/latest/freedraw/

### 3. `ink-stroke-modeler-rs`

Status:
- promising helper
- not a full brush engine

What it gives us:
- smoothing / modeling for raw stylus or pointer input
- a cleaner stream before dab placement

What it does not give us:
- brush rendering
- dab masks
- blending
- stroke accumulation

Recommendation:
- do not block V1 on it
- keep it as an optional later experiment if our own lightweight smoothing is not good enough

Relevant source:
- https://docs.rs/ink-stroke-modeler-rs/latest/ink_stroke_modeler_rs/

### 4. `kurbo`

Status:
- strong geometry utility crate
- useful later, not necessary for V1

What it is good for:
- path fitting
- simplification
- curves
- offset/path utilities

When it becomes useful:
- better stroke stabilization
- spline fitting
- shape/brush path workflows
- future path brush or pen tools

Recommendation:
- not needed for the first brush pass
- worth reconsidering when we want curve fitting or path-oriented tools

Relevant source:
- https://docs.rs/kurbo/latest/kurbo/

### 5. `zeno`

Status:
- interesting low-level path rasterizer
- not the right primitive for a first raster brush

What it gives us:
- efficient path masks
- stroke/fill rasterization
- reusable scratch memory

Why it is not first choice:
- our first brush should be dab-based, not path-mask-based
- a dab engine maps better to size/flow/hardness/pressure
- path stroking is more useful for a future ink/pen tool than for the main paint brush

Recommendation:
- keep on the radar for pen/path tools
- skip for V1 brush engine

Relevant source:
- https://docs.rs/zeno/latest/zeno/

### 6. `tiny-skia`

Status:
- already in the repo
- useful, but not as the primary brush engine

What it gives us:
- high quality shape/path rasterization
- anti-aliased fills and strokes

Why not use it for V1 painting:
- a brush engine needs dab spacing, pressure curves, flow accumulation, erasing, and dirty-rect behavior
- using a general vector rasterizer for every dab would be heavier than directly painting alpha masks into the target buffer

Recommendation:
- keep using it for shapes
- do not build the V1 brush engine around repeated `tiny-skia` calls

Relevant source:
- https://docs.rs/tiny-skia/latest/tiny_skia/

## Final Recommendation

### Build Ourselves

Implement a native `kimg-core` raster brush engine with:

- precomputed dab masks
- incremental stroke sampling
- direct RGBA compositing into raster buffers
- optional pressure mapping
- simple smoothing

### Crates To Keep In Mind

- `ink-stroke-modeler-rs`
  - possible future smoothing upgrade
- `kurbo`
  - future curve fitting / pen tool support
- `zeno`
  - future path-based ink/pen tool support

### Crates To Avoid For Core Brush

- `libmypaint`
  - too heavy / awkward for `kimg`
- `perfect_freehand` / `freedraw`
  - wrong primitive for raster painting

## Engine Design

## Brush Model

```rust
pub enum BrushTool {
    Paint,
    Erase,
}

pub struct BrushPreset {
    pub tool: BrushTool,
    pub color: Rgba,
    pub size: f32,
    pub opacity: f32,
    pub flow: f32,
    pub hardness: f32,
    pub spacing: f32,
    pub smoothing: f32,
    pub pressure_size: f32,
    pub pressure_opacity: f32,
}

pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub time_ms: f32,
}
```

Notes:

- `pressure` is normalized `0..=1`
- `tilt_*` may be zero when unsupported
- hosts collect input; `kimg` consumes normalized stroke points

## Public API Shape

The app layer should own pointer events. `kimg` should not try to become an input framework.

### Core API

The core should start with a batched stroke application API:

```rust
pub fn paint_stroke_layer(
    &mut self,
    layer_id: u32,
    brush: &BrushPreset,
    points: &[StrokePoint],
) -> bool
```

This is simpler to test, deterministic, and good enough for the first implementation.

### JS/WASM API

Start with the same batched model:

```ts
composition.paintStrokeLayer(layerId, brush, points)
```

Then add streaming only after the engine is stable:

```ts
const strokeId = composition.beginBrushStroke(layerId, brush)
composition.pushBrushPoints(strokeId, points)
composition.endBrushStroke(strokeId)
composition.cancelBrushStroke(strokeId)
```

Rationale:

- batch API is enough to ship
- streaming is ergonomic for interactive apps
- keeping batch as the primitive makes testing and serialization easier

## Rendering Algorithm

### V1 Approach: Dab Engine

For each stroke:

1. optionally smooth input points
2. resample the stroke into evenly spaced dabs
3. compute per-dab radius and alpha from pressure and brush settings
4. stamp a circular alpha mask into the target raster buffer
5. accumulate only inside the affected dirty rect

This is the right primitive for:

- paint
- eraser
- soft edges
- flow buildup
- future textured tips

### Dab Mask

Use a cached grayscale alpha tip:

- diameter derived from `size`
- falloff derived from `hardness`
- reused across many dabs of the same rounded size/hardness pair

This cache should live per brush renderer / scratch context, not globally unbounded.

## Compositing Rules

### Paint

Apply the dab mask as source alpha with:

- brush color
- per-dab effective alpha from `opacity * flow * pressure mapping`

### Eraser

Do not paint transparent black.
Reduce destination alpha directly based on the dab alpha.

That gives correct eraser behavior for semi-transparent painted content.

## Smoothing

### V1

Implement a lightweight internal smoother:

- simple low-pass filtering
- optional interpolation between sparse points

This is enough to remove jitter without adding a new dependency immediately.

### V2 Option

If needed, evaluate `ink-stroke-modeler-rs` as a better modeled input stage.

## Performance Strategy

### Required

- dirty-rect tracking per stroke
- mutate only the touched raster bounds
- tip cache keyed by rounded `(diameter, hardness)`
- avoid per-dab heap allocation
- reusable scratch buffers

### Later

- tile-based stroke application
- SIMD alpha blend hot path
- textured-tip cache atlas

## Layer Interaction

### V1

- only `Raster` layers are paintable
- painting invalidates raster transform caches
- masks, groups, blending, and filters continue to work because they happen after layer painting

### Not In V1

- paint on `Fill`, `Shape`, `Text`, or `Svg`
- implicit rasterization during painting

If the app wants to paint over those, it should rasterize/flatten first.

## Undo / History Shape

The engine should be designed around stroke-local dirty rects so the app layer can eventually store:

- affected bounds
- before pixels
- after pixels

Even if undo is not implemented in core right away, the stroke API should make that possible.

## Benchmarks To Add

- `brush/round_hard_small/256`
- `brush/round_soft_large/512`
- `brush/erase_soft/512`
- `brush/long_pressure_stroke/1024`
- `brush/repeated_short_strokes/512`

Metrics:

- total stroke time
- dirty rect size
- dabs per second

## Tests To Add

- single hard dab paints expected bounds
- soft dab falloff affects alpha monotonically
- pressure changes size and opacity
- spacing produces expected dab count
- eraser reduces alpha without color contamination
- painting invalidates raster transform cache
- batch stroke on wasm and JS matches core behavior

## Phases

### Phase 1: Core MVP

- add brush preset + stroke point data model
- implement batched dab engine for raster layers
- support paint + eraser
- add tests and benches

Exit criteria:

- brush strokes render correctly
- performance is interactive on moderate canvases

### Phase 2: JS/WASM Surface

- expose `paintStrokeLayer(...)`
- add demo coverage
- add package tests

Exit criteria:

- browser and Node can paint into raster layers

### Phase 3: Interactive / Streaming

- add begin/push/end stroke session API
- add incremental dirty-rect tracking
- reduce wasm boundary overhead for live painting

Exit criteria:

- interactive brush use no longer requires batching whole strokes in JS

### Phase 4: Quality Upgrade

- improve smoothing
- add tilt support
- add textured tips
- evaluate `ink-stroke-modeler-rs` if the internal smoother is not enough

### Phase 5: Advanced Tools

- alpha lock
- scatter/jitter
- symmetry
- smudge/wet tools
- selection-aware painting once selections exist

## Decision

Ship a native, dab-based raster brush engine first.

That is the simplest design that matches `kimg`:

- fast
- wasm-safe
- easy to reason about
- compatible with current raster layers
- extensible toward richer tools later

The correct initial dependency strategy is conservative:

- no external brush engine dependency
- optional later use of helper crates where they clearly improve one stage of the pipeline

## References

- libmypaint: https://github.com/mypaint/libmypaint
- perfect_freehand: https://docs.rs/perfect_freehand/latest/perfect_freehand/
- freedraw: https://docs.rs/freedraw/latest/freedraw/
- ink-stroke-modeler-rs: https://docs.rs/ink-stroke-modeler-rs/latest/ink_stroke_modeler_rs/
- kurbo: https://docs.rs/kurbo/latest/kurbo/
- zeno: https://docs.rs/zeno/latest/zeno/
- tiny-skia: https://docs.rs/tiny-skia/latest/tiny_skia/
