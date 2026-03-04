//! The compositing document — canvas, layer tree, and render pipeline.
//!
//! [`Document`] is the top-level container.  It holds a canvas size and an
//! ordered list of [`Layer`](crate::layer) values (bottom-to-top).
//! Call [`Document::render`] to flatten the layer tree into a single
//! [`ImageBuffer`].
//!
//! # Render order and scoping
//!
//! Layers are composited back-to-front.  A [`LayerKind::Filter`] layer applies
//! its adjustments to the composite built so far rather than to a pixel buffer
//! of its own.  Layers inside a [`LayerKind::Group`] are rendered to an
//! isolated buffer first (two-pass: non-filter layers, then filter layers),
//! mirroring Spriteform's `renderSmartVariantScoped` behaviour.

use crate::blend::{blend, blend_normal, BlendMode};
use crate::blit::{blit_transformed, Anchor, BlitParams, Rotation};
use crate::brush::{self, BrushPreset, StrokePoint};
use crate::buffer::ImageBuffer;
use crate::fill;
use crate::filter::{apply_hsl_filter, HslFilterConfig};
use crate::layer::{
    FillKind, FillLayerData, FilterLayerPatch, GradientDirection, GroupLayerData, Layer,
    LayerCommon, LayerKind, LayerPatch, LayerTransform, RasterLayerData, ShapeLayerData,
    SvgLayerData,
};
use crate::pixel::Rgba;
use crate::shape::render_shape;
use crate::svg::rasterize_svg;
use crate::text::render_text;
use crate::transform;

/// Location of a layer within the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerLocation {
    /// Parent group ID, or `None` for top-level layers.
    pub parent_id: Option<u32>,
    /// Zero-based index inside the parent container.
    pub index: usize,
    /// Zero-based depth in the tree.
    pub depth: usize,
}

/// A compositing document with a canvas size and a layer tree.
#[derive(Debug, Clone)]
pub struct Document {
    /// Canvas width in pixels.
    pub width: u32,
    /// Canvas height in pixels.
    pub height: u32,
    /// The ordered tree of layers (bottom to top).
    pub layers: Vec<Layer>,
    next_id: u32,
}

impl Document {
    /// Create a new empty document with the given canvas dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use kimg_core::document::Document;
    ///
    /// let doc = Document::new(512, 512);
    /// assert_eq!(doc.width, 512);
    /// assert!(doc.layers.is_empty());
    /// ```
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            layers: Vec::new(),
            next_id: 1,
        }
    }

    /// Allocate a new unique layer ID.
    pub fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn next_available_id(&self) -> u32 {
        self.next_id
    }

    pub(crate) fn set_next_available_id(&mut self, next_id: u32) {
        self.next_id = next_id.max(1);
    }

    /// Find a layer by ID (immutable), searching recursively through groups.
    ///
    /// Returns `None` if no layer with the given ID exists in the tree.
    pub fn find_layer(&self, id: u32) -> Option<&Layer> {
        fn search(layers: &[Layer], id: u32) -> Option<&Layer> {
            for layer in layers {
                if layer.common.id == id {
                    return Some(layer);
                }
                if let LayerKind::Group(g) = &layer.kind {
                    if let Some(found) = search(&g.children, id) {
                        return Some(found);
                    }
                }
            }
            None
        }
        search(&self.layers, id)
    }

    /// Find a layer by ID (mutable), searching recursively through groups.
    ///
    /// Returns `None` if no layer with the given ID exists in the tree.
    pub fn find_layer_mut(&mut self, id: u32) -> Option<&mut Layer> {
        fn search(layers: &mut [Layer], id: u32) -> Option<&mut Layer> {
            for layer in layers {
                if layer.common.id == id {
                    return Some(layer);
                }
                if let LayerKind::Group(g) = &mut layer.kind {
                    if let Some(found) = search(&mut g.children, id) {
                        return Some(found);
                    }
                }
            }
            None
        }
        search(&mut self.layers, id)
    }

    /// Add a child layer to a group. Returns the child's ID on success.
    ///
    /// # Errors
    ///
    /// Returns `Err("group not found")` if `group_id` does not exist, or
    /// `Err("layer is not a group")` if the identified layer is not a `Group`.
    pub fn add_child_to_group(&mut self, group_id: u32, child: Layer) -> Result<u32, &'static str> {
        let child_id = child.common.id;
        let layer = self.find_layer_mut(group_id).ok_or("group not found")?;
        match &mut layer.kind {
            LayerKind::Group(g) => {
                g.children.push(child);
                Ok(child_id)
            }
            _ => Err("layer is not a group"),
        }
    }

    /// Remove a child from a group by child ID.
    ///
    /// Returns `true` if the child was found and removed, `false` if the group or
    /// child was not found.
    pub fn remove_child_from_group(&mut self, group_id: u32, child_id: u32) -> bool {
        let layer = match self.find_layer_mut(group_id) {
            Some(l) => l,
            None => return false,
        };
        match &mut layer.kind {
            LayerKind::Group(g) => {
                let before = g.children.len();
                g.children.retain(|c| c.common.id != child_id);
                g.children.len() < before
            }
            _ => false,
        }
    }

    /// Remove a layer from the tree by ID.
    ///
    /// Returns `true` if the layer was found and removed.
    pub fn remove_layer(&mut self, id: u32) -> bool {
        take_layer(&mut self.layers, id).is_some()
    }

    /// Return a layer's location within the tree.
    pub fn layer_location(&self, id: u32) -> Option<LayerLocation> {
        find_layer_location(&self.layers, id, None, 0)
    }

    /// Move a layer to a new parent/index.
    ///
    /// `target_parent_id = None` moves the layer to the top level. `target_index = None`
    /// appends at the end of the destination container.
    pub fn move_layer(
        &mut self,
        id: u32,
        target_parent_id: Option<u32>,
        target_index: Option<usize>,
    ) -> bool {
        if target_parent_id == Some(id) {
            return false;
        }

        let source_location = match self.layer_location(id) {
            Some(location) => location,
            None => return false,
        };

        if let Some(parent_id) = target_parent_id {
            let Some(parent_layer) = self.find_layer(parent_id) else {
                return false;
            };

            if !matches!(parent_layer.kind, LayerKind::Group(_)) {
                return false;
            }

            let Some(layer) = self.find_layer(id) else {
                return false;
            };

            if contains_layer_id(layer, parent_id) {
                return false;
            }
        }

        let same_parent = source_location.parent_id == target_parent_id;
        let mut target_index = target_index;
        if same_parent {
            if let Some(index) = target_index {
                if index > source_location.index {
                    target_index = Some(index - 1);
                }
            }
        }

        let Some(layer) = take_layer(&mut self.layers, id) else {
            return false;
        };

        insert_layer_at(&mut self.layers, target_parent_id, target_index, layer)
    }

    /// Resize the document canvas.
    pub fn resize_canvas(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Apply a patch to a layer.
    ///
    /// Returns `false` if the layer does not exist.
    pub fn update_layer(&mut self, id: u32, patch: &LayerPatch) -> bool {
        let Some(layer) = self.find_layer_mut(id) else {
            return false;
        };

        if let Some(name) = &patch.name {
            layer.common.name = name.clone();
        }
        if let Some(visible) = patch.visible {
            layer.common.visible = visible;
        }
        if let Some(opacity) = patch.opacity {
            layer.common.opacity = opacity.clamp(0.0, 1.0);
        }
        if let Some(x) = patch.x {
            layer.common.x = x;
        }
        if let Some(y) = patch.y {
            layer.common.y = y;
        }
        if let Some(blend_mode) = patch.blend_mode {
            layer.common.blend_mode = blend_mode;
        }
        if let Some(mask_inverted) = patch.mask_inverted {
            layer.common.mask_inverted = mask_inverted;
        }
        if let Some(clip_to_below) = patch.clip_to_below {
            layer.common.clip_to_below = clip_to_below;
        }
        if let Some(alpha_locked) = patch.alpha_locked {
            if let LayerKind::Raster(raster) = &mut layer.kind {
                raster.alpha_locked = alpha_locked;
            }
        }

        if let Some(anchor) = patch.anchor {
            if let Some(transform) = layer_transform_mut(layer) {
                transform.anchor = anchor;
            }
        }

        if let Some(transform) = layer_transform_mut(layer) {
            if let Some(flip_x) = patch.flip_x {
                transform.flip_x = flip_x;
            }
            if let Some(flip_y) = patch.flip_y {
                transform.flip_y = flip_y;
            }
            if let Some(rotation) = patch.rotation {
                transform.rotation_deg = rotation;
            }
            if let Some(scale_x) = patch.scale_x {
                transform.scale_x = scale_x.abs().max(f64::EPSILON);
            }
            if let Some(scale_y) = patch.scale_y {
                transform.scale_y = scale_y.abs().max(f64::EPSILON);
            }
        }

        if let (LayerKind::Filter(filter), Some(filter_patch)) = (&mut layer.kind, &patch.filter) {
            apply_filter_patch(&mut filter.config, filter_patch);
        }

        if let (LayerKind::Text(text), Some(text_patch)) = (&mut layer.kind, &patch.text) {
            if let Some(content) = &text_patch.text {
                text.text = content.clone();
            }
            if let Some(color) = text_patch.color {
                text.color = color;
            }
            if let Some(font_family) = &text_patch.font_family {
                text.font_family = font_family.clone();
            }
            if let Some(font_weight) = text_patch.font_weight {
                text.font_weight = font_weight.max(1);
            }
            if let Some(font_style) = text_patch.font_style {
                text.font_style = font_style;
            }
            if let Some(font_size) = text_patch.font_size {
                text.font_size = font_size.max(1);
            }
            if let Some(line_height) = text_patch.line_height {
                text.line_height = line_height.max(1);
            }
            if let Some(letter_spacing) = text_patch.letter_spacing {
                text.letter_spacing = letter_spacing;
            }
            if let Some(align) = text_patch.align {
                text.align = align;
            }
            if let Some(wrap) = text_patch.wrap {
                text.wrap = wrap;
            }
            if let Some(box_width) = text_patch.box_width {
                text.box_width = box_width.map(|width| width.max(1));
            }
        }

        true
    }

    /// Flatten a group layer into a single raster layer in-place.
    ///
    /// The group is rendered to a canvas-sized buffer, then replaced with an
    /// `Raster` layer that preserves the group's common properties (id, name,
    /// opacity, blend mode, etc.).  The position is reset to (0, 0) because the
    /// rendered buffer is already in canvas coordinates.
    ///
    /// Returns `true` on success, `false` if the layer was not found or is not a group.
    pub fn flatten_group(&mut self, group_id: u32) -> bool {
        // First render the group to a buffer
        let layer = match self.find_layer(group_id) {
            Some(l) => l,
            None => return false,
        };
        let group = match &layer.kind {
            LayerKind::Group(g) => g,
            _ => return false,
        };

        let mut buf = ImageBuffer::new_transparent(self.width, self.height);
        render_group(group, &mut buf, 1.0);

        // Now replace the layer kind
        let layer = self.find_layer_mut(group_id).unwrap();
        layer.kind = LayerKind::Raster(RasterLayerData::new(buf));
        // Reset position since the buffer is already at canvas coordinates
        layer.common.x = 0;
        layer.common.y = 0;
        true
    }

    /// Bucket-fill a raster layer using layer-local coordinates.
    ///
    /// Matching is alpha-aware and uses per-channel tolerance across RGBA.
    /// Returns `false` when the layer does not exist, is not a pixel layer, or
    /// the start point is out of bounds.
    pub fn bucket_fill_layer(
        &mut self,
        id: u32,
        x: u32,
        y: u32,
        color: Rgba,
        contiguous: bool,
        tolerance: u8,
    ) -> bool {
        let layer = match self.find_layer_mut(id) {
            Some(layer) => layer,
            None => return false,
        };

        match &mut layer.kind {
            LayerKind::Raster(raster) => {
                let alpha_locked = raster.alpha_locked;
                raster.mutate_buffer(|buffer| {
                    fill::bucket_fill(buffer, x, y, color, contiguous, tolerance, alpha_locked)
                })
            }
            _ => false,
        }
    }

    /// Paint a batched brush stroke into a raster layer using layer-local coordinates.
    ///
    /// Returns `false` when the layer does not exist, is not a raster layer, or
    /// the stroke produced no visible changes.
    pub fn paint_stroke_layer(
        &mut self,
        id: u32,
        preset: &BrushPreset,
        points: &[StrokePoint],
    ) -> bool {
        let layer = match self.find_layer_mut(id) {
            Some(layer) => layer,
            None => return false,
        };

        match &mut layer.kind {
            LayerKind::Raster(raster) => {
                let alpha_locked = raster.alpha_locked;
                raster.mutate_buffer(|buffer| {
                    brush::paint_stroke(buffer, preset, points, alpha_locked)
                })
            }
            _ => false,
        }
    }

    /// Render the full document to a flat RGBA8 buffer.
    ///
    /// Traverses layers bottom-to-top (last element in `layers` = topmost).
    /// Returns a canvas-sized [`ImageBuffer`] with all visible layers composited.
    pub fn render(&self) -> ImageBuffer {
        let mut output = ImageBuffer::new_transparent(self.width, self.height);
        render_layers(&self.layers, &mut output);
        output
    }

    /// Rasterize an SVG layer into a raster layer in-place.
    ///
    /// The resulting raster layer keeps the SVG layer's common properties and
    /// non-destructive transform state.
    pub fn rasterize_svg_layer(&mut self, id: u32) -> bool {
        let Some(layer) = self.find_layer_mut(id) else {
            return false;
        };

        let (buffer, transform) = match &layer.kind {
            LayerKind::Svg(svg) => (render_svg_layer(svg, svg.width, svg.height), svg.transform),
            _ => return false,
        };

        layer.kind = LayerKind::Raster(RasterLayerData::with_transform(buffer, transform));
        true
    }
}

