use crate::buffer::ImageBuffer;
use crate::document::Document;
use crate::pixel::Rgba;
use crate::transform::resize_nearest;

// ── Sprite sheet packer ──

/// Represents a single sprite packed within a larger texture atlas.
#[derive(Debug, Clone)]
pub struct PackedSprite {
    /// Original index of the sprite in the input slice.
    pub index: usize,
    /// X coordinate of the sprite's top-left corner in the atlas.
    pub x: u32,
    /// Y coordinate of the sprite's top-left corner in the atlas.
    pub y: u32,
    /// Width of the sprite.
    pub width: u32,
    /// Height of the sprite.
    pub height: u32,
}

/// A packed sprite sheet atlas.
#[derive(Debug, Clone)]
pub struct SpriteSheet {
    /// The final merged atlas image.
    pub buffer: ImageBuffer,
    /// Locations of each packed sprite.
    pub sprites: Vec<PackedSprite>,
    /// Width of the atlas.
    pub width: u32,
    /// Height of the atlas.
    pub height: u32,
}

/// Pack multiple sprites into a single atlas using shelf next-fit.
pub fn pack_sprites(
    sprites: &[&ImageBuffer],
    padding: u32,
    max_size: u32,
    power_of_two: bool,
) -> SpriteSheet {
    if sprites.is_empty() {
        return SpriteSheet {
            buffer: ImageBuffer::new_transparent(0, 0),
            sprites: Vec::new(),
            width: 0,
            height: 0,
        };
    }

    // Sort by height descending, keeping original indices
    let mut indexed: Vec<(usize, &ImageBuffer)> = sprites.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.height.cmp(&a.1.height));

    let mut placements: Vec<PackedSprite> = Vec::with_capacity(sprites.len());
    let mut shelf_x: u32 = 0;
    let mut shelf_y: u32 = 0;
    let mut shelf_height: u32 = 0;
    let mut atlas_width: u32 = 0;

    for (idx, buf) in &indexed {
        let w = buf.width;
        let h = buf.height;

        // Check if sprite fits on current shelf
        if shelf_x + w > max_size {
            // Start new shelf
            shelf_y += shelf_height + padding;
            shelf_x = 0;
            shelf_height = 0;
        }

        placements.push(PackedSprite {
            index: *idx,
            x: shelf_x,
            y: shelf_y,
            width: w,
            height: h,
        });

        shelf_x += w + padding;
        atlas_width = atlas_width.max(shelf_x.saturating_sub(padding));
        shelf_height = shelf_height.max(h);
    }

    let mut atlas_height = shelf_y + shelf_height;

    if power_of_two {
        atlas_width = atlas_width.next_power_of_two();
        atlas_height = atlas_height.next_power_of_two();
    }

    atlas_width = atlas_width.min(max_size);
    atlas_height = atlas_height.min(max_size);

    // Blit all sprites onto the atlas
    let mut buffer = ImageBuffer::new_transparent(atlas_width, atlas_height);
    let aw = atlas_width as usize;

    for p in &placements {
        let src = sprites[p.index];
        let sw = src.width as usize;
        for sy in 0..src.height as usize {
            let dy = p.y as usize + sy;
            if dy >= atlas_height as usize {
                break;
            }
            for sx in 0..sw {
                let dx = p.x as usize + sx;
                if dx >= atlas_width as usize {
                    break;
                }
                let si = (sy * sw + sx) * 4;
                let di = (dy * aw + dx) * 4;
                buffer.data[di..di + 4].copy_from_slice(&src.data[si..si + 4]);
            }
        }
    }

    // Sort placements back by original index for the output
    placements.sort_by_key(|p| p.index);

    SpriteSheet {
        buffer,
        sprites: placements,
        width: atlas_width,
        height: atlas_height,
    }
}

// ── Contact sheet ──

/// Options for generating a uniform contact sheet.
#[non_exhaustive]
pub struct ContactSheetOptions {
    /// Number of columns in the grid. If 0, auto-computed.
    pub columns: u32,
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Padding between cells in pixels.
    pub padding: u32,
    /// Background color of the sheet.
    pub background: Rgba,
}

