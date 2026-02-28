use crate::blend::blend_normal;
use crate::blit::{blit_transformed, BlitParams};
use crate::buffer::ImageBuffer;
use crate::filter::apply_hsl_filter;
use crate::layer::{GroupLayerData, Layer, LayerKind};

/// A compositing document with a canvas size and a layer tree.
#[derive(Debug, Clone)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    next_id: u32,
}

impl Document {
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

    /// Render the full document to a flat RGBA buffer.
    /// Traverses layers back-to-front (last in list = topmost).
    pub fn render(&self) -> ImageBuffer {
        let mut output = ImageBuffer::new_transparent(self.width, self.height);
        render_layers(&self.layers, &mut output);
        output
    }
}

/// Render a list of layers onto `output`, back-to-front.
/// Layers at the end of the vec are drawn on top.
fn render_layers(layers: &[Layer], output: &mut ImageBuffer) {
    for layer in layers {
        if !layer.common.visible {
            continue;
        }

        match &layer.kind {
            LayerKind::Image(img) => {
                blit_transformed(
                    output,
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
                    output,
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
            LayerKind::Filter(filter) => {
                apply_hsl_filter(output, &filter.config);
            }
            LayerKind::Group(group) => {
                render_group(group, output, layer.common.opacity);
            }
        }
    }
}

/// Render a group into an isolated buffer, then blend onto the output.
fn render_group(group: &GroupLayerData, output: &mut ImageBuffer, opacity: f64) {
    let mut group_buf = ImageBuffer::new_transparent(output.width, output.height);
    render_layers(&group.children, &mut group_buf);

    if opacity < 1.0 {
        // Scale the group buffer's alpha by the group opacity
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
    use crate::blit::{Anchor, Rotation};
    use crate::layer::*;
    use crate::pixel::Rgba;

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
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.fill(Rgba::new(255, 0, 0, 255));
        doc.layers.push(Layer {
            common: LayerCommon { x: 1, y: 1, ..LayerCommon::new(id, "red") },
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        });
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
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.fill(Rgba::new(255, 0, 0, 255));
        doc.layers.push(Layer {
            common: LayerCommon {
                visible: false,
                ..LayerCommon::new(id, "hidden")
            },
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        });
        let result = doc.render();
        assert_eq!(result.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn two_layers_blend_correctly() {
        let mut doc = Document::new(1, 1);

        // Bottom: blue
        let id1 = doc.next_id();
        let mut buf1 = ImageBuffer::new_transparent(1, 1);
        buf1.set_pixel(0, 0, Rgba::new(0, 0, 255, 255));
        doc.layers.push(Layer {
            common: LayerCommon::new(id1, "blue"),
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf1,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        });

        // Top: semi-transparent red
        let id2 = doc.next_id();
        let mut buf2 = ImageBuffer::new_transparent(1, 1);
        buf2.set_pixel(0, 0, Rgba::new(255, 0, 0, 128));
        doc.layers.push(Layer {
            common: LayerCommon::new(id2, "red"),
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf2,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        });

        let result = doc.render();
        let p = result.get_pixel(0, 0);
        // Semi-transparent red over opaque blue
        assert!(p.r > 100, "r={}", p.r);
        assert!(p.b > 100, "b={}", p.b);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn group_layer_renders_isolated() {
        let mut doc = Document::new(2, 2);

        // A group containing a red pixel
        let id1 = doc.next_id();
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));

        let child_id = doc.next_id();
        let child = Layer {
            common: LayerCommon::new(child_id, "red"),
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
            }),
        };

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
}