/// Render a fill layer to a full-canvas buffer.
fn render_fill(fill: &FillLayerData, width: u32, height: u32) -> ImageBuffer {
    match &fill.kind {
        FillKind::Solid { color } => {
            let mut buf = ImageBuffer::new_transparent(width, height);
            buf.fill(*color);
            buf
        }
        FillKind::Gradient { stops, direction } => {
            render_gradient(stops, *direction, width, height)
        }
    }
}

/// Render a gradient to a full-canvas buffer.
fn render_gradient(
    stops: &[crate::layer::GradientStop],
    direction: GradientDirection,
    width: u32,
    height: u32,
) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(width, height);
    if stops.is_empty() || width == 0 || height == 0 {
        return buf;
    }

    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let t = match direction {
                GradientDirection::Horizontal => x as f64 / (w - 1).max(1) as f64,
                GradientDirection::Vertical => y as f64 / (h - 1).max(1) as f64,
                GradientDirection::DiagonalDown => {
                    (x as f64 / (w - 1).max(1) as f64 + y as f64 / (h - 1).max(1) as f64) / 2.0
                }
                GradientDirection::DiagonalUp => {
                    (x as f64 / (w - 1).max(1) as f64 + (1.0 - y as f64 / (h - 1).max(1) as f64))
                        / 2.0
                }
            };

            let color = sample_gradient(stops, t);
            let i = (y * w + x) * 4;
            buf.data[i] = color.r;
            buf.data[i + 1] = color.g;
            buf.data[i + 2] = color.b;
            buf.data[i + 3] = color.a;
        }
    }

    buf
}

/// Render a shape layer to its local buffer.
fn render_shape_layer(shape: &ShapeLayerData) -> ImageBuffer {
    render_shape(shape)
}

fn render_svg_layer(svg: &SvgLayerData, width: u32, height: u32) -> ImageBuffer {
    rasterize_svg(&svg.source, width.max(1), height.max(1))
        .unwrap_or_else(|_| ImageBuffer::new_transparent(width.max(1), height.max(1)))
}

fn scaled_dimension(base: u32, scale: f64) -> u32 {
    ((base as f64) * scale.abs()).round().max(1.0) as u32
}

fn svg_transform_without_scale(transform: LayerTransform) -> LayerTransform {
    LayerTransform {
        scale_x: 1.0,
        scale_y: 1.0,
        ..transform
    }
}

fn layer_transform_mut(layer: &mut Layer) -> Option<&mut LayerTransform> {
    match &mut layer.kind {
        LayerKind::Raster(raster) => Some(&mut raster.transform),
        LayerKind::Shape(shape) => Some(&mut shape.transform),
        LayerKind::Text(text) => Some(&mut text.transform),
        LayerKind::Svg(svg) => Some(&mut svg.transform),
        _ => None,
    }
}

fn flip_buffer(src: &ImageBuffer, flip_x: bool, flip_y: bool) -> ImageBuffer {
    if !flip_x && !flip_y {
        return src.clone();
    }

    let mut dst = ImageBuffer::new_transparent(src.width, src.height);
    let src_w = src.width as usize;
    let dst_w = dst.width as usize;
    for y in 0..src.height as usize {
        let sy = if flip_y {
            src.height as usize - 1 - y
        } else {
            y
        };
        for x in 0..src.width as usize {
            let sx = if flip_x {
                src.width as usize - 1 - x
            } else {
                x
            };
            let si = (sy * src_w + sx) * 4;
            let di = (y * dst_w + x) * 4;
            dst.data[di..di + 4].copy_from_slice(&src.data[si..si + 4]);
        }
    }
    dst
}

fn blit_pretransformed_raster_layer(
    target: &mut ImageBuffer,
    source: &ImageBuffer,
    common: &LayerCommon,
    anchor: Anchor,
) {
    blit_transformed(
        target,
        source,
        &BlitParams {
            dx: common.x,
            dy: common.y,
            anchor,
            flip_x: false,
            flip_y: false,
            rotation: Rotation::None,
            opacity: common.opacity,
        },
    );
}

