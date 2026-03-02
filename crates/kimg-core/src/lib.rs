//! kimg-core is a pure Rust pixel engine for layer-based image compositing.
//!
//! It provides primitives for layers, blend modes, filters, transforms, and codecs.
#![deny(missing_docs)]

pub mod blend;
pub mod blit;
pub mod buffer;
pub mod codec;
pub mod color;
pub mod convolution;
pub mod document;
pub mod filter;
pub mod layer;
pub mod pixel;
pub mod serialize;
pub mod sprite;
pub mod transform;
