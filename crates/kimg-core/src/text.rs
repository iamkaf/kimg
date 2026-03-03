//! Text rasterization helpers.
//!
//! By default this module uses an embedded bitmap font from `font8x8` so text
//! layers work in native builds and WebAssembly without runtime font loading.
//! When the `cosmic-text-backend` feature is enabled, text first tries the real
//! `cosmic-text` shaping/rasterization path and falls back to the bitmap path if
//! no usable font is available yet.

use crate::buffer::ImageBuffer;
#[cfg(feature = "cosmic-text-backend")]
use crate::layer::TextFontStyle;
use crate::layer::{TextAlign, TextLayerData, TextWrap};
use crate::pixel::Rgba;
#[cfg(feature = "cosmic-text-backend")]
use cosmic_text::{
    fontdb::{
        Database, Query, Source, Stretch as FontStretch, Style as FontDbStyle,
        Weight as FontDbWeight,
    },
    Align as CosmicAlign, Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics,
    Shaping, Style as CosmicStyle, SwashCache, Weight as CosmicWeight, Wrap as CosmicWrap,
};
use font8x8::{UnicodeFonts, MISC_FONTS};
use font8x8::{BASIC_FONTS, BLOCK_FONTS, BOX_FONTS, GREEK_FONTS, HIRAGANA_FONTS, LATIN_FONTS};
#[cfg(feature = "cosmic-text-backend")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(feature = "cosmic-text-backend")]
use wuff::{decompress_woff1, decompress_woff2};

const GLYPH_WIDTH: u32 = 8;
const GLYPH_HEIGHT: u32 = 8;

#[cfg(feature = "cosmic-text-backend")]
static REGISTERED_FONTS: OnceLock<Mutex<Vec<RegisteredFont>>> = OnceLock::new();

#[cfg(feature = "cosmic-text-backend")]
#[derive(Clone)]
struct RegisteredFont {
    bytes: Arc<Vec<u8>>,
    families: Vec<String>,
}

fn pixel_scale(font_size: u32) -> u32 {
    font_size.max(1).div_ceil(GLYPH_HEIGHT).max(1)
}

fn glyph_dimensions(font_size: u32) -> (u32, u32) {
    let scale = pixel_scale(font_size);
    (GLYPH_WIDTH * scale, GLYPH_HEIGHT * scale)
}

fn lookup_glyph(ch: char) -> Option<[u8; 8]> {
    BASIC_FONTS
        .get(ch)
        .or_else(|| LATIN_FONTS.get(ch))
        .or_else(|| BLOCK_FONTS.get(ch))
        .or_else(|| BOX_FONTS.get(ch))
        .or_else(|| GREEK_FONTS.get(ch))
        .or_else(|| HIRAGANA_FONTS.get(ch))
        .or_else(|| MISC_FONTS.get(ch))
}

fn fallback_glyph() -> [u8; 8] {
    BASIC_FONTS.get('?').unwrap_or([0; 8])
}

#[cfg(feature = "cosmic-text-backend")]
fn registered_fonts() -> &'static Mutex<Vec<RegisteredFont>> {
    REGISTERED_FONTS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "cosmic-text-backend")]
fn decode_runtime_font_bytes(bytes: Vec<u8>) -> Option<Vec<u8>> {
    if bytes.starts_with(b"wOF2") {
        return decompress_woff2(&bytes).ok();
    }

    if bytes.starts_with(b"wOFF") {
        return decompress_woff1(&bytes).ok();
    }

    Some(bytes)
}

/// Register raw font bytes for the `cosmic-text` backend.
///
/// Returns the number of faces successfully parsed from the input. Invalid or
/// unsupported font data returns `0` and is not retained.
#[cfg(feature = "cosmic-text-backend")]
pub fn register_font_bytes(bytes: Vec<u8>) -> usize {
    let bytes = match decode_runtime_font_bytes(bytes) {
        Some(bytes) => Arc::new(bytes),
        None => return 0,
    };
    let mut db = Database::new();
    let ids = db.load_font_source(Source::Binary(bytes.clone()));
    if ids.is_empty() {
        return 0;
    }

    let mut families: Vec<String> = Vec::new();
    for id in &ids {
        if let Some(face) = db.face(*id) {
            for (family, _) in &face.families {
                if !families
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(family))
                {
                    families.push(family.clone());
                }
            }
        }
    }

    registered_fonts()
        .lock()
        .expect("font registry poisoned")
        .push(RegisteredFont { bytes, families });
    ids.len()
}

