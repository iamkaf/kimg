//! WebAssembly bindings for kimg — the image compositing engine.
//!
//! Exposes a [`Composition`] class to JavaScript/TypeScript via `wasm-bindgen`.
//! The API mirrors the kimg-core document model: create layers, set properties,
//! and call `render()` to get a flat RGBA byte array.
//!
//! # JavaScript usage
//!
//! ```js
//! import init, { Composition } from "kimg_wasm";
//!
//! await init();
//! const doc = new Composition(512, 512);
//! const layerId = doc.add_solid_color_layer("bg", 255, 0, 0, 255);
//! const rgba = doc.render(); // Uint8Array, length = width * height * 4
//! ```

use wasm_bindgen::prelude::*;

use kimg_core::blend::BlendMode;
use kimg_core::blit::{Anchor, Rotation};
use kimg_core::buffer::ImageBuffer;
use kimg_core::codec;
use kimg_core::color;
use kimg_core::convolution;
use kimg_core::document::Document as CoreDocument;
use kimg_core::filter;
use kimg_core::layer::{
    FilterLayerData, GradientDirection, GradientLayerData, GradientStop, GroupLayerData,
    ImageLayerData, Layer, LayerCommon, LayerKind, PaintLayerData, SolidColorLayerData,
};
use kimg_core::pixel::Rgba;
use kimg_core::serialize;
use kimg_core::sprite;
use kimg_core::transform;

