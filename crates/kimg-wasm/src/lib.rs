use wasm_bindgen::prelude::*;

use kimg_core::blend::BlendMode;
use kimg_core::blit::{Anchor, Rotation};
use kimg_core::buffer::ImageBuffer;
use kimg_core::codec;
use kimg_core::color;
use kimg_core::convolution;
use kimg_core::document::Document as CoreDocument;
use kimg_core::filter::{self, HslFilterConfig};
use kimg_core::layer::{
    FilterLayerData, GradientDirection, GradientLayerData, GradientStop, GroupLayerData,
    ImageLayerData, Layer, LayerCommon, LayerKind, PaintLayerData, SolidColorLayerData,
};
use kimg_core::pixel::Rgba;
use kimg_core::transform;

/// WASM-exposed Document for image compositing.
#[wasm_bindgen]
pub struct Document {
    inner: CoreDocument,
}

#[wasm_bindgen]
impl Document {
    /// Create a new document with the given canvas dimensions.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Document {
        Document {
            inner: CoreDocument::new(width, height),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.inner.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.inner.height
    }

    // ── Top-level layer creation ──

    /// Add an image layer from raw RGBA data. Returns the layer ID.
    pub fn add_image_layer(
        &mut self,
        name: &str,
        rgba_data: &[u8],
        img_width: u32,
        img_height: u32,
        x: i32,
        y: i32,
    ) -> u32 {
        let buffer = ImageBuffer::from_rgba(img_width, img_height, rgba_data.to_vec())
            .expect("RGBA data length must match width * height * 4");
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon {
                x,
                y,
                ..LayerCommon::new(id, name)
            },
            kind: LayerKind::Image(ImageLayerData {
                buffer,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        });
        id
    }

