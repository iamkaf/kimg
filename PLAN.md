# kimg — Implementation Plan

## Phase 0: Foundation (done)

Bootstrap the project with end-to-end WASM compilation.

- [x] PRD written
- [x] Rust workspace initialized (`kimg-core` + `kimg-wasm`)
- [x] Core pixel types: `Rgba`, `ImageBuffer`
- [x] Alpha blending (Porter-Duff source-over)
- [x] Transformed blit (position, anchor, flip, rotation, opacity)
- [x] HSL filter pipeline (hue/sat/light/alpha/brightness/contrast/temp/tint/sharpen)
- [x] RGB↔HSL color conversion
- [x] Layer types: Image, Paint, Filter, Group
- [x] Document with canvas + layer tree + `render()`
- [x] WASM bindings via `wasm-bindgen`
- [x] Build script producing `.wasm` + `.js` + `.d.ts`
- [x] Browser demo page (`demo/index.html`) — static proof-of-concept
- [x] 28 unit tests passing (at time of Phase 0 completion)

## Phase 1: Spriteform Parity (v0.1) (done)

Feature-complete replacement for the JS compositor (core + WASM).

- [x] WASM API: layer property setters (opacity, visible, position, flip, rotation, anchor)
- [x] WASM API: filter config bulk setter (all 9 fields)
- [x] WASM API: group layer child management (add image/filter/group, remove)
- [x] PNG decode/encode (via `png` crate, `codec.rs`)
- [x] Raw RGBA import/export (`add_png_layer`, `export_png`, `get_layer_rgba`)
- [x] Scoped filter rendering (two-pass: render non-filter layers, then apply filters)
- [x] Color utilities: `hex_to_rgb`, `rgb_to_hex`, `srgb_to_linear`, `relative_luminance`, `contrast_ratio`, `dominant_rgb_from_rgba`
- [x] Recursive layer tree search (`find_layer`, `find_layer_mut`)
- [x] Interactive demo page with live filter sliders, PNG export/round-trip, color panel
- [x] 50 unit tests passing (42 core + 8 WASM)

## Phase 2: Extended Blend Modes & Masks (v0.2) (done)

- [x] All 16 Photoshop-compatible blend modes (Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity)
- [x] Layer masks (grayscale alpha, applied via red channel)
- [x] Clipping masks (`clip_to_below` flag)
- [x] SolidColorLayer, GradientLayer (horizontal, vertical, diagonal)
- [x] Flatten group to single image layer
- [x] WASM bindings: `set_blend_mode`, `set_layer_mask`, `remove_layer_mask`, `set_clip_to_below`, `add_solid_color_layer`, `add_gradient_layer`, `flatten_group`
- [x] 71 unit tests passing (57 core + 14 WASM)

## Phase 3: Advanced Filters & Transforms (v0.3) (done)

- [x] Convolution kernels (blur, edge detect, emboss, sharpen) — `convolution.rs`
- [x] Pixel filters: Invert, Posterize, Threshold, Levels, Gradient Map — `filter.rs`
- [x] Arbitrary-angle rotation (bilinear interpolation) — `transform.rs`
- [x] Resize (nearest-neighbor, bilinear, Lanczos3) — `transform.rs`
- [x] Crop, trim alpha — `transform.rs`
- [x] WASM bindings for all Phase 3 features (30 WASM tests, 82 core tests)
- [x] 112 unit tests passing (82 core + 30 WASM)

## Phase 4: Sprite & Game Dev Tools (v0.4) (done)

- [x] Sprite sheet packer (shelf next-fit bin-packing) — `sprite.rs`
- [x] Contact sheet / grid layout — `sprite.rs`
- [x] Pixel-art upscale (nearest-neighbor integer scale) — `sprite.rs`
- [x] Color quantization (median-cut) and palette generation — `sprite.rs`
- [x] Batch rendering pipeline — `sprite.rs`
- [x] WASM bindings for all Phase 4 features (35 WASM tests, 93 core tests)
- [x] 128 unit tests passing (93 core + 35 WASM)

## Phase 5: Format Support & Serialization (v0.5) (done)

- [x] JPEG decode/encode (`jpeg-decoder` + `jpeg-encoder`)
- [x] WebP decode/encode (lossless, `image-webp`)
- [x] GIF decode (animated frames → layers, `gif` crate)
- [x] PSD layer import (`psd` crate)
- [x] Format auto-detection via magic bytes (PNG, JPEG, WebP, GIF, PSD)
- [x] Document serialization (JSON metadata + binary pixel data) — `serialize.rs`
- [x] WASM bindings: `import_jpeg`, `import_webp`, `import_gif_frames`, `import_psd`, `import_auto`, `export_jpeg`, `export_webp`, `serialize`, `deserialize`, `detect_format`, `decode_image`
- [x] Demo page: drag-and-drop import, multi-format export, document save/load
- [x] 150 unit tests passing (109 core + 41 WASM)

## Phase 6: Stable Release (v1.0)

- [x] API stability guarantee
- [x] Comprehensive docs and examples
- [x] Published benchmarks
- [ ] WASM-SIMD optimized builds
- [x] Fuzz-tested codec paths
- [x] Security audit

## Backlog (unscheduled)

- [x] npm package with Node.js + Browser entrypoints
- [x] Integration tests comparing output to JS compositor golden images
- [x] Base64 RGBA encode/decode (JS-only, `atob`/`btoa`)
- [x] `readableTextColor` (trivial in JS once `relative_luminance` exposed)
