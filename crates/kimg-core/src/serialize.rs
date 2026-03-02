//! Document serialization and deserialization.
//!
//! Encodes a [`Document`] into a compact binary format suitable for persistence
//! and wire transfer, and decodes it back.
//!
//! # Format
//!
//! ```text
//! [4 bytes: JSON length (big-endian u32)] [JSON metadata] [pixel buffers]
//! ```
//!
//! The JSON section stores all layer metadata (ids, names, blend modes, transform
//! properties, gradient stops, etc.).  Raw pixel data (image buffers and masks)
//! is appended as a flat byte sequence after the JSON; each buffer entry in the
//! JSON includes `buffer_offset` and `buffer_len` fields pointing into that region.
//!
//! A hand-written minimal JSON parser is used to avoid a `serde` dependency.
//! It supports only the subset of JSON produced by the serializer.
//!
//! # Backward compatibility
//!
//! Optional fields (e.g. `mask_inverted`) default sensibly when absent, so
//! documents serialized with older versions of kimg can still be loaded.

use crate::blend::BlendMode;
use crate::blit::Anchor;
use crate::buffer::ImageBuffer;
use crate::document::Document;
use crate::filter::HslFilterConfig;
use crate::layer::*;
use crate::pixel::Rgba;

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
/// Currently infallible; returns `Ok` on all well-formed inputs.
pub fn serialize(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let mut pixel_data: Vec<u8> = Vec::new();
    let layers_json = serialize_layers(&doc.layers, &mut pixel_data);

    let json = format!(
        r#"{{"width":{},"height":{},"next_id":{},"layers":[{}]}}"#,
        doc.width,
        doc.height,
        get_next_id(doc),
        layers_json,
    );

    let json_bytes = json.as_bytes();
    let json_len = json_bytes.len() as u32;

    let mut output = Vec::with_capacity(4 + json_bytes.len() + pixel_data.len());
    output.extend_from_slice(&json_len.to_be_bytes());
    output.extend_from_slice(json_bytes);
    output.extend_from_slice(&pixel_data);

    Ok(output)
}

/// Deserialize a [`Document`] from the kimg binary format.
///
/// # Errors
///
/// Returns [`SerializeError::InvalidData`] if the input is truncated, the JSON
/// header is malformed, or any pixel buffer reference is out of bounds.
pub fn deserialize(data: &[u8]) -> Result<Document, SerializeError> {
    if data.len() < 4 {
        return Err(SerializeError::InvalidData("data too short".into()));
    }

    let json_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let json_end = 4usize
        .checked_add(json_len)
        .ok_or_else(|| SerializeError::InvalidData("JSON length overflow".into()))?;
    if data.len() < json_end {
        return Err(SerializeError::InvalidData("truncated JSON".into()));
    }

    let json_str = std::str::from_utf8(&data[4..json_end])
        .map_err(|e| SerializeError::InvalidData(e.to_string()))?;
    let pixel_data = &data[json_end..];

    let doc_obj = parse_json_object(json_str).map_err(SerializeError::InvalidData)?;

    let width = get_json_u32(&doc_obj, "width")?;
    let height = get_json_u32(&doc_obj, "height")?;
    let next_id = get_json_u32(&doc_obj, "next_id")?;

    let layers_str = get_json_str(&doc_obj, "layers").map_err(SerializeError::InvalidData)?;
    let layers = deserialize_layers(layers_str, pixel_data)?;

    let mut doc = Document::new(width, height);
    doc.layers = layers;
    // Advance next_id to the saved value
    while get_next_id(&doc) < next_id {
        doc.next_id();
    }

    Ok(doc)
}

// ── JSON serialization helpers ──