    /// Add a paint layer (empty editable RGBA buffer). Returns the layer ID.
    pub fn add_paint_layer(&mut self, name: &str, width: u32, height: u32) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::Paint(PaintLayerData {
                buffer: ImageBuffer::new_transparent(width, height),
                anchor: Anchor::TopLeft,
            }),
        });
        id
    }

    /// Add an HSL filter layer. Returns the layer ID.
    pub fn add_filter_layer(&mut self, name: &str) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::Filter(FilterLayerData {
                config: HslFilterConfig::default(),
            }),
        });
        id
    }

    /// Add a group layer. Returns the layer ID.
    pub fn add_group_layer(&mut self, name: &str) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::Group(GroupLayerData {
                children: Vec::new(),
            }),
        });
        id
    }

    /// Add a solid color fill layer. Returns the layer ID.
    pub fn add_solid_color_layer(&mut self, name: &str, r: u8, g: u8, b: u8, a: u8) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::SolidColor(SolidColorLayerData {
                color: Rgba::new(r, g, b, a),
            }),
        });
        id
    }

    /// Add a gradient layer. `stops` is a flat array of [pos_f32, r, g, b, a, ...].
    /// Each stop is 5 values: position (0-1 as f32 bits in a u8 pair? No — we use f64).
    /// Actually: stops_data is [r, g, b, a, r, g, b, a, ...] and stops_positions is [f64, f64, ...].
    /// Direction: 0=horizontal, 1=vertical, 2=diagonal-down, 3=diagonal-up.
    pub fn add_gradient_layer(
        &mut self,
        name: &str,
        stops_colors: &[u8],
        stops_positions: &[f64],
        direction: u8,
    ) -> u32 {
        let count = stops_positions.len();
        let mut stops = Vec::with_capacity(count);
        for i in 0..count {
            let ci = i * 4;
            if ci + 3 < stops_colors.len() {
                stops.push(GradientStop {
                    position: stops_positions[i],
                    color: Rgba::new(
                        stops_colors[ci],
                        stops_colors[ci + 1],
                        stops_colors[ci + 2],
                        stops_colors[ci + 3],
                    ),
                });
            }
        }
        let dir = match direction {
            1 => GradientDirection::Vertical,
            2 => GradientDirection::DiagonalDown,
            3 => GradientDirection::DiagonalUp,
            _ => GradientDirection::Horizontal,
        };
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::Gradient(GradientLayerData {
                stops,
                direction: dir,
            }),
        });
        id
    }

    // ── Layer property setters (any layer type) ──

    /// Set layer opacity (0.0 to 1.0).
    pub fn set_opacity(&mut self, id: u32, opacity: f64) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    /// Set layer visibility.
    pub fn set_visible(&mut self, id: u32, visible: bool) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.visible = visible;
        }
    }

    /// Set layer position.
    pub fn set_position(&mut self, id: u32, x: i32, y: i32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.x = x;
            layer.common.y = y;
        }
    }

    /// Set blend mode by name (e.g. "multiply", "screen", "color-dodge").
    pub fn set_blend_mode(&mut self, id: u32, mode: &str) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.blend_mode = BlendMode::from_str_lossy(mode);
        }
    }

    /// Set a grayscale layer mask from RGBA data. Uses the red channel as mask value.
    pub fn set_layer_mask(
        &mut self,
        id: u32,
        mask_data: &[u8],
        mask_width: u32,
        mask_height: u32,
    ) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let Some(buf) = ImageBuffer::from_rgba(mask_width, mask_height, mask_data.to_vec()) {
                layer.common.mask = Some(buf);
            }
        }
    }

    /// Remove a layer's mask.
    pub fn remove_layer_mask(&mut self, id: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.mask = None;
        }
    }

    /// Set whether a layer clips to the layer below it.
    pub fn set_clip_to_below(&mut self, id: u32, clip: bool) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.clip_to_below = clip;
        }
    }

    // ── Image-specific setters ──

    /// Set flip on an image layer.
    pub fn set_flip(&mut self, id: u32, flip_x: bool, flip_y: bool) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.flip_x = flip_x;
                img.flip_y = flip_y;
            }
        }
    }

    /// Set rotation on an image layer (snaps to nearest 90 degrees).
    pub fn set_rotation(&mut self, id: u32, degrees: f64) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.rotation = Rotation::from_degrees(degrees);
            }
        }
    }

    /// Set anchor on an image layer. 0 = TopLeft, 1 = Center.
    pub fn set_anchor(&mut self, id: u32, anchor: u8) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            let a = match anchor {
                1 => Anchor::Center,
                _ => Anchor::TopLeft,
            };
            match &mut layer.kind {
                LayerKind::Image(img) => img.anchor = a,
                LayerKind::Paint(paint) => paint.anchor = a,
                _ => {}
            }
        }
    }

    // ── Filter config setter ──

    /// Bulk-set all 9 filter config fields on a filter layer.
    #[allow(clippy::too_many_arguments)]
    pub fn set_filter_config(
        &mut self,
        id: u32,
        hue_deg: f64,
        saturation: f64,
        lightness: f64,
        alpha: f64,
        brightness: f64,
        contrast: f64,
        temperature: f64,
        tint: f64,
        sharpen: f64,
    ) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Filter(f) = &mut layer.kind {
                f.config = HslFilterConfig {
                    hue_deg,
                    saturation,
                    lightness,
                    alpha,
                    brightness,
                    contrast,
                    temperature,
                    tint,
                    sharpen,
                };
            }
        }
    }

    // ── Group child management ──

    /// Add an image layer as a child of a group. Returns the child layer ID.
    #[allow(clippy::too_many_arguments)]
    pub fn add_image_to_group(
        &mut self,
        group_id: u32,
        name: &str,
        rgba_data: &[u8],
        img_width: u32,
        img_height: u32,
        x: i32,
        y: i32,
    ) -> u32 {
        let buffer = ImageBuffer::from_rgba(img_width, img_height, rgba_data.to_vec())
            .expect("RGBA data length must match width * height * 4");
        let id = self.inner.next_id();
        let child = Layer {
            common: LayerCommon {
                x,
                y,
                ..LayerCommon::new(id, name)
            },
            kind: LayerKind::Image(ImageLayerData {
                buffer,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        };
        self.inner
            .add_child_to_group(group_id, child)
            .expect("group not found");
        id
    }

    /// Add a filter layer as a child of a group. Returns the child layer ID.
    pub fn add_filter_to_group(&mut self, group_id: u32, name: &str) -> u32 {
        let id = self.inner.next_id();
        let child = Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::Filter(FilterLayerData {
                config: HslFilterConfig::default(),
            }),
        };
        self.inner
            .add_child_to_group(group_id, child)
            .expect("group not found");
        id
    }

    /// Add a nested group as a child of a group. Returns the child layer ID.
    pub fn add_group_to_group(&mut self, group_id: u32, name: &str) -> u32 {
        let id = self.inner.next_id();
        let child = Layer {
            common: LayerCommon::new(id, name),
            kind: LayerKind::Group(GroupLayerData {
                children: Vec::new(),
            }),
        };
        self.inner
            .add_child_to_group(group_id, child)
            .expect("group not found");
        id
    }

    /// Remove a child from a group. Returns true if found and removed.
    pub fn remove_from_group(&mut self, group_id: u32, child_id: u32) -> bool {
        self.inner.remove_child_from_group(group_id, child_id)
    }

    /// Flatten a group layer into a single image layer. Returns true on success.
    pub fn flatten_group(&mut self, group_id: u32) -> bool {
        self.inner.flatten_group(group_id)
    }

    // ── PNG import/export ──

    /// Decode a PNG and add it as a top-level image layer. Returns the layer ID.
    pub fn add_png_layer(&mut self, name: &str, png_bytes: &[u8], x: i32, y: i32) -> u32 {
        let buf = codec::decode_png(png_bytes).expect("failed to decode PNG");
        let id = self.inner.next_id();
        self.inner.layers.push(Layer {
            common: LayerCommon {
                x,
                y,
                ..LayerCommon::new(id, name)
            },
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        });
        id
    }

    /// Render the document and encode as PNG.
    pub fn export_png(&self) -> Vec<u8> {
        let result = self.inner.render();
        codec::encode_png(&result).expect("failed to encode PNG")
    }

    /// Get a layer's raw RGBA pixel buffer. Returns empty vec if not an image/paint layer.
    pub fn get_layer_rgba(&self, id: u32) -> Vec<u8> {
        match self.inner.find_layer(id) {
            Some(layer) => match &layer.kind {
                LayerKind::Image(img) => img.buffer.data.clone(),
                LayerKind::Paint(paint) => paint.buffer.data.clone(),
                _ => Vec::new(),
            },
            None => Vec::new(),
        }
    }

    // ── Rendering ──

    /// Render the document and return the flat RGBA buffer.
    pub fn render(&self) -> Vec<u8> {
        let result = self.inner.render();
        result.data
    }

    /// Get the number of top-level layers.
    pub fn layer_count(&self) -> usize {
        self.inner.layers.len()
    }

    // ── Phase 3: Transform operations ──

    /// Resize a layer's buffer using nearest-neighbor (good for pixel art).
    pub fn resize_layer_nearest(&mut self, id: u32, new_width: u32, new_height: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = transform::resize_nearest(&img.buffer, new_width, new_height);
            }
        }
    }

    /// Resize a layer's buffer using bilinear interpolation.
    pub fn resize_layer_bilinear(&mut self, id: u32, new_width: u32, new_height: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = transform::resize_bilinear(&img.buffer, new_width, new_height);
            }
        }
    }

    /// Resize a layer's buffer using Lanczos3 interpolation (high quality).
    pub fn resize_layer_lanczos3(&mut self, id: u32, new_width: u32, new_height: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = transform::resize_lanczos3(&img.buffer, new_width, new_height);
            }
        }
    }

    /// Crop a layer's buffer to the given rectangle.
    pub fn crop_layer(&mut self, id: u32, x: u32, y: u32, width: u32, height: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = transform::crop(&img.buffer, x, y, width, height);
            }
        }
    }

    /// Trim transparent edges from a layer's buffer.
    pub fn trim_layer_alpha(&mut self, id: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = transform::trim_alpha(&img.buffer);
            }
        }
    }

    /// Rotate a layer's buffer by an arbitrary angle (degrees) with bilinear interpolation.
    pub fn rotate_layer(&mut self, id: u32, angle_deg: f64) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = transform::rotate_bilinear(&img.buffer, angle_deg);
            }
        }
    }

    // ── Phase 3: Convolution filters ──

    /// Apply a box blur to a layer. Radius 0 = no-op.
    pub fn box_blur_layer(&mut self, id: u32, radius: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = convolution::box_blur(&img.buffer, radius);
            }
        }
    }

    /// Apply a Gaussian blur to a layer. Radius 0 = no-op.
    pub fn gaussian_blur_layer(&mut self, id: u32, radius: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = convolution::gaussian_blur(&img.buffer, radius);
            }
        }
    }

    /// Apply a sharpen convolution to a layer.
    pub fn sharpen_layer(&mut self, id: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = convolution::convolve(&img.buffer, &convolution::Kernel::sharpen());
            }
        }
    }

    /// Apply edge detection (Laplacian) to a layer.
    pub fn edge_detect_layer(&mut self, id: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer =
                    convolution::convolve(&img.buffer, &convolution::Kernel::edge_detect());
            }
        }
    }

    /// Apply emboss effect to a layer.
    pub fn emboss_layer(&mut self, id: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = convolution::convolve(&img.buffer, &convolution::Kernel::emboss());
            }
        }
    }

    // ── Phase 3: Pixel filters ──

    /// Invert RGB channels of a layer.
    pub fn invert_layer(&mut self, id: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                filter::invert(&mut img.buffer);
            }
        }
    }

    /// Posterize a layer (reduce color levels per channel).
    pub fn posterize_layer(&mut self, id: u32, levels: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                filter::posterize(&mut img.buffer, levels);
            }
        }
    }

    /// Convert a layer to black/white based on luminance threshold (0–255).
    pub fn threshold_layer(&mut self, id: u32, thresh: u8) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                filter::threshold(&mut img.buffer, thresh);
            }
        }
    }

    /// Apply levels adjustment to a layer.
    #[allow(clippy::too_many_arguments)]
    pub fn levels_layer(
        &mut self,
        id: u32,
        in_black: u8,
        in_white: u8,
        gamma: f64,
        out_black: u8,
        out_white: u8,
    ) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                filter::levels(&mut img.buffer, in_black, in_white, gamma, out_black, out_white);
            }
        }
    }

    /// Apply a gradient map to a layer. `stops_colors` is [r,g,b,a, r,g,b,a, ...],
    /// `stops_positions` is [f64, f64, ...].
    pub fn gradient_map_layer(
        &mut self,
        id: u32,
        stops_colors: &[u8],
        stops_positions: &[f64],
    ) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                let count = stops_positions.len();
                let mut stops = Vec::with_capacity(count);
                for i in 0..count {
                    let ci = i * 4;
                    if ci + 3 < stops_colors.len() {
                        stops.push((
                            stops_positions[i],
                            Rgba::new(
                                stops_colors[ci],
                                stops_colors[ci + 1],
                                stops_colors[ci + 2],
                                stops_colors[ci + 3],
                            ),
                        ));
                    }
                }
                filter::gradient_map(&mut img.buffer, &stops);
            }
        }
    }
}

