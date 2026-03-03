# SVG Support Plan

## Goal

Add minimum-viable SVG support as a scalable asset layer:

- import SVG into a composition
- keep it scalable in normal use
- allow normal layer transforms like translate / scale / rotate
- allow normal kimg compositing and filters
- do not support path editing
- provide an explicit rasterize step only when the caller opts into it

## Product Shape

This is not an SVG editor.

The target user experience is:

- import an SVG logo / icon / illustration
- place it in the composition
- scale it without losing sharpness
- rotate / move / blend / mask / filter it like other layers
- rasterize it later only if needed

## Research Summary

### Recommended stack

- `usvg` for parsing and normalizing SVG input
- `resvg` for rasterizing the resolved tree

Why:

- pure Rust
- strong support for static SVG rendering
- workable wasm story
- good fit for a retained scalable asset that still renders into kimg's raster compositor

### Important constraints

#### 1. SVG should stay a layer asset, not become editable vector geometry

The MVP should preserve the original SVG source and rerasterize it when needed.

Do not try to translate arbitrary SVG into native shape/text/group layers in the first version.

#### 2. External resources should be out of scope initially

No general remote fetching from inside SVG.

Initial support should reject or ignore:

- remote images
- remote stylesheets
- scripts
- animation

#### 3. SVG text depends on registered fonts

If an imported SVG contains `<text>`, correct rendering depends on the same font-availability story as kimg's own text layers.

That means:

- simple path-based SVGs should work well
- text-heavy SVGs may require explicit font registration for fidelity

#### 4. Size matters

`resvg` / `usvg` are not tiny, especially for wasm.

SVG support should be planned with the same mindset as text:

- browser size cost matters
- a lazy-loaded browser path may be the right shipping model

## Target Model

Add a new engine layer kind:

- `Svg`

Stored data:

- original SVG bytes or UTF-8 string
- parsed / normalized SVG tree
- raster cache keyed by output size
- shared layer transform state

Behavior:

- source remains SVG-backed until explicit rasterization
- render path rasterizes to the target local size, then composes normally
- existing transform / opacity / blend / mask / clip behavior should apply after rasterization

## API Direction

### Core / wasm / JS

Likely public helpers:

```ts
const id = composition.addSvgLayer({
  name: "Logo",
  svg,
  width: 240,
  height: 240,
  x: 32,
  y: 32,
});
```

```ts
composition.updateLayer(id, {
  scaleX: 1.5,
  scaleY: 1.5,
  rotation: -8,
});
```

```ts
composition.rasterizeSvgLayer(id);
```

Notes:

- the layer should use the same non-destructive transform model as raster/shape/text
- filters should apply to the rendered result, not to live SVG internals

### Import helpers

Recommended helpers:

- `addSvgLayer(...)`
- `decodeSvg(...)` or `importSvg(...)` only if we want a standalone decode helper

I would avoid overdesigning the import surface initially. A single composition-layer helper is enough for MVP.

## Scope

### In scope

- static SVG import
- scalable retained SVG layer
- normal layer transforms
- normal compositing / filters after rasterization
- explicit rasterize-to-raster-layer conversion
- browser and wasm support if size is acceptable

### Out of scope

- path editing
- boolean operations on SVG paths
- animated SVG
- scripting
- remote resource loading
- full SVG authoring surface

## Work Plan

### Phase 1. Integration spike

- [x] Add a small `usvg` + `resvg` prototype in core
- [x] Verify native render from raw SVG bytes
- [x] Verify `wasm32-unknown-unknown` build
- [x] Measure wasm size impact
- [x] Render one representative icon/logo SVG in tests

Exit criteria:

- clear yes/no answer on native viability, wasm viability, and size cost

Result:

- native and `wasm32` viability: yes
- current eager shipping cost: `dist/kimg_wasm_bg.wasm` is about `2.9 MB` raw, `dist/kimg_wasm_text_bg.wasm` about `4.9 MB` raw
- decision: ship SVG support eagerly for now and revisit lazy-loading only if package size becomes a practical problem

### Phase 2. Engine layer

- [x] Add `SvgLayerData`
- [x] Store original source plus parsed tree / render-ready state
- [x] Add raster cache keyed by output size
- [x] Render SVG layers through the normal document pipeline
- [x] Reuse existing transform model

Exit criteria:

- SVG layers render in compositions and scale cleanly

### Phase 3. Serialization and API

- [x] Add `.kimg` serialization for SVG layers
- [x] Add wasm bindings
- [x] Add JS facade helper(s)
- [x] Add layer snapshot support

Exit criteria:

- SVG layers round-trip through save/load and are usable from JS

### Phase 4. Rasterization workflow

- [x] Add `rasterizeSvgLayer(id)` document operation
- [x] Preserve current common properties during conversion
- [x] Ensure resulting raster layer matches current visual output closely

Exit criteria:

- caller can opt into destructive rasterization explicitly

### Phase 5. Constraints and polish

- [x] Reject unsupported external-resource cases cleanly
- [x] Document SVG text/font caveats
- [x] Decide browser shipping model:
  - include in main wasm
  - or lazy-load in browser like text
- [x] Add demo coverage

Exit criteria:

- SVG support is clear, predictable, and honestly documented

## Shipping Recommendation

Current recommendation:

- Node: include SVG support eagerly
- Browser: include SVG support eagerly for now

If wasm size becomes a release problem later, the most likely follow-up is a browser-only lazy-loaded SVG build rather than changing the API surface.

## Validation Matrix

Run during implementation:

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run test:js`
- `npm run test:demo`

SVG-specific validation:

- simple logo/icon SVG
- gradients and masks inside SVG
- transformed SVG layer in composition
- save/load round-trip
- explicit rasterize conversion
- browser wasm build and smoke render

## Risks

- wasm size increase
- font fidelity for SVG text
- partial support expectations if users assume all SVG features work

## Notes

- The correct MVP is "scalable asset layer," not "editable vector layer."
- If Phase 1 size is too costly, SVG should follow the same browser lazy-load pattern as text.