/// Arrange images in a uniform grid.
pub fn contact_sheet(images: &[&ImageBuffer], opts: &ContactSheetOptions) -> ImageBuffer {
    if images.is_empty() {
        return ImageBuffer::new_transparent(0, 0);
    }

    let count = images.len() as u32;
    let columns = if opts.columns == 0 {
        (count as f64).sqrt().ceil() as u32
    } else {
        opts.columns
    };
    let rows = count.div_ceil(columns);

    let out_w = columns * (opts.cell_width + opts.padding) - opts.padding;
    let out_h = rows * (opts.cell_height + opts.padding) - opts.padding;

    let mut output = ImageBuffer::new_transparent(out_w, out_h);
    if opts.background.a > 0 {
        output.fill(opts.background);
    }

    let ow = out_w as usize;

    for (i, img) in images.iter().enumerate() {
        let col = i as u32 % columns;
        let row = i as u32 / columns;

        // Resize to fit within cell, preserving aspect ratio, nearest-neighbor
        let (fit_w, fit_h) =
            fit_dimensions(img.width, img.height, opts.cell_width, opts.cell_height);
        let resized = if fit_w == img.width && fit_h == img.height {
            (*img).clone()
        } else {
            resize_nearest(img, fit_w, fit_h)
        };

        // Center within cell
        let cell_x = col * (opts.cell_width + opts.padding);
        let cell_y = row * (opts.cell_height + opts.padding);
        let offset_x = cell_x + (opts.cell_width - fit_w) / 2;
        let offset_y = cell_y + (opts.cell_height - fit_h) / 2;

        let rw = resized.width as usize;
        for sy in 0..resized.height as usize {
            let dy = offset_y as usize + sy;
            if dy >= out_h as usize {
                break;
            }
            for sx in 0..rw {
                let dx = offset_x as usize + sx;
                if dx >= out_w as usize {
                    break;
                }
                let si = (sy * rw + sx) * 4;
                let di = (dy * ow + dx) * 4;
                // Simple alpha-over blit
                let sa = resized.data[si + 3];
                if sa == 255 {
                    output.data[di..di + 4].copy_from_slice(&resized.data[si..si + 4]);
                } else if sa > 0 {
                    let sf = sa as f64 / 255.0;
                    let inv = 1.0 - sf;
                    for c in 0..3 {
                        output.data[di + c] = (resized.data[si + c] as f64 * sf
                            + output.data[di + c] as f64 * inv)
                            as u8;
                    }
                    output.data[di + 3] = (sa as f64 + output.data[di + 3] as f64 * inv) as u8;
                }
            }
        }
    }

    output
}

/// Compute dimensions that fit within max_w x max_h while preserving aspect ratio.
fn fit_dimensions(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if src_w == 0 || src_h == 0 || max_w == 0 || max_h == 0 {
        return (0, 0);
    }
    let scale_x = max_w as f64 / src_w as f64;
    let scale_y = max_h as f64 / src_h as f64;
    let scale = scale_x.min(scale_y).min(1.0); // don't upscale
    let w = (src_w as f64 * scale).round() as u32;
    let h = (src_h as f64 * scale).round() as u32;
    (w.max(1), h.max(1))
}

// ── Pixel-art upscale ──

/// Integer-scale upscale using nearest-neighbor. Factor must be >= 1.
pub fn pixel_scale(src: &ImageBuffer, factor: u32) -> ImageBuffer {
    let factor = factor.max(1);
    resize_nearest(src, src.width * factor, src.height * factor)
}

// ── Color quantization ──

/// A color palette extracted from an image.
#[derive(Debug, Clone)]
pub struct Palette {
    /// The unique colors in the palette.
    pub colors: Vec<Rgba>,
}

