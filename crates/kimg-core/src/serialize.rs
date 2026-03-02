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
//! New documents use a compact binary metadata blob:
//!
//! ```text
//! [4 bytes: "KIMG"] [1 byte: metadata version] [postcard metadata]
//! ```
//!
//! Raw pixel data (image buffers and masks) is appended as a flat byte sequence
//! after the metadata blob; each buffer entry in the metadata includes
//! `buffer_offset` and `buffer_len` fields pointing into that region.
//!
//! # Backward compatibility
//!
//! Older documents with JSON metadata are still accepted on load.

use crate::blend::BlendMode;
use crate::blit::Anchor;
use crate::buffer::ImageBuffer;
use crate::document::Document;
use crate::filter::HslFilterConfig;
use crate::layer::*;
use crate::pixel::Rgba;
use serde::{Deserialize, Serialize};

const BINARY_METADATA_MAGIC: [u8; 4] = *b"KIMG";
const BINARY_METADATA_VERSION: u8 = 1;

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
enum LayerKindMetadata {
    Image {
        buffer: BufferRefMetadata,
        transform: LayerTransformMetadata,
    },
    Paint {
        buffer: BufferRefMetadata,
        transform: LayerTransformMetadata,
    },
    Filter {
        config: FilterMetadata,
    },
    Group {
        children: Vec<LayerMetadata>,
    },
    SolidColor {
        color: [u8; 4],
    },
    Gradient {
        direction: u8,
        stops: Vec<GradientStopMetadata>,
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
///
/// The returned bytes can be stored on disk or transmitted over a network and
/// later restored with [`deserialize`].
///
/// # Errors
///
/// Returns an error if the metadata or pixel sections exceed the supported
/// binary format limits.
pub fn serialize(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let mut pixel_data: Vec<u8> = Vec::new();
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
///
/// # Errors
///
/// Returns [`SerializeError::InvalidData`] if the input is truncated, the
/// metadata header is malformed, or any pixel buffer reference is out of bounds.
pub fn deserialize(data: &[u8]) -> Result<Document, SerializeError> {
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

    if metadata_bytes.starts_with(&BINARY_METADATA_MAGIC) {
        deserialize_binary_document(metadata_bytes, pixel_data)
    } else {
        deserialize_legacy_document(metadata_bytes, pixel_data)
    }
}

fn deserialize_binary_document(
    metadata_bytes: &[u8],
    pixel_data: &[u8],
) -> Result<Document, SerializeError> {
    if metadata_bytes.len() < BINARY_METADATA_MAGIC.len() + 1 {
        return Err(SerializeError::InvalidData(
            "truncated binary metadata header".into(),
        ));
    }

    let version = metadata_bytes[BINARY_METADATA_MAGIC.len()];
    if version != BINARY_METADATA_VERSION {
        return Err(SerializeError::InvalidData(format!(
            "unsupported metadata version: {version}"
        )));
    }

    let metadata: DocumentMetadata = postcard::from_bytes(&metadata_bytes[5..])
        .map_err(|e| SerializeError::InvalidData(format!("metadata decode failed: {e}")))?;
    document_from_metadata(metadata, pixel_data)
}

fn deserialize_legacy_document(
    metadata_bytes: &[u8],
    pixel_data: &[u8],
) -> Result<Document, SerializeError> {
    let json_str = std::str::from_utf8(metadata_bytes)
        .map_err(|e| SerializeError::InvalidData(e.to_string()))?;
    let doc_obj = parse_json_object(json_str).map_err(SerializeError::InvalidData)?;

    let width = get_json_u32(&doc_obj, "width")?;
    let height = get_json_u32(&doc_obj, "height")?;
    let next_id = get_json_u32(&doc_obj, "next_id")?;

    let layers_str = get_json_str(&doc_obj, "layers").map_err(SerializeError::InvalidData)?;
    let layers = deserialize_layers(layers_str, pixel_data)?;

    let mut doc = Document::new(width, height);
    doc.layers = layers;
    while get_next_id(&doc) < next_id {
        doc.next_id();
    }

    Ok(doc)
}

fn get_next_id(doc: &Document) -> u32 {
    fn max_id(layers: &[Layer]) -> u32 {
        let mut m = 0u32;
        for l in layers {
            m = m.max(l.common.id);
            if let LayerKind::Group(g) = &l.kind {
                m = m.max(max_id(&g.children));
            }
        }
        m
    }
    max_id(&doc.layers) + 1
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
        next_id: get_next_id(doc),
        layers,
    })
}

fn build_layer_metadata(
    layer: &Layer,
    pixel_data: &mut Vec<u8>,
) -> Result<LayerMetadata, SerializeError> {
    let common = build_common_metadata(&layer.common, pixel_data)?;
    let kind = match &layer.kind {
        LayerKind::Image(img) => LayerKindMetadata::Image {
            buffer: append_buffer(&img.buffer, pixel_data)?,
            transform: transform_to_metadata(img.transform),
        },
        LayerKind::Paint(paint) => LayerKindMetadata::Paint {
            buffer: append_buffer(&paint.buffer, pixel_data)?,
            transform: transform_to_metadata(paint.transform),
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
        LayerKind::SolidColor(solid) => LayerKindMetadata::SolidColor {
            color: rgba_to_array(solid.color),
        },
        LayerKind::Gradient(gradient) => LayerKindMetadata::Gradient {
            direction: gradient_direction_to_code(gradient.direction),
            stops: gradient
                .stops
                .iter()
                .map(|stop| GradientStopMetadata {
                    position: stop.position,
                    color: rgba_to_array(stop.color),
                })
                .collect(),
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
    };

    Ok(LayerMetadata { common, kind })
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
    while get_next_id(&doc) < metadata.next_id {
        doc.next_id();
    }
    Ok(doc)
}

fn layer_from_metadata(
    metadata: LayerMetadata,
    pixel_data: &[u8],
) -> Result<Layer, SerializeError> {
    let common = common_from_metadata(metadata.common, pixel_data)?;
    let kind = match metadata.kind {
        LayerKindMetadata::Image { buffer, transform } => LayerKind::Image(ImageLayerData {
            buffer: image_buffer_from_ref(buffer, pixel_data, "image pixel data")?,
            transform: transform_from_metadata(transform),
        }),
        LayerKindMetadata::Paint { buffer, transform } => LayerKind::Paint(PaintLayerData {
            buffer: image_buffer_from_ref(buffer, pixel_data, "paint pixel data")?,
            transform: transform_from_metadata(transform),
        }),
        LayerKindMetadata::Filter { config } => LayerKind::Filter(FilterLayerData {
            config: filter_from_metadata(config),
        }),
        LayerKindMetadata::Group { children } => LayerKind::Group(GroupLayerData {
            children: children
                .into_iter()
                .map(|child| layer_from_metadata(child, pixel_data))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        LayerKindMetadata::SolidColor { color } => LayerKind::SolidColor(SolidColorLayerData {
            color: rgba_from_array(color),
        }),
        LayerKindMetadata::Gradient { direction, stops } => {
            LayerKind::Gradient(GradientLayerData {
                direction: gradient_direction_from_code(direction),
                stops: stops
                    .into_iter()
                    .map(|stop| GradientStop {
                        position: stop.position,
                        color: rgba_from_array(stop.color),
                    })
                    .collect(),
            })
        }
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
    };

    Ok(Layer { common, kind })
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
    checked_slice(data, offset, len, what)
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
        ShapeType::RoundedRect => 1,
        ShapeType::Ellipse => 2,
        ShapeType::Line => 3,
        ShapeType::Polygon => 4,
    }
}

fn shape_type_from_code(code: u8) -> ShapeType {
    match code {
        1 => ShapeType::RoundedRect,
        2 => ShapeType::Ellipse,
        3 => ShapeType::Line,
        4 => ShapeType::Polygon,
        _ => ShapeType::Rectangle,
    }
}

// ── Minimal JSON parser ──
// Hand-written and retained only for legacy document compatibility.

type JsonObject = Vec<(String, String)>;

fn get_json_str<'a>(obj: &'a JsonObject, key: &str) -> Result<&'a str, String> {
    obj.iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| format!("missing key: {key}"))
}

fn get_json_u32(obj: &JsonObject, key: &str) -> Result<u32, SerializeError> {
    let v = get_json_str(obj, key).map_err(SerializeError::InvalidData)?;
    v.parse()
        .map_err(|_| SerializeError::InvalidData(format!("invalid u32: {v}")))
}

fn get_json_f64(obj: &JsonObject, key: &str) -> Result<f64, String> {
    let v = get_json_str(obj, key)?;
    v.parse().map_err(|_| format!("invalid f64: {v}"))
}

fn get_json_bool(obj: &JsonObject, key: &str) -> Result<bool, String> {
    let v = get_json_str(obj, key)?;
    match v {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid bool: {v}")),
    }
}

/// Parse a JSON object at the top level into key-value pairs.
/// Values are stored as raw strings (not recursively parsed).
fn parse_json_object(s: &str) -> Result<JsonObject, String> {
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err("expected JSON object".into());
    }
    let inner = &s[1..s.len() - 1];
    let mut result = Vec::new();
    let mut pos = 0;
    let bytes = inner.as_bytes();