/// Clear all registered runtime fonts for the `cosmic-text` backend.
#[cfg(feature = "cosmic-text-backend")]
pub fn clear_registered_fonts() {
    registered_fonts()
        .lock()
        .expect("font registry poisoned")
        .clear();
}

/// Count the number of registered runtime font binaries.
#[cfg(feature = "cosmic-text-backend")]
pub fn registered_font_count() -> usize {
    registered_fonts()
        .lock()
        .expect("font registry poisoned")
        .len()
}

#[cfg(feature = "cosmic-text-backend")]
fn build_font_system() -> FontSystem {
    let mut db = Database::new();
    db.load_system_fonts();
    let fonts = registered_fonts()
        .lock()
        .expect("font registry poisoned")
        .clone();
    let mut primary_family = None::<String>;
    for font in fonts {
        if primary_family.is_none() {
            primary_family = font.families.first().cloned();
        }
        db.load_font_source(Source::Binary(font.bytes));
    }
    if let Some(primary_family) = primary_family {
        db.set_sans_serif_family(primary_family.clone());
        db.set_serif_family(primary_family.clone());
        db.set_monospace_family(primary_family);
    }
    FontSystem::new_with_locale_and_db("en-US".to_string(), db)
}

#[cfg(feature = "cosmic-text-backend")]
fn resolve_registered_family(font_family: &str) -> Option<String> {
    let fonts = registered_fonts()
        .lock()
        .expect("font registry poisoned")
        .clone();
    let mut families: Vec<String> = Vec::new();
    for font in fonts {
        for family in font.families {
            if !families
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&family))
            {
                families.push(family);
            }
        }
    }

    if let Some(found) = families
        .iter()
        .find(|family| family.eq_ignore_ascii_case(font_family))
    {
        return Some(found.clone());
    }

    families.into_iter().next()
}

#[cfg(all(feature = "cosmic-text-backend", test))]
fn registered_family_names() -> Vec<String> {
    let fonts = registered_fonts()
        .lock()
        .expect("font registry poisoned")
        .clone();
    let mut families: Vec<String> = Vec::new();
    for font in fonts {
        for family in font.families {
            if !families
                .iter()
                .any(|known| known.eq_ignore_ascii_case(&family))
            {
                families.push(family);
            }
        }
    }
    families
}

#[cfg(feature = "cosmic-text-backend")]
fn text_font_style_to_fontdb(style: TextFontStyle) -> FontDbStyle {
    match style {
        TextFontStyle::Normal => FontDbStyle::Normal,
        TextFontStyle::Italic => FontDbStyle::Italic,
        TextFontStyle::Oblique => FontDbStyle::Oblique,
    }
}

#[cfg(feature = "cosmic-text-backend")]
fn has_cosmic_font_match(
    font_system: &FontSystem,
    family_name: &str,
    weight: u16,
    style: TextFontStyle,
) -> bool {
    let families = [text_family_to_cosmic(family_name)];
    font_system
        .db()
        .query(&Query {
            families: &families,
            weight: FontDbWeight(weight),
            stretch: FontStretch::Normal,
            style: text_font_style_to_fontdb(style),
        })
        .is_some()
}

fn glyph_width_for_line(line: &str, glyph_w: u32, spacing_x: u32) -> u32 {
    let glyphs = line.chars().count() as u32;
    match glyphs {
        0 => 0,
        1 => glyph_w,
        _ => glyph_w * glyphs + spacing_x * (glyphs - 1),
    }
}

fn max_glyphs_for_width(box_width: u32, glyph_w: u32, spacing_x: u32) -> usize {
    if box_width <= glyph_w {
        return 1;
    }
    ((box_width + spacing_x) / (glyph_w + spacing_x)).max(1) as usize
}

