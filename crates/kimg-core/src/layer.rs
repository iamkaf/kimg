//! Layer data types for the compositing document.
//!
//! Every layer in a [`Document`](crate::document::Document) is represented by a
//! [`Layer`] value that pairs a [`LayerCommon`] (shared properties: id, name,
//! opacity, blend mode, mask, position) with a [`LayerKind`] variant that carries
//! the type-specific data.
//!
//! | Variant | Description |
//! |---------|-------------|
//! | [`LayerKind::Image`] | An RGBA buffer with optional flip/rotation/anchor |
//! | [`LayerKind::Paint`] | An editable RGBA buffer |
//! | [`LayerKind::Filter`] | Non-destructive HSL/brightness/contrast adjustment |
//! | [`LayerKind::Group`] | A folder containing child layers |
//! | [`LayerKind::SolidColor`] | A flat color fill |
//! | [`LayerKind::Gradient`] | A linear color gradient fill |
//! | [`LayerKind::Shape`] | A rasterized vector-style shape primitive |
//! | [`LayerKind::Text`] | A rasterized text layer |

use crate::blend::BlendMode;
use crate::blit::Anchor;
use crate::buffer::ImageBuffer;
use crate::filter::HslFilterConfig;
use crate::pixel::Rgba;
use std::cell::RefCell;

/// Unique layer identifier.
pub type LayerId = u32;

/// Patch payload for filter-layer configuration.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct FilterLayerPatch {
    /// Hue offset in degrees.
    pub hue_deg: Option<f64>,
    /// Saturation delta, -1.0 to 1.0.
    pub saturation: Option<f64>,
    /// Lightness delta, -1.0 to 1.0.
    pub lightness: Option<f64>,
    /// Alpha delta, -1.0 to 1.0.
    pub alpha: Option<f64>,
    /// Brightness delta, -1.0 to 1.0.
    pub brightness: Option<f64>,
    /// Contrast delta, -1.0 to 1.0.
    pub contrast: Option<f64>,
    /// Temperature shift, -1.0 to 1.0.
    pub temperature: Option<f64>,
    /// Tint shift, -1.0 to 1.0.
    pub tint: Option<f64>,
    /// Sharpen strength, 0.0 to 1.0.
    pub sharpen: Option<f64>,
}

/// Patch payload for updating text-layer content and style.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct TextLayerPatch {
    /// Replace the layer text.
    pub text: Option<String>,
    /// Replace the text color.
    pub color: Option<Rgba>,
    /// Replace the requested font family.
    pub font_family: Option<String>,
    /// Replace the requested font weight.
    pub font_weight: Option<u16>,
    /// Replace the requested font style.
    pub font_style: Option<TextFontStyle>,
    /// Replace the requested text size in pixels.
    pub font_size: Option<u32>,
    /// Replace the line advance in pixels.
    pub line_height: Option<u32>,
    /// Replace the letter spacing in pixels.
    pub letter_spacing: Option<u32>,
    /// Replace the horizontal alignment in the text box.
    pub align: Option<TextAlign>,
    /// Replace the wrapping mode.
    pub wrap: Option<TextWrap>,
    /// Replace the optional text box width. `Some(None)` clears it.
    pub box_width: Option<Option<u32>>,
}

