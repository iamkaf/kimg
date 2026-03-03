# Text Backend Plan

Branch: `text-cosmic-plan`

## Goal

Replace the current bitmap text scaffolding with real text rendering based on `cosmic-text`, while keeping the public `Text` layer shape easy to use from `@iamkaf/kimg`.

The browser path must include an easy Google Fonts integration story.

## Research Summary

### 1. `cosmic-text` is the right backend to target first

Why:

- It already covers shaping, layout, fallback, bidi, and rasterization in one stack.
- Its main model is close to what we need: `FontSystem`, `Buffer`, `Metrics`, `Attrs`, and `SwashCache`.
- It exposes a `wasm-web` feature, so web support is an intended target, not an afterthought.

What that means for kimg:

- We should not keep building our own text layout pipeline past the current placeholder phase.
- We should keep the current high-level `Text` layer API shape, but swap the rasterizer/backend underneath it.

### 2. `fontdb` is useful, but it changes the browser font-loading story

Important detail:

- `fontdb` explicitly supports loading fonts from files, directories, and raw `Vec<u8>` data.
- `fontdb` also explicitly calls out support other than TrueType as a non-goal.

Implication:

- The engine should expect to receive raw font bytes that it can register into the text system.
- Browser-native CSS font loading by itself is not enough for kimg, because `cosmic-text` still needs font bytes on the Rust side.

### 3. Google Fonts has two relevant APIs, and they are not equivalent

#### CSS2 API

Pros:

- No API key.
- Official, stable, and designed for browser use.
- Supports variable font axis requests.
- Supports `text=` optimization.

Cons:

- It returns CSS, not clean font metadata.
- In browser-oriented usage it commonly resolves to `woff2` font files.
- `woff2` is not a guaranteed direct fit for the `fontdb` / `cosmic-text` ingestion path.

#### Developer API

Pros:

- Returns machine-readable family metadata.
- Documents variable font axis metadata.
- Can return non-`woff2` font file URLs.

Cons:

- Requires an API key.
- That is a bad default UX for "easy Google Fonts integration."

### 4. Browser `FontFace` is helpful, but not sufficient

The CSS Font Loading API can load a font from a URL or `ArrayBuffer` and register it with `document.fonts`.

That is useful for:

- browser preview parity
- ensuring the page itself can use the same font

But it does not solve kimg's core problem by itself:

- kimg still needs raw font bytes inside wasm for `cosmic-text`.

## Decision Summary

### Chosen backend

- Use `cosmic-text` for the real text backend.

### Public API direction

- Keep `addTextLayer(...)` and `updateLayer(..., { textConfig })`.
- Expand text style fields rather than replacing the API entirely.

### Google Fonts direction

Target UX:

- a browser helper with no API key in the common case

Recommended approach:

1. Primary browser path: use Google Fonts CSS2 requests as the public-facing API.
2. Internally resolve the returned font files into bytes usable by wasm.
3. If CSS2 yields `woff2`, decode it before registration.

This is the key engineering decision:

- if we want truly easy Google Fonts integration, we likely need our own CSS parsing + font fetch + `woff2` decode path
- not just a thin wrapper over the Google Fonts Developer API

The Developer API can still be an optional advanced path later, but it should not be the primary browser UX.

## Target Public API

### Text layers

```ts
const id = composition.addTextLayer({
  name: "Title",
  text: "Hello world",
  fontFamily: "Inter",
  fontSize: 48,
  fontWeight: 700,
  fontStyle: "normal",
  lineHeight: 56,
  letterSpacing: 0,
  align: "left",
  color: [24, 77, 163, 255],
  width: 320,
  wrap: "word",
  x: 24,
  y: 24,
});
```

```ts
composition.updateLayer(id, {
  anchor: "center",
  rotation: -12,
  textConfig: {
    text: "Updated title",
    fontFamily: "Inter",
    fontWeight: 800,
    align: "center",
  },
});
```

### Font registration

Low-level:

```ts
await registerFont({
  family: "Inter",
  bytes,
  weight: 100,
  style: "normal",
});
```

Browser Google Fonts helper:

```ts
await loadGoogleFont({
  family: "Inter",
  weights: [400, 700],
  ital: [0, 1],
  text: "Hello world",
});
```

Optional later:

```ts
await preloadGoogleFont({
  family: "Inter",
  weights: [100, 900],
  variable: true,
});
```

## Architecture Plan

### 1. Core text model

Expand `TextLayerData` to hold real style/layout fields:

- `text`
- `font_family`
- `font_weight`
- `font_style`
- `font_size`
- `line_height`
- `letter_spacing`
- `align`
- `wrap`
- optional `box_width`
- `color`
- transform fields

The layer should keep the same raster cache and transformed cache pattern already used by shape/image/paint.

### 2. Runtime font registry

Add a font registry layer above the document model.

Recommendation:

- do not embed raw font bytes directly into every text layer
- keep font bytes in a runtime registry keyed by family/style/weight

Likely structure:

- `kimg-core`: text renderer object that owns `FontSystem` and `SwashCache`
- `kimg-wasm`: module-global or composition-shared font registry
- JS facade: registration helpers that forward bytes into wasm

### 3. Serialization

First pass:

- serialize text layer style and family reference
- do not serialize raw font bytes yet

Tradeoff:

- deserialized documents require the same fonts to be registered again before rendering

That is acceptable for the first real backend pass.

Optional later:

- add embedded-font packaging to `.kimg`

### 4. Browser Google Fonts loader

Implementation target:

- JS-side helper in the main package
- fetch CSS2 stylesheet URL
- parse `@font-face` rules
- fetch font binaries
- convert `woff2` to OpenType if needed
- register resulting bytes into wasm

This should be browser-only sugar over the low-level `registerFont()` path.

### 5. Demo and docs

The demo must gain text cards that prove:

- font family changes
- weight/style changes
- wrapping
- alignment
- multiline layout
- Google Fonts browser loading

## Open Technical Questions

### Q1. Can we keep keyless Google Fonts integration?

Probably yes, but only if the CSS2 + fetched font file path can be converted into bytes usable by `cosmic-text`.

Action:

- spike CSS2 parsing + font fetch
- verify the actual returned file types
- verify the chosen decode path on `wasm32-unknown-unknown`

### Q2. Do we need a `woff2` decode step?

Probably yes for the browser helper.

This is an inference from the current Google Fonts browser-facing flow plus `fontdb`'s stated TrueType focus.

Action:

- spike a `woff2` decode path on native and wasm
- only proceed with the CSS2 helper once that works

### Q3. Where should the font registry live?

Recommendation:

- not inside serialized document state
- runtime-side, shared per wasm module or per composition host

Action:

- prototype a single module-global registry first
- only complicate it if composition isolation becomes necessary

## Phases

### Phase 1. Research spikes

- [x] Verify `cosmic-text` integration in `kimg-core` with one hardcoded TTF font
- [x] Verify `cosmic-text` on `wasm32-unknown-unknown`
- [x] Verify browser-side font registration from raw bytes
- [x] Verify Google Fonts CSS2 fetch and stylesheet parsing
- [x] Verify `woff2` decode path on native
- [x] Verify `woff2` decode path on wasm

Exit criteria:

- one rendered text sample from real `cosmic-text` on native and wasm
- a clear yes/no answer on keyless Google Fonts support

Phase 1 findings:

- Native probe: `cosmic-text` rendered an Inter sample from raw TTF bytes with `loaded_faces=1`, `rects=9169`, `bbox=226x94`.
- Browser wasm probe: the same sample rendered successfully from:
  - embedded TTF bytes in wasm
  - raw TTF bytes passed in from JS
  - decoded Google Fonts CSS2 `woff2` bytes
- All three wasm probe paths produced the same output metrics: `loaded_faces=1`, `rects=9169`, `bbox=226x94`.
- Google Fonts CSS2 is viable for a keyless browser helper, but real browser-style requests return `woff2`, not TTF. The helper will need stylesheet parsing plus `woff2` decode.
- The `woff2` crate is not viable on this Rust 1.91 toolchain here. It failed to compile in the spike. `wuff` compiled on native and `wasm32-unknown-unknown`, and successfully decoded the CSS2 font payload.
- Main pain revealed by Phase 1: code size. The minimal wasm-bindgen text probe produced a `3,085,236` byte wasm file before `wasm-opt` or gzip, versus the current baseline `kimg_wasm_bg.wasm` at `1,079,184` bytes.