fn get_next_id(doc: &Document) -> u32 {
    // The next_id is private; we can infer it by scanning all layer IDs
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

fn serialize_layers(layers: &[Layer], pixel_data: &mut Vec<u8>) -> String {
    let parts: Vec<String> = layers
        .iter()
        .map(|l| serialize_layer(l, pixel_data))
        .collect();
    parts.join(",")
}

fn serialize_layer(layer: &Layer, pixel_data: &mut Vec<u8>) -> String {
    let c = &layer.common;
    let mut props = format!(
        r#""id":{},"name":"{}","visible":{},"opacity":{},"x":{},"y":{},"blend_mode":"{}","clip_to_below":{},"mask_inverted":{}"#,
        c.id,
        escape_json_string(&c.name),
        c.visible,
        c.opacity,
        c.x,
        c.y,
        c.blend_mode.as_str(),
        c.clip_to_below,
        c.mask_inverted,
    );

    // Mask
    if let Some(mask) = &c.mask {
        let offset = pixel_data.len();
        pixel_data.extend_from_slice(&mask.data);
        props.push_str(&format!(
            r#","mask":{{"width":{},"height":{},"buffer_offset":{},"buffer_len":{}}}"#,
            mask.width,
            mask.height,
            offset,
            mask.data.len()
        ));
    }

    let kind_json = match &layer.kind {
        LayerKind::Image(img) => {
            let offset = pixel_data.len();
            pixel_data.extend_from_slice(&img.buffer.data);
            format!(
                r#""kind":"image","width":{},"height":{},"buffer_offset":{},"buffer_len":{},{}"#,
                img.buffer.width,
                img.buffer.height,
                offset,
                img.buffer.data.len(),
                serialize_transform_props(&img.transform),
            )
        }
        LayerKind::Paint(paint) => {
            let offset = pixel_data.len();
            pixel_data.extend_from_slice(&paint.buffer.data);
            format!(
                r#""kind":"paint","width":{},"height":{},"buffer_offset":{},"buffer_len":{},{}"#,
                paint.buffer.width,
                paint.buffer.height,
                offset,
                paint.buffer.data.len(),
                serialize_transform_props(&paint.transform),
            )
        }
        LayerKind::Filter(f) => {
            format!(
                r#""kind":"filter","hue_deg":{},"saturation":{},"lightness":{},"alpha":{},"brightness":{},"contrast":{},"temperature":{},"tint":{},"sharpen":{}"#,
                f.config.hue_deg,
                f.config.saturation,
                f.config.lightness,
                f.config.alpha,
                f.config.brightness,
                f.config.contrast,
                f.config.temperature,
                f.config.tint,
                f.config.sharpen,
            )
        }
        LayerKind::Group(g) => {
            let children = serialize_layers(&g.children, pixel_data);
            format!(r#""kind":"group","children":[{}]"#, children)
        }
        LayerKind::SolidColor(sc) => {
            format!(
                r#""kind":"solid_color","color":[{},{},{},{}]"#,
                sc.color.r, sc.color.g, sc.color.b, sc.color.a,
            )
        }
        LayerKind::Gradient(grad) => {
            let stops: Vec<String> = grad
                .stops
                .iter()
                .map(|s| {
                    format!(
                        r#"{{"position":{},"color":[{},{},{},{}]}}"#,
                        s.position, s.color.r, s.color.g, s.color.b, s.color.a,
                    )
                })
                .collect();
            format!(
                r#""kind":"gradient","direction":"{}","stops":[{}]"#,
                gradient_dir_str(grad.direction),
                stops.join(","),
            )
        }
        LayerKind::Shape(shape) => {
            let points: Vec<String> = shape
                .points
                .iter()
                .map(|point| format!(r#"{{"x":{},"y":{}}}"#, point.x, point.y))
                .collect();
            let mut props = format!(
                r#""kind":"shape","shape_type":"{}","width":{},"height":{},"radius":{},"points":[{}]"#,
                shape.shape_type.as_str(),
                shape.width,
                shape.height,
                shape.radius,
                points.join(","),
            );
            props.push_str(&format!(",{}", serialize_transform_props(&shape.transform)));

            if let Some(fill) = shape.fill {
                props.push_str(&format!(
                    r#","fill":[{},{},{},{}]"#,
                    fill.r, fill.g, fill.b, fill.a,
                ));
            }

            if let Some(stroke) = shape.stroke {
                props.push_str(&format!(
                    r#","stroke":{{"width":{},"color":[{},{},{},{}]}}"#,
                    stroke.width, stroke.color.r, stroke.color.g, stroke.color.b, stroke.color.a,
                ));
            }

            props
        }
    };

    format!("{{{},{}}}", props, kind_json)
}

fn anchor_str(a: Anchor) -> &'static str {
    match a {
        Anchor::TopLeft => "top_left",
        Anchor::Center => "center",
    }
}

fn serialize_transform_props(transform: &LayerTransform) -> String {
    format!(
        r#""anchor":"{}","flip_x":{},"flip_y":{},"rotation_deg":{},"scale_x":{},"scale_y":{}"#,
        anchor_str(transform.anchor),
        transform.flip_x,
        transform.flip_y,
        transform.rotation_deg,
        transform.scale_x,
        transform.scale_y,
    )
}

fn gradient_dir_str(d: GradientDirection) -> &'static str {
    match d {
        GradientDirection::Horizontal => "horizontal",
        GradientDirection::Vertical => "vertical",
        GradientDirection::DiagonalDown => "diagonal_down",
        GradientDirection::DiagonalUp => "diagonal_up",
    }
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

// ── Minimal JSON parser ──
// Hand-written to avoid serde dependency. Supports the subset we produce.

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
    fn deserialize_rejects_invalid_utf8_boundaries_in_strings() {
        let data = [
            0, 0, 0, 32, 32, 123, 34, 92, 117, 70, 65, 123, 34, 32, 49, 92, 117, 65, 49, 49, 49,
            32, 49, 92, 117, 65, 49, 49, 212, 138, 32, 125, 125, 125, 13, 125, 13,
        ];

        assert!(deserialize(&data).is_err());
    }
}