/// Patch payload for updating a layer through one stable mutation path.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct LayerPatch {
    /// Replace the layer name.
    pub name: Option<String>,
    /// Set layer visibility.
    pub visible: Option<bool>,
    /// Set layer opacity.
    pub opacity: Option<f64>,
    /// Set layer X position.
    pub x: Option<i32>,
    /// Set layer Y position.
    pub y: Option<i32>,
    /// Set the layer blend mode.
    pub blend_mode: Option<BlendMode>,
    /// Set whether the layer mask is inverted.
    pub mask_inverted: Option<bool>,
    /// Set whether the layer clips to the layer below it.
    pub clip_to_below: Option<bool>,
    /// Set the anchor for image/paint/shape layers.
    pub anchor: Option<Anchor>,
    /// Set horizontal flip for image/paint/shape layers.
    pub flip_x: Option<bool>,
    /// Set vertical flip for image/paint/shape layers.
    pub flip_y: Option<bool>,
    /// Set non-destructive rotation in degrees for image/paint/shape layers.
    pub rotation: Option<f64>,
    /// Set horizontal scale for image/paint/shape layers.
    pub scale_x: Option<f64>,
    /// Set vertical scale for image/paint/shape layers.
    pub scale_y: Option<f64>,
    /// Patch filter-layer configuration values.
    pub filter: Option<FilterLayerPatch>,
    /// Patch text-layer content and style.
    pub text: Option<TextLayerPatch>,
}

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
    /// When true, the mask luminance is inverted before application (black = visible, white = hidden).
    pub mask_inverted: bool,
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
            mask_inverted: false,
            clip_to_below: false,
        }
    }
}

/// Shared non-destructive transform data for rasterized layer kinds.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LayerTransform {
    /// Origin point for positioning the transformed bounds.
    pub anchor: Anchor,
    /// Flip along the X axis before scaling/rotation.
    pub flip_x: bool,
    /// Flip along the Y axis before scaling/rotation.
    pub flip_y: bool,
    /// Arbitrary clockwise rotation in degrees.
    pub rotation_deg: f64,
    /// Horizontal scale multiplier.
    pub scale_x: f64,
    /// Vertical scale multiplier.
    pub scale_y: f64,
}

impl LayerTransform {
    /// Create the default identity transform.
    pub const fn new() -> Self {
        Self {
            anchor: Anchor::TopLeft,
            flip_x: false,
            flip_y: false,
            rotation_deg: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

impl Default for LayerTransform {
    fn default() -> Self {
        Self::new()
    }
}

/// Image layer with transform properties.
#[derive(Debug)]
#[non_exhaustive]
pub struct ImageLayerData {
    /// The source image buffer.
    pub buffer: ImageBuffer,
    /// Shared non-destructive transform state.
    pub transform: LayerTransform,
    revision: u64,
    transformed_cache: RefCell<Option<RasterTransformCache>>,
}

impl ImageLayerData {
    /// Create an image layer with default transform properties (TopLeft anchor, no flip, no rotation).
    pub fn new(buffer: ImageBuffer) -> Self {
        Self {
            buffer,
            transform: LayerTransform::new(),
            revision: 0,
            transformed_cache: RefCell::new(None),
        }
    }

    /// Create an image layer with explicit transform properties.
    pub fn with_transform(buffer: ImageBuffer, transform: LayerTransform) -> Self {
        let mut layer = Self::new(buffer);
        layer.transform = transform;
        layer
    }

    /// Replace the source buffer and invalidate any cached transformed raster.
    pub fn set_buffer(&mut self, buffer: ImageBuffer) {
        self.buffer = buffer;
        self.bump_revision();
    }

    /// Mutate the source buffer in place and invalidate any cached transformed raster.
    pub fn mutate_buffer<R>(&mut self, mutate: impl FnOnce(&mut ImageBuffer) -> R) -> R {
        let result = mutate(&mut self.buffer);
        self.bump_revision();
        result
    }

    /// Reuse or refresh the transformed raster for the current source buffer and transform.
    pub fn with_cached_transformed_raster<F, R>(
        &self,
        render: F,
        use_buffer: impl FnOnce(&ImageBuffer) -> R,
    ) -> R
    where
        F: FnOnce() -> ImageBuffer,
    {
        let key = RasterTransformCacheKey::new(self.revision, self.transform);
        let needs_refresh = self
            .transformed_cache
            .borrow()
            .as_ref()
            .is_none_or(|cached| cached.key != key);

        if needs_refresh {
            let buffer = render();
            *self.transformed_cache.borrow_mut() = Some(RasterTransformCache { key, buffer });
        }

        let cached = self.transformed_cache.borrow();
        let entry = cached
            .as_ref()
            .expect("image transform cache should be populated");
        use_buffer(&entry.buffer)
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.transformed_cache.get_mut().take();
    }
}

impl Clone for ImageLayerData {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            transform: self.transform,
            revision: self.revision,
            transformed_cache: RefCell::new(None),
        }
    }
}

/// Paint layer — an editable RGBA buffer.
#[derive(Debug)]
#[non_exhaustive]
pub struct PaintLayerData {
    /// The editable pixel buffer.
    pub buffer: ImageBuffer,
    /// Shared non-destructive transform state.
    pub transform: LayerTransform,
    revision: u64,
    transformed_cache: RefCell<Option<RasterTransformCache>>,
}

impl PaintLayerData {
    /// Create a paint layer with a TopLeft anchor.
    pub fn new(buffer: ImageBuffer) -> Self {
        Self {
            buffer,
            transform: LayerTransform::new(),
            revision: 0,
            transformed_cache: RefCell::new(None),
        }
    }