    loop {
        // Skip whitespace
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        // Parse key
        if bytes[pos] != b'"' {
            return Err(format!("expected '\"' at pos {pos}"));
        }
        let (key, end) = parse_json_string_at(inner, pos)?;
        pos = end;

        // Skip ':'
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] != b':' {
            return Err(format!("expected ':' at pos {pos}"));
        }
        pos += 1;

        // Skip whitespace
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Parse value (as raw string)
        let (value, end) = parse_json_value_raw(inner, pos)?;
        pos = end;
        result.push((key, value));

        // Skip whitespace and comma
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b',' {
            pos += 1;
        }
    }

    Ok(result)
}

/// Parse a JSON string starting at `pos`, return (unescaped string, end position after closing quote)
fn parse_json_string_at(s: &str, pos: usize) -> Result<(String, usize), String> {
    let tail = s
        .get(pos..)
        .ok_or_else(|| format!("invalid string start at {pos}"))?;
    if !tail.starts_with('"') {
        return Err(format!("expected '\"' at {pos}"));
    }

    let mut chars = tail.char_indices();
    chars.next();
    let mut result = String::new();

    while let Some((rel_idx, ch)) = chars.next() {
        match ch {
            '\\' => {
                let Some((_, escaped)) = chars.next() else {
                    return Err("unterminated escape".into());
                };

                match escaped {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'u' => {
                        let mut hex = String::with_capacity(4);
                        for _ in 0..4 {
                            let Some((_, digit)) = chars.next() else {
                                return Err("incomplete unicode escape".into());
                            };
                            if !digit.is_ascii_hexdigit() {
                                return Err(format!("invalid unicode escape digit: {digit}"));
                            }
                            hex.push(digit);
                        }
                        let cp = u32::from_str_radix(&hex, 16)
                            .map_err(|_| format!("invalid unicode escape: {hex}"))?;
                        let decoded = char::from_u32(cp)
                            .ok_or_else(|| format!("invalid unicode codepoint: {hex}"))?;
                        result.push(decoded);
                    }
                    other => result.push(other),
                }
            }
            '"' => return Ok((result, pos + rel_idx + 1)),
            _ => result.push(ch),
        }
    }
    Err("unterminated string".into())
}

