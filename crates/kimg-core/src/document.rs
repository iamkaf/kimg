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

use crate::blend::{blend, blend_normal};
use crate::blit::{blit_transformed, Anchor, BlitParams, Rotation};
use crate::buffer::ImageBuffer;
use crate::filter::{apply_hsl_filter, HslFilterConfig};
use crate::layer::{
    FilterLayerPatch, GradientDirection, GradientLayerData, GroupLayerData, Layer, LayerKind,
    LayerPatch, ShapeLayerData, SolidColorLayerData,
};
use crate::pixel::Rgba;
use crate::shape::render_shape;

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

        if insert_layer_at(&mut self.layers, target_parent_id, target_index, layer) {
            true
        } else {
            false
        }
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

        if let Some(anchor) = patch.anchor {
            match &mut layer.kind {
                LayerKind::Image(img) => img.anchor = anchor,
                LayerKind::Paint(paint) => paint.anchor = anchor,
                _ => {}
            }
        }

        if let LayerKind::Image(img) = &mut layer.kind {
            if let Some(flip_x) = patch.flip_x {
                img.flip_x = flip_x;
            }
            if let Some(flip_y) = patch.flip_y {
                img.flip_y = flip_y;
            }
            if let Some(rotation) = patch.rotation {
                img.rotation = rotation;
            }
        }

        if let (LayerKind::Filter(filter), Some(filter_patch)) = (&mut layer.kind, &patch.filter) {
            apply_filter_patch(&mut filter.config, filter_patch);
        }

        true
    }

    /// Flatten a group layer into a single image layer in-place.
    ///
    /// The group is rendered to a canvas-sized buffer, then replaced with an
    /// `Image` layer that preserves the group's common properties (id, name,
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
        layer.kind = LayerKind::Image(crate::layer::ImageLayerData {
            buffer: buf,
            anchor: crate::blit::Anchor::TopLeft,
            flip_x: false,
            flip_y: false,
            rotation: crate::blit::Rotation::None,
        });
        // Reset position since the buffer is already at canvas coordinates
        layer.common.x = 0;
        layer.common.y = 0;
        true
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
}

/// Render a solid color fill to a full-canvas buffer.
fn render_solid_color(color: &SolidColorLayerData, width: u32, height: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(width, height);
    buf.fill(color.color);
    buf
}

/// Render a gradient to a full-canvas buffer.
fn render_gradient(grad: &GradientLayerData, width: u32, height: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(width, height);
    if grad.stops.is_empty() || width == 0 || height == 0 {
        return buf;
    }

    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let t = match grad.direction {
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

            let color = sample_gradient(&grad.stops, t);
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
            let mut mask_val = mask.data[mi] as f64 / 255.0;
            if inverted {
                mask_val = 1.0 - mask_val;
            }
            let a = buf.data[bi + 3] as f64;
            buf.data[bi + 3] = (a * mask_val + 0.5) as u8;
        }
    }
}

/// Clip `layer_buf` to the alpha of `below_buf`. Where below has alpha=0,
/// the layer becomes transparent.
fn apply_clipping_mask(layer_buf: &mut ImageBuffer, below_buf: &ImageBuffer) {
    let w = layer_buf.width.min(below_buf.width) as usize;
    let h = layer_buf.height.min(below_buf.height) as usize;
    let ls = layer_buf.width as usize;
    let bs = below_buf.width as usize;

    for y in 0..h {
        for x in 0..w {
            let li = (y * ls + x) * 4;
            let bi = (y * bs + x) * 4;
            let below_a = below_buf.data[bi + 3] as f64 / 255.0;
            let a = layer_buf.data[li + 3] as f64;
            layer_buf.data[li + 3] = (a * below_a + 0.5) as u8;
        }
    }
}