/// Extract a palette of up to `max_colors` from the image using median-cut.
/// Ignores fully transparent pixels.
pub fn extract_palette(src: &ImageBuffer, max_colors: u32) -> Palette {
    let max_colors = max_colors.max(1) as usize;

    // Collect non-transparent pixels
    let pixel_count = (src.width * src.height) as usize;
    let mut pixels: Vec<[u8; 3]> = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        let idx = i * 4;
        if src.data[idx + 3] > 0 {
            pixels.push([src.data[idx], src.data[idx + 1], src.data[idx + 2]]);
        }
    }

    if pixels.is_empty() {
        return Palette {
            colors: vec![Rgba::TRANSPARENT],
        };
    }

    // Median-cut: split boxes until we have enough
    let mut boxes: Vec<ColorBox> = vec![ColorBox::new(&pixels)];

    while boxes.len() < max_colors {
        // Find box with largest range
        let (split_idx, _) = boxes
            .iter()
            .enumerate()
            .max_by_key(|(_, b)| b.range())
            .unwrap();

        let b = boxes.remove(split_idx);
        if b.pixels.len() <= 1 || b.range() == 0 {
            boxes.push(b);
            break;
        }
        let (a, c) = b.split();
        boxes.push(a);
        boxes.push(c);
    }

    // Average each box to get palette colors
    let mut colors: Vec<Rgba> = boxes.iter().map(|b| b.average()).collect();

    // Sort by luminance
    colors.sort_by(|a, b| {
        let la = luminance(a.r, a.g, a.b);
        let lb = luminance(b.r, b.g, b.b);
        la.partial_cmp(&lb).unwrap()
    });

    Palette { colors }
}

/// Quantize the image to only use colors from the given palette.
/// Each non-transparent pixel is mapped to its nearest palette color.
pub fn quantize(src: &ImageBuffer, palette: &Palette) -> ImageBuffer {
    if palette.colors.is_empty() {
        return src.clone();
    }

    let mut dst = src.clone();
    let pixel_count = (src.width * src.height) as usize;

    for i in 0..pixel_count {
        let idx = i * 4;
        if dst.data[idx + 3] == 0 {
            continue;
        }
        let r = dst.data[idx] as i32;
        let g = dst.data[idx + 1] as i32;
        let b = dst.data[idx + 2] as i32;

        let mut best_dist = i32::MAX;
        let mut best = &palette.colors[0];

        for color in &palette.colors {
            let dr = r - color.r as i32;
            let dg = g - color.g as i32;
            let db = b - color.b as i32;
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best = color;
            }
        }

        dst.data[idx] = best.r;
        dst.data[idx + 1] = best.g;
        dst.data[idx + 2] = best.b;
        // preserve alpha
    }

    dst
}

fn luminance(r: u8, g: u8, b: u8) -> f64 {
    0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64
}

struct ColorBox {
    pixels: Vec<[u8; 3]>,
}

impl ColorBox {
    fn new(pixels: &[[u8; 3]]) -> Self {
        Self {
            pixels: pixels.to_vec(),
        }
    }

    fn bounds(&self) -> ([u8; 3], [u8; 3]) {
        let mut min = [255u8; 3];
        let mut max = [0u8; 3];
        for p in &self.pixels {
            for c in 0..3 {
                min[c] = min[c].min(p[c]);
                max[c] = max[c].max(p[c]);
            }
        }
        (min, max)
    }

    fn range(&self) -> u32 {
        let (min, max) = self.bounds();
        let mut r = 0u32;
        for c in 0..3 {
            r = r.max((max[c] as u32).saturating_sub(min[c] as u32));
        }
        r
    }

    fn longest_axis(&self) -> usize {
        let (min, max) = self.bounds();
        let mut best_axis = 0;
        let mut best_range = 0u32;
        for c in 0..3 {
            let range = (max[c] as u32).saturating_sub(min[c] as u32);
            if range > best_range {
                best_range = range;
                best_axis = c;
            }
        }
        best_axis
    }

    fn split(mut self) -> (ColorBox, ColorBox) {
        let axis = self.longest_axis();
        self.pixels.sort_by_key(|p| p[axis]);
        let mid = self.pixels.len() / 2;
        let right = self.pixels.split_off(mid);
        (
            ColorBox {
                pixels: self.pixels,
            },
            ColorBox { pixels: right },
        )
    }

    fn average(&self) -> Rgba {
        if self.pixels.is_empty() {
            return Rgba::TRANSPARENT;
        }
        let mut sum = [0u64; 3];
        for p in &self.pixels {
            for c in 0..3 {
                sum[c] += p[c] as u64;
            }
        }
        let n = self.pixels.len() as u64;
        Rgba::new(
            (sum[0] / n) as u8,
            (sum[1] / n) as u8,
            (sum[2] / n) as u8,
            255,
        )
    }
}

