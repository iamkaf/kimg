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

This cleanup shipped as a pre-release canonical rewrite.

- no old `.kimg` migration layer was kept
- helper creation methods such as `addImageLayer()`, `addPaintLayer()`, `addSolidColorLayer()`, and `addGradientLayer()` remain for ergonomics
- layer snapshots and serialization now use the canonical kinds only

## Work Plan

### Phase 1. Raster merge

- [x] Introduce `RasterLayerData`
- [x] Replace `LayerKind::Image` and `LayerKind::Paint` with `LayerKind::Raster`
- [x] Migrate document render, transforms, fill, flatten, and destructive ops
- [x] Rewrite serialization to only encode/decode the canonical form
- [x] Update wasm snapshots / patches
- [x] Update JS facade types and helpers

Exit criteria:

- no engine-level `Image` / `Paint` split remains
- existing image/paint workflows still behave the same

### Phase 2. Fill merge

- [x] Introduce `FillLayerData` and `FillKind`
- [x] Replace `LayerKind::SolidColor` and `LayerKind::Gradient` with `LayerKind::Fill`
- [x] Preserve current render behavior for solid and gradient fills
- [x] Rewrite serialization to only encode/decode the canonical form
- [x] Update wasm snapshots / patches
- [x] Update JS facade types and helpers

Exit criteria:

- solid and gradient fills share one engine-level layer kind
- current fill-layer behavior is unchanged

### Phase 3. Shape cleanup

- [x] Remove `ShapeType::RoundedRect`
- [x] Keep `ShapeType::Rectangle` with `radius`
- [x] Update shape renderer and serialization
- [x] Update demo/docs/tests to use `rectangle + radius`

Exit criteria:

- no dedicated rounded-rectangle variant remains
- existing rounded-rectangle documents still load correctly

### Phase 4. Polish and cleanup

- [x] Simplify docs to describe fewer layer families
- [x] Update benchmark/demo labels if needed
- [x] Remove dead compatibility shims that are no longer used internally

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
