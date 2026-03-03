//! Document serialization and deserialization.
//!
//! Encodes a [`Document`] into a compact binary format suitable for persistence
//! and wire transfer, and decodes it back.
//!
//! # Format
//!
//! ```text
//! [4 bytes: metadata length (big-endian u32)] [metadata blob] [pixel buffers]
//! ```
//!
//! Metadata bytes are:
//!
//! ```text
//! [4 bytes: "KIMG"] [1 byte: metadata version] [postcard metadata]
//! ```
//!
//! Raw layer data (raster buffers, masks, and retained asset sources) is
//! appended as a flat byte sequence after the metadata blob; each metadata
//! entry includes offsets and lengths pointing into that region.

use crate::blend::BlendMode;
use crate::blit::Anchor;
use crate::buffer::ImageBuffer;
use crate::document::Document;
use crate::filter::HslFilterConfig;
use crate::layer::*;
use crate::pixel::Rgba;
use serde::{Deserialize, Serialize};

const BINARY_METADATA_MAGIC: [u8; 4] = *b"KIMG";
const BINARY_METADATA_VERSION: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentMetadata {
    width: u32,
    height: u32,
    next_id: u32,
    layers: Vec<LayerMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayerMetadata {
    common: LayerCommonMetadata,
    kind: LayerKindMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayerCommonMetadata {
    id: u32,
    name: String,
    visible: bool,
    opacity: f64,
    x: i32,
    y: i32,
    blend_mode: u8,
    mask: Option<BufferRefMetadata>,
    mask_inverted: bool,
    clip_to_below: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct BufferRefMetadata {
    width: u32,
    height: u32,
    buffer_offset: u64,
    buffer_len: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct BlobRefMetadata {
    offset: u64,
    len: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct LayerTransformMetadata {
    anchor: u8,
    flip_x: bool,
    flip_y: bool,
    rotation_deg: f64,
    scale_x: f64,
    scale_y: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct FilterMetadata {
    hue_deg: f64,
    saturation: f64,
    lightness: f64,
    alpha: f64,
    brightness: f64,
    contrast: f64,
    temperature: f64,
    tint: f64,
    sharpen: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct GradientStopMetadata {
    position: f64,
    color: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum FillMetadata {
    Solid {
        color: [u8; 4],
    },
    Gradient {
        direction: u8,
        stops: Vec<GradientStopMetadata>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ShapePointMetadata {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ShapeStrokeMetadata {
    color: [u8; 4],
    width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextMetadata {
    text: String,
    color: [u8; 4],
    font_family: String,
    font_weight: u16,
    font_style: u8,
    font_size: u32,
    line_height: u32,
    letter_spacing: u32,
    align: u8,
    wrap: u8,
    box_width: Option<u32>,
    transform: LayerTransformMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum LayerKindMetadata {
    Raster {
        buffer: BufferRefMetadata,
        transform: LayerTransformMetadata,
    },
    Filter {
        config: FilterMetadata,
    },
    Group {
        children: Vec<LayerMetadata>,
    },
    Fill {
        fill: FillMetadata,
    },
    Shape {
        shape_type: u8,
        width: u32,
        height: u32,
        radius: u32,
        fill: Option<[u8; 4]>,
        stroke: Option<ShapeStrokeMetadata>,
        points: Vec<ShapePointMetadata>,
        transform: LayerTransformMetadata,
    },
    Text {
        text: TextMetadata,
    },
    Svg {
        source: BlobRefMetadata,
        width: u32,
        height: u32,
        transform: LayerTransformMetadata,
    },
}

/// Errors that can occur during document serialization and deserialization.
#[derive(Debug)]
#[non_exhaustive]
pub enum SerializeError {
    /// The input data was invalid or corrupted.
    InvalidData(String),
}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializeError::InvalidData(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for SerializeError {}

/// Serialize a [`Document`] to the kimg binary format.
pub fn serialize(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let mut pixel_data = Vec::new();
    let metadata = build_document_metadata(doc, &mut pixel_data)?;
    let postcard_bytes = postcard::to_allocvec(&metadata)
        .map_err(|e| SerializeError::InvalidData(format!("metadata encode failed: {e}")))?;

    let mut metadata_bytes = Vec::with_capacity(5 + postcard_bytes.len());
    metadata_bytes.extend_from_slice(&BINARY_METADATA_MAGIC);
    metadata_bytes.push(BINARY_METADATA_VERSION);
    metadata_bytes.extend_from_slice(&postcard_bytes);

    let metadata_len: u32 = metadata_bytes
        .len()
        .try_into()
        .map_err(|_| SerializeError::InvalidData("metadata too large".into()))?;

    let mut output = Vec::with_capacity(4 + metadata_bytes.len() + pixel_data.len());
    output.extend_from_slice(&metadata_len.to_be_bytes());
    output.extend_from_slice(&metadata_bytes);
    output.extend_from_slice(&pixel_data);
    Ok(output)
}

/// Deserialize a [`Document`] from the kimg binary format.
pub fn deserialize(data: &[u8]) -> Result<Document, SerializeError> {
    let (metadata, pixel_data) = decode_binary_metadata(data)?;
    document_from_metadata(metadata, pixel_data)
}

/// Check whether serialized document data contains any SVG layers.
pub fn document_has_svg_layers(data: &[u8]) -> Result<bool, SerializeError> {
    let (metadata, _) = decode_binary_metadata(data)?;
    Ok(layer_metadata_has_svg(&metadata.layers))
}

fn decode_binary_metadata(data: &[u8]) -> Result<(DocumentMetadata, &[u8]), SerializeError> {
    if data.len() < 4 {
        return Err(SerializeError::InvalidData("data too short".into()));
    }

    let metadata_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let metadata_end = 4usize
        .checked_add(metadata_len)
        .ok_or_else(|| SerializeError::InvalidData("metadata length overflow".into()))?;
    if data.len() < metadata_end {
        return Err(SerializeError::InvalidData("truncated metadata".into()));
    }

    let metadata_bytes = &data[4..metadata_end];
    let pixel_data = &data[metadata_end..];

    if metadata_bytes.len() < BINARY_METADATA_MAGIC.len() + 1 {
        return Err(SerializeError::InvalidData(
            "truncated binary metadata header".into(),
        ));
    }
    if !metadata_bytes.starts_with(&BINARY_METADATA_MAGIC) {
        return Err(SerializeError::InvalidData("missing metadata magic".into()));
    }

    let version = metadata_bytes[BINARY_METADATA_MAGIC.len()];
    if version != BINARY_METADATA_VERSION {
        return Err(SerializeError::InvalidData(format!(
            "unsupported metadata version: {version}"
        )));
    }

    let metadata: DocumentMetadata = postcard::from_bytes(&metadata_bytes[5..])
        .map_err(|e| SerializeError::InvalidData(format!("metadata decode failed: {e}")))?;
    Ok((metadata, pixel_data))
}

fn layer_metadata_has_svg(layers: &[LayerMetadata]) -> bool {
    for layer in layers {
        match &layer.kind {
            LayerKindMetadata::Svg { .. } => return true,
            LayerKindMetadata::Group { children } => {
                if layer_metadata_has_svg(children) {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

fn min_next_id(doc: &Document) -> u32 {
    fn max_layer_id(layers: &[Layer]) -> u32 {
        let mut max_id = 0u32;
        for layer in layers {
            max_id = max_id.max(layer.common.id);
            if let LayerKind::Group(group) = &layer.kind {
                max_id = max_id.max(max_layer_id(&group.children));
            }
        }
        max_id
    }

    max_layer_id(&doc.layers) + 1
}

fn build_document_metadata(
    doc: &Document,
    pixel_data: &mut Vec<u8>,
) -> Result<DocumentMetadata, SerializeError> {
    let layers = doc
        .layers
        .iter()
        .map(|layer| build_layer_metadata(layer, pixel_data))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DocumentMetadata {
        width: doc.width,
        height: doc.height,
        next_id: doc.next_available_id().max(min_next_id(doc)),
        layers,
    })
}

fn build_layer_metadata(
    layer: &Layer,
    pixel_data: &mut Vec<u8>,
) -> Result<LayerMetadata, SerializeError> {
    let common = build_common_metadata(&layer.common, pixel_data)?;
    let kind = match &layer.kind {
        LayerKind::Raster(raster) => LayerKindMetadata::Raster {
            buffer: append_buffer(&raster.buffer, pixel_data)?,
            transform: transform_to_metadata(raster.transform),
        },
        LayerKind::Filter(filter) => LayerKindMetadata::Filter {
            config: filter_to_metadata(&filter.config),
        },
        LayerKind::Group(group) => LayerKindMetadata::Group {
            children: group
                .children
                .iter()
                .map(|child| build_layer_metadata(child, pixel_data))
                .collect::<Result<Vec<_>, _>>()?,
        },
        LayerKind::Fill(fill) => LayerKindMetadata::Fill {
            fill: fill_to_metadata(fill),
        },
        LayerKind::Shape(shape) => LayerKindMetadata::Shape {
            shape_type: shape_type_to_code(shape.shape_type),
            width: shape.width,
            height: shape.height,
            radius: shape.radius,
            fill: shape.fill.map(rgba_to_array),
            stroke: shape.stroke.map(|stroke| ShapeStrokeMetadata {
                color: rgba_to_array(stroke.color),
                width: stroke.width,
            }),
            points: shape
                .points
                .iter()
                .map(|point| ShapePointMetadata {
                    x: point.x,
                    y: point.y,
                })
                .collect(),
            transform: transform_to_metadata(shape.transform),
        },
        LayerKind::Text(text) => LayerKindMetadata::Text {
            text: TextMetadata {
                text: text.text.clone(),
                color: rgba_to_array(text.color),
                font_family: text.font_family.clone(),
                font_weight: text.font_weight,
                font_style: text_font_style_to_code(text.font_style),
                font_size: text.font_size,
                line_height: text.line_height,
                letter_spacing: text.letter_spacing,
                align: text_align_to_code(text.align),
                wrap: text_wrap_to_code(text.wrap),
                box_width: text.box_width,
                transform: transform_to_metadata(text.transform),
            },
        },
        LayerKind::Svg(svg) => LayerKindMetadata::Svg {
            source: append_blob(&svg.source, pixel_data)?,
            width: svg.width,
            height: svg.height,
            transform: transform_to_metadata(svg.transform),
        },
    };

    Ok(LayerMetadata { common, kind })
}

fn fill_to_metadata(fill: &FillLayerData) -> FillMetadata {
    match &fill.kind {
        FillKind::Solid { color } => FillMetadata::Solid {
            color: rgba_to_array(*color),
        },
        FillKind::Gradient { direction, stops } => FillMetadata::Gradient {
            direction: gradient_direction_to_code(*direction),
            stops: stops
                .iter()
                .map(|stop| GradientStopMetadata {
                    position: stop.position,
                    color: rgba_to_array(stop.color),
                })
                .collect(),
        },
    }
}

fn build_common_metadata(
    common: &LayerCommon,
    pixel_data: &mut Vec<u8>,
) -> Result<LayerCommonMetadata, SerializeError> {
    Ok(LayerCommonMetadata {
        id: common.id,
        name: common.name.clone(),
        visible: common.visible,
        opacity: common.opacity,
        x: common.x,
        y: common.y,
        blend_mode: blend_mode_to_code(common.blend_mode),
        mask: common
            .mask
            .as_ref()
            .map(|mask| append_buffer(mask, pixel_data))
            .transpose()?,
        mask_inverted: common.mask_inverted,
        clip_to_below: common.clip_to_below,
    })
}

fn append_buffer(
    buffer: &ImageBuffer,
    pixel_data: &mut Vec<u8>,
) -> Result<BufferRefMetadata, SerializeError> {
    let buffer_offset = u64::try_from(pixel_data.len())
        .map_err(|_| SerializeError::InvalidData("pixel data offset overflow".into()))?;
    let buffer_len = u64::try_from(buffer.data.len())
        .map_err(|_| SerializeError::InvalidData("pixel data length overflow".into()))?;

    pixel_data.extend_from_slice(&buffer.data);

    Ok(BufferRefMetadata {
        width: buffer.width,
        height: buffer.height,
        buffer_offset,
        buffer_len,
    })
}

fn append_blob(data: &[u8], pixel_data: &mut Vec<u8>) -> Result<BlobRefMetadata, SerializeError> {
    let offset = u64::try_from(pixel_data.len())
        .map_err(|_| SerializeError::InvalidData("blob data offset overflow".into()))?;
    let len = u64::try_from(data.len())
        .map_err(|_| SerializeError::InvalidData("blob data length overflow".into()))?;
    pixel_data.extend_from_slice(data);
    Ok(BlobRefMetadata { offset, len })
}

fn document_from_metadata(
    metadata: DocumentMetadata,
    pixel_data: &[u8],
) -> Result<Document, SerializeError> {
    let layers = metadata
        .layers
        .into_iter()
        .map(|layer| layer_from_metadata(layer, pixel_data))
        .collect::<Result<Vec<_>, _>>()?;

    let mut doc = Document::new(metadata.width, metadata.height);
    doc.layers = layers;
    doc.set_next_available_id(metadata.next_id.max(min_next_id(&doc)));
    Ok(doc)
}

fn layer_from_metadata(
    metadata: LayerMetadata,
    pixel_data: &[u8],
) -> Result<Layer, SerializeError> {
    let common = common_from_metadata(metadata.common, pixel_data)?;
    let kind = match metadata.kind {
        LayerKindMetadata::Raster { buffer, transform } => {
            LayerKind::Raster(RasterLayerData::with_transform(
                image_buffer_from_ref(buffer, pixel_data, "raster pixel data")?,
                transform_from_metadata(transform),
            ))
        }
        LayerKindMetadata::Filter { config } => LayerKind::Filter(FilterLayerData {
            config: filter_from_metadata(config),
        }),
        LayerKindMetadata::Group { children } => LayerKind::Group(GroupLayerData {
            children: children
                .into_iter()
                .map(|child| layer_from_metadata(child, pixel_data))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        LayerKindMetadata::Fill { fill } => LayerKind::Fill(fill_from_metadata(fill)),
        LayerKindMetadata::Shape {
            shape_type,
            width,
            height,
            radius,
            fill,
            stroke,
            points,
            transform,
        } => {
            let mut shape = ShapeLayerData::new(
                shape_type_from_code(shape_type),
                width,
                height,
                radius,
                fill.map(rgba_from_array),
                stroke.map(|stroke| ShapeStroke::new(rgba_from_array(stroke.color), stroke.width)),
                points
                    .into_iter()
                    .map(|point| ShapePoint::new(point.x, point.y))
                    .collect(),
            );
            shape.transform = transform_from_metadata(transform);
            LayerKind::Shape(shape)
        }
        LayerKindMetadata::Text { text } => {
            let mut layer = TextLayerData::new(
                text.text,
                rgba_from_array(text.color),
                text.font_size,
                text.line_height,
                text.letter_spacing,
            );
            layer.font_family = text.font_family;
            layer.font_weight = text.font_weight.max(1);
            layer.font_style = text_font_style_from_code(text.font_style);
            layer.align = text_align_from_code(text.align);
            layer.wrap = text_wrap_from_code(text.wrap);
            layer.box_width = text.box_width.map(|width| width.max(1));
            layer.transform = transform_from_metadata(text.transform);
            LayerKind::Text(layer)
        }
        LayerKindMetadata::Svg {
            source,
            width,
            height,
            transform,
        } => {
            let mut layer = SvgLayerData::new(
                checked_slice_u64(pixel_data, source.offset, source.len, "svg source")?.to_vec(),
                width,
                height,
            );
            layer.transform = transform_from_metadata(transform);
            LayerKind::Svg(layer)
        }
    };

    Ok(Layer { common, kind })
}

fn fill_from_metadata(metadata: FillMetadata) -> FillLayerData {
    match metadata {
        FillMetadata::Solid { color } => FillLayerData::solid(rgba_from_array(color)),
        FillMetadata::Gradient { direction, stops } => FillLayerData::gradient(
            stops
                .into_iter()
                .map(|stop| GradientStop {
                    position: stop.position,
                    color: rgba_from_array(stop.color),
                })
                .collect(),
            gradient_direction_from_code(direction),
        ),
    }
}

fn common_from_metadata(
    metadata: LayerCommonMetadata,
    pixel_data: &[u8],
) -> Result<LayerCommon, SerializeError> {
    let mut common = LayerCommon::new(metadata.id, metadata.name);
    common.visible = metadata.visible;
    common.opacity = metadata.opacity;
    common.x = metadata.x;
    common.y = metadata.y;
    common.blend_mode = blend_mode_from_code(metadata.blend_mode);
    common.mask = metadata
        .mask
        .map(|mask| image_buffer_from_ref(mask, pixel_data, "mask pixel data"))
        .transpose()?;
    common.mask_inverted = metadata.mask_inverted;
    common.clip_to_below = metadata.clip_to_below;
    Ok(common)
}

fn image_buffer_from_ref(
    buffer: BufferRefMetadata,
    pixel_data: &[u8],
    what: &str,
) -> Result<ImageBuffer, SerializeError> {
    let bytes = checked_slice_u64(pixel_data, buffer.buffer_offset, buffer.buffer_len, what)?;
    ImageBuffer::from_rgba(buffer.width, buffer.height, bytes.to_vec())
        .ok_or_else(|| SerializeError::InvalidData(format!("{what} size mismatch")))
}

fn checked_slice_u64<'a>(
    data: &'a [u8],
    offset: u64,
    len: u64,
    what: &str,
) -> Result<&'a [u8], SerializeError> {
    let offset: usize = offset
        .try_into()
        .map_err(|_| SerializeError::InvalidData(format!("{what} offset overflow")))?;
    let len: usize = len
        .try_into()
        .map_err(|_| SerializeError::InvalidData(format!("{what} length overflow")))?;
    let end = offset
        .checked_add(len)
        .ok_or_else(|| SerializeError::InvalidData(format!("{what} range overflow")))?;
    data.get(offset..end)
        .ok_or_else(|| SerializeError::InvalidData(format!("{what} out of bounds")))
}

fn filter_to_metadata(config: &HslFilterConfig) -> FilterMetadata {
    FilterMetadata {
        hue_deg: config.hue_deg,
        saturation: config.saturation,
        lightness: config.lightness,
        alpha: config.alpha,
        brightness: config.brightness,
        contrast: config.contrast,
        temperature: config.temperature,
        tint: config.tint,
        sharpen: config.sharpen,
    }
}

fn filter_from_metadata(config: FilterMetadata) -> HslFilterConfig {
    HslFilterConfig {
        hue_deg: config.hue_deg,
        saturation: config.saturation,
        lightness: config.lightness,
        alpha: config.alpha,
        brightness: config.brightness,
        contrast: config.contrast,
        temperature: config.temperature,
        tint: config.tint,
        sharpen: config.sharpen,
    }
}

fn transform_to_metadata(transform: LayerTransform) -> LayerTransformMetadata {
    LayerTransformMetadata {
        anchor: anchor_to_code(transform.anchor),
        flip_x: transform.flip_x,
        flip_y: transform.flip_y,
        rotation_deg: transform.rotation_deg,
        scale_x: transform.scale_x,
        scale_y: transform.scale_y,
    }
}

fn transform_from_metadata(transform: LayerTransformMetadata) -> LayerTransform {
    LayerTransform {
        anchor: anchor_from_code(transform.anchor),
        flip_x: transform.flip_x,
        flip_y: transform.flip_y,
        rotation_deg: transform.rotation_deg,
        scale_x: normalize_scale(transform.scale_x),
        scale_y: normalize_scale(transform.scale_y),
    }
}

fn rgba_to_array(color: Rgba) -> [u8; 4] {
    [color.r, color.g, color.b, color.a]
}

fn rgba_from_array(color: [u8; 4]) -> Rgba {
    Rgba::new(color[0], color[1], color[2], color[3])
}

fn blend_mode_to_code(mode: BlendMode) -> u8 {
    match mode {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay => 3,
        BlendMode::Darken => 4,
        BlendMode::Lighten => 5,
        BlendMode::ColorDodge => 6,
        BlendMode::ColorBurn => 7,
        BlendMode::HardLight => 8,
        BlendMode::SoftLight => 9,
        BlendMode::Difference => 10,
        BlendMode::Exclusion => 11,
        BlendMode::Hue => 12,
        BlendMode::Saturation => 13,
        BlendMode::Color => 14,
        BlendMode::Luminosity => 15,
    }
}

fn blend_mode_from_code(code: u8) -> BlendMode {
    match code {
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Overlay,
        4 => BlendMode::Darken,
        5 => BlendMode::Lighten,
        6 => BlendMode::ColorDodge,
        7 => BlendMode::ColorBurn,
        8 => BlendMode::HardLight,
        9 => BlendMode::SoftLight,
        10 => BlendMode::Difference,
        11 => BlendMode::Exclusion,
        12 => BlendMode::Hue,
        13 => BlendMode::Saturation,
        14 => BlendMode::Color,
        15 => BlendMode::Luminosity,
        _ => BlendMode::Normal,
    }
}

fn anchor_to_code(anchor: Anchor) -> u8 {
    match anchor {
        Anchor::TopLeft => 0,
        Anchor::Center => 1,
    }
}

fn anchor_from_code(code: u8) -> Anchor {
    match code {
        1 => Anchor::Center,
        _ => Anchor::TopLeft,
    }
}

fn gradient_direction_to_code(direction: GradientDirection) -> u8 {
    match direction {
        GradientDirection::Horizontal => 0,
        GradientDirection::Vertical => 1,
        GradientDirection::DiagonalDown => 2,
        GradientDirection::DiagonalUp => 3,
    }
}

fn gradient_direction_from_code(code: u8) -> GradientDirection {
    match code {
        1 => GradientDirection::Vertical,
        2 => GradientDirection::DiagonalDown,
        3 => GradientDirection::DiagonalUp,
        _ => GradientDirection::Horizontal,
    }
}

fn shape_type_to_code(shape_type: ShapeType) -> u8 {
    match shape_type {
        ShapeType::Rectangle => 0,
        ShapeType::Ellipse => 1,
        ShapeType::Line => 2,
        ShapeType::Polygon => 3,
    }
}

fn shape_type_from_code(code: u8) -> ShapeType {
    match code {
        1 => ShapeType::Ellipse,
        2 => ShapeType::Line,
        3 => ShapeType::Polygon,
        _ => ShapeType::Rectangle,
    }
}

fn text_font_style_to_code(style: TextFontStyle) -> u8 {
    match style {
        TextFontStyle::Normal => 0,
        TextFontStyle::Italic => 1,
        TextFontStyle::Oblique => 2,
    }
}

fn text_font_style_from_code(code: u8) -> TextFontStyle {
    match code {
        1 => TextFontStyle::Italic,
        2 => TextFontStyle::Oblique,
        _ => TextFontStyle::Normal,
    }
}

fn text_align_to_code(align: TextAlign) -> u8 {
    match align {
        TextAlign::Left => 0,
        TextAlign::Center => 1,
        TextAlign::Right => 2,
    }
}

fn text_align_from_code(code: u8) -> TextAlign {
    match code {
        1 => TextAlign::Center,
        2 => TextAlign::Right,
        _ => TextAlign::Left,
    }
}

fn text_wrap_to_code(wrap: TextWrap) -> u8 {
    match wrap {
        TextWrap::None => 0,
        TextWrap::Word => 1,
    }
}

fn text_wrap_from_code(code: u8) -> TextWrap {
    match code {
        1 => TextWrap::Word,
        _ => TextWrap::None,
    }
}

fn normalize_scale(value: f64) -> f64 {
    value.abs().max(f64::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_empty_document() {
        let doc = Document::new(10, 20);
        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();
        assert_eq!(restored.width, 10);
        assert_eq!(restored.height, 20);
        assert!(restored.layers.is_empty());
    }

    #[test]
    fn serialize_writes_binary_metadata_header() {
        let doc = Document::new(10, 20);
        let data = serialize(&doc).unwrap();

        assert_eq!(&data[4..8], &BINARY_METADATA_MAGIC);
        assert_eq!(data[8], BINARY_METADATA_VERSION);
    }

    #[test]
    fn serialize_raster_layer() {
        let mut doc = Document::new(4, 4);
        let id = doc.next_id();
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        buf.set_pixel(1, 1, Rgba::new(0, 255, 0, 128));
        doc.layers.push(Layer {
            common: LayerCommon {
                x: 1,
                y: 2,
                ..LayerCommon::new(id, "test raster")
            },
            kind: LayerKind::Raster(RasterLayerData::new(buf)),
        });

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();
        let layer = &restored.layers[0];

        assert_eq!(layer.common.name, "test raster");
        match &layer.kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.buffer.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
                assert_eq!(raster.buffer.get_pixel(1, 1), Rgba::new(0, 255, 0, 128));
            }
            _ => panic!("expected raster layer"),
        }
    }

    #[test]
    fn serialize_fill_layers() {
        let mut doc = Document::new(4, 4);
        let solid_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(solid_id, "solid"),
            kind: LayerKind::Fill(FillLayerData::solid(Rgba::new(100, 200, 50, 255))),
        });

        let gradient_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(gradient_id, "gradient"),
            kind: LayerKind::Fill(FillLayerData::gradient(
                vec![
                    GradientStop::new(0.0, Rgba::new(0, 0, 0, 255)),
                    GradientStop::new(1.0, Rgba::new(255, 255, 255, 255)),
                ],
                GradientDirection::Vertical,
            )),
        });

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();

        match &restored.layers[0].kind {
            LayerKind::Fill(FillLayerData {
                kind: FillKind::Solid { color },
            }) => assert_eq!(*color, Rgba::new(100, 200, 50, 255)),
            _ => panic!("expected solid fill"),
        }

        match &restored.layers[1].kind {
            LayerKind::Fill(FillLayerData {
                kind: FillKind::Gradient { stops, direction },
            }) => {
                assert_eq!(stops.len(), 2);
                assert_eq!(*direction, GradientDirection::Vertical);
            }
            _ => panic!("expected gradient fill"),
        }
    }

    #[test]
    fn serialize_shape_layer() {
        let mut doc = Document::new(8, 8);
        let id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon {
                x: 2,
                y: 3,
                ..LayerCommon::new(id, "shape")
            },
            kind: LayerKind::Shape(ShapeLayerData::new(
                ShapeType::Rectangle,
                5,
                4,
                2,
                Some(Rgba::new(255, 0, 0, 255)),
                Some(ShapeStroke::new(Rgba::new(255, 255, 255, 255), 1)),
                Vec::new(),
            )),
        });

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();
        match &restored.layers[0].kind {
            LayerKind::Shape(shape) => {
                assert_eq!(shape.shape_type, ShapeType::Rectangle);
                assert_eq!(shape.radius, 2);
            }
            _ => panic!("expected shape layer"),
        }
    }

    #[test]
    fn serialize_text_layer() {
        let mut doc = Document::new(32, 16);
        let id = doc.next_id();
        let mut layer = Layer {
            common: LayerCommon::new(id, "text"),
            kind: LayerKind::Text(TextLayerData::new(
                "Hello\nWorld",
                Rgba::new(12, 34, 56, 255),
                16,
                20,
                2,
            )),
        };
        if let LayerKind::Text(text) = &mut layer.kind {
            text.font_family = "Inter".into();
            text.font_weight = 700;
            text.font_style = TextFontStyle::Italic;
            text.align = TextAlign::Center;
            text.wrap = TextWrap::Word;
            text.box_width = Some(64);
        }
        doc.layers.push(layer);

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();
        match &restored.layers[0].kind {
            LayerKind::Text(text) => {
                assert_eq!(text.font_family, "Inter");
                assert_eq!(text.font_weight, 700);
                assert_eq!(text.font_style, TextFontStyle::Italic);
                assert_eq!(text.align, TextAlign::Center);
                assert_eq!(text.wrap, TextWrap::Word);
                assert_eq!(text.box_width, Some(64));
            }
            _ => panic!("expected text layer"),
        }
    }

    #[cfg(feature = "svg-backend")]
    #[test]
    fn serialize_svg_layer() {
        let mut doc = Document::new(32, 32);
        let id = doc.next_id();
        let mut layer = Layer {
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
        };
        if let LayerKind::Svg(svg) = &mut layer.kind {
            svg.transform.rotation_deg = 18.0;
            svg.transform.scale_x = 1.5;
            svg.transform.scale_y = 0.75;
        }
        doc.layers.push(layer);

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();
        match &restored.layers[0].kind {
            LayerKind::Svg(svg) => {
                assert_eq!(svg.width, 24);
                assert_eq!(svg.height, 24);
                assert_eq!(svg.transform.rotation_deg, 18.0);
                assert_eq!(svg.transform.scale_x, 1.5);
                assert_eq!(svg.transform.scale_y, 0.75);
                assert!(svg.source.starts_with(b"\n                    <svg"));
            }
            _ => panic!("expected svg layer"),
        }
    }

    #[test]
    fn serialize_preserves_common_and_transform_properties() {
        let mut doc = Document::new(4, 4);
        let id = doc.next_id();
        let buf = ImageBuffer::new_transparent(2, 2);
        let mut transform = LayerTransform::new();
        transform.anchor = Anchor::Center;
        transform.flip_x = true;
        transform.flip_y = true;
        transform.rotation_deg = 90.0;
        transform.scale_x = 1.5;
        transform.scale_y = 0.5;
        let mut layer = Layer {
            common: LayerCommon::new(id, "props"),
            kind: LayerKind::Raster(RasterLayerData::with_transform(buf, transform)),
        };
        layer.common.opacity = 0.75;
        layer.common.visible = false;
        layer.common.x = -5;
        layer.common.y = 10;
        layer.common.blend_mode = BlendMode::Multiply;
        layer.common.clip_to_below = true;
        doc.layers.push(layer);

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();
        let layer = &restored.layers[0];

        assert_eq!(layer.common.opacity, 0.75);
        assert!(!layer.common.visible);
        assert_eq!(layer.common.x, -5);
        assert_eq!(layer.common.y, 10);
        assert_eq!(layer.common.blend_mode, BlendMode::Multiply);
        assert!(layer.common.clip_to_below);

        match &layer.kind {
            LayerKind::Raster(raster) => {
                assert_eq!(raster.transform.anchor, Anchor::Center);
                assert!(raster.transform.flip_x);
                assert!(raster.transform.flip_y);
                assert_eq!(raster.transform.rotation_deg, 90.0);
                assert_eq!(raster.transform.scale_x, 1.5);
                assert_eq!(raster.transform.scale_y, 0.5);
            }
            _ => panic!("expected raster layer"),
        }
    }

    #[test]
    fn serialize_preserves_next_layer_id() {
        let mut doc = Document::new(8, 8);
        let first_id = doc.next_id();
        let second_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(first_id, "bottom"),
            kind: LayerKind::Fill(FillLayerData::solid(Rgba::new(10, 20, 30, 255))),
        });
        doc.layers.push(Layer {
            common: LayerCommon::new(second_id, "top"),
            kind: LayerKind::Fill(FillLayerData::solid(Rgba::new(40, 50, 60, 255))),
        });

        let reserved_id = doc.next_id();
        assert_eq!(reserved_id, 3);

        let data = serialize(&doc).unwrap();
        let mut restored = deserialize(&data).unwrap();
        let added_id = restored.next_id();

        assert_eq!(added_id, reserved_id + 1);
    }
}