/// Parse a raw JSON value at position, returning (raw_text, end_position).
/// Handles strings, numbers, bools, arrays, objects.
fn parse_json_value_raw(s: &str, pos: usize) -> Result<(String, usize), String> {
    let bytes = s.as_bytes();
    if pos >= bytes.len() {
        return Err("unexpected end of input".into());
    }

    match bytes[pos] {
        b'"' => {
            let (val, end) = parse_json_string_at(s, pos)?;
            Ok((val, end))
        }
        b'[' => {
            let end = find_matching_bracket(s, pos, b'[', b']')?;
            Ok((s[pos..end].to_string(), end))
        }
        b'{' => {
            let end = find_matching_bracket(s, pos, b'{', b'}')?;
            Ok((s[pos..end].to_string(), end))
        }
        _ => {
            // number, bool, null
            let mut end = pos;
            while end < bytes.len()
                && bytes[end] != b','
                && bytes[end] != b'}'
                && bytes[end] != b']'
                && !bytes[end].is_ascii_whitespace()
            {
                end += 1;
            }
            Ok((s[pos..end].to_string(), end))
        }
    }
}

fn find_matching_bracket(s: &str, pos: usize, open: u8, close: u8) -> Result<usize, String> {
    let bytes = s.as_bytes();
    let mut depth = 0;
    let mut i = pos;
    let mut in_string = false;
    while i < bytes.len() {
        if in_string {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
        } else if bytes[i] == b'"' {
            in_string = true;
        } else if bytes[i] == open {
            depth += 1;
        } else if bytes[i] == close {
            depth -= 1;
            if depth == 0 {
                return Ok(i + 1);
            }
        }
        i += 1;
    }
    Err(format!("unmatched bracket at {pos}"))
}