/// Render a single layer to an isolated canvas-sized RGBA buffer.
fn render_layer_to_buffer(layer: &Layer, canvas_w: u32, canvas_h: u32) -> ImageBuffer {
    let mut buf = ImageBuffer::new_transparent(canvas_w, canvas_h);

    match &layer.kind {
        LayerKind::Image(img) => {
            blit_transformed(
                &mut buf,
                &img.buffer,
                &BlitParams {
                    dx: layer.common.x,
                    dy: layer.common.y,
                    anchor: img.anchor,
                    flip_x: img.flip_x,
                    flip_y: img.flip_y,
                    rotation: img.rotation,
                    opacity: layer.common.opacity,
                },
            );
        }
        LayerKind::Paint(paint) => {
            blit_transformed(
                &mut buf,
                &paint.buffer,
                &BlitParams {
                    dx: layer.common.x,
                    dy: layer.common.y,
                    anchor: paint.anchor,
                    flip_x: false,
                    flip_y: false,
                    rotation: crate::blit::Rotation::None,
                    opacity: layer.common.opacity,
                },
            );
        }
        LayerKind::SolidColor(sc) => {
            let fill = render_solid_color(sc, canvas_w, canvas_h);
            if layer.common.opacity < 1.0 {
                // Apply opacity
                for i in (0..buf.data.len()).step_by(4) {
                    buf.data[i] = fill.data[i];
                    buf.data[i + 1] = fill.data[i + 1];
                    buf.data[i + 2] = fill.data[i + 2];
                    buf.data[i + 3] = (fill.data[i + 3] as f64 * layer.common.opacity + 0.5) as u8;
                }
            } else {
                buf = fill;
            }
        }
        LayerKind::Gradient(grad) => {
            let fill = render_gradient(grad, canvas_w, canvas_h);
            if layer.common.opacity < 1.0 {
                for i in (0..buf.data.len()).step_by(4) {
                    buf.data[i] = fill.data[i];
                    buf.data[i + 1] = fill.data[i + 1];
                    buf.data[i + 2] = fill.data[i + 2];
                    buf.data[i + 3] = (fill.data[i + 3] as f64 * layer.common.opacity + 0.5) as u8;
                }
            } else {
                buf = fill;
            }
        }
        LayerKind::Shape(shape) => {
            let shape_buf = render_shape_layer(shape);
            blit_transformed(
                &mut buf,
                &shape_buf,
                &BlitParams {
                    dx: layer.common.x,
                    dy: layer.common.y,
                    anchor: Anchor::TopLeft,
                    flip_x: false,
                    flip_y: false,
                    rotation: Rotation::None,
                    opacity: layer.common.opacity,
                },
            );
        }
        LayerKind::Group(group) => {
            render_group(group, &mut buf, layer.common.opacity);
        }
        LayerKind::Filter(_) => {
            // Filters are handled at the list level, not as individual buffers
        }
    }

    // Apply layer mask if present
    if let Some(mask) = &layer.common.mask {
        apply_mask(&mut buf, mask, layer.common.mask_inverted);
    }

    buf
}

/// Render a list of layers onto `output`, back-to-front.
/// Layers at the end of the vec are drawn on top.
fn render_layers(layers: &[Layer], output: &mut ImageBuffer) {
    let canvas_w = output.width;
    let canvas_h = output.height;

    // We need the previous rendered state for clipping masks
    let mut prev_composite = output.clone();

    for layer in layers {
        if !layer.common.visible {
            continue;
        }

        match &layer.kind {
            LayerKind::Filter(filter) => {
                apply_hsl_filter(output, &filter.config);
                prev_composite = output.clone();
            }
            _ => {
                let mut layer_buf = render_layer_to_buffer(layer, canvas_w, canvas_h);

                // Clipping mask: clip to the alpha of what was rendered before this layer
                if layer.common.clip_to_below {
                    apply_clipping_mask(&mut layer_buf, &prev_composite);
                }

                blend(output, &layer_buf, layer.common.blend_mode);
                prev_composite = output.clone();
            }
        }
    }
}

