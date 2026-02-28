use crate::blend::BlendMode;
use crate::blit::{Anchor, Rotation};
use crate::buffer::ImageBuffer;
use crate::filter::HslFilterConfig;
use crate::pixel::Rgba;

/// Unique layer identifier.
pub type LayerId = u32;

/// Common properties shared by all layer types.
#[derive(Debug, Clone)]
pub struct LayerCommon {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub opacity: f64,
    pub x: i32,
    pub y: i32,
    pub blend_mode: BlendMode,
    /// Optional grayscale mask. White = fully visible, black = fully hidden.
    pub mask: Option<ImageBuffer>,
    /// When true, this layer is clipped to the alpha of the layer directly below it.
    pub clip_to_below: bool,
}

impl LayerCommon {
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            opacity: 1.0,
            x: 0,
            y: 0,
            blend_mode: BlendMode::Normal,
            mask: None,
            clip_to_below: false,
        }
    }
}

/// Image layer with transform properties.
#[derive(Debug, Clone)]
pub struct ImageLayerData {
    pub buffer: ImageBuffer,
    pub anchor: Anchor,
    pub flip_x: bool,
    pub flip_y: bool,
    pub rotation: Rotation,
}

/// Paint layer — an editable RGBA buffer.
#[derive(Debug, Clone)]
pub struct PaintLayerData {
    pub buffer: ImageBuffer,
    pub anchor: Anchor,
}

/// Filter layer — non-destructive adjustment applied to layers beneath.
#[derive(Debug, Clone)]
pub struct FilterLayerData {
    pub config: HslFilterConfig,
}

/// Group layer — contains child layers.
#[derive(Debug, Clone)]
pub struct GroupLayerData {
    pub children: Vec<Layer>,
}

/// Solid color fill layer.
#[derive(Debug, Clone)]
pub struct SolidColorLayerData {
    pub color: Rgba,
}

/// A stop in a linear gradient.
#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    /// Position along the gradient, 0.0 to 1.0.
    pub position: f64,
    pub color: Rgba,
}

/// Gradient fill direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientDirection {
    /// Left to right.
    #[default]
    Horizontal,
    /// Top to bottom.
    Vertical,
    /// Top-left to bottom-right.
    DiagonalDown,
    /// Bottom-left to top-right.
    DiagonalUp,
}

/// Linear gradient fill layer.
#[derive(Debug, Clone)]
pub struct GradientLayerData {
    pub stops: Vec<GradientStop>,
    pub direction: GradientDirection,
}

/// A layer in the compositing document.
#[derive(Debug, Clone)]
pub struct Layer {
    pub common: LayerCommon,
    pub kind: LayerKind,
}

/// The specific data for each layer type.
#[derive(Debug, Clone)]
pub enum LayerKind {
    Image(ImageLayerData),
    Paint(PaintLayerData),
    Filter(FilterLayerData),
    Group(GroupLayerData),
    SolidColor(SolidColorLayerData),
    Gradient(GradientLayerData),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_common_defaults() {
        let c = LayerCommon::new(1, "test");
        assert_eq!(c.id, 1);
        assert_eq!(c.name, "test");
        assert!(c.visible);
        assert_eq!(c.opacity, 1.0);
        assert_eq!(c.x, 0);
        assert_eq!(c.y, 0);
        assert_eq!(c.blend_mode, BlendMode::Normal);
        assert!(c.mask.is_none());
        assert!(!c.clip_to_below);
    }
}