fn apply_non_destructive_transform(
    src: &ImageBuffer,
    transform_data: &LayerTransform,
) -> ImageBuffer {
    let mut current = flip_buffer(src, transform_data.flip_x, transform_data.flip_y);

    let scaled_width = ((current.width as f64) * transform_data.scale_x.abs())
        .round()
        .max(1.0) as u32;
    let scaled_height = ((current.height as f64) * transform_data.scale_y.abs())
        .round()
        .max(1.0) as u32;
    if scaled_width != current.width || scaled_height != current.height {
        current = transform::resize_bilinear(&current, scaled_width, scaled_height);
    }

    if transform_data.rotation_deg.abs() > f64::EPSILON {
        current = transform::rotate_bilinear(&current, transform_data.rotation_deg);
    }

    current
}

/// Sample a gradient at position t (0..1) using sorted stops.
fn sample_gradient(stops: &[crate::layer::GradientStop], t: f64) -> Rgba {
    if stops.len() == 1 {
        return stops[0].color;
    }
    let t = t.clamp(0.0, 1.0);

    // Find the two stops surrounding t
    let mut left = &stops[0];
    let mut right = &stops[stops.len() - 1];
    for i in 0..stops.len() - 1 {
        if t >= stops[i].position && t <= stops[i + 1].position {
            left = &stops[i];
            right = &stops[i + 1];
            break;
        }
    }

    let span = right.position - left.position;
    if span <= 0.0 {
        return left.color;
    }
    let f = (t - left.position) / span;
    let inv = 1.0 - f;

    Rgba::new(
        (left.color.r as f64 * inv + right.color.r as f64 * f + 0.5) as u8,
        (left.color.g as f64 * inv + right.color.g as f64 * f + 0.5) as u8,
        (left.color.b as f64 * inv + right.color.b as f64 * f + 0.5) as u8,
        (left.color.a as f64 * inv + right.color.a as f64 * f + 0.5) as u8,
    )
}

fn apply_filter_patch(config: &mut HslFilterConfig, patch: &FilterLayerPatch) {
    if let Some(hue_deg) = patch.hue_deg {
        config.hue_deg = hue_deg;
    }
    if let Some(saturation) = patch.saturation {
        config.saturation = saturation;
    }
    if let Some(lightness) = patch.lightness {
        config.lightness = lightness;
    }
    if let Some(alpha) = patch.alpha {
        config.alpha = alpha;
    }
    if let Some(brightness) = patch.brightness {
        config.brightness = brightness;
    }
    if let Some(contrast) = patch.contrast {
        config.contrast = contrast;
    }
    if let Some(temperature) = patch.temperature {
        config.temperature = temperature;
    }
    if let Some(tint) = patch.tint {
        config.tint = tint;
    }
    if let Some(sharpen) = patch.sharpen {
        config.sharpen = sharpen;
    }
}

fn scale_alpha(buf: &mut ImageBuffer, opacity: f64) {
    if opacity >= 1.0 {
        return;
    }
    let scale = (opacity.clamp(0.0, 1.0) * 255.0).round() as u16;
    for chunk in buf.data.chunks_exact_mut(4) {
        chunk[3] = ((chunk[3] as u16 * scale + 127) / 255) as u8;
    }
}

fn refresh_alpha_snapshot(snapshot: &mut [u8], buffer: &ImageBuffer) {
    for (alpha, pixel) in snapshot.iter_mut().zip(buffer.data.chunks_exact(4)) {
        *alpha = pixel[3];
    }
}

