use crate::blend::BlendMode;
use crate::blit::{Anchor, Rotation};
use crate::buffer::ImageBuffer;
use crate::document::Document;
use crate::filter::HslFilterConfig;
use crate::layer::*;
use crate::pixel::Rgba;

#[derive(Debug)]
pub enum SerializeError {
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

/// Serialize a Document to a binary format:
/// [4 bytes: JSON length (big-endian u32)] [JSON metadata] [pixel buffers]
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

/// Deserialize a Document from the binary format.
pub fn deserialize(data: &[u8]) -> Result<Document, SerializeError> {
    if data.len() < 4 {
        return Err(SerializeError::InvalidData("data too short".into()));
    }

    let json_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if data.len() < 4 + json_len {
        return Err(SerializeError::InvalidData("truncated JSON".into()));
    }

    let json_str = std::str::from_utf8(&data[4..4 + json_len])
        .map_err(|e| SerializeError::InvalidData(e.to_string()))?;
    let pixel_data = &data[4 + json_len..];

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
        r#""id":{},"name":"{}","visible":{},"opacity":{},"x":{},"y":{},"blend_mode":"{}","clip_to_below":{}"#,
        c.id,
        escape_json_string(&c.name),
        c.visible,
        c.opacity,
        c.x,
        c.y,
        c.blend_mode.as_str(),
        c.clip_to_below,
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
                r#""kind":"image","width":{},"height":{},"buffer_offset":{},"buffer_len":{},"anchor":"{}","flip_x":{},"flip_y":{},"rotation":"{}""#,
                img.buffer.width,
                img.buffer.height,
                offset,
                img.buffer.data.len(),
                anchor_str(img.anchor),
                img.flip_x,
                img.flip_y,
                rotation_str(img.rotation),
            )
        }
        LayerKind::Paint(paint) => {
            let offset = pixel_data.len();
            pixel_data.extend_from_slice(&paint.buffer.data);
            format!(
                r#""kind":"paint","width":{},"height":{},"buffer_offset":{},"buffer_len":{},"anchor":"{}""#,
                paint.buffer.width,
                paint.buffer.height,
                offset,
                paint.buffer.data.len(),
                anchor_str(paint.anchor),
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
    };

    format!("{{{},{}}}", props, kind_json)
}

fn anchor_str(a: Anchor) -> &'static str {
    match a {
        Anchor::TopLeft => "top_left",
        Anchor::Center => "center",
    }
}

fn rotation_str(r: Rotation) -> &'static str {
    match r {
        Rotation::None => "none",
        Rotation::Cw90 => "cw90",
        Rotation::Cw180 => "cw180",
        Rotation::Cw270 => "cw270",
    }
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
    let bytes = s.as_bytes();
    if bytes[pos] != b'"' {
        return Err(format!("expected '\"' at {pos}"));
    }
    let mut i = pos + 1;
    let mut result = String::new();
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'"' => {
                    result.push('"');
                    i += 2;
                }
                b'\\' => {
                    result.push('\\');
                    i += 2;
                }
                b'n' => {
                    result.push('\n');
                    i += 2;
                }
                b'r' => {
                    result.push('\r');
                    i += 2;
                }
                b't' => {
                    result.push('\t');
                    i += 2;
                }
                b'u' if i + 5 < bytes.len() => {
                    let hex = &s[i + 2..i + 6];
                    if let Ok(cp) = u32::from_str_radix(hex, 16) {
                        if let Some(c) = char::from_u32(cp) {
                            result.push(c);
                        }
                    }
                    i += 6;
                }
                _ => {
                    result.push(bytes[i + 1] as char);
                    i += 2;
                }
            }
        } else if bytes[i] == b'"' {
            return Ok((result, i + 1));
        } else {
            result.push(bytes[i] as char);
            i += 1;
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
        if moff + mlen > pixel_data.len() {
            return Err(SerializeError::InvalidData(
                "mask pixel data out of bounds".into(),
            ));
        }
        ImageBuffer::from_rgba(mw, mh, pixel_data[moff..moff + mlen].to_vec())
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
    common.mask = mask;

    let kind_str = get_json_str(&obj, "kind").map_err(e)?;
    let kind = match kind_str {
        "image" => {
            let w = get_json_u32(&obj, "width")?;
            let h = get_json_u32(&obj, "height")?;
            let offset = parse_usize(&obj, "buffer_offset")?;
            let len = parse_usize(&obj, "buffer_len")?;
            if offset + len > pixel_data.len() {
                return Err(SerializeError::InvalidData(
                    "pixel data out of bounds".into(),
                ));
            }
            let buffer = ImageBuffer::from_rgba(w, h, pixel_data[offset..offset + len].to_vec())
                .ok_or_else(|| SerializeError::InvalidData("buffer size mismatch".into()))?;
            let anchor = parse_anchor(get_json_str(&obj, "anchor").unwrap_or("top_left"));
            let flip_x = get_json_bool(&obj, "flip_x").unwrap_or(false);
            let flip_y = get_json_bool(&obj, "flip_y").unwrap_or(false);
            let rotation = parse_rotation(get_json_str(&obj, "rotation").unwrap_or("none"));
            LayerKind::Image(ImageLayerData {
                buffer,
                anchor,
                flip_x,
                flip_y,
                rotation,
            })
        }
        "paint" => {
            let w = get_json_u32(&obj, "width")?;
            let h = get_json_u32(&obj, "height")?;
            let offset = parse_usize(&obj, "buffer_offset")?;
            let len = parse_usize(&obj, "buffer_len")?;
            if offset + len > pixel_data.len() {
                return Err(SerializeError::InvalidData(
                    "pixel data out of bounds".into(),
                ));
            }
            let buffer = ImageBuffer::from_rgba(w, h, pixel_data[offset..offset + len].to_vec())
                .ok_or_else(|| SerializeError::InvalidData("buffer size mismatch".into()))?;
            let anchor = parse_anchor(get_json_str(&obj, "anchor").unwrap_or("top_left"));
            LayerKind::Paint(PaintLayerData { buffer, anchor })
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

fn parse_rotation(s: &str) -> Rotation {
    match s {
        "cw90" => Rotation::Cw90,
        "cw180" => Rotation::Cw180,
        "cw270" => Rotation::Cw270,
        _ => Rotation::None,
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
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
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
    fn serialize_preserves_properties() {
        let mut doc = Document::new(4, 4);
        let id = doc.next_id();
        let buf = ImageBuffer::new_transparent(2, 2);
        let mut layer = Layer {
            common: LayerCommon::new(id, "props"),
            kind: LayerKind::Image(ImageLayerData {
                buffer: buf,
                anchor: Anchor::Center,
                flip_x: true,
                flip_y: true,
                rotation: Rotation::Cw90,
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
            assert_eq!(img.anchor, Anchor::Center);
            assert!(img.flip_x);
            assert!(img.flip_y);
            assert_eq!(img.rotation, Rotation::Cw90);
        }
    }
}
