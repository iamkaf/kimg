# kimg Feature Roadmap

This roadmap is feature-first and aimed at one end-state:

- server-friendly original-image pipelines
- browser-native, Photoshop-class layered editing

## P0: Core Editing Foundation

These unlock the largest block of expected editor behavior.

### 1) Selection System

- rectangular, elliptical, polygonal, and lasso selections
- color-range / magic-wand selection with tolerance
- selection operations: add, subtract, intersect, invert
- selection refinement: feather, grow, shrink, border
- selection masks as first-class data

### 2) Selection-Aware Operations

- brush constrained by active selection
- bucket fill constrained by selection (contiguous and non-contiguous)
- filters constrained to selection region
- move/transform selected pixels with non-destructive preview

### 3) Adjustment Layers (Non-Destructive)

- levels
- curves
- hue/saturation
- color balance
- gradient map
- per-adjustment mask + blend mode + opacity

## P1: Professional Paint and Masking

### 4) Brush Engine Depth

- symmetry and mirror painting
- stamp/texture tips
- jitter/scatter controls
- smudge and wet/mixer-like behavior
- blend-aware painting modes

### 5) Advanced Masking

- layer masks + clipping masks + vector masks working together
- mask density and feather controls
- mask compositing ops (add/subtract/intersect)
- mask visualization modes for debugging and authoring

### 6) Better Fill Tools

- tolerance and anti-alias controls
- reference-all-layers option
- grow/shrink before fill
- pattern and gradient fills

## P2: Text and Vector Power

### 7) Text Engine Parity

- shaping quality and predictable fallback chains
- kerning/ligatures/OpenType controls
- paragraph layout controls (leading, tracking, justification)
- text-on-path
- reliable cross-runtime metrics parity (Node/WASM/browser)

### 8) Shape Layer Expansion

- unified shape model with corner radius (already merged for rectangle family)
- boolean operations across shapes
- richer stroke controls (dashes, joins, caps, alignment)
- gradient and pattern fills per shape
- editable vector paths

### 9) SVG Layer Workflow

- retained SVG layer transforms + filters before rasterization
- better font handling for SVG `<text>` content
- explicit rasterization controls and quality settings
- expanded safe SVG feature support (without scripts/external refs)

## P3: Color and Image Fidelity

### 10) Color Management

- explicit color-space handling (sRGB/linear)
- ICC profile awareness and conversion policy
- consistent blend/filter behavior across runtimes

### 11) Higher Bit Depth

- 16-bit/channel internal pipeline path
- per-feature fallback rules when 8-bit-only operations are used

### 12) Format Fidelity

- stronger PSD compatibility (still experimental until parity improves)
- better metadata round-tripping for major formats
- SVG and text import/export quality upgrades

## P4: Composition and Automation Depth

### 13) Smart Source Layers

- linked or embedded source assets that stay live until explicit rasterize
- update propagation when source asset changes
- per-layer transform/filter stacks preserved

### 14) Procedural/Generated Layers

- gradient, pattern, and noise layers as first-class sources
- parameterized generators editable after creation

### 15) Pipeline-Friendly APIs

- graph-level render requests (partial region, scale, quality tiers)
- deterministic render options for reproducible backend output
- better batching primitives for large server workloads

## P5: Future Feature Track

Features intentionally tracked for later:

- full history/timeline editing model
- collaborative layer editing semantics
- AI-assisted selection/masking/retouching helpers

## Current Priority Order

If we only do three things next, they should be:

1. selection system
2. selection-aware operations
3. adjustment layers