fn capture_pretransformed_alpha_snapshot(
    snapshot: &mut [u8],
    canvas_w: u32,
    canvas_h: u32,
    source: &ImageBuffer,
    common: &LayerCommon,
    anchor: Anchor,
) {
    snapshot.fill(0);

    let src_w = source.width as i32;
    let src_h = source.height as i32;
    let (x0, y0) = match anchor {
        Anchor::Center => (
            (common.x as f64 - src_w as f64 / 2.0).round() as i32,
            (common.y as f64 - src_h as f64 / 2.0).round() as i32,
        ),
        Anchor::TopLeft => (common.x, common.y),
    };

    let opacity = (common.opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    if opacity == 0 {
        return;
    }

    let dst_w = canvas_w as i32;
    let dst_h = canvas_h as i32;

    for sy in 0..src_h {
        for sx in 0..src_w {
            let si = ((sy * src_w + sx) * 4) as usize;
            let sa = ((source.data[si + 3] as u32) * opacity + 127) / 255;
            if sa == 0 {
                continue;
            }

            let dx = x0 + sx;
            let dy = y0 + sy;
            if dx < 0 || dy < 0 || dx >= dst_w || dy >= dst_h {
                continue;
            }

            let di = (dy * dst_w + dx) as usize;
            snapshot[di] = sa as u8;
        }
    }
}

fn can_render_layer_direct(layer: &Layer) -> bool {
    layer.common.mask.is_none()
        && !layer.common.clip_to_below
        && layer.common.blend_mode == BlendMode::Normal
        && matches!(
            layer.kind,
            LayerKind::Raster(_) | LayerKind::Shape(_) | LayerKind::Text(_) | LayerKind::Svg(_)
        )
}

fn render_layer_direct(layer: &Layer, output: &mut ImageBuffer) -> bool {
    if !can_render_layer_direct(layer) {
        return false;
    }

    match &layer.kind {
        LayerKind::Raster(raster) => {
            raster.with_cached_transformed_raster(
                || apply_non_destructive_transform(&raster.buffer, &raster.transform),
                |transformed| {
                    blit_pretransformed_raster_layer(
                        output,
                        transformed,
                        &layer.common,
                        raster.transform.anchor,
                    );
                },
            );
            true
        }
        LayerKind::Shape(shape) => {
            shape.with_cached_transformed_raster(
                || {
                    let shape_buf = render_shape_layer(shape);
                    apply_non_destructive_transform(&shape_buf, &shape.transform)
                },
                |shape_buf| {
                    blit_pretransformed_raster_layer(
                        output,
                        shape_buf,
                        &layer.common,
                        shape.transform.anchor,
                    );
                },
            );
            true
        }
        LayerKind::Text(text) => {
            text.with_cached_transformed_raster(
                || {
                    let text_buf = text.cached_local_raster(|| render_text(text));
                    apply_non_destructive_transform(&text_buf, &text.transform)
                },
                |text_buf| {
                    blit_pretransformed_raster_layer(
                        output,
                        text_buf,
                        &layer.common,
                        text.transform.anchor,
                    );
                },
            );
            true
        }
        LayerKind::Svg(svg) => {
            svg.with_cached_transformed_raster(
                || {
                    let raster_width = scaled_dimension(svg.width, svg.transform.scale_x);
                    let raster_height = scaled_dimension(svg.height, svg.transform.scale_y);
                    let svg_buf = svg.cached_local_raster(raster_width, raster_height, || {
                        render_svg_layer(svg, raster_width, raster_height)
                    });
                    apply_non_destructive_transform(
                        &svg_buf,
                        &svg_transform_without_scale(svg.transform),
                    )
                },
                |svg_buf| {
                    blit_pretransformed_raster_layer(
                        output,
                        svg_buf,
                        &layer.common,
                        svg.transform.anchor,
                    );
                },
            );
            true
        }
        _ => false,
    }
}

fn capture_direct_layer_alpha(
    layer: &Layer,
    canvas_w: u32,
    canvas_h: u32,
    snapshot: &mut [u8],
) -> bool {
    if !can_render_layer_direct(layer) {
        return false;
    }

    match &layer.kind {
        LayerKind::Raster(raster) => {
            raster.with_cached_transformed_raster(
                || apply_non_destructive_transform(&raster.buffer, &raster.transform),
                |transformed| {
                    capture_pretransformed_alpha_snapshot(
                        snapshot,
                        canvas_w,
                        canvas_h,
                        transformed,
                        &layer.common,
                        raster.transform.anchor,
                    );
                },
            );
            true
        }
        LayerKind::Shape(shape) => {
            shape.with_cached_transformed_raster(
                || {
                    let shape_buf = render_shape_layer(shape);
                    apply_non_destructive_transform(&shape_buf, &shape.transform)
                },
                |transformed| {
                    capture_pretransformed_alpha_snapshot(
                        snapshot,
                        canvas_w,
                        canvas_h,
                        transformed,
                        &layer.common,
                        shape.transform.anchor,
                    );
                },
            );
            true
        }
        LayerKind::Text(text) => {
            text.with_cached_transformed_raster(
                || {
                    let text_buf = text.cached_local_raster(|| render_text(text));
                    apply_non_destructive_transform(&text_buf, &text.transform)
                },
                |transformed| {
                    capture_pretransformed_alpha_snapshot(
                        snapshot,
                        canvas_w,
                        canvas_h,
                        transformed,
                        &layer.common,
                        text.transform.anchor,
                    );
                },
            );
            true
        }
        LayerKind::Svg(svg) => {
            svg.with_cached_transformed_raster(
                || {
                    let raster_width = scaled_dimension(svg.width, svg.transform.scale_x);
                    let raster_height = scaled_dimension(svg.height, svg.transform.scale_y);
                    let svg_buf = svg.cached_local_raster(raster_width, raster_height, || {
                        render_svg_layer(svg, raster_width, raster_height)
                    });
                    apply_non_destructive_transform(
                        &svg_buf,
                        &svg_transform_without_scale(svg.transform),
                    )
                },
                |transformed| {
                    capture_pretransformed_alpha_snapshot(
                        snapshot,
                        canvas_w,
                        canvas_h,
                        transformed,
                        &layer.common,
                        svg.transform.anchor,
                    );
                },
            );
            true
        }
        _ => false,
    }
}

fn contains_layer_id(layer: &Layer, id: u32) -> bool {
    if layer.common.id == id {
        return true;
    }

    match &layer.kind {
        LayerKind::Group(group) => group
            .children
            .iter()
            .any(|child| contains_layer_id(child, id)),
        _ => false,
    }
}

fn find_layer_location(
    layers: &[Layer],
    id: u32,
    parent_id: Option<u32>,
    depth: usize,
) -> Option<LayerLocation> {
    for (index, layer) in layers.iter().enumerate() {
        if layer.common.id == id {
            return Some(LayerLocation {
                parent_id,
                index,
                depth,
            });
        }

        if let LayerKind::Group(group) = &layer.kind {
            if let Some(location) =
                find_layer_location(&group.children, id, Some(layer.common.id), depth + 1)
            {
                return Some(location);
            }
        }
    }

    None
}

fn take_layer(layers: &mut Vec<Layer>, id: u32) -> Option<Layer> {
    if let Some(index) = layers.iter().position(|layer| layer.common.id == id) {
        return Some(layers.remove(index));
    }

    for layer in layers {
        if let LayerKind::Group(group) = &mut layer.kind {
            if let Some(found) = take_layer(&mut group.children, id) {
                return Some(found);
            }
        }
    }

    None
}

fn group_children_mut(layers: &mut Vec<Layer>, group_id: u32) -> Option<&mut Vec<Layer>> {
    for layer in layers {
        if layer.common.id == group_id {
            return match &mut layer.kind {
                LayerKind::Group(group) => Some(&mut group.children),
                _ => None,
            };
        }

        if let LayerKind::Group(group) = &mut layer.kind {
            if let Some(children) = group_children_mut(&mut group.children, group_id) {
                return Some(children);
            }
        }
    }

    None
}

fn insert_into_container(container: &mut Vec<Layer>, index: Option<usize>, layer: Layer) {
    let insert_index = index.unwrap_or(container.len()).min(container.len());
    container.insert(insert_index, layer);
}

fn insert_layer_at(
    layers: &mut Vec<Layer>,
    target_parent_id: Option<u32>,
    target_index: Option<usize>,
    layer: Layer,
) -> bool {
    match target_parent_id {
        Some(parent_id) => match group_children_mut(layers, parent_id) {
            Some(children) => {
                insert_into_container(children, target_index, layer);
                true
            }
            None => false,
        },
        None => {
            insert_into_container(layers, target_index, layer);
            true
        }
    }
}

/// Apply a layer mask to a rendered layer buffer. The mask's luminance
/// (using only the first channel as grayscale) multiplies the alpha channel.
/// When `inverted` is true the mask value is flipped before application.
fn apply_mask(buf: &mut ImageBuffer, mask: &ImageBuffer, inverted: bool) {
    let w = buf.width.min(mask.width) as usize;
    let h = buf.height.min(mask.height) as usize;
    let buf_stride = buf.width as usize;
    let mask_stride = mask.width as usize;

    for y in 0..h {
        for x in 0..w {
            let bi = (y * buf_stride + x) * 4;
            let mi = (y * mask_stride + x) * 4;
            let mut mask_val = mask.data[mi];
            if inverted {
                mask_val = 255 - mask_val;
            }
            buf.data[bi + 3] = ((buf.data[bi + 3] as u16 * mask_val as u16 + 127) / 255) as u8;
        }
    }
}

/// Clip `layer_buf` to the alpha of `below_buf`. Where below has alpha=0,
/// the layer becomes transparent.
fn apply_clipping_mask(layer_buf: &mut ImageBuffer, below_alpha: &[u8]) {
    for (pixel, &alpha) in layer_buf.data.chunks_exact_mut(4).zip(below_alpha.iter()) {
        pixel[3] = ((pixel[3] as u16 * alpha as u16 + 127) / 255) as u8;
    }
}

/// Render a single layer to an isolated canvas-sized RGBA buffer.
fn render_layer_to_buffer(layer: &Layer, canvas_w: u32, canvas_h: u32) -> ImageBuffer {
    match &layer.kind {
        LayerKind::Raster(raster) => {
            let mut buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
            raster.with_cached_transformed_raster(
                || apply_non_destructive_transform(&raster.buffer, &raster.transform),
                |transformed| {
                    blit_pretransformed_raster_layer(
                        &mut buf,
                        transformed,
                        &layer.common,
                        raster.transform.anchor,
                    );
                },
            );
            if let Some(mask) = &layer.common.mask {
                apply_mask(&mut buf, mask, layer.common.mask_inverted);
            }
            buf
        }
        LayerKind::Fill(fill) => {
            let mut fill = render_fill(fill, canvas_w, canvas_h);
            scale_alpha(&mut fill, layer.common.opacity);
            if let Some(mask) = &layer.common.mask {
                apply_mask(&mut fill, mask, layer.common.mask_inverted);
            }
            fill
        }
        LayerKind::Shape(shape) => {
            let mut buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
            shape.with_cached_transformed_raster(
                || {
                    let shape_buf = render_shape_layer(shape);
                    apply_non_destructive_transform(&shape_buf, &shape.transform)
                },
                |shape_buf| {
                    blit_pretransformed_raster_layer(
                        &mut buf,
                        shape_buf,
                        &layer.common,
                        shape.transform.anchor,
                    );
                },
            );
            if let Some(mask) = &layer.common.mask {
                apply_mask(&mut buf, mask, layer.common.mask_inverted);
            }
            buf
        }
        LayerKind::Text(text) => {
            let mut buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
            text.with_cached_transformed_raster(
                || {
                    let text_buf = text.cached_local_raster(|| render_text(text));
                    apply_non_destructive_transform(&text_buf, &text.transform)
                },
                |transformed| {
                    blit_pretransformed_raster_layer(
                        &mut buf,
                        transformed,
                        &layer.common,
                        text.transform.anchor,
                    );
                },
            );
            if let Some(mask) = &layer.common.mask {
                apply_mask(&mut buf, mask, layer.common.mask_inverted);
            }
            buf
        }
        LayerKind::Svg(svg) => {
            let mut buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
            svg.with_cached_transformed_raster(
                || {
                    let raster_width = scaled_dimension(svg.width, svg.transform.scale_x);
                    let raster_height = scaled_dimension(svg.height, svg.transform.scale_y);
                    let svg_buf = svg.cached_local_raster(raster_width, raster_height, || {
                        render_svg_layer(svg, raster_width, raster_height)
                    });
                    apply_non_destructive_transform(
                        &svg_buf,
                        &svg_transform_without_scale(svg.transform),
                    )
                },
                |transformed| {
                    blit_pretransformed_raster_layer(
                        &mut buf,
                        transformed,
                        &layer.common,
                        svg.transform.anchor,
                    );
                },
            );
            if let Some(mask) = &layer.common.mask {
                apply_mask(&mut buf, mask, layer.common.mask_inverted);
            }
            buf
        }
        LayerKind::Group(group) => {
            let mut buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
            render_group(group, &mut buf, layer.common.opacity);
            if let Some(mask) = &layer.common.mask {
                apply_mask(&mut buf, mask, layer.common.mask_inverted);
            }
            buf
        }
        LayerKind::Filter(_) => {
            // Filters are handled at the list level, not as individual buffers
            ImageBuffer::new_transparent(canvas_w, canvas_h)
        }
    }
}

/// Render a list of layers onto `output`, back-to-front.
/// Layers at the end of the vec are drawn on top.
fn render_layers(layers: &[Layer], output: &mut ImageBuffer) {
    let mut prev_alpha = vec![0; (output.width as usize) * (output.height as usize)];
    for layer in layers {
        if !layer.common.visible {
            continue;
        }

        match &layer.kind {
            LayerKind::Filter(filter) => {
                apply_hsl_filter(output, &filter.config);
                refresh_alpha_snapshot(&mut prev_alpha, output);
            }
            _ => {
                if render_layer_direct(layer, output) {
                    capture_direct_layer_alpha(layer, output.width, output.height, &mut prev_alpha);
                    continue;
                }

                let mut layer_buf = render_layer_to_buffer(layer, output.width, output.height);

                // Clipping mask: clip to the alpha of what was rendered before this layer
                if layer.common.clip_to_below {
                    apply_clipping_mask(&mut layer_buf, &prev_alpha);
                }

                blend(output, &layer_buf, layer.common.blend_mode);
                refresh_alpha_snapshot(&mut prev_alpha, &layer_buf);
            }
        }
    }
}

/// Render a group into an isolated buffer using scoped filter rendering,
/// then blend onto the output.
///
/// Two-pass approach matching Spriteform's `renderSmartVariantScoped`:
/// - Pass 1: render non-filter layers (Raster, nested Group, etc.) back-to-front
/// - Pass 2: apply all filter layers in order to the composited group buffer
fn render_group(group: &GroupLayerData, output: &mut ImageBuffer, opacity: f64) {
    let canvas_w = output.width;
    let canvas_h = output.height;
    let mut group_buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
    let mut filters: Vec<&HslFilterConfig> = Vec::new();

    let mut prev_alpha = vec![0; (canvas_w as usize) * (canvas_h as usize)];

    // Pass 1: render non-filter layers, collect filters
    for layer in &group.children {
        if !layer.common.visible {
            continue;
        }
        match &layer.kind {
            LayerKind::Filter(filter) => {
                filters.push(&filter.config);
            }
            _ => {
                if render_layer_direct(layer, &mut group_buf) {
                    capture_direct_layer_alpha(layer, canvas_w, canvas_h, &mut prev_alpha);
                    continue;
                }

                let mut layer_buf = render_layer_to_buffer(layer, canvas_w, canvas_h);

                if layer.common.clip_to_below {
                    apply_clipping_mask(&mut layer_buf, &prev_alpha);
                }

                blend(&mut group_buf, &layer_buf, layer.common.blend_mode);
                refresh_alpha_snapshot(&mut prev_alpha, &layer_buf);
            }
        }
    }

    // Pass 2: apply all collected filters to the composited group buffer
    for config in &filters {
        apply_hsl_filter(&mut group_buf, config);
    }

    scale_alpha(&mut group_buf, opacity);

    blend_normal(output, &group_buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blend::BlendMode;
    use crate::layer::*;
    use crate::pixel::Rgba;

    fn img_layer(id: u32, name: &str, buf: ImageBuffer, x: i32, y: i32) -> Layer {
        Layer {
            common: LayerCommon {
                x,
                y,
                ..LayerCommon::new(id, name)
            },
            kind: LayerKind::Raster(RasterLayerData::new(buf)),
        }
    }

    fn solid_buf(w: u32, h: u32, color: Rgba) -> ImageBuffer {
        let mut buf = ImageBuffer::new_transparent(w, h);
        buf.fill(color);
        buf
    }

    #[test]
    fn empty_document_renders_transparent() {
        let doc = Document::new(4, 4);
        let result = doc.render();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(result.get_pixel(x, y), Rgba::TRANSPARENT);
            }
        }
    }

    #[test]
    fn single_opaque_image_layer() {
        let mut doc = Document::new(4, 4);
        let id = doc.next_id();
        doc.layers.push(img_layer(
            id,
            "red",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            1,
            1,
        ));
        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::TRANSPARENT);
        assert_eq!(result.get_pixel(1, 1), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(2, 2), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(3, 3), Rgba::TRANSPARENT);
    }

    #[test]
    fn hidden_layer_is_skipped() {
        let mut doc = Document::new(2, 2);
        let id = doc.next_id();
        let mut layer = img_layer(
            id,
            "hidden",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        );
        layer.common.visible = false;
        doc.layers.push(layer);
        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn two_layers_blend_correctly() {
        let mut doc = Document::new(1, 1);
        let id1 = doc.next_id();
        doc.layers.push(img_layer(
            id1,
            "blue",
            solid_buf(1, 1, Rgba::new(0, 0, 255, 255)),
            0,
            0,
        ));
        let id2 = doc.next_id();
        doc.layers.push(img_layer(
            id2,
            "red",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 128)),
            0,
            0,
        ));
        let result = doc.render();
        let p = result.get_pixel(0, 0);
        assert!(p.r > 100, "r={}", p.r);
        assert!(p.b > 100, "b={}", p.b);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn find_layer_in_tree() {
        let mut doc = Document::new(2, 2);
        let id1 = doc.next_id();
        let child_id = doc.next_id();
        let child = img_layer(
            child_id,
            "child",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        );
        doc.layers.push(Layer {
            common: LayerCommon::new(id1, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![child],
            }),
        });
        assert!(doc.find_layer(child_id).is_some());
        assert!(doc.find_layer(id1).is_some());
        assert!(doc.find_layer(999).is_none());
        let layer = doc.find_layer_mut(child_id).unwrap();
        layer.common.visible = false;
    }

    #[test]
    fn add_remove_child_from_group() {
        let mut doc = Document::new(2, 2);
        let group_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: Vec::new(),
            }),
        });
        let child_id = doc.next_id();
        let child = Layer {
            common: LayerCommon::new(child_id, "child"),
            kind: LayerKind::Filter(FilterLayerData {
                config: HslFilterConfig::default(),
            }),
        };
        assert!(doc.add_child_to_group(group_id, child).is_ok());
        assert!(doc.find_layer(child_id).is_some());
        assert!(doc.remove_child_from_group(group_id, child_id));
        assert!(doc.find_layer(child_id).is_none());
    }

    #[test]
    fn remove_layer_handles_top_level_and_nested_layers() {
        let mut doc = Document::new(2, 2);
        let top_id = doc.next_id();
        let group_id = doc.next_id();
        let child_id = doc.next_id();

        doc.layers.push(img_layer(
            top_id,
            "top",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));
        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![img_layer(
                    child_id,
                    "child",
                    solid_buf(1, 1, Rgba::new(0, 255, 0, 255)),
                    0,
                    0,
                )],
            }),
        });

        assert!(doc.remove_layer(top_id));
        assert!(doc.find_layer(top_id).is_none());

        assert!(doc.remove_layer(child_id));
        assert!(doc.find_layer(child_id).is_none());
        assert!(!doc.remove_layer(999));
    }

    #[test]
    fn move_layer_reorders_and_reparents() {
        let mut doc = Document::new(2, 2);
        let a_id = doc.next_id();
        let b_id = doc.next_id();
        let group_id = doc.next_id();

        doc.layers.push(img_layer(
            a_id,
            "a",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));
        doc.layers.push(img_layer(
            b_id,
            "b",
            solid_buf(1, 1, Rgba::new(0, 255, 0, 255)),
            0,
            0,
        ));
        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData::new()),
        });

        assert!(doc.move_layer(b_id, None, Some(0)));
        assert_eq!(doc.layer_location(b_id).unwrap().index, 0);

        assert!(doc.move_layer(a_id, Some(group_id), Some(0)));
        let moved = doc.layer_location(a_id).unwrap();
        assert_eq!(moved.parent_id, Some(group_id));
        assert_eq!(moved.depth, 1);
    }

    #[test]
    fn move_layer_rejects_invalid_targets() {
        let mut doc = Document::new(2, 2);
        let group_id = doc.next_id();
        let child_group_id = doc.next_id();
        let child_id = doc.next_id();

        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![Layer {
                    common: LayerCommon::new(child_group_id, "child-group"),
                    kind: LayerKind::Group(GroupLayerData {
                        children: vec![img_layer(
                            child_id,
                            "child",
                            solid_buf(1, 1, Rgba::new(255, 0, 0, 255)),
                            0,
                            0,
                        )],
                    }),
                }],
            }),
        });

        assert!(!doc.move_layer(group_id, Some(child_group_id), Some(0)));
        assert!(!doc.move_layer(child_id, Some(child_id), Some(0)));
        assert!(!doc.move_layer(child_id, Some(999), Some(0)));
    }

    #[test]
    fn update_layer_applies_common_and_filter_fields() {
        let mut doc = Document::new(4, 4);
        let image_id = doc.next_id();
        let paint_id = doc.next_id();
        let shape_id = doc.next_id();
        let filter_id = doc.next_id();
        let text_id = doc.next_id();
        doc.layers.push(img_layer(
            image_id,
            "img",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));
        doc.layers.push(Layer {
            common: LayerCommon::new(paint_id, "paint"),
            kind: LayerKind::Raster(RasterLayerData::new(ImageBuffer::new_transparent(2, 2))),
        });
        doc.layers.push(Layer {
            common: LayerCommon::new(shape_id, "shape"),
            kind: LayerKind::Shape(ShapeLayerData::new(
                ShapeType::Rectangle,
                2,
                2,
                0,
                Some(Rgba::new(0, 255, 0, 255)),
                None,
                Vec::new(),
            )),
        });
        doc.layers.push(Layer {
            common: LayerCommon::new(filter_id, "filter"),
            kind: LayerKind::Filter(FilterLayerData::new()),
        });
        doc.layers.push(Layer {
            common: LayerCommon::new(text_id, "text"),
            kind: LayerKind::Text(TextLayerData::new(
                "Hello",
                Rgba::new(255, 255, 255, 255),
                16,
                18,
                0,
            )),
        });

        assert!(doc.update_layer(
            image_id,
            &LayerPatch {
                alpha_locked: Some(true),
                blend_mode: Some(BlendMode::Multiply),
                clip_to_below: Some(true),
                flip_x: Some(true),
                name: Some("renamed".to_string()),
                opacity: Some(0.5),
                rotation: Some(90.0),
                scale_x: Some(1.5),
                scale_y: Some(2.0),
                visible: Some(false),
                x: Some(10),
                y: Some(20),
                ..Default::default()
            },
        ));
        assert!(doc.update_layer(
            text_id,
            &LayerPatch {
                text: Some(TextLayerPatch {
                    text: Some("World".into()),
                    color: Some(Rgba::new(0, 0, 0, 255)),
                    font_family: Some("monospace".into()),
                    font_weight: Some(500),
                    font_style: Some(TextFontStyle::Italic),
                    font_size: Some(24),
                    line_height: Some(28),
                    letter_spacing: Some(2),
                    align: Some(TextAlign::Center),
                    wrap: Some(TextWrap::Word),
                    box_width: Some(Some(48)),
                }),
                anchor: Some(crate::blit::Anchor::Center),
                rotation: Some(12.0),
                ..Default::default()
            },
        ));
        assert!(doc.update_layer(
            paint_id,
            &LayerPatch {
                anchor: Some(crate::blit::Anchor::Center),
                flip_y: Some(true),
                rotation: Some(15.0),
                scale_x: Some(2.0),
                scale_y: Some(0.5),
                ..Default::default()
            },
        ));
        assert!(doc.update_layer(
            shape_id,
            &LayerPatch {
                anchor: Some(crate::blit::Anchor::Center),
                flip_x: Some(true),
                rotation: Some(30.0),
                scale_x: Some(1.25),
                scale_y: Some(0.75),
                ..Default::default()
            },
        ));
        assert!(doc.update_layer(
            filter_id,
            &LayerPatch {
                filter: Some(FilterLayerPatch {
                    hue_deg: Some(45.0),
                    sharpen: Some(0.7),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ));

        let image_layer = doc.find_layer(image_id).unwrap();
        assert_eq!(image_layer.common.name, "renamed");
        assert_eq!(image_layer.common.opacity, 0.5);
        assert!(!image_layer.common.visible);
        assert_eq!(image_layer.common.x, 10);
        assert_eq!(image_layer.common.y, 20);
        assert_eq!(image_layer.common.blend_mode, BlendMode::Multiply);
        assert!(image_layer.common.clip_to_below);
        match &image_layer.kind {
            LayerKind::Raster(raster) => {
                assert!(raster.alpha_locked);
                assert!(raster.transform.flip_x);
                assert_eq!(raster.transform.rotation_deg, 90.0);
                assert_eq!(raster.transform.scale_x, 1.5);
                assert_eq!(raster.transform.scale_y, 2.0);
            }
            _ => panic!("expected raster layer"),
        }

        match &doc.find_layer(paint_id).unwrap().kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.transform.anchor, crate::blit::Anchor::Center);
                assert!(raster.transform.flip_y);
                assert_eq!(raster.transform.rotation_deg, 15.0);
                assert_eq!(raster.transform.scale_x, 2.0);
                assert_eq!(raster.transform.scale_y, 0.5);
            }
            _ => panic!("expected raster layer"),
        }

        match &doc.find_layer(shape_id).unwrap().kind {
            LayerKind::Shape(shape) => {
                assert_eq!(shape.transform.anchor, crate::blit::Anchor::Center);
                assert!(shape.transform.flip_x);
                assert_eq!(shape.transform.rotation_deg, 30.0);
                assert_eq!(shape.transform.scale_x, 1.25);
                assert_eq!(shape.transform.scale_y, 0.75);
            }
            _ => panic!("expected shape layer"),
        }

        match &doc.find_layer(filter_id).unwrap().kind {
            LayerKind::Filter(filter) => {
                assert_eq!(filter.config.hue_deg, 45.0);
                assert_eq!(filter.config.sharpen, 0.7);
            }
            _ => panic!("expected filter layer"),
        }

        match &doc.find_layer(text_id).unwrap().kind {
            LayerKind::Text(text) => {
                assert_eq!(text.text, "World");
                assert_eq!(text.color, Rgba::new(0, 0, 0, 255));
                assert_eq!(text.font_family, "monospace");
                assert_eq!(text.font_weight, 500);
                assert_eq!(text.font_style, TextFontStyle::Italic);
                assert_eq!(text.font_size, 24);
                assert_eq!(text.line_height, 28);
                assert_eq!(text.letter_spacing, 2);
                assert_eq!(text.align, TextAlign::Center);
                assert_eq!(text.wrap, TextWrap::Word);
                assert_eq!(text.box_width, Some(48));
                assert_eq!(text.transform.anchor, crate::blit::Anchor::Center);
                assert_eq!(text.transform.rotation_deg, 12.0);
            }
            _ => panic!("expected text layer"),
        }
    }

    #[test]
    fn text_layer_renders_visible_pixels() {
        let mut doc = Document::new(64, 24);
        let id = doc.next_id();
        let mut common = LayerCommon::new(id, "text");
        common.x = 2;
        common.y = 3;
        doc.layers.push(Layer {
            common,
            kind: LayerKind::Text(TextLayerData::new(
                "Hi",
                Rgba::new(255, 0, 0, 255),
                16,
                18,
                0,
            )),
        });

        let result = doc.render();
        assert!(result.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }

    #[cfg(feature = "svg-backend")]
    #[test]
    fn svg_layer_renders_visible_pixels() {
        let mut doc = Document::new(32, 32);
        let id = doc.next_id();
        let mut common = LayerCommon::new(id, "logo");
        common.x = 4;
        common.y = 5;
        doc.layers.push(Layer {
            common,
            kind: LayerKind::Svg(SvgLayerData::new(
                br##"
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
                      <rect x="2" y="2" width="20" height="20" rx="4" fill="#d9482b"/>
                      <circle cx="12" cy="12" r="5" fill="#f2c94c"/>
                    </svg>
                "##
                .to_vec(),
                24,
                24,
            )),
        });

        let result = doc.render();
        assert!(result.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }

    #[cfg(feature = "svg-backend")]
    #[test]
    fn rasterize_svg_layer_converts_to_raster() {
        let mut doc = Document::new(32, 32);
        let id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id, "logo"),
            kind: LayerKind::Svg(SvgLayerData::new(
                br##"
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
                      <rect x="2" y="2" width="20" height="20" rx="4" fill="#d9482b"/>
                      <circle cx="12" cy="12" r="5" fill="#f2c94c"/>
                    </svg>
                "##
                .to_vec(),
                24,
                24,
            )),
        });

        let before = doc.render().data;
        assert!(doc.rasterize_svg_layer(id));
        assert!(matches!(
            doc.find_layer(id).unwrap().kind,
            LayerKind::Raster(_)
        ));
        let after = doc.render().data;
        assert_eq!(before, after);
    }

    #[test]
    fn resize_canvas_updates_document_dimensions() {
        let mut doc = Document::new(4, 4);
        doc.resize_canvas(16, 9);
        assert_eq!(doc.width, 16);
        assert_eq!(doc.height, 9);
    }

    #[test]
    fn bucket_fill_layer_mutates_pixel_layers_only() {
        let mut doc = Document::new(4, 4);
        let image_id = doc.next_id();
        let paint_id = doc.next_id();
        let group_id = doc.next_id();
        let mut image = solid_buf(2, 2, Rgba::new(10, 10, 10, 255));
        image.set_pixel(1, 1, Rgba::new(200, 0, 0, 255));
        doc.layers.push(img_layer(image_id, "img", image, 0, 0));
        doc.layers.push(Layer {
            common: LayerCommon::new(paint_id, "paint"),
            kind: LayerKind::Raster(RasterLayerData::new(ImageBuffer::new_transparent(2, 2))),
        });
        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData::new()),
        });

        assert!(doc.bucket_fill_layer(image_id, 0, 0, Rgba::new(0, 255, 0, 255), true, 0,));
        assert!(doc.bucket_fill_layer(paint_id, 0, 0, Rgba::new(0, 0, 255, 255), false, 0,));
        assert!(!doc.bucket_fill_layer(group_id, 0, 0, Rgba::new(255, 255, 255, 255), true, 0,));

        match &doc.find_layer(image_id).unwrap().kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.buffer.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
                assert_eq!(raster.buffer.get_pixel(1, 1), Rgba::new(200, 0, 0, 255));
            }
            _ => panic!("expected raster layer"),
        }

        match &doc.find_layer(paint_id).unwrap().kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.buffer.get_pixel(0, 0), Rgba::new(0, 0, 255, 255));
                assert_eq!(raster.buffer.get_pixel(1, 1), Rgba::new(0, 0, 255, 255));
            }
            _ => panic!("expected raster layer"),
        }
    }

    #[test]
    fn bucket_fill_layer_respects_alpha_lock() {
        let mut doc = Document::new(3, 1);
        let raster_id = doc.next_id();
        let mut raster = ImageBuffer::new_transparent(3, 1);
        raster.set_pixel(0, 0, Rgba::new(10, 10, 10, 255));
        raster.set_pixel(1, 0, Rgba::new(10, 10, 10, 0));
        raster.set_pixel(2, 0, Rgba::new(10, 10, 10, 255));
        let mut layer = Layer::new(
            LayerCommon::new(raster_id, "raster"),
            LayerKind::Raster(RasterLayerData::new(raster)),
        );
        if let LayerKind::Raster(raster) = &mut layer.kind {
            raster.alpha_locked = true;
        }
        doc.layers.push(layer);

        assert!(doc.bucket_fill_layer(raster_id, 0, 0, Rgba::new(0, 255, 0, 255), false, 0,));

        match &doc.find_layer(raster_id).unwrap().kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.buffer.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
                assert_eq!(raster.buffer.get_pixel(1, 0), Rgba::new(10, 10, 10, 0));
                assert_eq!(raster.buffer.get_pixel(2, 0), Rgba::new(0, 255, 0, 255));
            }
            _ => panic!("expected raster layer"),
        }
    }

    #[test]
    fn paint_stroke_layer_mutates_raster_layers_only() {
        let mut doc = Document::new(8, 8);

        let raster_id = doc.next_id();
        let shape_id = doc.next_id();
        let mut raster = ImageBuffer::new_transparent(8, 8);
        raster.fill(Rgba::new(0, 0, 0, 0));
        doc.layers.push(Layer::new(
            LayerCommon::new(raster_id, "raster"),
            LayerKind::Raster(RasterLayerData::new(raster)),
        ));
        doc.layers.push(Layer::new(
            LayerCommon::new(shape_id, "shape"),
            LayerKind::Shape(ShapeLayerData::new(
                crate::layer::ShapeType::Rectangle,
                4,
                4,
                0,
                Some(Rgba::new(255, 0, 0, 255)),
                None,
                Vec::new(),
            )),
        ));

        let preset = BrushPreset {
            color: Rgba::new(0, 255, 0, 255),
            size: 4.0,
            ..BrushPreset::default()
        };
        let points = [StrokePoint::new(4.0, 4.0, 1.0)];

        assert!(doc.paint_stroke_layer(raster_id, &preset, &points));
        assert!(!doc.paint_stroke_layer(shape_id, &preset, &points));

        match &doc.find_layer(raster_id).unwrap().kind {
            LayerKind::Raster(raster) => {
                assert!(raster.buffer.data.iter().any(|&value| value != 0));
            }
            _ => panic!("expected raster layer"),
        }
    }

    #[test]
    fn paint_stroke_layer_invalidates_raster_transform_cache() {
        let mut doc = Document::new(8, 8);
        let raster_id = doc.next_id();
        let mut layer = Layer::new(
            LayerCommon::new(raster_id, "raster"),
            LayerKind::Raster(RasterLayerData::new(ImageBuffer::new_transparent(4, 4))),
        );
        layer.common.x = 2;
        layer.common.y = 2;
        if let LayerKind::Raster(raster) = &mut layer.kind {
            raster.transform.rotation_deg = 30.0;
        }
        doc.layers.push(layer);

        let before = doc.render().data;
        let preset = BrushPreset {
            color: Rgba::new(255, 0, 0, 255),
            size: 3.0,
            ..BrushPreset::default()
        };
        assert!(doc.paint_stroke_layer(raster_id, &preset, &[StrokePoint::new(1.0, 1.0, 1.0)],));
        let after = doc.render().data;

        assert_ne!(before, after);
    }

    #[test]
    fn paint_stroke_layer_respects_alpha_lock() {
        let mut doc = Document::new(8, 8);
        let raster_id = doc.next_id();
        let mut raster = ImageBuffer::new_transparent(8, 8);
        raster.set_pixel(4, 4, Rgba::new(20, 20, 20, 255));
        let mut layer = Layer::new(
            LayerCommon::new(raster_id, "raster"),
            LayerKind::Raster(RasterLayerData::new(raster)),
        );
        if let LayerKind::Raster(raster) = &mut layer.kind {
            raster.alpha_locked = true;
        }
        doc.layers.push(layer);

        let preset = BrushPreset {
            color: Rgba::new(255, 0, 0, 255),
            size: 6.0,
            ..BrushPreset::default()
        };
        assert!(doc.paint_stroke_layer(raster_id, &preset, &[StrokePoint::new(4.0, 4.0, 1.0)],));

        match &doc.find_layer(raster_id).unwrap().kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.buffer.get_pixel(0, 0), Rgba::TRANSPARENT);
                assert_eq!(raster.buffer.get_pixel(4, 4).r, 255);
            }
            _ => panic!("expected raster layer"),
        }
    }

    #[test]
    fn render_invalidates_transformed_raster_cache_after_pixel_mutation() {
        let mut doc = Document::new(8, 4);
        let image_id = doc.next_id();
        let paint_id = doc.next_id();

        doc.layers.push(img_layer(
            image_id,
            "img",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));

        let mut paint = ImageBuffer::new_transparent(2, 2);
        paint.fill(Rgba::new(0, 0, 255, 255));
        let mut paint_layer = Layer {
            common: LayerCommon::new(paint_id, "paint"),
            kind: LayerKind::Raster(RasterLayerData::new(paint)),
        };
        paint_layer.common.x = 4;
        doc.layers.push(paint_layer);

        assert!(doc.update_layer(
            image_id,
            &LayerPatch {
                scale_x: Some(2.0),
                scale_y: Some(2.0),
                ..Default::default()
            },
        ));
        assert!(doc.update_layer(
            paint_id,
            &LayerPatch {
                scale_x: Some(2.0),
                scale_y: Some(2.0),
                ..Default::default()
            },
        ));

        let before = doc.render();
        assert_eq!(before.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(before.get_pixel(4, 0), Rgba::new(0, 0, 255, 255));

        assert!(doc.bucket_fill_layer(image_id, 0, 0, Rgba::new(0, 255, 0, 255), true, 0,));
        assert!(doc.bucket_fill_layer(paint_id, 0, 0, Rgba::new(255, 255, 0, 255), true, 0,));

        let after = doc.render();
        assert_eq!(after.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
        assert_eq!(after.get_pixel(4, 0), Rgba::new(255, 255, 0, 255));
    }

    #[test]
    fn scoped_filter_applies_only_within_group() {
        let mut doc = Document::new(1, 1);
        let id_bg = doc.next_id();
        doc.layers.push(img_layer(
            id_bg,
            "bg",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));
        let group_id = doc.next_id();
        let child_img_id = doc.next_id();
        let child_img = img_layer(
            child_img_id,
            "green",
            solid_buf(1, 1, Rgba::new(0, 255, 0, 128)),
            0,
            0,
        );
        let filter_id = doc.next_id();
        let child_filter = Layer {
            common: LayerCommon::new(filter_id, "filter"),
            kind: LayerKind::Filter(FilterLayerData {
                config: HslFilterConfig {
                    hue_deg: 120.0,
                    ..Default::default()
                },
            }),
        };
        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![child_img, child_filter],
            }),
        });
        let _result = doc.render();
    }

    #[test]
    fn group_layer_renders_isolated() {
        let mut doc = Document::new(2, 2);
        let id1 = doc.next_id();
        let child_id = doc.next_id();
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        let child = img_layer(child_id, "red", buf, 0, 0);
        doc.layers.push(Layer {
            common: LayerCommon::new(id1, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![child],
            }),
        });
        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(1, 1), Rgba::TRANSPARENT);
    }

    // ── Phase 2 tests ──

    #[test]
    fn blend_mode_multiply_in_document() {
        let mut doc = Document::new(1, 1);
        let id1 = doc.next_id();
        doc.layers.push(img_layer(
            id1,
            "base",
            solid_buf(1, 1, Rgba::new(200, 100, 50, 255)),
            0,
            0,
        ));
        let id2 = doc.next_id();
        let mut layer = img_layer(
            id2,
            "top",
            solid_buf(1, 1, Rgba::new(128, 128, 128, 255)),
            0,
            0,
        );
        layer.common.blend_mode = BlendMode::Multiply;
        doc.layers.push(layer);

        let result = doc.render();
        let p = result.get_pixel(0, 0);
        // multiply: 200/255 * 128/255 * 255 ≈ 100
        assert!(p.r > 95 && p.r < 110, "r={}", p.r);
    }

    #[test]
    fn layer_mask_hides_pixels() {
        let mut doc = Document::new(2, 1);
        let id = doc.next_id();
        let mut layer = img_layer(id, "red", solid_buf(2, 1, Rgba::new(255, 0, 0, 255)), 0, 0);
        // Mask: left pixel white (visible), right pixel black (hidden)
        let mut mask = ImageBuffer::new_transparent(2, 1);
        mask.set_pixel(0, 0, Rgba::new(255, 255, 255, 255));
        mask.set_pixel(1, 0, Rgba::new(0, 0, 0, 255));
        layer.common.mask = Some(mask);
        doc.layers.push(layer);

        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(1, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn layer_mask_inverted_reverses_visibility() {
        let mut doc = Document::new(2, 1);
        let id = doc.next_id();
        let mut layer = img_layer(id, "red", solid_buf(2, 1, Rgba::new(255, 0, 0, 255)), 0, 0);
        // Same mask as layer_mask_hides_pixels: white left, black right
        let mut mask = ImageBuffer::new_transparent(2, 1);
        mask.set_pixel(0, 0, Rgba::new(255, 255, 255, 255));
        mask.set_pixel(1, 0, Rgba::new(0, 0, 0, 255));
        layer.common.mask = Some(mask);
        layer.common.mask_inverted = true; // invert: black becomes visible, white hidden
        doc.layers.push(layer);

        let result = doc.render();
        // With inversion: white mask → hidden, black mask → visible
        assert_eq!(result.get_pixel(0, 0), Rgba::TRANSPARENT);
        assert_eq!(result.get_pixel(1, 0), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn clipping_mask_clips_to_below() {
        let mut doc = Document::new(2, 1);

        // Bottom layer: only left pixel has content
        let id1 = doc.next_id();
        let mut below_buf = ImageBuffer::new_transparent(2, 1);
        below_buf.set_pixel(0, 0, Rgba::new(0, 255, 0, 255));
        doc.layers.push(img_layer(id1, "below", below_buf, 0, 0));

        // Top layer: full red, clipped to below
        let id2 = doc.next_id();
        let mut layer = img_layer(
            id2,
            "clipped",
            solid_buf(2, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        );
        layer.common.clip_to_below = true;
        doc.layers.push(layer);

        let result = doc.render();
        // Left pixel: red clipped to green alpha (opaque) → red over green
        let p0 = result.get_pixel(0, 0);
        assert_eq!(p0.r, 255);
        assert_eq!(p0.g, 0);
        // Right pixel: red clipped to transparent → transparent
        assert_eq!(result.get_pixel(1, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn clipping_mask_ignores_older_opaque_background_layers() {
        let mut doc = Document::new(2, 1);

        let background_id = doc.next_id();
        doc.layers.push(img_layer(
            background_id,
            "background",
            solid_buf(2, 1, Rgba::new(240, 240, 240, 255)),
            0,
            0,
        ));

        let below_id = doc.next_id();
        let mut below_buf = ImageBuffer::new_transparent(2, 1);
        below_buf.set_pixel(0, 0, Rgba::new(0, 255, 0, 255));
        doc.layers
            .push(img_layer(below_id, "below", below_buf, 0, 0));

        let clipped_id = doc.next_id();
        let mut clipped = img_layer(
            clipped_id,
            "clipped",
            solid_buf(2, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        );
        clipped.common.clip_to_below = true;
        doc.layers.push(clipped);

        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(1, 0), Rgba::new(240, 240, 240, 255));
    }

    #[test]
    fn solid_color_layer() {
        let mut doc = Document::new(2, 2);
        let id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id, "fill"),
            kind: LayerKind::Fill(FillLayerData::solid(Rgba::new(100, 200, 50, 255))),
        });
        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(100, 200, 50, 255));
        assert_eq!(result.get_pixel(1, 1), Rgba::new(100, 200, 50, 255));
    }

    #[test]
    fn gradient_layer_horizontal() {
        let mut doc = Document::new(3, 1);
        let id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id, "gradient"),
            kind: LayerKind::Fill(FillLayerData::gradient(
                vec![
                    GradientStop {
                        position: 0.0,
                        color: Rgba::new(0, 0, 0, 255),
                    },
                    GradientStop {
                        position: 1.0,
                        color: Rgba::new(255, 255, 255, 255),
                    },
                ],
                GradientDirection::Horizontal,
            )),
        });
        let result = doc.render();
        let p0 = result.get_pixel(0, 0);
        let p1 = result.get_pixel(1, 0);
        let p2 = result.get_pixel(2, 0);
        assert!(p0.r < 10, "left should be ~black, r={}", p0.r);
        assert!(p1.r > 120 && p1.r < 136, "mid should be ~128, r={}", p1.r);
        assert!(p2.r > 250, "right should be ~white, r={}", p2.r);
    }

    #[test]
    fn shape_layer_rectangle_renders_at_position() {
        let mut doc = Document::new(6, 6);
        let id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon {
                x: 1,
                y: 2,
                ..LayerCommon::new(id, "shape")
            },
            kind: LayerKind::Shape(ShapeLayerData::new(
                ShapeType::Rectangle,
                3,
                2,
                0,
                Some(Rgba::new(255, 0, 0, 255)),
                None,
                Vec::new(),
            )),
        });

        let result = doc.render();
        assert_eq!(result.get_pixel(1, 2), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(3, 3), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn shape_layer_respects_clip_and_blend_stack() {
        let mut doc = Document::new(4, 4);
        let base_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(base_id, "base"),
            kind: LayerKind::Shape(ShapeLayerData::new(
                ShapeType::Rectangle,
                2,
                4,
                0,
                Some(Rgba::new(0, 255, 0, 255)),
                None,
                Vec::new(),
            )),
        });

        let top_id = doc.next_id();
        let mut common = LayerCommon::new(top_id, "top");
        common.clip_to_below = true;
        common.blend_mode = BlendMode::Normal;
        doc.layers.push(Layer {
            common,
            kind: LayerKind::Shape(ShapeLayerData::new(
                ShapeType::Rectangle,
                4,
                4,
                0,
                Some(Rgba::new(255, 0, 0, 255)),
                None,
                Vec::new(),
            )),
        });

        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(result.get_pixel(3, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn flatten_group_produces_raster() {
        let mut doc = Document::new(2, 2);
        let group_id = doc.next_id();
        let child_id = doc.next_id();
        let child = img_layer(
            child_id,
            "red",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        );
        doc.layers.push(Layer {
            common: LayerCommon::new(group_id, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![child],
            }),
        });

        assert!(doc.flatten_group(group_id));

        // Should now be a raster layer
        let layer = doc.find_layer(group_id).unwrap();
        assert!(matches!(layer.kind, LayerKind::Raster(_)));

        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn solid_color_with_opacity() {
        let mut doc = Document::new(1, 1);
        let id = doc.next_id();
        let mut layer = Layer {
            common: LayerCommon::new(id, "fill"),
            kind: LayerKind::Fill(FillLayerData::solid(Rgba::new(255, 0, 0, 255))),
        };
        layer.common.opacity = 0.5;
        doc.layers.push(layer);

        let result = doc.render();
        let p = result.get_pixel(0, 0);
        assert!(p.a > 120 && p.a < 136, "a={}", p.a);
    }

    // ── Golden-image integration tests ──

    #[test]
    fn golden_solid_over_solid() {
        // Two opaque 2×2 layers: blue base, red top → every pixel should be pure red
        let mut doc = Document::new(2, 2);
        let id1 = doc.next_id();
        doc.layers.push(img_layer(
            id1,
            "blue",
            solid_buf(2, 2, Rgba::new(0, 0, 255, 255)),
            0,
            0,
        ));
        let id2 = doc.next_id();
        doc.layers.push(img_layer(
            id2,
            "red",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));

        let result = doc.render();
        let expected = Rgba::new(255, 0, 0, 255);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(result.get_pixel(x, y), expected, "pixel ({x},{y}) mismatch");
            }
        }
    }

    #[test]
    fn golden_opacity_blend() {
        // 1×1 canvas: blue base (opaque) + red layer at 50% opacity
        // Porter-Duff source-over: resulting color should blend
        let mut doc = Document::new(1, 1);
        let id1 = doc.next_id();
        doc.layers.push(img_layer(
            id1,
            "blue",
            solid_buf(1, 1, Rgba::new(0, 0, 255, 255)),
            0,
            0,
        ));
        let id2 = doc.next_id();
        // Create a red pixel with alpha = 128 (≈50%)
        doc.layers.push(img_layer(
            id2,
            "red-50",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 128)),
            0,
            0,
        ));

        let result = doc.render();
        let p = result.get_pixel(0, 0);
        // alpha = 128 + 255*(1 - 128/255) = 255 (fully opaque output)
        assert_eq!(p.a, 255, "alpha should be 255");
        // red ≈ 255 * (128/255) / 1.0 = 128
        assert!(p.r > 120 && p.r < 136, "r={} expected ~128", p.r);
        // blue ≈ 255 * (1 - 128/255) = ~127
        assert!(p.b > 120 && p.b < 136, "b={} expected ~127", p.b);
        assert!(p.g < 5, "g={} expected ~0", p.g);
    }

    #[test]
    fn golden_filter_pipeline() {
        // 2×2 canvas with red pixels, apply a saturation=-1 filter to desaturate to grayscale
        let mut doc = Document::new(2, 2);
        let id1 = doc.next_id();
        doc.layers.push(img_layer(
            id1,
            "red",
            solid_buf(2, 2, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));
        let filter_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(filter_id, "desat"),
            kind: LayerKind::Filter(FilterLayerData {
                config: HslFilterConfig {
                    saturation: -1.0, // fully desaturate
                    ..Default::default()
                },
            }),
        });

        let result = doc.render();
        for y in 0..2 {
            for x in 0..2 {
                let p = result.get_pixel(x, y);
                // Desaturated red → grayscale, all RGB channels should be equal
                assert_eq!(p.r, p.g, "pixel ({x},{y}) r={} != g={}", p.r, p.g);
                assert_eq!(p.g, p.b, "pixel ({x},{y}) g={} != b={}", p.g, p.b);
                assert_eq!(p.a, 255, "pixel ({x},{y}) alpha should be 255");
                // Luminance of pure red in HSL → L=0.5, so grayscale ≈ 128
                assert!(
                    p.r > 120 && p.r < 136,
                    "pixel ({x},{y}) gray value {}, expected ~128",
                    p.r
                );
            }
        }
    }
}