/// Parse a JSON array of objects, returning each object as a raw string.
fn parse_json_array(s: &str) -> Result<Vec<String>, String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Err("expected JSON array".into());
    }
    let inner = &s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let mut pos = 0;
    let bytes = inner.as_bytes();

    loop {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        let (val, end) = parse_json_value_raw(inner, pos)?;
        items.push(val);
        pos = end;

        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b',' {
            pos += 1;
        }
    }

    Ok(items)
}

/// Parse a JSON array of u8 values like "[255,0,0,255]".
fn parse_json_u8_array(s: &str) -> Result<Vec<u8>, String> {
    let items = parse_json_array(s)?;
    items
        .iter()
        .map(|v| v.parse::<u8>().map_err(|_| format!("invalid u8: {v}")))
        .collect()
}

// ── Deserialization helpers ──

fn ensure_array_wrapped(s: &str) -> String {
    if s.trim().starts_with('[') {
        s.to_string()
    } else {
        format!("[{}]", s)
    }
}

fn ensure_object_wrapped(s: &str) -> String {
    if s.trim().starts_with('{') {
        s.to_string()
    } else {
        format!("{{{}}}", s)
    }
}

fn checked_slice<'a>(
    data: &'a [u8],
    offset: usize,
    len: usize,
    what: &str,
) -> Result<&'a [u8], SerializeError> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| SerializeError::InvalidData(format!("{what} range overflow")))?;

    data.get(offset..end)
        .ok_or_else(|| SerializeError::InvalidData(format!("{what} out of bounds")))
}

fn deserialize_layers(s: &str, pixel_data: &[u8]) -> Result<Vec<Layer>, SerializeError> {
    let wrapped = ensure_array_wrapped(s);
    let raw_items = parse_json_array(&wrapped).map_err(SerializeError::InvalidData)?;
    let mut layers = Vec::new();
    for item in &raw_items {
        layers.push(deserialize_layer(item, pixel_data)?);
    }
    Ok(layers)
}

