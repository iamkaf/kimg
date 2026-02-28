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

## Phase 3: Advanced Filters & Transforms (v0.3)

- [ ] Convolution kernels (blur, edge detect, emboss)
- [ ] Levels/Curves, Posterize, Threshold, Invert, Gradient Map
- [ ] Arbitrary-angle rotation (bilinear interpolation)
- [ ] Resize (bilinear, Lanczos3)
- [ ] Crop, trim alpha

## Phase 4: Sprite & Game Dev Tools (v0.4)

- [ ] Sprite sheet packer (bin-packing)
- [ ] Contact sheet / grid layout
- [ ] Pixel-art upscale (nearest-neighbor integer scale)
- [ ] Color quantization and palette generation
- [ ] Batch rendering pipeline

## Phase 5: Format Support & Serialization (v0.5)

- [ ] JPEG decode/encode
- [ ] WebP decode/encode
- [ ] GIF decode (animated frames → layers)
- [ ] Document serialization (JSON + binary)
- [ ] PSD layer import

## Phase 6: Stable Release (v1.0)

- [ ] API stability guarantee
- [ ] Comprehensive docs and examples
- [ ] Published benchmarks
- [ ] WASM-SIMD optimized builds
- [ ] Fuzz-tested codec paths
- [ ] Security audit

## Backlog (unscheduled)

- [ ] npm package with Node.js + Browser entrypoints
- [ ] Integration tests comparing output to JS compositor golden images
- [ ] Base64 RGBA encode/decode (JS-only, `atob`/`btoa`)
- [ ] `readableTextColor` (trivial in JS once `relative_luminance` exposed)
