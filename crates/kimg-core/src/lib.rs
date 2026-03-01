//! kimg-core is a pure Rust pixel engine for layer-based image compositing.
//!
//! It provides primitives for layers, blend modes, filters, transforms, and codecs.
#![deny(missing_docs)]

/// Blend modes for compositing layers (Normal, Multiply, Screen, etc.)
pub mod blend;
/// Transformed blitting (position, anchor, rotation, flip, opacity)
pub mod blit;
/// Image buffer and pixel data representation
pub mod buffer;
/// Decoding and encoding image formats (PNG, JPEG, WebP, GIF, PSD)
pub mod codec;
/// Color space conversions and utilities (RGB, HSL, luminance)
pub mod color;
/// Matrix convolution filters (Blur, Sharpen, Edge Detect, etc.)
pub mod convolution;
/// The main document containing the layer tree and render pipeline
pub mod document;
/// HSL and pixel-level filters (Invert, Threshold, Levels, Gradient Map)
pub mod filter;
/// Layer definitions and data structures (Image, Paint, Filter, Group, etc.)
pub mod layer;
/// RGBA pixel representation
pub mod pixel;
/// Binary serialization and deserialization of the document
pub mod serialize;
/// Sprite sheet packing, contact sheets, upscale, and quantization
pub mod sprite;
/// Image transformations (Resize, Rotate, Crop, Trim)
pub mod transform;