fn deserialize_layer(s: &str, pixel_data: &[u8]) -> Result<Layer, SerializeError> {
    // s may be a full object "{...}" or inner content; ensure it's wrapped
    let wrapped = if s.trim().starts_with('{') {
        s.to_string()
    } else {
        format!("{{{}}}", s)
    };
    let obj = parse_json_object(&wrapped).map_err(SerializeError::InvalidData)?;

    let e = |s: String| SerializeError::InvalidData(s);

    let id = get_json_u32(&obj, "id")?;
    let name = get_json_str(&obj, "name").map_err(e)?.to_string();
    let visible = get_json_bool(&obj, "visible").map_err(e)?;
    let opacity = get_json_f64(&obj, "opacity").map_err(e)?;
    let x: i32 = get_json_str(&obj, "x")
        .map_err(e)?
        .parse()
        .map_err(|_| SerializeError::InvalidData("invalid x".into()))?;
    let y: i32 = get_json_str(&obj, "y")
        .map_err(e)?
        .parse()
        .map_err(|_| SerializeError::InvalidData("invalid y".into()))?;
    let blend_mode_str = get_json_str(&obj, "blend_mode").map_err(e)?;
    let blend_mode = BlendMode::from_str_lossy(blend_mode_str);
    let clip_to_below = get_json_bool(&obj, "clip_to_below").map_err(e)?;
    // mask_inverted is optional for backward compatibility with older serialized documents
    let mask_inverted = get_json_bool(&obj, "mask_inverted").unwrap_or(false);

    let mask = if let Ok(mask_str) = get_json_str(&obj, "mask") {
        let mask_wrapped = ensure_object_wrapped(mask_str);
        let mask_obj = parse_json_object(&mask_wrapped).map_err(SerializeError::InvalidData)?;
        let mw = get_json_u32(&mask_obj, "width")?;
        let mh = get_json_u32(&mask_obj, "height")?;
        let moff: usize = get_json_str(&mask_obj, "buffer_offset")
            .map_err(SerializeError::InvalidData)?
            .parse()
            .map_err(|_| SerializeError::InvalidData("invalid mask offset".into()))?;
        let mlen: usize = get_json_str(&mask_obj, "buffer_len")
            .map_err(SerializeError::InvalidData)?
            .parse()
            .map_err(|_| SerializeError::InvalidData("invalid mask len".into()))?;
        let mask_bytes = checked_slice(pixel_data, moff, mlen, "mask pixel data")?;
        ImageBuffer::from_rgba(mw, mh, mask_bytes.to_vec())
    } else {
        None
    };

    let mut common = LayerCommon::new(id, name);
    common.visible = visible;
    common.opacity = opacity;
    common.x = x;
    common.y = y;
    common.blend_mode = blend_mode;
    common.clip_to_below = clip_to_below;
    common.mask_inverted = mask_inverted;
    common.mask = mask;

    let kind_str = get_json_str(&obj, "kind").map_err(e)?;
    let kind = match kind_str {
        "image" => {
            let w = get_json_u32(&obj, "width")?;
            let h = get_json_u32(&obj, "height")?;
            let offset = parse_usize(&obj, "buffer_offset")?;
            let len = parse_usize(&obj, "buffer_len")?;
            let buffer_bytes = checked_slice(pixel_data, offset, len, "pixel data")?;
            let buffer = ImageBuffer::from_rgba(w, h, buffer_bytes.to_vec())
                .ok_or_else(|| SerializeError::InvalidData("buffer size mismatch".into()))?;
            LayerKind::Image(ImageLayerData {
                buffer,
                transform: parse_transform(&obj),
            })
        }
        "paint" => {
            let w = get_json_u32(&obj, "width")?;
            let h = get_json_u32(&obj, "height")?;
            let offset = parse_usize(&obj, "buffer_offset")?;
            let len = parse_usize(&obj, "buffer_len")?;
            let buffer_bytes = checked_slice(pixel_data, offset, len, "pixel data")?;
            let buffer = ImageBuffer::from_rgba(w, h, buffer_bytes.to_vec())
                .ok_or_else(|| SerializeError::InvalidData("buffer size mismatch".into()))?;
            LayerKind::Paint(PaintLayerData {
                buffer,
                transform: parse_transform(&obj),
            })
        }
        "filter" => {
            let config = HslFilterConfig {
                hue_deg: get_json_f64(&obj, "hue_deg").unwrap_or(0.0),
                saturation: get_json_f64(&obj, "saturation").unwrap_or(0.0),
                lightness: get_json_f64(&obj, "lightness").unwrap_or(0.0),
                alpha: get_json_f64(&obj, "alpha").unwrap_or(0.0),
                brightness: get_json_f64(&obj, "brightness").unwrap_or(0.0),
                contrast: get_json_f64(&obj, "contrast").unwrap_or(0.0),
                temperature: get_json_f64(&obj, "temperature").unwrap_or(0.0),
                tint: get_json_f64(&obj, "tint").unwrap_or(0.0),
                sharpen: get_json_f64(&obj, "sharpen").unwrap_or(0.0),
            };
            LayerKind::Filter(FilterLayerData { config })
        }
        "group" => {
            let children_str = get_json_str(&obj, "children").map_err(e)?;
            let children_wrapped = ensure_array_wrapped(children_str);
            let children = deserialize_layers(&children_wrapped, pixel_data)?;
            LayerKind::Group(GroupLayerData { children })
        }
        "solid_color" => {
            let color_str = get_json_str(&obj, "color").map_err(e)?;
            let color_wrapped = if color_str.trim().starts_with('[') {
                color_str.to_string()
            } else {
                format!("[{}]", color_str)
            };
            let rgba = parse_json_u8_array(&color_wrapped).map_err(SerializeError::InvalidData)?;
            if rgba.len() != 4 {
                return Err(SerializeError::InvalidData(
                    "solid_color needs 4 values".into(),
                ));
            }
            LayerKind::SolidColor(SolidColorLayerData {
                color: Rgba::new(rgba[0], rgba[1], rgba[2], rgba[3]),
            })
        }
        "gradient" => {
            let dir_str = get_json_str(&obj, "direction").unwrap_or("horizontal");
            let direction = match dir_str {
                "vertical" => GradientDirection::Vertical,
                "diagonal_down" => GradientDirection::DiagonalDown,
                "diagonal_up" => GradientDirection::DiagonalUp,
                _ => GradientDirection::Horizontal,
            };
            let stops_str = get_json_str(&obj, "stops").map_err(e)?;
            let stops_wrapped = ensure_array_wrapped(stops_str);
            let stop_items =
                parse_json_array(&stops_wrapped).map_err(SerializeError::InvalidData)?;
            let mut stops = Vec::new();
            for item in &stop_items {
                let stop_wrapped = ensure_object_wrapped(item);
                let stop_obj =
                    parse_json_object(&stop_wrapped).map_err(SerializeError::InvalidData)?;
                let position =
                    get_json_f64(&stop_obj, "position").map_err(SerializeError::InvalidData)?;
                let color_str =
                    get_json_str(&stop_obj, "color").map_err(SerializeError::InvalidData)?;
                let color_wrapped = ensure_array_wrapped(color_str);
                let rgba =
                    parse_json_u8_array(&color_wrapped).map_err(SerializeError::InvalidData)?;
                if rgba.len() != 4 {
                    return Err(SerializeError::InvalidData(
                        "gradient stop needs 4 values".into(),
                    ));
                }
                stops.push(GradientStop {
                    position,
                    color: Rgba::new(rgba[0], rgba[1], rgba[2], rgba[3]),
                });
            }
            LayerKind::Gradient(GradientLayerData { stops, direction })
        }
        "shape" => {
            let shape_type =
                parse_shape_type(get_json_str(&obj, "shape_type").unwrap_or("rectangle"));
            let width = get_json_u32(&obj, "width")?;
            let height = get_json_u32(&obj, "height")?;
            let radius = get_json_u32(&obj, "radius").unwrap_or(0);

            let fill = if let Ok(fill_str) = get_json_str(&obj, "fill") {
                let fill = parse_json_u8_array(&ensure_array_wrapped(fill_str))
                    .map_err(SerializeError::InvalidData)?;
                if fill.len() != 4 {
                    return Err(SerializeError::InvalidData(
                        "shape fill needs 4 values".into(),
                    ));
                }
                Some(Rgba::new(fill[0], fill[1], fill[2], fill[3]))
            } else {
                None
            };

            let stroke = if let Ok(stroke_str) = get_json_str(&obj, "stroke") {
                let stroke_obj = parse_json_object(&ensure_object_wrapped(stroke_str))
                    .map_err(SerializeError::InvalidData)?;
                let stroke_width = get_json_u32(&stroke_obj, "width")?;
                let color = parse_json_u8_array(&ensure_array_wrapped(
                    get_json_str(&stroke_obj, "color").map_err(SerializeError::InvalidData)?,
                ))
                .map_err(SerializeError::InvalidData)?;
                if color.len() != 4 {
                    return Err(SerializeError::InvalidData(
                        "shape stroke color needs 4 values".into(),
                    ));
                }
                Some(ShapeStroke::new(
                    Rgba::new(color[0], color[1], color[2], color[3]),
                    stroke_width,
                ))
            } else {
                None
            };

            let points = if let Ok(points_str) = get_json_str(&obj, "points") {
                let point_items = parse_json_array(&ensure_array_wrapped(points_str))
                    .map_err(SerializeError::InvalidData)?;
                let mut points = Vec::with_capacity(point_items.len());
                for item in &point_items {
                    let point_obj = parse_json_object(&ensure_object_wrapped(item))
                        .map_err(SerializeError::InvalidData)?;
                    let x: i32 = get_json_str(&point_obj, "x")
                        .map_err(SerializeError::InvalidData)?
                        .parse()
                        .map_err(|_| SerializeError::InvalidData("invalid shape point x".into()))?;
                    let y: i32 = get_json_str(&point_obj, "y")
                        .map_err(SerializeError::InvalidData)?
                        .parse()
                        .map_err(|_| SerializeError::InvalidData("invalid shape point y".into()))?;
                    points.push(ShapePoint::new(x, y));
                }
                points
            } else {
                Vec::new()
            };

            let mut shape =
                ShapeLayerData::new(shape_type, width, height, radius, fill, stroke, points);
            shape.transform = parse_transform(&obj);
            LayerKind::Shape(shape)
        }
        other => {
            return Err(SerializeError::InvalidData(format!(
                "unknown layer kind: {other}"
            )));
        }
    };

    Ok(Layer { common, kind })
}