fn wrap_bitmap_line(line: &str, max_glyphs: usize) -> Vec<String> {
    if max_glyphs == 0 {
        return vec![String::new()];
    }
    if line.is_empty() {
        return vec![String::new()];
    }

    let mut output = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        let word_len = word.chars().count();
        if current.is_empty() {
            if word_len <= max_glyphs {
                current.push_str(word);
                continue;
            }
        } else if current.chars().count() + 1 + word_len <= max_glyphs {
            current.push(' ');
            current.push_str(word);
            continue;
        } else {
            output.push(std::mem::take(&mut current));
        }

        if word_len <= max_glyphs {
            current.push_str(word);
            continue;
        }

        let chars: Vec<char> = word.chars().collect();
        for chunk in chars.chunks(max_glyphs) {
            output.push(chunk.iter().collect());
        }
    }

    if !current.is_empty() {
        output.push(current);
    }

    if output.is_empty() {
        output.push(String::new());
    }

    output
}

fn layout_bitmap_lines(text: &TextLayerData) -> Vec<String> {
    let (glyph_w, _) = glyph_dimensions(text.font_size);
    let max_glyphs = text
        .box_width
        .map(|width| max_glyphs_for_width(width.max(1), glyph_w, text.letter_spacing));

    let mut lines = Vec::new();
    for line in text.text.split('\n') {
        if matches!(text.wrap, TextWrap::Word) {
            if let Some(max_glyphs) = max_glyphs {
                lines.extend(wrap_bitmap_line(line, max_glyphs));
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Measure the local raster dimensions for a text layer.
pub fn measure_text(text: &TextLayerData) -> (u32, u32) {
    #[cfg(feature = "cosmic-text-backend")]
    if let Some(dimensions) = measure_text_cosmic(text) {
        return dimensions;
    }

    measure_text_bitmap(text)
}

/// Rasterize a text layer into a local RGBA buffer.
pub fn render_text(text: &TextLayerData) -> ImageBuffer {
    #[cfg(feature = "cosmic-text-backend")]
    if let Some(buffer) = render_text_cosmic(text) {
        return buffer;
    }

    render_text_bitmap(text)
}

fn measure_text_bitmap(text: &TextLayerData) -> (u32, u32) {
    let lines = layout_bitmap_lines(text);
    let line_count = lines.len().max(1) as u32;
    let (glyph_w, glyph_h) = glyph_dimensions(text.font_size);
    let advance_y = text.line_height.max(glyph_h);
    let mut max_width = 0u32;
    for line in lines {
        max_width = max_width.max(glyph_width_for_line(&line, glyph_w, text.letter_spacing));
    }
    if let Some(box_width) = text.box_width {
        max_width = box_width.max(1);
    }

    let height = if line_count == 0 {
        0
    } else {
        glyph_h + advance_y * (line_count - 1)
    };

    (max_width.max(1), height.max(1))
}

fn render_text_bitmap(text: &TextLayerData) -> ImageBuffer {
    let (width, height) = measure_text_bitmap(text);
    let mut buffer = ImageBuffer::new_transparent(width, height);
    let scale = pixel_scale(text.font_size);
    let (glyph_w, glyph_h) = glyph_dimensions(text.font_size);
    let advance_y = text.line_height.max(glyph_h);
    let fallback = fallback_glyph();
    let lines = layout_bitmap_lines(text);

    for (line_index, line) in lines.into_iter().enumerate() {
        let line_width = glyph_width_for_line(&line, glyph_w, text.letter_spacing);
        let mut pen_x = match text.align {
            TextAlign::Left => 0,
            TextAlign::Center => width.saturating_sub(line_width) / 2,
            TextAlign::Right => width.saturating_sub(line_width),
        };
        let pen_y = line_index as u32 * advance_y;
        for ch in line.chars() {
            let glyph = lookup_glyph(ch).unwrap_or(fallback);
            draw_glyph(&mut buffer, &glyph, pen_x, pen_y, scale, text.color);
            pen_x = pen_x.saturating_add(glyph_w + text.letter_spacing);
        }
    }

    buffer
}

fn draw_glyph(
    target: &mut ImageBuffer,
    glyph: &[u8; 8],
    origin_x: u32,
    origin_y: u32,
    scale: u32,
    color: Rgba,
) {
    for (row, bits) in glyph.iter().copied().enumerate() {
        for column in 0..8u32 {
            if bits & (1 << column) == 0 {
                continue;
            }

            let pixel_x = origin_x + column * scale;
            let pixel_y = origin_y + row as u32 * scale;
            for sy in 0..scale {
                for sx in 0..scale {
                    let x = pixel_x + sx;
                    let y = pixel_y + sy;
                    if x < target.width && y < target.height {
                        target.set_pixel(x, y, color);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "cosmic-text-backend")]
fn measure_text_cosmic(text: &TextLayerData) -> Option<(u32, u32)> {
    let mut font_system = build_font_system();
    let family_name =
        resolve_registered_family(&text.font_family).unwrap_or_else(|| text.font_family.clone());
    if !has_cosmic_font_match(
        &font_system,
        &family_name,
        text.font_weight.max(1),
        text.font_style,
    ) {
        return None;
    }
    let metrics = Metrics::new(text.font_size as f32, text.line_height as f32);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    let mut buffer = buffer.borrow_with(&mut font_system);
    buffer.set_wrap(text_wrap_to_cosmic(text.wrap));
    buffer.set_size(text.box_width.map(|width| width as f32), None);

    let mut attrs = Attrs::new()
        .family(text_family_to_cosmic(&family_name))
        .weight(CosmicWeight(text.font_weight.max(1)))
        .style(text_font_style_to_cosmic(text.font_style));
    if text.letter_spacing > 0 {
        attrs = attrs.letter_spacing(text.letter_spacing as f32);
    }

    buffer.set_text(
        &text.text,
        &attrs,
        Shaping::Advanced,
        Some(text_align_to_cosmic(text.align)),
    );
    buffer.shape_until_scroll(true);

    let mut max_width = 0.0f32;
    let mut max_height = 0.0f32;
    let mut saw_run = false;
    for run in buffer.layout_runs() {
        saw_run = true;
        max_width = max_width.max(run.line_w);
        max_height = max_height.max(run.line_top + run.line_height);
    }

    if !saw_run {
        return None;
    }

    if let Some(box_width) = text.box_width {
        max_width = box_width as f32;
    }

    Some((
        max_width.ceil().max(1.0) as u32,
        max_height.ceil().max(1.0) as u32,
    ))
}

#[cfg(feature = "cosmic-text-backend")]
fn render_text_cosmic(text: &TextLayerData) -> Option<ImageBuffer> {
    let mut font_system = build_font_system();
    let family_name =
        resolve_registered_family(&text.font_family).unwrap_or_else(|| text.font_family.clone());
    if !has_cosmic_font_match(
        &font_system,
        &family_name,
        text.font_weight.max(1),
        text.font_style,
    ) {
        return None;
    }
    let (measure_width, measure_height) = measure_text_cosmic(text)?;
    let metrics = Metrics::new(text.font_size as f32, text.line_height as f32);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    let mut buffer = buffer.borrow_with(&mut font_system);
    buffer.set_wrap(text_wrap_to_cosmic(text.wrap));
    buffer.set_size(text.box_width.map(|width| width as f32), None);

    let mut attrs = Attrs::new()
        .family(text_family_to_cosmic(&family_name))
        .weight(CosmicWeight(text.font_weight.max(1)))
        .style(text_font_style_to_cosmic(text.font_style));
    if text.letter_spacing > 0 {
        attrs = attrs.letter_spacing(text.letter_spacing as f32);
    }

    buffer.set_text(
        &text.text,
        &attrs,
        Shaping::Advanced,
        Some(text_align_to_cosmic(text.align)),
    );
    buffer.shape_until_scroll(true);

    let mut swash_cache = SwashCache::new();
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut out = ImageBuffer::new_transparent(measure_width.max(1), measure_height.max(1));
    buffer.draw(
        &mut swash_cache,
        CosmicColor::rgba(text.color.r, text.color.g, text.color.b, text.color.a),
        |x, y, w, h, color| {
            if w == 0 || h == 0 || color.a() == 0 {
                return;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            let rgba = Rgba::new(color.r(), color.g(), color.b(), color.a());
            for yy in 0..h {
                for xx in 0..w {
                    let dst_x = x + xx as i32;
                    let dst_y = y + yy as i32;
                    if dst_x >= 0
                        && dst_y >= 0
                        && (dst_x as u32) < out.width
                        && (dst_y as u32) < out.height
                    {
                        out.set_pixel(dst_x as u32, dst_y as u32, rgba);
                    }
                }
            }
        },
    );

    if min_x == i32::MAX || min_y == i32::MAX {
        return None;
    }

    Some(out)
}

#[cfg(feature = "cosmic-text-backend")]
fn text_family_to_cosmic(font_family: &str) -> Family<'_> {
    if font_family.eq_ignore_ascii_case("serif") {
        Family::Serif
    } else if font_family.eq_ignore_ascii_case("sans-serif")
        || font_family.eq_ignore_ascii_case("sansserif")
        || font_family.eq_ignore_ascii_case("sans")
    {
        Family::SansSerif
    } else if font_family.eq_ignore_ascii_case("monospace") {
        Family::Monospace
    } else if font_family.eq_ignore_ascii_case("cursive") {
        Family::Cursive
    } else if font_family.eq_ignore_ascii_case("fantasy") {
        Family::Fantasy
    } else {
        Family::Name(font_family)
    }
}

#[cfg(feature = "cosmic-text-backend")]
fn text_font_style_to_cosmic(style: TextFontStyle) -> CosmicStyle {
    match style {
        TextFontStyle::Normal => CosmicStyle::Normal,
        TextFontStyle::Italic => CosmicStyle::Italic,
        TextFontStyle::Oblique => CosmicStyle::Oblique,
    }
}

#[cfg(feature = "cosmic-text-backend")]
fn text_align_to_cosmic(align: TextAlign) -> CosmicAlign {
    match align {
        TextAlign::Left => CosmicAlign::Left,
        TextAlign::Center => CosmicAlign::Center,
        TextAlign::Right => CosmicAlign::Right,
    }
}

#[cfg(feature = "cosmic-text-backend")]
fn text_wrap_to_cosmic(wrap: TextWrap) -> CosmicWrap {
    match wrap {
        TextWrap::None => CosmicWrap::None,
        TextWrap::Word => CosmicWrap::Word,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::TextLayerData;

    #[test]
    fn measures_multiline_text() {
        let text = TextLayerData::new("Hi\nX", Rgba::new(255, 255, 255, 255), 16, 20, 2);
        #[cfg(not(feature = "cosmic-text-backend"))]
        assert_eq!(measure_text(&text), (34, 36));

        #[cfg(feature = "cosmic-text-backend")]
        {
            let (width, height) = measure_text(&text);
            assert!(width > 0);
            assert!(height > 0);
        }
    }

    #[test]
    fn renders_visible_pixels() {
        let text = TextLayerData::new("A", Rgba::new(255, 0, 0, 255), 16, 18, 0);
        let rendered = render_text(&text);
        assert!(rendered.data.chunks_exact(4).any(|px| px[3] > 0));
    }

    #[cfg(feature = "cosmic-text-backend")]
    #[test]
    fn invalid_runtime_font_is_rejected() {
        clear_registered_fonts();
        assert_eq!(register_font_bytes(vec![1, 2, 3, 4]), 0);
        assert_eq!(registered_font_count(), 0);
    }

    #[cfg(feature = "cosmic-text-backend")]
    #[test]
    fn google_subset_font_reports_family_names() {
        clear_registered_fonts();
        let bytes = include_bytes!("../../../tests/fixtures/inter-kimg.woff2");
        assert!(register_font_bytes(bytes.to_vec()) > 0);
        let families = registered_family_names();
        println!("registered families: {families:?}");
        assert!(!families.is_empty());
    }
}
