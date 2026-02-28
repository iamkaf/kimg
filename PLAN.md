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
- [x] 28 unit tests passing

## Phase 1: Spriteform Parity (v0.1) (current)

Feature-complete replacement for the JS compositor.

- [ ] WASM API: layer property setters (opacity, visible, position, flip, rotation, anchor)
- [ ] WASM API: filter config setters (hue, saturation, lightness, brightness, etc.)
- [ ] WASM API: group layer child management
- [ ] PNG decode/encode (via `png` crate)
- [ ] Raw RGBA import/export
- [ ] Base64 RGBA encode/decode
- [ ] Scoped filter application (filters within group affect only that group)
- [ ] npm package with Node.js + Browser entrypoints
- [ ] Color utilities: hex conversion, dominant color, luminance, contrast ratio
- [ ] Integration tests comparing output to JS compositor golden images

## Phase 2: Extended Blend Modes & Masks (v0.2)

- [ ] All 16 Photoshop-compatible blend modes
- [ ] Layer masks (grayscale alpha)
- [ ] Clipping masks
- [ ] SolidColorLayer, GradientLayer
- [ ] Flatten group to single layer

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