fn parse_usize(obj: &JsonObject, key: &str) -> Result<usize, SerializeError> {
    let v = get_json_str(obj, key).map_err(SerializeError::InvalidData)?;
    v.parse()
        .map_err(|_| SerializeError::InvalidData(format!("invalid usize: {v}")))
}

fn parse_anchor(s: &str) -> Anchor {
    match s {
        "center" => Anchor::Center,
        _ => Anchor::TopLeft,
    }
}

fn parse_rotation_degrees(s: &str) -> f64 {
    match s {
        "cw90" => 90.0,
        "cw180" => 180.0,
        "cw270" => 270.0,
        "none" => 0.0,
        other => other.parse().unwrap_or(0.0),
    }
}

fn normalize_scale(value: f64) -> f64 {
    value.abs().max(f64::EPSILON)
}

fn parse_transform(obj: &JsonObject) -> LayerTransform {
    let rotation_deg = get_json_f64(obj, "rotation_deg").unwrap_or_else(|_| {
        get_json_str(obj, "rotation")
            .map(parse_rotation_degrees)
            .unwrap_or(0.0)
    });

    LayerTransform {
        anchor: parse_anchor(get_json_str(obj, "anchor").unwrap_or("top_left")),
        flip_x: get_json_bool(obj, "flip_x").unwrap_or(false),
        flip_y: get_json_bool(obj, "flip_y").unwrap_or(false),
        rotation_deg,
        scale_x: normalize_scale(get_json_f64(obj, "scale_x").unwrap_or(1.0)),
        scale_y: normalize_scale(get_json_f64(obj, "scale_y").unwrap_or(1.0)),
    }
}