Decision after Phase 1:

- Proceed with `cosmic-text`.
- Use a low-level raw-byte `registerFont()` API.
- Keep the browser Google Fonts plan keyless via CSS2.
- Plan around `wuff` as the current `woff2` decode path unless a better option appears later.
- Ship the browser text backend as a lazy-loaded path.
- Keep the Node path eager: text support is included in the normal package flow there.

Shipping decision:

- Browser:
  - text backend and Google Fonts helper load on first text use
  - documents without text should not pay the text wasm/code-size cost up front
- Node:
  - include the text backend in the main package path
  - prioritize API simplicity over bundle size

Implication for Phase 2 and Phase 3:

- core text rendering should be integrated normally
- wasm/JS packaging should preserve a browser-specific split point for text runtime loading

### Phase 2. Core backend replacement

- [ ] Replace bitmap rasterizer with `cosmic-text`
- [x] Expand `TextLayerData` style model
- [x] Keep render caching
- [x] Update serialization for the new text metadata
- [x] Add core render and round-trip tests

Current state:

- Initial slice landed on this branch: `kimg-core` now has a feature-gated `cosmic-text` backend seam with bitmap fallback, and `kimg-wasm` forwards that feature for native and `wasm32` compilation.
- The text model now includes family, weight, style, align, wrap, and optional box width.
- Runtime font registration plumbing exists in `kimg-core`, and `kimg-wasm` exposes low-level font registration calls.
- The remaining blocker for closing Phase 2 is making the real backend the primary path instead of a feature-gated seam with bitmap fallback.

Exit criteria:

- text layers render real fonts in native and wasm
- current text API still works with the expanded style model

### Phase 3. WASM and JS font API

- [x] Add raw wasm font registration functions
- [x] Add JS `registerFont()`
- [x] Add JS text-style normalization
- [x] Add facade tests for text layer creation and updates

Exit criteria:

- callers can register arbitrary font bytes and render text

### Phase 4. Browser Google Fonts integration

- [x] Add browser-only `loadGoogleFont()`
- [x] Fetch and parse CSS2 responses
- [x] Fetch font binaries
- [x] Decode `woff2` if required
- [x] Register fonts into wasm
- [x] Cache loaded Google Fonts per session

Exit criteria:

- one-line browser setup for Google Fonts
- no API key required in the common path

### Phase 5. Demo and docs

- [x] Replace bitmap-text demo card with real-font cards
- [ ] Add a Google Fonts demo card
- [x] Update README examples
- [ ] Document limitations clearly

Exit criteria:

- visual suite makes text regressions obvious
- README does not overclaim

## Validation Matrix

Every phase should be validated with:

- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run test:js`
- `npm run test:demo`

Additional text-specific validation:

- Latin text
- multiline wrapping
- centered and rotated text
- mixed weight/style
- at least one non-Latin shaping case
- browser-loaded Google Font

## Sources

- `cosmic-text` docs: https://pop-os.github.io/cosmic-text/cosmic_text/
- `cosmic-text` repo README: https://github.com/pop-os/cosmic-text
- `cosmic-text` features on docs.rs: https://docs.rs/crate/cosmic-text/latest/features
- `fontdb` docs: https://docs.rs/fontdb/
- Google Fonts CSS2 API docs: https://developers.google.com/fonts/docs/css2
- Google Fonts getting started docs: https://developers.google.com/fonts/docs/getting_started
- Google Fonts technical considerations: https://developers.google.com/fonts/docs/technical_considerations
- Google Fonts Developer API docs: https://developers.google.com/fonts/docs/developer_api
- CSS Font Loading API: https://developer.mozilla.org/en-US/docs/Web/API/CSS_Font_Loading_API
- `FontFace` API: https://developer.mozilla.org/docs/Web/API/FontFace
- `woff2` crate docs: https://docs.rs/woff2/latest/woff2/
