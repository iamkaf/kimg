use wasm_bindgen::prelude::*;

use kimg_core::blit::{Anchor, Rotation};
use kimg_core::buffer::ImageBuffer;
use kimg_core::document::Document as CoreDocument;
use kimg_core::filter::HslFilterConfig;
use kimg_core::layer::{
    FilterLayerData, GroupLayerData, ImageLayerData, Layer, LayerCommon, LayerKind,
    PaintLayerData,
};

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

    /// Add an image layer from raw RGBA data.
    /// Returns the layer ID.
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

    /// Add a paint layer (empty editable RGBA buffer).
    /// Returns the layer ID.
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

    /// Add an HSL filter layer.
    /// Returns the layer ID.
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
    /// Note: child layers must be added separately via the core API for now.
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

    /// Render the document and return the flat RGBA buffer as a `Uint8Array`.
    pub fn render(&self) -> Vec<u8> {
        let result = self.inner.render();
        result.data
    }

    /// Get the number of top-level layers.
    pub fn layer_count(&self) -> usize {
        self.inner.layers.len()
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
        assert_eq!(result[0], 255); // r
        assert_eq!(result[1], 0); // g
        assert_eq!(result[2], 0); // b
        assert_eq!(result[3], 255); // a
    }
}