fn parse_shape_type(s: &str) -> ShapeType {
    match s {
        "rounded_rect" | "roundedRect" => ShapeType::RoundedRect,
        "ellipse" => ShapeType::Ellipse,
        "line" => ShapeType::Line,
        "polygon" => ShapeType::Polygon,
        _ => ShapeType::Rectangle,
    }
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
        assert_eq!(restored.layers.len(), 0);
    }

    #[test]
    fn serialize_writes_binary_metadata_header() {
        let doc = Document::new(10, 20);
        let data = serialize(&doc).unwrap();

        assert_eq!(&data[4..8], &BINARY_METADATA_MAGIC);
        assert_eq!(data[8], BINARY_METADATA_VERSION);
    }

    #[test]
    fn serialize_with_image_layers() {
        let mut doc = Document::new(4, 4);
        let id = doc.next_id();
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        buf.set_pixel(1, 1, Rgba::new(0, 255, 0, 128));
        doc.layers.push(Layer {
            common: LayerCommon {
                x: 1,
                y: 2,
                ..LayerCommon::new(id, "test image")
            },
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                transform: LayerTransform::new(),
            }),
        });

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();

        assert_eq!(restored.width, 4);
        assert_eq!(restored.height, 4);
        assert_eq!(restored.layers.len(), 1);

        let layer = &restored.layers[0];
        assert_eq!(layer.common.id, id);
        assert_eq!(layer.common.name, "test image");
        assert_eq!(layer.common.x, 1);
        assert_eq!(layer.common.y, 2);
        if let LayerKind::Image(img) = &layer.kind {
            assert_eq!(img.buffer.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
            assert_eq!(img.buffer.get_pixel(1, 1), Rgba::new(0, 255, 0, 128));
        } else {
            panic!("expected image layer");
        }
    }

    #[test]
    fn serialize_with_all_layer_types() {
        let mut doc = Document::new(4, 4);

        // Solid color
        let id1 = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id1, "solid"),
            kind: LayerKind::SolidColor(SolidColorLayerData {
                color: Rgba::new(100, 200, 50, 255),
            }),
        });

        // Gradient
        let id2 = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id2, "gradient"),
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
                direction: GradientDirection::Vertical,
            }),
        });

        // Filter
        let id3 = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id3, "filter"),
            kind: LayerKind::Filter(FilterLayerData {
                config: HslFilterConfig {
                    hue_deg: 45.0,
                    saturation: 0.1,
                    ..Default::default()
                },
            }),
        });

        // Group with a child
        let id4 = doc.next_id();
        let child_id = doc.next_id();
        doc.layers.push(Layer {
            common: LayerCommon::new(id4, "group"),
            kind: LayerKind::Group(GroupLayerData {
                children: vec![Layer {
                    common: LayerCommon::new(child_id, "child"),
                    kind: LayerKind::SolidColor(SolidColorLayerData {
                        color: Rgba::new(10, 20, 30, 40),
                    }),
                }],
            }),
        });

        let data = serialize(&doc).unwrap();
        let restored = deserialize(&data).unwrap();

        assert_eq!(restored.layers.len(), 4);
        assert!(matches!(restored.layers[0].kind, LayerKind::SolidColor(_)));
        assert!(matches!(restored.layers[1].kind, LayerKind::Gradient(_)));
        assert!(matches!(restored.layers[2].kind, LayerKind::Filter(_)));
        assert!(matches!(restored.layers[3].kind, LayerKind::Group(_)));

        if let LayerKind::SolidColor(sc) = &restored.layers[0].kind {
            assert_eq!(sc.color, Rgba::new(100, 200, 50, 255));
        }
        if let LayerKind::Gradient(g) = &restored.layers[1].kind {
            assert_eq!(g.stops.len(), 2);
            assert_eq!(g.direction, GradientDirection::Vertical);
        }
        if let LayerKind::Filter(f) = &restored.layers[2].kind {
            assert_eq!(f.config.hue_deg, 45.0);
            assert_eq!(f.config.saturation, 0.1);
        }
        if let LayerKind::Group(g) = &restored.layers[3].kind {
            assert_eq!(g.children.len(), 1);
            assert_eq!(g.children[0].common.name, "child");
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
                ShapeType::RoundedRect,
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
        let layer = &restored.layers[0];

        assert_eq!(layer.common.x, 2);
        assert_eq!(layer.common.y, 3);
        match &layer.kind {
            LayerKind::Shape(shape) => {
                assert_eq!(shape.shape_type, ShapeType::RoundedRect);
                assert_eq!(shape.width, 5);
                assert_eq!(shape.height, 4);
                assert_eq!(shape.radius, 2);
                assert_eq!(shape.fill, Some(Rgba::new(255, 0, 0, 255)));
                assert_eq!(
                    shape.stroke,
                    Some(ShapeStroke::new(Rgba::new(255, 255, 255, 255), 1))
                );
            }
            _ => panic!("expected shape layer"),
        }
    }

    #[test]
    fn serialize_preserves_properties() {
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
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                transform,
            }),
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

        let l = &restored.layers[0];
        assert_eq!(l.common.opacity, 0.75);
        assert!(!l.common.visible);
        assert_eq!(l.common.x, -5);
        assert_eq!(l.common.y, 10);
        assert_eq!(l.common.blend_mode, BlendMode::Multiply);
        assert!(l.common.clip_to_below);
        if let LayerKind::Image(img) = &l.kind {
            assert_eq!(img.transform.anchor, Anchor::Center);
            assert!(img.transform.flip_x);
            assert!(img.transform.flip_y);
            assert_eq!(img.transform.rotation_deg, 90.0);
            assert_eq!(img.transform.scale_x, 1.5);
            assert_eq!(img.transform.scale_y, 0.5);
        }
    }

    #[test]
    fn deserialize_accepts_legacy_image_rotation_field() {
        let json = concat!(
            r#"{"width":4,"height":4,"next_id":2,"layers":["#,
            r#"{"id":1,"name":"legacy","visible":true,"opacity":1,"x":0,"y":0,"blend_mode":"normal","clip_to_below":false,"mask_inverted":false,"kind":"image","width":1,"height":1,"buffer_offset":0,"buffer_len":4,"anchor":"center","flip_x":true,"flip_y":false,"rotation":"cw90"}"#,
            r#"]}"#
        );
        let mut data = Vec::new();
        data.extend_from_slice(&(json.len() as u32).to_be_bytes());
        data.extend_from_slice(json.as_bytes());
        data.extend_from_slice(&[255, 0, 0, 255]);

        let restored = deserialize(&data).unwrap();
        match &restored.layers[0].kind {
            LayerKind::Image(img) => {
                assert_eq!(img.transform.anchor, Anchor::Center);
                assert!(img.transform.flip_x);
                assert!(!img.transform.flip_y);
                assert_eq!(img.transform.rotation_deg, 90.0);
                assert_eq!(img.transform.scale_x, 1.0);
                assert_eq!(img.transform.scale_y, 1.0);
            }
            _ => panic!("expected image layer"),
        }
    }

    #[test]
    fn deserialize_rejects_unknown_binary_metadata_version() {
        let doc = Document::new(1, 1);
        let mut data = serialize(&doc).unwrap();
        data[8] = BINARY_METADATA_VERSION + 1;

        assert!(deserialize(&data).is_err());
    }

    #[test]
    fn deserialize_rejects_invalid_utf8_boundaries_in_strings() {
        let data = [
            0, 0, 0, 32, 32, 123, 34, 92, 117, 70, 65, 123, 34, 32, 49, 92, 117, 65, 49, 49, 49,
            32, 49, 92, 117, 65, 49, 49, 212, 138, 32, 125, 125, 125, 13, 125, 13,
        ];

        assert!(deserialize(&data).is_err());
    }
}