    /// Create a paint layer with explicit transform properties.
    pub fn with_transform(buffer: ImageBuffer, transform: LayerTransform) -> Self {
        let mut layer = Self::new(buffer);
        layer.transform = transform;
        layer
    }

    /// Replace the source buffer and invalidate any cached transformed raster.
    pub fn set_buffer(&mut self, buffer: ImageBuffer) {
        self.buffer = buffer;
        self.bump_revision();
    }

    /// Mutate the source buffer in place and invalidate any cached transformed raster.
    pub fn mutate_buffer<R>(&mut self, mutate: impl FnOnce(&mut ImageBuffer) -> R) -> R {
        let result = mutate(&mut self.buffer);
        self.bump_revision();
        result
    }

    /// Reuse or refresh the transformed raster for the current source buffer and transform.
    pub fn with_cached_transformed_raster<F, R>(
        &self,
        render: F,
        use_buffer: impl FnOnce(&ImageBuffer) -> R,
    ) -> R
    where
        F: FnOnce() -> ImageBuffer,
    {
        let key = RasterTransformCacheKey::new(self.revision, self.transform);
        let needs_refresh = self
            .transformed_cache
            .borrow()
            .as_ref()
            .is_none_or(|cached| cached.key != key);

        if needs_refresh {
            let buffer = render();
            *self.transformed_cache.borrow_mut() = Some(RasterTransformCache { key, buffer });
        }

        let cached = self.transformed_cache.borrow();
        let entry = cached
            .as_ref()
            .expect("paint transform cache should be populated");
        use_buffer(&entry.buffer)
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.transformed_cache.get_mut().take();
    }
}

impl Clone for PaintLayerData {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            transform: self.transform,
            revision: self.revision,
            transformed_cache: RefCell::new(None),
        }
    }
}

/// Filter layer — non-destructive adjustment applied to layers beneath.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FilterLayerData {
    /// Configuration defining brightness, contrast, HSL shifts, etc.
    pub config: HslFilterConfig,
}

impl FilterLayerData {
    /// Create a filter layer with all adjustments at zero (identity).
    pub fn new() -> Self {
        Self {
            config: HslFilterConfig::default(),
        }
    }
}

impl Default for FilterLayerData {
    fn default() -> Self {
        Self::new()
    }
}

/// Group layer — contains child layers.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GroupLayerData {
    /// The child layers inside this group, ordered bottom to top.
    pub children: Vec<Layer>,
}

impl GroupLayerData {
    /// Create an empty group layer.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl Default for GroupLayerData {
    fn default() -> Self {
        Self::new()
    }
}

/// Solid color fill layer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SolidColorLayerData {
    /// The RGBA color to fill.
    pub color: Rgba,
}