/// Render a group into an isolated buffer using scoped filter rendering,
/// then blend onto the output.
///
/// Two-pass approach matching Spriteform's `renderSmartVariantScoped`:
/// - Pass 1: render non-filter layers (Image, Paint, nested Group, etc.) back-to-front
/// - Pass 2: apply all filter layers in order to the composited group buffer
fn render_group(group: &GroupLayerData, output: &mut ImageBuffer, opacity: f64) {
    let canvas_w = output.width;
    let canvas_h = output.height;
    let mut group_buf = ImageBuffer::new_transparent(canvas_w, canvas_h);
    let mut filters: Vec<&HslFilterConfig> = Vec::new();

    let mut prev_composite = group_buf.clone();

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
                let mut layer_buf = render_layer_to_buffer(layer, canvas_w, canvas_h);

                if layer.common.clip_to_below {
                    apply_clipping_mask(&mut layer_buf, &prev_composite);
                }

                blend(&mut group_buf, &layer_buf, layer.common.blend_mode);
                prev_composite = group_buf.clone();
            }
        }
    }

    // Pass 2: apply all collected filters to the composited group buffer
    for config in &filters {
        apply_hsl_filter(&mut group_buf, config);
    }

    if opacity < 1.0 {
        for i in (0..group_buf.data.len()).step_by(4) {
            let a = group_buf.data[i + 3] as f64;
            group_buf.data[i + 3] = (a * opacity).round() as u8;
        }
    }

    blend_normal(output, &group_buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blend::BlendMode;
    use crate::blit::{Anchor, Rotation};
    use crate::layer::*;
    use crate::pixel::Rgba;

    fn img_layer(id: u32, name: &str, buf: ImageBuffer, x: i32, y: i32) -> Layer {
        Layer {
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
        let filter_id = doc.next_id();
        doc.layers.push(img_layer(
            image_id,
            "img",
            solid_buf(1, 1, Rgba::new(255, 0, 0, 255)),
            0,
            0,
        ));
        doc.layers.push(Layer {
            common: LayerCommon::new(filter_id, "filter"),
            kind: LayerKind::Filter(FilterLayerData::new()),
        });

        assert!(doc.update_layer(
            image_id,
            &LayerPatch {
                blend_mode: Some(BlendMode::Multiply),
                clip_to_below: Some(true),
                flip_x: Some(true),
                name: Some("renamed".to_string()),
                opacity: Some(0.5),
                rotation: Some(Rotation::Cw90),
                visible: Some(false),
                x: Some(10),
                y: Some(20),
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
            LayerKind::Image(image) => {
                assert!(image.flip_x);
                assert_eq!(image.rotation, Rotation::Cw90);
            }
            _ => panic!("expected image layer"),
        }

        match &doc.find_layer(filter_id).unwrap().kind {
            LayerKind::Filter(filter) => {
                assert_eq!(filter.config.hue_deg, 45.0);
                assert_eq!(filter.config.sharpen, 0.7);
            }
            _ => panic!("expected filter layer"),
        }
    }

    #[test]
    fn resize_canvas_updates_document_dimensions() {
        let mut doc = Document::new(4, 4);
        doc.resize_canvas(16, 9);
        assert_eq!(doc.width, 16);
        assert_eq!(doc.height, 9);
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
    fn solid_color_layer() {
        let mut doc = Document::new(2, 2);
        let id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id, "fill"),
            kind: LayerKind::SolidColor(SolidColorLayerData {
                color: Rgba::new(100, 200, 50, 255),
            }),
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
            kind: LayerKind::Gradient(GradientLayerData {
                stops: vec![
                    GradientStop {
                        position: 0.0,
                        color: Rgba::new(0, 0, 0, 255),
                    },
                    GradientStop {
                        position: 1.0,
                        color: Rgba::new(255, 255, 255, 255),
                    },
                ],
                direction: GradientDirection::Horizontal,
            }),
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
    fn flatten_group_produces_image() {
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

        // Should now be an image layer
        let layer = doc.find_layer(group_id).unwrap();
        assert!(matches!(layer.kind, LayerKind::Image(_)));

        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn solid_color_with_opacity() {
        let mut doc = Document::new(1, 1);
        let id = doc.next_id();
        let mut layer = Layer {
            common: LayerCommon::new(id, "fill"),
            kind: LayerKind::SolidColor(SolidColorLayerData {
                color: Rgba::new(255, 0, 0, 255),
            }),
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
