use crate::blend::BlendMode;
use crate::blit::{Anchor, Rotation};
use crate::buffer::ImageBuffer;
use crate::filter::HslFilterConfig;
use crate::pixel::Rgba;

/// Unique layer identifier.
pub type LayerId = u32;

/// Common properties shared by all layer types.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayerCommon {
    /// Unique identifier for this layer in the document.
    pub id: LayerId,
    /// Human-readable layer name.
    pub name: String,
    /// Whether the layer should be rendered.
    pub visible: bool,
    /// Global opacity multiplier [0.0, 1.0].
    pub opacity: f64,
    /// X offset from the top-left of the canvas.
    pub x: i32,
    /// Y offset from the top-left of the canvas.
    pub y: i32,
    /// How this layer blends with the content below it.
    pub blend_mode: BlendMode,
    /// Optional grayscale mask. White = fully visible, black = fully hidden.
    pub mask: Option<ImageBuffer>,
    /// When true, this layer is clipped to the alpha of the layer directly below it.
    pub clip_to_below: bool,
}

impl LayerCommon {
    /// Create a new LayerCommon with default positioning, 100% opacity, and Normal blend mode.
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
#[non_exhaustive]
pub struct ImageLayerData {
    /// The source image buffer.
    pub buffer: ImageBuffer,
    /// Origin point for transforms (e.g. TopLeft vs Center)
    pub anchor: Anchor,
    /// Flip along the X axis.
    pub flip_x: bool,
    /// Flip along the Y axis.
    pub flip_y: bool,
    /// Orthogonal rotation (0, 90, 180, 270)
    pub rotation: Rotation,
}

/// Paint layer — an editable RGBA buffer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PaintLayerData {
    /// The editable pixel buffer.
    pub buffer: ImageBuffer,
    /// Origin point when applying document position offsets.
    pub anchor: Anchor,
}

/// Filter layer — non-destructive adjustment applied to layers beneath.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FilterLayerData {
    /// Configuration defining brightness, contrast, HSL shifts, etc.
    pub config: HslFilterConfig,
}

/// Group layer — contains child layers.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GroupLayerData {
    /// The child layers inside this group, ordered bottom to top.
    pub children: Vec<Layer>,
}

/// Solid color fill layer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SolidColorLayerData {
    /// The RGBA color to fill.
    pub color: Rgba,
}

/// A stop in a linear gradient.
#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    /// Position along the gradient, 0.0 to 1.0.
    pub position: f64,
    /// Color at this position.
    pub color: Rgba,
}

/// Gradient fill direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct GradientLayerData {
    /// The list of color stops.
    pub stops: Vec<GradientStop>,
    /// The direction of the linear gradient.
    pub direction: GradientDirection,
}

/// A layer in the compositing document.
#[derive(Debug, Clone)]
pub struct Layer {
    /// Common layer properties (id, name, opacity, blend_mode)
    pub common: LayerCommon,
    /// Type-specific data for the layer.
    pub kind: LayerKind,
}

/// The specific data for each layer type.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum LayerKind {
    /// An image buffer with transform properties.
    Image(ImageLayerData),
    /// An editable paint buffer.
    Paint(PaintLayerData),
    /// An adjustment layer.
    Filter(FilterLayerData),
    /// A folder for organizing child layers.
    Group(GroupLayerData),
    /// A solid color fill.
    SolidColor(SolidColorLayerData),
    /// A linear gradient fill.
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