impl SolidColorLayerData {
    /// Create a solid color layer with the given fill color.
    pub fn new(color: Rgba) -> Self {
        Self { color }
    }
}

/// A stop in a linear gradient.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct GradientStop {
    /// Position along the gradient, 0.0 to 1.0.
    pub position: f64,
    /// Color at this position.
    pub color: Rgba,
}

impl GradientStop {
    /// Create a gradient stop at the given position with the given color.
    pub fn new(position: f64, color: Rgba) -> Self {
        Self { position, color }
    }
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

impl GradientLayerData {
    /// Create a gradient layer with the given stops and direction.
    pub fn new(stops: Vec<GradientStop>, direction: GradientDirection) -> Self {
        Self { stops, direction }
    }
}

/// Font style for a text layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextFontStyle {
    /// Upright text.
    Normal,
    /// Italic text.
    Italic,
    /// Oblique text.
    Oblique,
}

impl TextFontStyle {
    /// Stable string form used in JS snapshots and serialization.
    pub const fn as_str(self) -> &'static str {
        match self {
            TextFontStyle::Normal => "normal",
            TextFontStyle::Italic => "italic",
            TextFontStyle::Oblique => "oblique",
        }
    }

    /// Parse a string into a font style, falling back to `Normal`.
    pub fn from_str_lossy(value: &str) -> Self {
        match value {
            "italic" => TextFontStyle::Italic,
            "oblique" => TextFontStyle::Oblique,
            _ => TextFontStyle::Normal,
        }
    }
}

/// Horizontal alignment for a text layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextAlign {
    /// Align text to the left edge of the text box.
    Left,
    /// Center text within the text box.
    Center,
    /// Align text to the right edge of the text box.
    Right,
}

impl TextAlign {
    /// Stable string form used in JS snapshots and serialization.
    pub const fn as_str(self) -> &'static str {
        match self {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        }
    }

    /// Parse a string into an alignment, falling back to `Left`.
    pub fn from_str_lossy(value: &str) -> Self {
        match value {
            "center" => TextAlign::Center,
            "right" => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }
}

/// Wrapping mode for a text layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextWrap {
    /// Do not wrap text automatically.
    None,
    /// Wrap text at word boundaries.
    Word,
}

impl TextWrap {
    /// Stable string form used in JS snapshots and serialization.
    pub const fn as_str(self) -> &'static str {
        match self {
            TextWrap::None => "none",
            TextWrap::Word => "word",
        }
    }

    /// Parse a string into a wrapping mode, falling back to `None`.
    pub fn from_str_lossy(value: &str) -> Self {
        match value {
            "word" => TextWrap::Word,
            _ => TextWrap::None,
        }
    }
}

/// Text layer data stored as styled content and rasterized at render time.
#[derive(Debug)]
#[non_exhaustive]
pub struct TextLayerData {
    /// Text content. Newlines create multiple lines.
    pub text: String,
    /// RGBA text color.
    pub color: Rgba,
    /// Requested font family.
    pub font_family: String,
    /// Requested font weight.
    pub font_weight: u16,
    /// Requested font style.
    pub font_style: TextFontStyle,
    /// Requested font size in pixels.
    pub font_size: u32,
    /// Line advance in pixels.
    pub line_height: u32,
    /// Letter spacing in pixels.
    pub letter_spacing: u32,
    /// Horizontal alignment within the optional text box.
    pub align: TextAlign,
    /// Wrapping mode for multiline text layout.
    pub wrap: TextWrap,
    /// Optional text box width used for wrapping and alignment.
    pub box_width: Option<u32>,
    /// Shared non-destructive transform state.
    pub transform: LayerTransform,
    raster_cache: RefCell<Option<TextRasterCache>>,
    transformed_cache: RefCell<Option<TextTransformedCache>>,
}