// ── Batch rendering ──

/// A single document to be rendered in a batch process.
pub struct BatchItem {
    /// The document to render.
    pub document: Document,
    /// The name/identifier for the output.
    pub name: String,
}

/// Result of a batch render operation.
pub struct BatchResult {
    /// The name/identifier of the rendered output.
    pub name: String,
    /// The rendered output image buffer.
    pub buffer: ImageBuffer,
}

/// Render multiple documents sequentially.
pub fn batch_render(items: &[BatchItem]) -> Vec<BatchResult> {
    items
        .iter()
        .map(|item| BatchResult {
            name: item.name.clone(),
            buffer: item.document.render(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_buf(w: u32, h: u32, color: Rgba) -> ImageBuffer {
        let mut buf = ImageBuffer::new_transparent(w, h);
        buf.fill(color);
        buf
    }

    #[test]
    fn pack_single_sprite() {
        let buf = solid_buf(16, 16, Rgba::new(255, 0, 0, 255));
        let sheet = pack_sprites(&[&buf], 0, 4096, false);
        assert_eq!(sheet.sprites.len(), 1);
        assert_eq!(sheet.width, 16);
        assert_eq!(sheet.height, 16);
        assert_eq!(sheet.sprites[0].x, 0);
        assert_eq!(sheet.sprites[0].y, 0);
    }

    #[test]
    fn pack_multiple_sprites() {
        let a = solid_buf(10, 20, Rgba::new(255, 0, 0, 255));
        let b = solid_buf(15, 10, Rgba::new(0, 255, 0, 255));
        let c = solid_buf(8, 15, Rgba::new(0, 0, 255, 255));
        let sheet = pack_sprites(&[&a, &b, &c], 1, 4096, false);
        assert_eq!(sheet.sprites.len(), 3);

        // Verify no overlap
        for i in 0..sheet.sprites.len() {
            for j in (i + 1)..sheet.sprites.len() {
                let si = &sheet.sprites[i];
                let sj = &sheet.sprites[j];
                let no_overlap = si.x + si.width <= sj.x
                    || sj.x + sj.width <= si.x
                    || si.y + si.height <= sj.y
                    || sj.y + sj.height <= si.y;
                assert!(no_overlap, "sprites {} and {} overlap", i, j);
            }
        }
    }

    #[test]
    fn pack_power_of_two() {
        let a = solid_buf(10, 10, Rgba::new(255, 0, 0, 255));
        let b = solid_buf(10, 10, Rgba::new(0, 255, 0, 255));
        let sheet = pack_sprites(&[&a, &b], 0, 4096, true);
        assert!(sheet.width.is_power_of_two(), "width={}", sheet.width);
        assert!(sheet.height.is_power_of_two(), "height={}", sheet.height);
    }

    #[test]
    fn contact_sheet_grid() {
        let a = solid_buf(8, 8, Rgba::new(255, 0, 0, 255));
        let b = solid_buf(8, 8, Rgba::new(0, 255, 0, 255));
        let c = solid_buf(8, 8, Rgba::new(0, 0, 255, 255));
        let d = solid_buf(8, 8, Rgba::new(255, 255, 0, 255));

        let result = contact_sheet(
            &[&a, &b, &c, &d],
            &ContactSheetOptions {
                columns: 2,
                cell_width: 8,
                cell_height: 8,
                padding: 2,
                background: Rgba::TRANSPARENT,
            },
        );

        // 2 columns, 2 rows: (2*8 + 2) - 2 = 18 wide, 18 tall
        assert_eq!(result.width, 18);
        assert_eq!(result.height, 18);
    }

    #[test]
    fn contact_sheet_auto_columns() {
        let imgs: Vec<ImageBuffer> = (0..9)
            .map(|_| solid_buf(4, 4, Rgba::new(255, 0, 0, 255)))
            .collect();
        let refs: Vec<&ImageBuffer> = imgs.iter().collect();

        let result = contact_sheet(
            &refs,
            &ContactSheetOptions {
                columns: 0, // auto
                cell_width: 4,
                cell_height: 4,
                padding: 0,
                background: Rgba::TRANSPARENT,
            },
        );

        // ceil(sqrt(9)) = 3 columns, 3 rows → 12x12
        assert_eq!(result.width, 12);
        assert_eq!(result.height, 12);
    }

    #[test]
    fn pixel_scale_doubles() {
        let src = solid_buf(4, 4, Rgba::new(255, 0, 0, 255));
        let dst = pixel_scale(&src, 2);
        assert_eq!(dst.width, 8);
        assert_eq!(dst.height, 8);
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(dst.get_pixel(7, 7), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn pixel_scale_factor_1_noop() {
        let src = solid_buf(4, 4, Rgba::new(255, 0, 0, 255));
        let dst = pixel_scale(&src, 1);
        assert_eq!(dst.width, 4);
        assert_eq!(dst.height, 4);
    }

    #[test]
    fn extract_palette_single_color() {
        let src = solid_buf(4, 4, Rgba::new(100, 200, 50, 255));
        let palette = extract_palette(&src, 4);
        assert_eq!(palette.colors.len(), 1);
        assert_eq!(palette.colors[0], Rgba::new(100, 200, 50, 255));
    }

    #[test]
    fn extract_palette_multiple() {
        // Create an image with distinct color regions
        let mut buf = ImageBuffer::new_transparent(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                let color = if y < 2 {
                    if x < 2 {
                        Rgba::new(255, 0, 0, 255)
                    } else {
                        Rgba::new(0, 255, 0, 255)
                    }
                } else if x < 2 {
                    Rgba::new(0, 0, 255, 255)
                } else {
                    Rgba::new(255, 255, 0, 255)
                };
                buf.set_pixel(x, y, color);
            }
        }
        let palette = extract_palette(&buf, 4);
        assert_eq!(palette.colors.len(), 4);
    }

    #[test]
    fn quantize_reduces_colors() {
        // Create a gradient-like image
        let mut buf = ImageBuffer::new_transparent(10, 1);
        for x in 0..10 {
            let v = (x * 25) as u8;
            buf.set_pixel(x, 0, Rgba::new(v, v, v, 255));
        }

        let palette = Palette {
            colors: vec![
                Rgba::new(0, 0, 0, 255),
                Rgba::new(128, 128, 128, 255),
                Rgba::new(255, 255, 255, 255),
            ],
        };

        let quantized = quantize(&buf, &palette);

        // Every pixel should be one of the palette colors
        for x in 0..10 {
            let p = quantized.get_pixel(x, 0);
            let is_palette = palette
                .colors
                .iter()
                .any(|c| c.r == p.r && c.g == p.g && c.b == p.b);
            assert!(is_palette, "pixel at x={} is ({},{},{})", x, p.r, p.g, p.b);
        }
    }

    #[test]
    fn batch_render_multiple() {
        let mut doc1 = Document::new(2, 2);
        let mut doc2 = Document::new(4, 4);

        // Add a layer to each so they render non-empty
        let id1 = doc1.next_id();
        doc1.layers.push(crate::layer::Layer {
            common: crate::layer::LayerCommon::new(id1, "fill"),
            kind: crate::layer::LayerKind::SolidColor(crate::layer::SolidColorLayerData {
                color: Rgba::new(255, 0, 0, 255),
            }),
        });
        let id2 = doc2.next_id();
        doc2.layers.push(crate::layer::Layer {
            common: crate::layer::LayerCommon::new(id2, "fill"),
            kind: crate::layer::LayerKind::SolidColor(crate::layer::SolidColorLayerData {
                color: Rgba::new(0, 255, 0, 255),
            }),
        });

        let items = vec![
            BatchItem {
                document: doc1,
                name: "first".into(),
            },
            BatchItem {
                document: doc2,
                name: "second".into(),
            },
        ];

        let results = batch_render(&items);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "first");
        assert_eq!(results[0].buffer.width, 2);
        assert_eq!(results[0].buffer.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(results[1].name, "second");
        assert_eq!(results[1].buffer.width, 4);
        assert_eq!(results[1].buffer.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
    }
}
