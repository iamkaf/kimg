//! Bitmap text rasterization helpers.
//!
//! This first text backend uses an embedded bitmap font from `font8x8` so text
//! layers work in native builds and WebAssembly without runtime font loading.

use crate::buffer::ImageBuffer;
use crate::layer::TextLayerData;
use crate::pixel::Rgba;
use font8x8::{UnicodeFonts, MISC_FONTS};
use font8x8::{BASIC_FONTS, BLOCK_FONTS, BOX_FONTS, GREEK_FONTS, HIRAGANA_FONTS, LATIN_FONTS};

const GLYPH_WIDTH: u32 = 8;
const GLYPH_HEIGHT: u32 = 8;

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

/// Measure the local raster dimensions for a text layer.
pub fn measure_text(text: &TextLayerData) -> (u32, u32) {
    let lines: Vec<&str> = text.text.split('\n').collect();
    let line_count = lines.len().max(1) as u32;
    let (glyph_w, glyph_h) = glyph_dimensions(text.font_size);
    let advance_y = text.line_height.max(glyph_h);
    let spacing_x = text.letter_spacing;

    let mut max_width = 0u32;
    for line in lines {
        let glyphs = line.chars().count() as u32;
        let width = match glyphs {
            0 => 0,
            1 => glyph_w,
            _ => glyph_w * glyphs + spacing_x * (glyphs - 1),
        };
        max_width = max_width.max(width);
    }

    let height = if line_count == 0 {
        0
    } else {
        glyph_h + advance_y * (line_count - 1)
    };

    (max_width.max(1), height.max(1))
}

/// Rasterize a text layer into a local RGBA buffer.
pub fn render_text(text: &TextLayerData) -> ImageBuffer {
    let (width, height) = measure_text(text);
    let mut buffer = ImageBuffer::new_transparent(width, height);
    let scale = pixel_scale(text.font_size);
    let (glyph_w, glyph_h) = glyph_dimensions(text.font_size);
    let advance_y = text.line_height.max(glyph_h);
    let fallback = fallback_glyph();

    for (line_index, line) in text.text.split('\n').enumerate() {
        let mut pen_x = 0u32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::TextLayerData;

    #[test]
    fn measures_multiline_text() {
        let text = TextLayerData::new("Hi\nX", Rgba::new(255, 255, 255, 255), 16, 20, 2);
        assert_eq!(measure_text(&text), (34, 36));
    }

    #[test]
    fn renders_visible_pixels() {
        let text = TextLayerData::new("A", Rgba::new(255, 0, 0, 255), 16, 18, 0);
        let rendered = render_text(&text);
        assert!(rendered.data.chunks_exact(4).any(|px| px[3] > 0));
    }
}