impl TextLayerData {
    /// Create a text layer with the provided content and style.
    pub fn new(
        text: impl Into<String>,
        color: Rgba,
        font_size: u32,
        line_height: u32,
        letter_spacing: u32,
    ) -> Self {
        let font_size = font_size.max(1);
        let line_height = line_height.max(font_size);
        Self {
            text: text.into(),
            color,
            font_family: "sans-serif".to_string(),
            font_weight: 400,
            font_style: TextFontStyle::Normal,
            font_size,
            line_height,
            letter_spacing,
            align: TextAlign::Left,
            wrap: TextWrap::None,
            box_width: None,
            transform: LayerTransform::new(),
            raster_cache: RefCell::new(None),
            transformed_cache: RefCell::new(None),
        }
    }

    pub(crate) fn cached_local_raster<F>(&self, render: F) -> ImageBuffer
    where
        F: FnOnce() -> ImageBuffer,
    {
        let key = TextRasterCacheKey::from_text(self);
        if let Some(cached) = self.raster_cache.borrow().as_ref() {
            if cached.key == key {
                return cached.buffer.clone();
            }
        }

        let buffer = render();
        *self.raster_cache.borrow_mut() = Some(TextRasterCache {
            key,
            buffer: buffer.clone(),
        });
        buffer
    }

    pub(crate) fn with_cached_transformed_raster<F, R>(
        &self,
        render: F,
        use_buffer: impl FnOnce(&ImageBuffer) -> R,
    ) -> R
    where
        F: FnOnce() -> ImageBuffer,
    {
        let key = TextTransformedCacheKey::from_text(self);
        let needs_refresh = self
            .transformed_cache
            .borrow()
            .as_ref()
            .is_none_or(|cached| cached.key != key);

        if needs_refresh {
            let buffer = render();
            *self.transformed_cache.borrow_mut() = Some(TextTransformedCache { key, buffer });
        }

        let cached = self.transformed_cache.borrow();
        let entry = cached
            .as_ref()
            .expect("transformed text cache should be populated");
        use_buffer(&entry.buffer)
    }
}

