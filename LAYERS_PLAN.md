# Layer Model Cleanup Plan

## Goal

Reduce layer-type surface area so the engine model matches actual behavior more closely and is easier to evolve.

The intended cleanup is:

- merge `Image` and `Paint` into one raster layer kind
- merge `SolidColor` and `Gradient` into one fill layer kind
- merge rectangle and rounded rectangle into one rectangle shape with corner radius

## Why

### 1. `Image` and `Paint` are semantically different, but technically the same today

Both are buffer-backed raster layers with the same transform model, nearly identical render paths, and the same destructive-edit behavior.

That means the split is currently more application-facing than engine-facing.

Target:

- one raster layer kind in core
- application code can still decide whether a raster layer came from import, painting, or some other workflow

### 2. `SolidColor` and `Gradient` are both fill layers

Both currently behave as generated full-canvas fills:

- tiny parameter-only state
- no local bounds
- no independent transform model
- same masking / blending / clipping behavior

Target:

- one fill layer kind with fill variants underneath it

### 3. Rectangle and rounded rectangle are the same primitive with one parameter

`roundedRect` is just `rectangle` with `radius > 0`.

Target:

- keep `rectangle`
- remove the dedicated `roundedRect` variant
- keep `radius` on rectangle

## Target Model

### Raster

Replace:

- `Image`
- `Paint`

With:

- `Raster`

Properties:

- `buffer`
- shared transform state
- transformed-raster cache

Public API direction:

- keep ergonomic aliases like `addImageLayer(...)` and `addPaintLayer(...)` if desired
- both create the same underlying `Raster` layer kind
- layer snapshots should report one canonical kind

### Fill

Replace:

- `SolidColor`
- `Gradient`

With:

- `Fill`

Backed by:

- `FillKind::Solid { color }`
- `FillKind::Gradient { stops, direction }`

Public API direction:

- keep ergonomic creation helpers if desired:
  - `addSolidColorLayer(...)`
  - `addGradientLayer(...)`
- both create the same underlying `Fill` layer kind

### Shape rectangle

Replace:

- `ShapeType::Rectangle`
- `ShapeType::RoundedRect`

With:

- `ShapeType::Rectangle`

Behavior:

- `radius = 0` means sharp corners
- `radius > 0` means rounded corners
- clamp radius against width and height

## Scope

### In scope

- core layer model changes
- render-path updates
- serialization compatibility
- wasm snapshot / patch updates
- JS facade updates
- demo updates if any card labels or metadata need to change
- docs and tests

### Out of scope

- changing the high-level product semantics of layers
- adding new layer roles / source metadata
- adding new fill types
- adding new vector/path editing features

## Compatibility Strategy

### 1. Serialized documents

Existing `.kimg` files should continue to load.

Compatibility rules:

- legacy `Image` and `Paint` decode into `Raster`
- legacy `SolidColor` and `Gradient` decode into `Fill`
- legacy `rounded_rect` decodes into `rectangle` with `radius`

Write path:

- new saves should write only the canonical forms

### 2. Raw wasm / JS API

Recommended direction:

- keep current helper names for compatibility where reasonable
- normalize snapshots and list/get responses to the canonical kinds

That means callers may still create a raster layer via an image-oriented or paint-oriented helper, but the returned layer kind should no longer pretend they are fundamentally different engine entities.

## Work Plan

### Phase 1. Raster merge

- [ ] Introduce `RasterLayerData`
- [ ] Replace `LayerKind::Image` and `LayerKind::Paint` with `LayerKind::Raster`
- [ ] Migrate document render, transforms, fill, flatten, and destructive ops
- [ ] Update serialization to decode old forms and write new form
- [ ] Update wasm snapshots / patches
- [ ] Update JS facade types and helpers

Exit criteria:

- no engine-level `Image` / `Paint` split remains
- existing image/paint workflows still behave the same

### Phase 2. Fill merge

- [ ] Introduce `FillLayerData` and `FillKind`
- [ ] Replace `LayerKind::SolidColor` and `LayerKind::Gradient` with `LayerKind::Fill`
- [ ] Preserve current render behavior for solid and gradient fills
- [ ] Update serialization compatibility
- [ ] Update wasm snapshots / patches
- [ ] Update JS facade types and helpers

Exit criteria:

- solid and gradient fills share one engine-level layer kind
- current fill-layer behavior is unchanged

### Phase 3. Shape cleanup

- [ ] Remove `ShapeType::RoundedRect`
- [ ] Keep `ShapeType::Rectangle` with `radius`
- [ ] Update shape renderer and serde compatibility
- [ ] Update demo/docs/tests to use `rectangle + radius`

Exit criteria:

- no dedicated rounded-rectangle variant remains
- existing rounded-rectangle documents still load correctly

### Phase 4. Polish and cleanup

- [ ] Simplify docs to describe fewer layer families
- [ ] Update benchmark/demo labels if needed
- [ ] Remove dead compatibility shims that are no longer used internally

Exit criteria:

- public docs describe the cleaned model clearly
- internal code is actually simpler, not just renamed

## Validation Matrix

Run after each phase:

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run test:js`
- `npm run test:demo`

Additional checks:

- document round-trip across legacy and new layer kinds
- flatten / group / clip / mask behavior on raster and fill layers
- visual demo sanity for shape rectangle radius handling

## Notes

- This plan is an engine cleanup, not a user-facing feature push.
- The intended result is fewer true layer kinds:
  - `Raster`
  - `Filter`
  - `Group`
  - `Fill`
  - `Shape`
  - `Text`