// ── Free functions: color utilities ──

/// Parse hex color to RGB. Returns 3 bytes [r, g, b] or empty vec on failure.
#[wasm_bindgen]
pub fn hex_to_rgb(hex: &str) -> Vec<u8> {
    match color::hex_to_rgb(hex) {
        Some(rgb) => vec![rgb.r, rgb.g, rgb.b],
        None => Vec::new(),
    }
}

/// Format RGB as hex string.
#[wasm_bindgen]
pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    color::rgb_to_hex(color::Rgb { r, g, b })
}

/// WCAG 2.x relative luminance. Returns -1.0 on failure.
#[wasm_bindgen]
pub fn relative_luminance(hex: &str) -> f64 {
    color::relative_luminance(hex).unwrap_or(-1.0)
}

/// WCAG 2.x contrast ratio. Returns -1.0 on failure.
#[wasm_bindgen]
pub fn contrast_ratio(a: &str, b: &str) -> f64 {
    color::contrast_ratio(a, b).unwrap_or(-1.0)
}

/// Dominant color from RGBA pixel data. Returns 3 bytes [r, g, b] or empty on failure.
#[wasm_bindgen]
pub fn dominant_rgb_from_rgba(data: &[u8], w: u32, h: u32) -> Vec<u8> {
    match color::dominant_rgb_from_rgba(data, w, h, 4096) {
        Some(rgb) => vec![rgb.r, rgb.g, rgb.b],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_render_empty() {
        let doc = Document::new(4, 4);
        let data = doc.render();
        assert_eq!(data.len(), 4 * 4 * 4);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn add_image_layer_and_render() {
        let mut doc = Document::new(2, 2);
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255];
        doc.add_image_layer("test", &rgba, 2, 2, 0, 0);
        let result = doc.render();
        assert_eq!(result[0], 255);
        assert_eq!(result[1], 0);
        assert_eq!(result[2], 0);
        assert_eq!(result[3], 255);
    }

    #[test]
    fn setter_methods() {
        let mut doc = Document::new(4, 4);
        let rgba = vec![255u8; 2 * 2 * 4];
        let id = doc.add_image_layer("img", &rgba, 2, 2, 0, 0);

        doc.set_opacity(id, 0.5);
        doc.set_visible(id, false);
        doc.set_position(id, 10, 20);
        doc.set_flip(id, true, false);
        doc.set_rotation(id, 90.0);
        doc.set_anchor(id, 1);
        doc.set_blend_mode(id, "multiply");

        let layer = doc.inner.find_layer(id).unwrap();
        assert_eq!(layer.common.opacity, 0.5);
        assert!(!layer.common.visible);
        assert_eq!(layer.common.x, 10);
        assert_eq!(layer.common.y, 20);
        assert_eq!(layer.common.blend_mode, BlendMode::Multiply);
        if let LayerKind::Image(img) = &layer.kind {
            assert!(img.flip_x);
            assert!(!img.flip_y);
            assert_eq!(img.rotation, Rotation::Cw90);
            assert_eq!(img.anchor, Anchor::Center);
        } else {
            panic!("expected image layer");
        }
    }

    #[test]
    fn filter_config_setter() {
        let mut doc = Document::new(4, 4);
        let id = doc.add_filter_layer("f");
        doc.set_filter_config(id, 30.0, 0.1, 0.2, -0.1, 0.3, 0.4, 0.5, -0.2, 0.8);

        let layer = doc.inner.find_layer(id).unwrap();
        if let LayerKind::Filter(f) = &layer.kind {
            assert_eq!(f.config.hue_deg, 30.0);
            assert_eq!(f.config.sharpen, 0.8);
        } else {
            panic!("expected filter layer");
        }
    }

    #[test]
    fn group_child_management() {
        let mut doc = Document::new(4, 4);
        let gid = doc.add_group_layer("g");
        let cid = doc.add_filter_to_group(gid, "child-filter");
        assert!(doc.inner.find_layer(cid).is_some());
        assert!(doc.remove_from_group(gid, cid));
        assert!(doc.inner.find_layer(cid).is_none());
    }

    #[test]
    fn png_roundtrip_via_wasm() {
        let mut doc = Document::new(2, 2);
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255];
        doc.add_image_layer("test", &rgba, 2, 2, 0, 0);

        let png_bytes = doc.export_png();

        let mut doc2 = Document::new(2, 2);
        doc2.add_png_layer("from_png", &png_bytes, 0, 0);
        let result = doc2.render();
        assert_eq!(result[0], 255);
        assert_eq!(result[3], 255);
    }

    #[test]
    fn get_layer_rgba_returns_buffer() {
        let mut doc = Document::new(2, 2);
        let rgba = vec![1u8; 2 * 2 * 4];
        let id = doc.add_image_layer("img", &rgba, 2, 2, 0, 0);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 16);
        assert!(buf.iter().all(|&b| b == 1));
    }

    #[test]
    fn color_free_functions() {
        let rgb = hex_to_rgb("#ff0000");
        assert_eq!(rgb, vec![255, 0, 0]);

        let hex = rgb_to_hex(255, 0, 0);
        assert_eq!(hex, "#ff0000");

        let lum = relative_luminance("#ffffff");
        assert!((lum - 1.0).abs() < 0.001);

        let cr = contrast_ratio("#000000", "#ffffff");
        assert!((cr - 21.0).abs() < 0.1);

        assert_eq!(relative_luminance("invalid"), -1.0);
        assert_eq!(contrast_ratio("x", "y"), -1.0);
    }

    // ── Phase 2 tests ──

    #[test]
    fn blend_mode_setter() {
        let mut doc = Document::new(1, 1);
        let rgba = vec![200u8, 100, 50, 255];
        let id1 = doc.add_image_layer("base", &rgba, 1, 1, 0, 0);
        let id2 = doc.add_image_layer("top", &[128, 128, 128, 255], 1, 1, 0, 0);
        doc.set_blend_mode(id2, "multiply");

        let layer = doc.inner.find_layer(id2).unwrap();
        assert_eq!(layer.common.blend_mode, BlendMode::Multiply);

        let _ = doc.render(); // should not panic
        let _ = id1;
    }

    #[test]
    fn mask_set_remove() {
        let mut doc = Document::new(2, 1);
        let id = doc.add_image_layer("img", &[255, 0, 0, 255, 255, 0, 0, 255], 2, 1, 0, 0);

        // Set mask
        let mask = vec![255, 255, 255, 255, 0, 0, 0, 255]; // white, black
        doc.set_layer_mask(id, &mask, 2, 1);
        let result = doc.render();
        assert_eq!(result[3], 255); // left visible
        assert_eq!(result[7], 0); // right hidden

        // Remove mask
        doc.remove_layer_mask(id);
        let result = doc.render();
        assert_eq!(result[3], 255);
        assert_eq!(result[7], 255); // now visible again
    }

    #[test]
    fn clip_to_below_setter() {
        let mut doc = Document::new(2, 1);
        // Below: only left pixel
        let mut below = vec![0u8; 8];
        below[0] = 0;
        below[1] = 255;
        below[2] = 0;
        below[3] = 255;
        doc.add_image_layer("below", &below, 2, 1, 0, 0);

        let id = doc.add_image_layer("top", &[255, 0, 0, 255, 255, 0, 0, 255], 2, 1, 0, 0);
        doc.set_clip_to_below(id, true);

        let result = doc.render();
        assert!(result[3] > 0); // left visible (clipped to green alpha)
        assert_eq!(result[7], 0); // right hidden (below is transparent)
    }

    #[test]
    fn solid_color_layer_wasm() {
        let mut doc = Document::new(2, 2);
        doc.add_solid_color_layer("fill", 100, 200, 50, 255);
        let result = doc.render();
        assert_eq!(result[0], 100);
        assert_eq!(result[1], 200);
        assert_eq!(result[2], 50);
        assert_eq!(result[3], 255);
    }

    #[test]
    fn gradient_layer_wasm() {
        let mut doc = Document::new(3, 1);
        let colors = vec![0, 0, 0, 255, 255, 255, 255, 255]; // black to white
        let positions = vec![0.0, 1.0];
        doc.add_gradient_layer("grad", &colors, &positions, 0);

        let result = doc.render();
        assert!(result[0] < 10); // left ≈ black
        assert!(result[8] > 245); // right ≈ white
    }

    #[test]
    fn flatten_group_wasm() {
        let mut doc = Document::new(2, 2);
        let gid = doc.add_group_layer("g");
        let red_rgba: Vec<u8> = [255, 0, 0, 255].iter().copied().cycle().take(16).collect();
        doc.add_image_to_group(gid, "red", &red_rgba, 2, 2, 0, 0);

        assert!(doc.flatten_group(gid));

        let layer = doc.inner.find_layer(gid).unwrap();
        assert!(matches!(layer.kind, LayerKind::Image(_)));
    }

    // ── Phase 3 tests ──

    fn make_red_4x4() -> (Document, u32) {
        let mut doc = Document::new(4, 4);
        let rgba: Vec<u8> = [255, 0, 0, 255].iter().copied().cycle().take(4 * 4 * 4).collect();
        let id = doc.add_image_layer("red", &rgba, 4, 4, 0, 0);
        (doc, id)
    }

    #[test]
    fn resize_layer_nearest_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.resize_layer_nearest(id, 2, 2);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 2 * 2 * 4);
        assert_eq!(buf[0], 255); // still red
    }

    #[test]
    fn resize_layer_bilinear_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.resize_layer_bilinear(id, 8, 8);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 8 * 8 * 4);
    }

    #[test]
    fn resize_layer_lanczos3_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.resize_layer_lanczos3(id, 8, 8);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 8 * 8 * 4);
    }

    #[test]
    fn crop_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.crop_layer(id, 1, 1, 2, 2);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 2 * 2 * 4);
    }

    #[test]
    fn trim_layer_alpha_wasm() {
        let mut doc = Document::new(4, 4);
        // Only center 2x2 has alpha
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for y in 1..3 {
            for x in 1..3 {
                let i = (y * 4 + x) * 4;
                rgba[i] = 255;
                rgba[i + 3] = 255;
            }
        }
        let id = doc.add_image_layer("img", &rgba, 4, 4, 0, 0);
        doc.trim_layer_alpha(id);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 2 * 2 * 4);
    }

    #[test]
    fn rotate_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.rotate_layer(id, 45.0);
        // Rotated buffer should be larger than original
        let buf = doc.get_layer_rgba(id);
        assert!(buf.len() > 4 * 4 * 4);
    }

    #[test]
    fn box_blur_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.box_blur_layer(id, 1);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 4 * 4 * 4);
    }

    #[test]
    fn gaussian_blur_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.gaussian_blur_layer(id, 1);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 4 * 4 * 4);
    }

    #[test]
    fn sharpen_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.sharpen_layer(id);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf[0], 255); // uniform red stays red after sharpen
    }

    #[test]
    fn edge_detect_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.edge_detect_layer(id);
        // Should not panic
        let _ = doc.get_layer_rgba(id);
    }

    #[test]
    fn emboss_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.emboss_layer(id);
        let _ = doc.get_layer_rgba(id);
    }

    #[test]
    fn invert_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.invert_layer(id);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf[0], 0);   // 255 inverted
        assert_eq!(buf[1], 255); // 0 inverted
        assert_eq!(buf[2], 255); // 0 inverted
    }

    #[test]
    fn posterize_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.posterize_layer(id, 2);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf[0], 255); // red stays 255 with 2 levels
    }

    #[test]
    fn threshold_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.threshold_layer(id, 128);
        let buf = doc.get_layer_rgba(id);
        // Red (255,0,0) luminance ≈ 76, below 128 → black
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn levels_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.levels_layer(id, 0, 255, 1.0, 0, 128);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf[0], 128); // 255 remapped to 128
    }

    #[test]
    fn gradient_map_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        let colors = vec![0, 0, 255, 255, 255, 255, 0, 255]; // blue to yellow
        let positions = vec![0.0, 1.0];
        doc.gradient_map_layer(id, &colors, &positions);
        // Red luminance ≈ 76/255 ≈ 0.298 → interpolated between blue and yellow
        let buf = doc.get_layer_rgba(id);
        assert!(buf[3] == 255); // alpha preserved
    }
}