impl Clone for TextLayerData {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            color: self.color,
            font_family: self.font_family.clone(),
            font_weight: self.font_weight,
            font_style: self.font_style,
            font_size: self.font_size,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            align: self.align,
            wrap: self.wrap,
            box_width: self.box_width,
            transform: self.transform,
            raster_cache: RefCell::new(None),
            transformed_cache: RefCell::new(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextRasterCacheKey {
    text: String,
    color: Rgba,
    font_family: String,
    font_weight: u16,
    font_style: TextFontStyle,
    font_size: u32,
    line_height: u32,
    letter_spacing: u32,
    align: TextAlign,
    wrap: TextWrap,
    box_width: Option<u32>,
}

impl TextRasterCacheKey {
    fn from_text(text: &TextLayerData) -> Self {
        Self {
            text: text.text.clone(),
            color: text.color,
            font_family: text.font_family.clone(),
            font_weight: text.font_weight,
            font_style: text.font_style,
            font_size: text.font_size,
            line_height: text.line_height,
            letter_spacing: text.letter_spacing,
            align: text.align,
            wrap: text.wrap,
            box_width: text.box_width,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextTransformedCacheKey {
    raster: TextRasterCacheKey,
    anchor: Anchor,
    flip_x: bool,
    flip_y: bool,
    rotation_bits: u64,
    scale_x_bits: u64,
    scale_y_bits: u64,
}

impl TextTransformedCacheKey {
    fn from_text(text: &TextLayerData) -> Self {
        Self {
            raster: TextRasterCacheKey::from_text(text),
            anchor: text.transform.anchor,
            flip_x: text.transform.flip_x,
            flip_y: text.transform.flip_y,
            rotation_bits: text.transform.rotation_deg.to_bits(),
            scale_x_bits: text.transform.scale_x.to_bits(),
            scale_y_bits: text.transform.scale_y.to_bits(),
        }
    }
}

#[derive(Debug, Clone)]
struct TextRasterCache {
    key: TextRasterCacheKey,
    buffer: ImageBuffer,
}

#[derive(Debug, Clone)]
struct TextTransformedCache {
    key: TextTransformedCacheKey,
    buffer: ImageBuffer,
}

/// The primitive geometry for a shape layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeType {
    /// An axis-aligned rectangle.
    Rectangle,
    /// An axis-aligned rounded rectangle.
    RoundedRect,
    /// An axis-aligned ellipse.
    Ellipse,
    /// A straight line from the top-left to the bottom-right of the local bounds.
    Line,
    /// A closed polygon using the provided local points.
    Polygon,
}

impl ShapeType {
    /// Stable string form used in JS snapshots and serialization.
    pub fn as_str(self) -> &'static str {
        match self {
            ShapeType::Rectangle => "rectangle",
            ShapeType::RoundedRect => "rounded_rect",
            ShapeType::Ellipse => "ellipse",
            ShapeType::Line => "line",
            ShapeType::Polygon => "polygon",
        }
    }
}

/// A local point in a shape layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShapePoint {
    /// Horizontal coordinate in layer-local pixels.
    pub x: i32,
    /// Vertical coordinate in layer-local pixels.
    pub y: i32,
}

impl ShapePoint {
    /// Create a new local point.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Stroke style for a shape layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShapeStroke {
    /// RGBA stroke color.
    pub color: Rgba,
    /// Stroke width in pixels.
    pub width: u32,
}

impl ShapeStroke {
    /// Create a new stroke style.
    pub const fn new(color: Rgba, width: u32) -> Self {
        Self { color, width }
    }
}

/// Shape layer data stored as primitive parameters and rasterized at render time.
#[derive(Debug)]
#[non_exhaustive]
pub struct ShapeLayerData {
    /// Shape primitive type.
    pub shape_type: ShapeType,
    /// Local raster width in pixels.
    pub width: u32,
    /// Local raster height in pixels.
    pub height: u32,
    /// Corner radius for rounded rectangles.
    pub radius: u32,
    /// Optional fill color.
    pub fill: Option<Rgba>,
    /// Optional stroke style.
    pub stroke: Option<ShapeStroke>,
    /// Polygon points in local space. Ignored for non-polygon shapes.
    pub points: Vec<ShapePoint>,
    /// Shared non-destructive transform state.
    pub transform: LayerTransform,
    raster_cache: RefCell<Option<ShapeRasterCache>>,
    transformed_cache: RefCell<Option<ShapeTransformedCache>>,
}

impl ShapeLayerData {
    /// Create a new shape layer description.
    pub fn new(
        shape_type: ShapeType,
        width: u32,
        height: u32,
        radius: u32,
        fill: Option<Rgba>,
        stroke: Option<ShapeStroke>,
        points: Vec<ShapePoint>,
    ) -> Self {
        Self {
            shape_type,
            width,
            height,
            radius,
            fill,
            stroke,
            points,
            transform: LayerTransform::new(),
            raster_cache: RefCell::new(None),
            transformed_cache: RefCell::new(None),
        }
    }

    pub(crate) fn cached_local_raster<F>(&self, render: F) -> ImageBuffer
    where
        F: FnOnce() -> ImageBuffer,
    {
        let key = ShapeRasterCacheKey::from_shape(self);
        if let Some(cached) = self.raster_cache.borrow().as_ref() {
            if cached.key == key {
                return cached.buffer.clone();
            }
        }

        let buffer = render();
        *self.raster_cache.borrow_mut() = Some(ShapeRasterCache {
            key,
            buffer: buffer.clone(),
        });
        buffer
    }

    pub(crate) fn with_cached_transformed_raster<F, R>(
        &self,
        render: F,
        use_buffer: impl FnOnce(&ImageBuffer) -> R,
    ) -> R
    where
        F: FnOnce() -> ImageBuffer,
    {
        let key = ShapeTransformedCacheKey::from_shape(self);
        let needs_refresh = self
            .transformed_cache
            .borrow()
            .as_ref()
            .is_none_or(|cached| cached.key != key);

        if needs_refresh {
            let buffer = render();
            *self.transformed_cache.borrow_mut() = Some(ShapeTransformedCache { key, buffer });
        }

        let cached = self.transformed_cache.borrow();
        let entry = cached
            .as_ref()
            .expect("transformed shape cache should be populated");
        use_buffer(&entry.buffer)
    }
}

impl Clone for ShapeLayerData {
    fn clone(&self) -> Self {
        Self {
            shape_type: self.shape_type,
            width: self.width,
            height: self.height,
            radius: self.radius,
            fill: self.fill,
            stroke: self.stroke,
            points: self.points.clone(),
            transform: self.transform,
            raster_cache: RefCell::new(None),
            transformed_cache: RefCell::new(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShapeRasterCacheKey {
    shape_type: ShapeType,
    width: u32,
    height: u32,
    radius: u32,
    fill: Option<Rgba>,
    stroke: Option<ShapeStroke>,
    points: Vec<ShapePoint>,
}

impl ShapeRasterCacheKey {
    fn from_shape(shape: &ShapeLayerData) -> Self {
        Self {
            shape_type: shape.shape_type,
            width: shape.width,
            height: shape.height,
            radius: shape.radius,
            fill: shape.fill,
            stroke: shape.stroke,
            points: shape.points.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShapeTransformedCacheKey {
    raster: ShapeRasterCacheKey,
    anchor: Anchor,
    flip_x: bool,
    flip_y: bool,
    rotation_bits: u64,
    scale_x_bits: u64,
    scale_y_bits: u64,
}

impl ShapeTransformedCacheKey {
    fn from_shape(shape: &ShapeLayerData) -> Self {
        Self {
            raster: ShapeRasterCacheKey::from_shape(shape),
            anchor: shape.transform.anchor,
            flip_x: shape.transform.flip_x,
            flip_y: shape.transform.flip_y,
            rotation_bits: shape.transform.rotation_deg.to_bits(),
            scale_x_bits: shape.transform.scale_x.to_bits(),
            scale_y_bits: shape.transform.scale_y.to_bits(),
        }
    }
}

#[derive(Debug, Clone)]
struct ShapeRasterCache {
    key: ShapeRasterCacheKey,
    buffer: ImageBuffer,
}

#[derive(Debug, Clone)]
struct ShapeTransformedCache {
    key: ShapeTransformedCacheKey,
    buffer: ImageBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RasterTransformCacheKey {
    revision: u64,
    flip_x: bool,
    flip_y: bool,
    rotation_bits: u64,
    scale_x_bits: u64,
    scale_y_bits: u64,
}

impl RasterTransformCacheKey {
    fn new(revision: u64, transform: LayerTransform) -> Self {
        Self {
            revision,
            flip_x: transform.flip_x,
            flip_y: transform.flip_y,
            rotation_bits: transform.rotation_deg.to_bits(),
            scale_x_bits: transform.scale_x.to_bits(),
            scale_y_bits: transform.scale_y.to_bits(),
        }
    }
}

#[derive(Debug, Clone)]
struct RasterTransformCache {
    key: RasterTransformCacheKey,
    buffer: ImageBuffer,
}

/// A layer in the compositing document.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Layer {
    /// Common layer properties (id, name, opacity, blend_mode)
    pub common: LayerCommon,
    /// Type-specific data for the layer.
    pub kind: LayerKind,
}

impl Layer {
    /// Create a layer with the given common properties and kind.
    pub fn new(common: LayerCommon, kind: LayerKind) -> Self {
        Self { common, kind }
    }
}

/// The specific data for each layer type.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
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
    /// A rasterized shape primitive.
    Shape(ShapeLayerData),
    /// A rasterized text layer.
    Text(TextLayerData),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

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

    #[test]
    fn image_transformed_cache_reuses_and_invalidates() {
        let mut image = ImageLayerData::new(ImageBuffer::new_transparent(8, 8));
        image.transform.rotation_deg = 45.0;
        let calls = Cell::new(0);

        image.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 12)
            },
            Clone::clone,
        );
        image.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 12)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 1);

        image.mutate_buffer(|buffer| {
            buffer.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        });
        image.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 12)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn paint_transformed_cache_reuses_and_invalidates() {
        let mut paint = PaintLayerData::new(ImageBuffer::new_transparent(8, 8));
        paint.transform.scale_x = 1.5;
        let calls = Cell::new(0);

        paint.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 8)
            },
            Clone::clone,
        );
        paint.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 8)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 1);

        paint.set_buffer(ImageBuffer::new_transparent(10, 10));
        paint.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(15, 10)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn shape_local_cache_reuses_and_invalidates() {
        let shape = ShapeLayerData::new(
            ShapeType::Rectangle,
            8,
            8,
            0,
            Some(Rgba::new(255, 255, 255, 255)),
            None,
            vec![],
        );
        let calls = Cell::new(0);

        let first = shape.cached_local_raster(|| {
            calls.set(calls.get() + 1);
            ImageBuffer::new_transparent(8, 8)
        });
        let second = shape.cached_local_raster(|| {
            calls.set(calls.get() + 1);
            ImageBuffer::new_transparent(8, 8)
        });

        assert_eq!(calls.get(), 1);
        assert_eq!(first.data, second.data);

        let mut changed = shape.clone();
        changed.width = 9;
        changed.cached_local_raster(|| {
            calls.set(calls.get() + 1);
            ImageBuffer::new_transparent(9, 8)
        });
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn shape_transformed_cache_reuses_and_invalidates() {
        let mut shape = ShapeLayerData::new(
            ShapeType::Rectangle,
            8,
            8,
            0,
            Some(Rgba::new(255, 255, 255, 255)),
            None,
            vec![],
        );
        shape.transform.rotation_deg = 45.0;
        let calls = Cell::new(0);

        shape.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 12)
            },
            Clone::clone,
        );
        shape.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(12, 12)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 1);

        shape.transform.scale_x = 2.0;
        shape.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(24, 12)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn text_local_cache_reuses_and_invalidates() {
        let text = TextLayerData::new("Hello", Rgba::new(255, 255, 255, 255), 16, 18, 1);
        let calls = Cell::new(0);

        let first = text.cached_local_raster(|| {
            calls.set(calls.get() + 1);
            ImageBuffer::new_transparent(40, 16)
        });
        let second = text.cached_local_raster(|| {
            calls.set(calls.get() + 1);
            ImageBuffer::new_transparent(40, 16)
        });

        assert_eq!(calls.get(), 1);
        assert_eq!(first.data, second.data);

        let mut changed = text.clone();
        changed.text = "Hello!".into();
        changed.cached_local_raster(|| {
            calls.set(calls.get() + 1);
            ImageBuffer::new_transparent(48, 16)
        });
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn text_transformed_cache_reuses_and_invalidates() {
        let mut text = TextLayerData::new("Hi", Rgba::new(255, 255, 255, 255), 16, 18, 0);
        text.transform.rotation_deg = 22.0;
        let calls = Cell::new(0);

        text.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(32, 20)
            },
            Clone::clone,
        );
        text.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(32, 20)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 1);

        text.transform.scale_x = 1.5;
        text.with_cached_transformed_raster(
            || {
                calls.set(calls.get() + 1);
                ImageBuffer::new_transparent(48, 20)
            },
            Clone::clone,
        );
        assert_eq!(calls.get(), 2);
    }
}