/// WASM-exposed Composition for image compositing.
#[wasm_bindgen(js_name = Composition)]
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
        let mut common = LayerCommon::new(id, name);
        common.x = x;
        common.y = y;
        self.inner.layers.push(Layer::new(
            common,
            LayerKind::Image(ImageLayerData::new(buffer)),
        ));
        id
    }

    /// Add a paint layer (empty editable RGBA buffer). Returns the layer ID.
    pub fn add_paint_layer(&mut self, name: &str, width: u32, height: u32) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer::new(
            LayerCommon::new(id, name),
            LayerKind::Paint(PaintLayerData::new(ImageBuffer::new_transparent(
                width, height,
            ))),
        ));
        id
    }

    /// Add an HSL filter layer. Returns the layer ID.
    pub fn add_filter_layer(&mut self, name: &str) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer::new(
            LayerCommon::new(id, name),
            LayerKind::Filter(FilterLayerData::new()),
        ));
        id
    }

    /// Add a group layer. Returns the layer ID.
    pub fn add_group_layer(&mut self, name: &str) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer::new(
            LayerCommon::new(id, name),
            LayerKind::Group(GroupLayerData::new()),
        ));
        id
    }

    /// Add a solid color fill layer. Returns the layer ID.
    pub fn add_solid_color_layer(&mut self, name: &str, r: u8, g: u8, b: u8, a: u8) -> u32 {
        let id = self.inner.next_id();
        self.inner.layers.push(Layer::new(
            LayerCommon::new(id, name),
            LayerKind::SolidColor(SolidColorLayerData::new(Rgba::new(r, g, b, a))),
        ));
        id
    }

    /// Add a gradient layer. Returns the layer ID.
    ///
    /// - `stops_colors`: flat RGBA bytes for each stop, 4 bytes per stop
    ///   (e.g. `[r0, g0, b0, a0, r1, g1, b1, a1, ...]`).
    /// - `stops_positions`: gradient position for each stop, in `[0.0, 1.0]`.
    /// - `direction`: `0` = horizontal, `1` = vertical, `2` = diagonal-down, `3` = diagonal-up.
    pub fn add_gradient_layer(
        &mut self,
        name: &str,
        stops_colors: &[u8],
        stops_positions: &[f64],
        direction: u8,
    ) -> u32 {
        let count = stops_positions.len();
        let mut stops = Vec::with_capacity(count);
        for (i, &pos) in stops_positions.iter().enumerate().take(count) {
            let ci = i * 4;
            if ci + 3 < stops_colors.len() {
                stops.push(GradientStop::new(
                    pos,
                    Rgba::new(
                        stops_colors[ci],
                        stops_colors[ci + 1],
                        stops_colors[ci + 2],
                        stops_colors[ci + 3],
                    ),
                ));
            }
        }
        let dir = match direction {
            1 => GradientDirection::Vertical,
            2 => GradientDirection::DiagonalDown,
            3 => GradientDirection::DiagonalUp,
            _ => GradientDirection::Horizontal,
        };
        let id = self.inner.next_id();
        self.inner.layers.push(Layer::new(
            LayerCommon::new(id, name),
            LayerKind::Gradient(GradientLayerData::new(stops, dir)),
        ));
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
    pub fn set_layer_mask(&mut self, id: u32, mask_data: &[u8], mask_width: u32, mask_height: u32) {
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

    /// Set whether the layer mask is inverted (black = visible, white = hidden).
    pub fn set_mask_inverted(&mut self, id: u32, inverted: bool) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            layer.common.mask_inverted = inverted;
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
                f.config.hue_deg = hue_deg;
                f.config.saturation = saturation;
                f.config.lightness = lightness;
                f.config.alpha = alpha;
                f.config.brightness = brightness;
                f.config.contrast = contrast;
                f.config.temperature = temperature;
                f.config.tint = tint;
                f.config.sharpen = sharpen;
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
        let mut common = LayerCommon::new(id, name);
        common.x = x;
        common.y = y;
        let child = Layer::new(common, LayerKind::Image(ImageLayerData::new(buffer)));
        self.inner
            .add_child_to_group(group_id, child)
            .expect("group not found");
        id
    }

    /// Add a filter layer as a child of a group. Returns the child layer ID.
    pub fn add_filter_to_group(&mut self, group_id: u32, name: &str) -> u32 {
        let id = self.inner.next_id();
        let child = Layer::new(
            LayerCommon::new(id, name),
            LayerKind::Filter(FilterLayerData::new()),
        );
        self.inner
            .add_child_to_group(group_id, child)
            .expect("group not found");
        id
    }

    /// Add a nested group as a child of a group. Returns the child layer ID.
    pub fn add_group_to_group(&mut self, group_id: u32, name: &str) -> u32 {
        let id = self.inner.next_id();
        let child = Layer::new(
            LayerCommon::new(id, name),
            LayerKind::Group(GroupLayerData::new()),
        );
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
        let mut common = LayerCommon::new(id, name);
        common.x = x;
        common.y = y;
        self.inner.layers.push(Layer::new(
            common,
            LayerKind::Image(ImageLayerData::new(buf)),
        ));
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
                filter::levels(
                    &mut img.buffer,
                    in_black,
                    in_white,
                    gamma,
                    out_black,
                    out_white,
                );
            }
        }
    }

    /// Apply a gradient map to a layer. `stops_colors` is [r,g,b,a, r,g,b,a, ...],
    /// `stops_positions` is [f64, f64, ...].
    pub fn gradient_map_layer(&mut self, id: u32, stops_colors: &[u8], stops_positions: &[f64]) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                let count = stops_positions.len();
                let mut stops = Vec::with_capacity(count);
                for (i, &pos) in stops_positions.iter().enumerate().take(count) {
                    let ci = i * 4;
                    if ci + 3 < stops_colors.len() {
                        stops.push((
                            pos,
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
    // ── Phase 4: Sprite & Game Dev Tools ──

    /// Pack layers by ID into a sprite sheet atlas. Returns RGBA buffer of the atlas.
    pub fn pack_sprites(
        &self,
        layer_ids: &[u32],
        padding: u32,
        max_size: u32,
        power_of_two: bool,
    ) -> Vec<u8> {
        let buffers = self.collect_layer_buffers(layer_ids);
        let refs: Vec<&ImageBuffer> = buffers.iter().collect();
        let sheet = sprite::pack_sprites(&refs, padding, max_size, power_of_two);
        sheet.buffer.data
    }

    /// Pack layers by ID into a sprite sheet. Returns JSON metadata.
    pub fn pack_sprites_json(
        &self,
        layer_ids: &[u32],
        padding: u32,
        max_size: u32,
        power_of_two: bool,
    ) -> String {
        let buffers = self.collect_layer_buffers(layer_ids);
        let refs: Vec<&ImageBuffer> = buffers.iter().collect();
        let sheet = sprite::pack_sprites(&refs, padding, max_size, power_of_two);

        let sprites_json: Vec<String> = sheet
            .sprites
            .iter()
            .map(|s| {
                format!(
                    r#"{{"index":{},"x":{},"y":{},"w":{},"h":{}}}"#,
                    s.index, s.x, s.y, s.width, s.height
                )
            })
            .collect();
        format!(
            r#"{{"sprites":[{}],"width":{},"height":{}}}"#,
            sprites_json.join(","),
            sheet.width,
            sheet.height
        )
    }

    /// Render a contact sheet from layers. Returns RGBA buffer.
    pub fn contact_sheet(
        &self,
        layer_ids: &[u32],
        columns: u32,
        cell_w: u32,
        cell_h: u32,
        padding: u32,
    ) -> Vec<u8> {
        let buffers = self.collect_layer_buffers(layer_ids);
        let refs: Vec<&ImageBuffer> = buffers.iter().collect();
        let result = sprite::contact_sheet(
            &refs,
            &sprite::ContactSheetOptions::new(columns, cell_w, cell_h, padding, Rgba::TRANSPARENT),
        );
        result.data
    }

    /// Scale a layer in-place by an integer factor using nearest-neighbor.
    pub fn pixel_scale_layer(&mut self, id: u32, factor: u32) {
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = sprite::pixel_scale(&img.buffer, factor);
            }
        }
    }

    /// Extract a palette from a layer. Returns flat [r,g,b,a, r,g,b,a, ...].
    pub fn extract_palette(&self, id: u32, max_colors: u32) -> Vec<u8> {
        match self.inner.find_layer(id) {
            Some(layer) => {
                let buf = match &layer.kind {
                    LayerKind::Image(img) => &img.buffer,
                    LayerKind::Paint(paint) => &paint.buffer,
                    _ => return Vec::new(),
                };
                let palette = sprite::extract_palette(buf, max_colors);
                palette_to_flat(&palette)
            }
            None => Vec::new(),
        }
    }

    /// Quantize a layer in-place to the given palette colors.
    /// `palette_colors` is flat [r,g,b,a, r,g,b,a, ...].
    pub fn quantize_layer(&mut self, id: u32, palette_colors: &[u8]) {
        let palette = flat_to_palette(palette_colors);
        if let Some(layer) = self.inner.find_layer_mut(id) {
            if let LayerKind::Image(img) = &mut layer.kind {
                img.buffer = sprite::quantize(&img.buffer, &palette);
            }
        }
    }

    // ── Phase 5: Format Import/Export ──

    /// Decode a JPEG and add it as a top-level image layer. Returns the layer ID.
    pub fn import_jpeg(&mut self, name: &str, jpeg_bytes: &[u8], x: i32, y: i32) -> u32 {
        let buf = codec::decode_jpeg(jpeg_bytes).expect("failed to decode JPEG");
        self.add_decoded_layer(name, buf, x, y)
    }

    /// Decode a WebP and add it as a top-level image layer. Returns the layer ID.
    pub fn import_webp(&mut self, name: &str, webp_bytes: &[u8], x: i32, y: i32) -> u32 {
        let buf = codec::decode_webp(webp_bytes).expect("failed to decode WebP");
        self.add_decoded_layer(name, buf, x, y)
    }

    /// Decode a GIF and add each frame as a separate layer. Returns layer IDs.
    pub fn import_gif_frames(&mut self, gif_bytes: &[u8]) -> Vec<u32> {
        let frames = codec::decode_gif(gif_bytes).expect("failed to decode GIF");
        let mut ids = Vec::with_capacity(frames.len());
        for (i, frame) in frames.into_iter().enumerate() {
            let name = format!("frame_{}", i);
            let id = self.add_decoded_layer(&name, frame.buffer, 0, 0);
            ids.push(id);
        }
        ids
    }

    /// Import PSD layers. Returns layer IDs.
    pub fn import_psd(&mut self, psd_bytes: &[u8]) -> Vec<u32> {
        let (_w, _h, layers) = codec::import_psd(psd_bytes).expect("failed to decode PSD");
        let mut ids = Vec::with_capacity(layers.len());
        for psd_layer in layers {
            let id = self.inner.next_id();
            let mut common = LayerCommon::new(id, &psd_layer.name);
            common.x = psd_layer.x;
            common.y = psd_layer.y;
            common.opacity = psd_layer.opacity;
            common.visible = psd_layer.visible;
            self.inner.layers.push(Layer::new(
                common,
                LayerKind::Image(ImageLayerData::new(psd_layer.buffer)),
            ));
            ids.push(id);
        }
        ids
    }

    /// Auto-detect format and import as a layer. Returns the layer ID.
    pub fn import_auto(&mut self, name: &str, bytes: &[u8], x: i32, y: i32) -> u32 {
        let buf = codec::decode_auto(bytes).expect("failed to decode image");
        self.add_decoded_layer(name, buf, x, y)
    }

    /// Render the document and encode as JPEG.
    pub fn export_jpeg(&self, quality: u8) -> Vec<u8> {
        let result = self.inner.render();
        codec::encode_jpeg(&result, quality).expect("failed to encode JPEG")
    }

    /// Render the document and encode as lossless WebP.
    pub fn export_webp(&self) -> Vec<u8> {
        let result = self.inner.render();
        codec::encode_webp(&result).expect("failed to encode WebP")
    }

    // ── Phase 5: Document Serialization ──

    /// Serialize the document to a binary format.
    pub fn serialize(&self) -> Vec<u8> {
        serialize::serialize(&self.inner).expect("failed to serialize")
    }

    /// Deserialize a document from binary data.
    pub fn deserialize(data: &[u8]) -> Document {
        let inner = serialize::deserialize(data).expect("failed to deserialize");
        Document { inner }
    }

    // ── Internal helpers ──

    fn add_decoded_layer(&mut self, name: &str, buf: ImageBuffer, x: i32, y: i32) -> u32 {
        let id = self.inner.next_id();
        let mut common = LayerCommon::new(id, name);
        common.x = x;
        common.y = y;
        self.inner.layers.push(Layer::new(
            common,
            LayerKind::Image(ImageLayerData::new(buf)),
        ));
        id
    }

    fn collect_layer_buffers(&self, layer_ids: &[u32]) -> Vec<ImageBuffer> {
        layer_ids
            .iter()
            .filter_map(|&id| {
                self.inner
                    .find_layer(id)
                    .and_then(|layer| match &layer.kind {
                        LayerKind::Image(img) => Some(img.buffer.clone()),
                        LayerKind::Paint(paint) => Some(paint.buffer.clone()),
                        _ => None,
                    })
            })
            .collect()
    }
}

fn palette_to_flat(palette: &sprite::Palette) -> Vec<u8> {
    let mut out = Vec::with_capacity(palette.colors.len() * 4);
    for c in &palette.colors {
        out.push(c.r);
        out.push(c.g);
        out.push(c.b);
        out.push(c.a);
    }
    out
}

fn flat_to_palette(data: &[u8]) -> sprite::Palette {
    let count = data.len() / 4;
    let mut colors = Vec::with_capacity(count);
    for i in 0..count {
        let idx = i * 4;
        colors.push(Rgba::new(
            data[idx],
            data[idx + 1],
            data[idx + 2],
            data[idx + 3],
        ));
    }
    sprite::Palette::new(colors)
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
    color::rgb_to_hex(color::Rgb::new(r, g, b))
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

// ── Free functions: format detection ──

/// Detect the image format of the given data. Returns a string: "png", "jpeg", "webp", "gif", "psd", or "unknown".
#[wasm_bindgen]
pub fn detect_format(data: &[u8]) -> String {
    codec::detect_format(data).as_str().to_string()
}

/// Auto-detect format, decode to RGBA, return flat [w_u32_be(4 bytes), h_u32_be(4 bytes), rgba...].
#[wasm_bindgen]
pub fn decode_image(data: &[u8]) -> Vec<u8> {
    let buf = codec::decode_auto(data).expect("failed to decode image");
    let mut out = Vec::with_capacity(8 + buf.data.len());
    out.extend_from_slice(&buf.width.to_be_bytes());
    out.extend_from_slice(&buf.height.to_be_bytes());
    out.extend_from_slice(&buf.data);
    out
}

// ── Free functions: sprite utilities ──

/// Extract a palette from raw RGBA data. Returns flat [r,g,b,a, ...].
#[wasm_bindgen]
pub fn extract_palette_from_rgba(data: &[u8], w: u32, h: u32, max_colors: u32) -> Vec<u8> {
    if let Some(buf) = ImageBuffer::from_rgba(w, h, data.to_vec()) {
        let palette = sprite::extract_palette(&buf, max_colors);
        palette_to_flat(&palette)
    } else {
        Vec::new()
    }
}

/// Compute a per-channel histogram for raw RGBA data.
/// Returns a flat array of 1024 u32 values: r[0..256], g[256..512], b[512..768], a[768..1024].
#[wasm_bindgen]
pub fn histogram_rgba(data: &[u8], w: u32, h: u32) -> Vec<u32> {
    let hist = color::histogram(data, w, h);
    let mut out = Vec::with_capacity(1024);
    out.extend_from_slice(&hist.r);
    out.extend_from_slice(&hist.g);
    out.extend_from_slice(&hist.b);
    out.extend_from_slice(&hist.a);
    out
}

/// Quantize raw RGBA data to the given palette. Returns RGBA buffer.
#[wasm_bindgen]
pub fn quantize_rgba(data: &[u8], w: u32, h: u32, palette: &[u8]) -> Vec<u8> {
    if let Some(buf) = ImageBuffer::from_rgba(w, h, data.to_vec()) {
        let pal = flat_to_palette(palette);
        sprite::quantize(&buf, &pal).data
    } else {
        Vec::new()
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
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
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
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
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
    fn histogram_rgba_flat_layout() {
        // 2 red pixels, 1 blue pixel, 1 transparent
        let data: Vec<u8> = vec![
            255, 0, 0, 255, // red opaque
            255, 0, 0, 255, // red opaque
            0, 0, 255, 255, // blue opaque
            0, 0, 0, 0, // transparent
        ];
        let hist = histogram_rgba(&data, 2, 2);
        assert_eq!(hist.len(), 1024);
        // r channel: index 255 → 2 red pixels; index 0 → 2 non-red
        assert_eq!(hist[255], 2); // r[255]
        assert_eq!(hist[0], 2); // r[0]
                                // b channel starts at offset 512: index 512+255 → 1 blue pixel
        assert_eq!(hist[512 + 255], 1);
        // a channel starts at offset 768: 3 opaque (255) and 1 transparent (0)
        assert_eq!(hist[768 + 255], 3);
        assert_eq!(hist[768], 1);
    }

    #[test]
    fn mask_inverted_setter() {
        let mut doc = Document::new(2, 1);
        let id = doc.add_image_layer("img", &[255, 0, 0, 255, 255, 0, 0, 255], 2, 1, 0, 0);
        // White = left visible, black = right hidden
        doc.set_layer_mask(id, &[255, 255, 255, 255, 0, 0, 0, 255], 2, 1);

        // Without inversion: left visible, right hidden
        let result = doc.render();
        assert_eq!(result[3], 255);
        assert_eq!(result[7], 0);

        // Invert: left hidden, right visible
        doc.set_mask_inverted(id, true);
        let result = doc.render();
        assert_eq!(result[3], 0);
        assert_eq!(result[7], 255);
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
        let rgba: Vec<u8> = [255, 0, 0, 255]
            .iter()
            .copied()
            .cycle()
            .take(4 * 4 * 4)
            .collect();
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
        assert_eq!(buf[0], 0); // 255 inverted
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

    // ── Phase 4 tests ──

    #[test]
    fn pack_sprites_wasm() {
        let mut doc = Document::new(32, 32);
        let rgba1: Vec<u8> = [255, 0, 0, 255]
            .iter()
            .copied()
            .cycle()
            .take(8 * 8 * 4)
            .collect();
        let rgba2: Vec<u8> = [0, 255, 0, 255]
            .iter()
            .copied()
            .cycle()
            .take(8 * 8 * 4)
            .collect();
        let id1 = doc.add_image_layer("a", &rgba1, 8, 8, 0, 0);
        let id2 = doc.add_image_layer("b", &rgba2, 8, 8, 0, 0);

        let atlas = doc.pack_sprites(&[id1, id2], 0, 4096, false);
        assert!(!atlas.is_empty());

        let json = doc.pack_sprites_json(&[id1, id2], 0, 4096, false);
        assert!(json.contains("\"sprites\""));
        assert!(json.contains("\"width\""));
    }

    #[test]
    fn contact_sheet_wasm() {
        let mut doc = Document::new(32, 32);
        let rgba: Vec<u8> = [255, 0, 0, 255]
            .iter()
            .copied()
            .cycle()
            .take(8 * 8 * 4)
            .collect();
        let id1 = doc.add_image_layer("a", &rgba, 8, 8, 0, 0);
        let id2 = doc.add_image_layer("b", &rgba, 8, 8, 0, 0);

        let result = doc.contact_sheet(&[id1, id2], 2, 8, 8, 0);
        // 2 columns, 1 row, 8x8 cells, no padding = 16x8 = 512 bytes
        assert_eq!(result.len(), 16 * 8 * 4);
    }

    #[test]
    fn pixel_scale_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        doc.pixel_scale_layer(id, 2);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf.len(), 8 * 8 * 4);
    }

    #[test]
    fn extract_palette_wasm() {
        let (doc, id) = make_red_4x4();
        let palette = doc.extract_palette(id, 4);
        // Should have at least 4 bytes (one RGBA color)
        assert!(palette.len() >= 4);
        assert_eq!(palette[0], 255); // red
        assert_eq!(palette[1], 0);
        assert_eq!(palette[2], 0);
        assert_eq!(palette[3], 255);
    }

    #[test]
    fn quantize_layer_wasm() {
        let (mut doc, id) = make_red_4x4();
        // Quantize to a palette of just green
        let palette = vec![0, 255, 0, 255];
        doc.quantize_layer(id, &palette);
        let buf = doc.get_layer_rgba(id);
        assert_eq!(buf[0], 0); // was red, now green
        assert_eq!(buf[1], 255);
        assert_eq!(buf[2], 0);
    }

    // ── Phase 5 tests ──

    #[test]
    fn import_jpeg_wasm() {
        let mut doc = Document::new(4, 4);
        let mut buf = kimg_core::buffer::ImageBuffer::new_transparent(4, 4);
        buf.fill(Rgba::new(128, 128, 128, 255));
        let jpeg_bytes = kimg_core::codec::encode_jpeg(&buf, 90).unwrap();
        let id = doc.import_jpeg("test", &jpeg_bytes, 0, 0);
        assert!(doc.inner.find_layer(id).is_some());
    }

    #[test]
    fn import_webp_wasm() {
        let mut doc = Document::new(4, 4);
        let mut buf = kimg_core::buffer::ImageBuffer::new_transparent(4, 4);
        buf.fill(Rgba::new(128, 128, 128, 255));
        let webp_bytes = kimg_core::codec::encode_webp(&buf).unwrap();
        let id = doc.import_webp("test", &webp_bytes, 0, 0);
        assert!(doc.inner.find_layer(id).is_some());
    }

    #[test]
    fn export_jpeg_wasm() {
        let mut doc = Document::new(4, 4);
        let rgba: Vec<u8> = [128, 128, 128, 255]
            .iter()
            .copied()
            .cycle()
            .take(4 * 4 * 4)
            .collect();
        doc.add_image_layer("test", &rgba, 4, 4, 0, 0);
        let jpeg = doc.export_jpeg(80);
        assert!(!jpeg.is_empty());
        assert_eq!(jpeg[0], 0xFF); // JPEG magic
        assert_eq!(jpeg[1], 0xD8);
    }

    #[test]
    fn export_webp_wasm() {
        let mut doc = Document::new(4, 4);
        let rgba: Vec<u8> = [128, 128, 128, 255]
            .iter()
            .copied()
            .cycle()
            .take(4 * 4 * 4)
            .collect();
        doc.add_image_layer("test", &rgba, 4, 4, 0, 0);
        let webp = doc.export_webp();
        assert!(!webp.is_empty());
        assert_eq!(&webp[0..4], b"RIFF");
    }

    #[test]
    fn serialize_deserialize_wasm() {
        let mut doc = Document::new(4, 4);
        let rgba: Vec<u8> = [255, 0, 0, 255]
            .iter()
            .copied()
            .cycle()
            .take(4 * 4 * 4)
            .collect();
        doc.add_image_layer("red", &rgba, 4, 4, 0, 0);
        doc.add_solid_color_layer("fill", 0, 255, 0, 255);

        let data = doc.serialize();
        let restored = Document::deserialize(&data);
        assert_eq!(restored.width(), 4);
        assert_eq!(restored.height(), 4);
        assert_eq!(restored.layer_count(), 2);

        let result = restored.render();
        // Green solid layer on top of red image
        assert_eq!(result[0], 0); // green
        assert_eq!(result[1], 255);
    }

    #[test]
    fn detect_format_wasm() {
        assert_eq!(detect_format(&[0x89, 0x50, 0x4E, 0x47, 0, 0, 0, 0]), "png");
        assert_eq!(detect_format(&[0xFF, 0xD8, 0xFF, 0xE0]), "jpeg");
        assert_eq!(detect_format(b"RIFF\x00\x00\x00\x00WEBP"), "webp");
        assert_eq!(detect_format(b"GIF89a"), "gif");
        assert_eq!(detect_format(b"8BPS\x00\x01"), "psd");
        assert_eq!(detect_format(&[0, 0, 0, 0]), "unknown");
    }
}
