//! Raster brush engine for painting into [`Raster`](crate::layer::LayerKind::Raster) layers.
//!
//! The first brush engine is deliberately simple and deterministic:
//!
//! - round dab-based brush
//! - paint and erase modes
//! - size, opacity, flow, hardness, spacing, and smoothing
//! - optional pressure-driven size and opacity
//! - direct RGBA compositing into raster buffers
//!
//! It is designed for `kimg`'s current document model rather than as a general
//! vector stroke system.

use std::collections::HashMap;

use crate::buffer::ImageBuffer;
use crate::pixel::Rgba;

/// Brush tool mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushTool {
    /// Paint colored dabs into the target raster.
    Paint,
    /// Remove alpha from the target raster using the brush tip as an eraser.
    Erase,
}

/// Brush preset parameters for a stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushPreset {
    /// Brush tool mode.
    pub tool: BrushTool,
    /// Brush color.
    pub color: Rgba,
    /// Base brush diameter in pixels.
    pub size: f32,
    /// Global stroke opacity multiplier in `[0.0, 1.0]`.
    pub opacity: f32,
    /// Per-dab flow multiplier in `[0.0, 1.0]`.
    pub flow: f32,
    /// Hardness in `[0.0, 1.0]`, where `1.0` is a hard edge.
    pub hardness: f32,
    /// Spacing as a fraction of the current brush diameter.
    pub spacing: f32,
    /// Smoothing/stabilization amount in `[0.0, 1.0]`.
    pub smoothing: f32,
    /// Pressure influence on brush size in `[0.0, 1.0]`.
    pub pressure_size: f32,
    /// Pressure influence on opacity in `[0.0, 1.0]`.
    pub pressure_opacity: f32,
}

impl Default for BrushPreset {
    fn default() -> Self {
        Self {
            tool: BrushTool::Paint,
            color: Rgba::new(0, 0, 0, 255),
            size: 12.0,
            opacity: 1.0,
            flow: 1.0,
            hardness: 1.0,
            spacing: 0.25,
            smoothing: 0.0,
            pressure_size: 1.0,
            pressure_opacity: 0.0,
        }
    }
}

/// One sampled input point in layer-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokePoint {
    /// X coordinate in layer-local pixels.
    pub x: f32,
    /// Y coordinate in layer-local pixels.
    pub y: f32,
    /// Normalized pressure in `[0.0, 1.0]`.
    pub pressure: f32,
}

impl StrokePoint {
    /// Create a new stroke point.
    pub fn new(x: f32, y: f32, pressure: f32) -> Self {
        Self { x, y, pressure }
    }
}

/// Incremental brush stroke session for interactive painting.
///
/// This preserves smoothing, spacing, and tip-cache state across multiple
/// `apply_points` calls so hosts can stream pointer samples into a raster layer
/// without repainting the full stroke each time.
#[derive(Debug, Clone)]
pub struct BrushStrokeSession {
    preset: BrushPreset,
    tip_cache: HashMap<(u32, u8), DabMask>,
    pending: f32,
    last_smoothed: Option<StrokePoint>,
    last_stamp: Option<StrokePoint>,
}

impl BrushStrokeSession {
    /// Create a new incremental brush session for the given preset.
    pub fn new(preset: BrushPreset) -> Self {
        Self {
            preset,
            tip_cache: HashMap::new(),
            pending: 0.0,
            last_smoothed: None,
            last_stamp: None,
        }
    }

    /// Apply streamed points into the target raster buffer.
    ///
    /// Returns `true` when the session touched the buffer and `false` when the
    /// provided points or brush settings produced no visible output.
    pub fn apply_points(&mut self, buffer: &mut ImageBuffer, points: &[StrokePoint]) -> bool {
        if buffer.width == 0 || buffer.height == 0 || points.is_empty() {
            return false;
        }

        let size = self.preset.size.max(0.0);
        if size <= 0.0 {
            return false;
        }

        let opacity = clamp01(self.preset.opacity);
        let flow = clamp01(self.preset.flow);
        if opacity <= 0.0 || flow <= 0.0 {
            return false;
        }

        let smoothing = clamp01(self.preset.smoothing);
        let alpha = 1.0 - smoothing.clamp(0.0, 0.95);
        let spacing_factor = self.preset.spacing.max(0.01);
        let mut any = false;

        for raw_point in points.iter().copied() {
            let point = StrokePoint {
                pressure: raw_point.pressure.clamp(0.0, 1.0),
                ..raw_point
            };

            let Some(previous) = self.last_smoothed else {
                any |= stamp_dab(buffer, &self.preset, point, &mut self.tip_cache);
                self.last_smoothed = Some(point);
                self.last_stamp = Some(point);
                continue;
            };

            let smoothed = if smoothing <= 0.0 {
                point
            } else {
                StrokePoint {
                    x: previous.x + (point.x - previous.x) * alpha,
                    y: previous.y + (point.y - previous.y) * alpha,
                    pressure: (previous.pressure + (point.pressure - previous.pressure) * alpha)
                        .clamp(0.0, 1.0),
                }
            };

            any |= self.apply_segment(buffer, previous, smoothed, spacing_factor);
            any |= self.stamp_tail(buffer, smoothed);
            self.last_smoothed = Some(smoothed);
        }

        any
    }

    fn apply_segment(
        &mut self,
        buffer: &mut ImageBuffer,
        from: StrokePoint,
        to: StrokePoint,
        spacing_factor: f32,
    ) -> bool {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f32::EPSILON {
            return false;
        }

        let average_pressure = ((from.pressure + to.pressure) * 0.5).clamp(0.0, 1.0);
        let average_size = effective_size(
            self.preset.size.max(1.0),
            self.preset.pressure_size,
            average_pressure,
        );
        let spacing_px = (average_size * spacing_factor).max(1.0);
        let mut distance = if self.pending <= 0.0 {
            spacing_px
        } else {
            self.pending
        };
        let mut any = false;

        while distance <= length {
            let t = distance / length;
            let dab = StrokePoint {
                x: lerp(from.x, to.x, t),
                y: lerp(from.y, to.y, t),
                pressure: lerp(from.pressure, to.pressure, t).clamp(0.0, 1.0),
            };
            any |= stamp_dab(buffer, &self.preset, dab, &mut self.tip_cache);
            self.last_stamp = Some(dab);
            distance += spacing_px;
        }

        self.pending = distance - length;
        any
    }

    fn stamp_tail(&mut self, buffer: &mut ImageBuffer, point: StrokePoint) -> bool {
        let should_stamp = self.last_stamp.is_none_or(|last| {
            let dx = point.x - last.x;
            let dy = point.y - last.y;
            (dx * dx + dy * dy).sqrt() >= 0.5
        });

        if !should_stamp {
            return false;
        }

        let stamped = stamp_dab(buffer, &self.preset, point, &mut self.tip_cache);
        if stamped {
            self.last_stamp = Some(point);
        }
        stamped
    }
}

#[derive(Debug, Clone)]
struct DabMask {
    diameter: u32,
    alpha: Vec<u8>,
}

/// Paint a batched stroke into a raster buffer.
///
/// Returns `true` when the stroke touched the buffer and `false` when nothing
/// was applied (for example, empty input or fully transparent brush settings).
pub fn paint_stroke(
    buffer: &mut ImageBuffer,
    preset: &BrushPreset,
    points: &[StrokePoint],
) -> bool {
    let mut session = BrushStrokeSession::new(*preset);
    session.apply_points(buffer, points)
}

fn stamp_dab(
    buffer: &mut ImageBuffer,
    preset: &BrushPreset,
    point: StrokePoint,
    tip_cache: &mut HashMap<(u32, u8), DabMask>,
) -> bool {
    let pressure = point.pressure.clamp(0.0, 1.0);
    let size = effective_size(preset.size.max(1.0), preset.pressure_size, pressure);
    let alpha_scale = clamp01(
        effective_opacity(preset.opacity, preset.pressure_opacity, pressure) * clamp01(preset.flow),
    );

    if size <= 0.0 || alpha_scale <= 0.0 {
        return false;
    }

    let diameter = size.round().max(1.0) as u32;
    let hardness_key = (clamp01(preset.hardness) * 255.0).round() as u8;
    let mask = tip_cache
        .entry((diameter, hardness_key))
        .or_insert_with(|| build_dab_mask(diameter, clamp01(preset.hardness)));

    apply_mask(buffer, mask, point.x, point.y, preset, alpha_scale)
}

fn apply_mask(
    buffer: &mut ImageBuffer,
    mask: &DabMask,
    center_x: f32,
    center_y: f32,
    preset: &BrushPreset,
    alpha_scale: f32,
) -> bool {
    let radius = mask.diameter as f32 / 2.0;
    let left = (center_x - radius).floor() as i32;
    let top = (center_y - radius).floor() as i32;
    let right = (center_x + radius).ceil() as i32;
    let bottom = (center_y + radius).ceil() as i32;

    let mut any = false;
    let width = buffer.width as i32;
    let height = buffer.height as i32;
    let src_alpha_scale = clamp01(alpha_scale) * (preset.color.a as f32 / 255.0);

    for y in top.max(0)..bottom.min(height) {
        for x in left.max(0)..right.min(width) {
            let mask_x = x - left;
            let mask_y = y - top;
            if mask_x < 0 || mask_y < 0 {
                continue;
            }
            let mask_x = mask_x as u32;
            let mask_y = mask_y as u32;
            if mask_x >= mask.diameter || mask_y >= mask.diameter {
                continue;
            }

            let mask_alpha = mask.alpha[(mask_y * mask.diameter + mask_x) as usize];
            if mask_alpha == 0 {
                continue;
            }

            let applied_alpha =
                (src_alpha_scale * (mask_alpha as f32 / 255.0) * 255.0).round() as u8;
            if applied_alpha == 0 {
                continue;
            }

            let pixel_index = ((y as u32 * buffer.width + x as u32) * 4) as usize;
            match preset.tool {
                BrushTool::Paint => composite_paint_pixel(
                    &mut buffer.data[pixel_index..pixel_index + 4],
                    preset.color,
                    applied_alpha,
                ),
                BrushTool::Erase => erase_pixel(
                    &mut buffer.data[pixel_index..pixel_index + 4],
                    applied_alpha,
                ),
            }
            any = true;
        }
    }

    any
}

fn composite_paint_pixel(dst: &mut [u8], color: Rgba, src_alpha: u8) {
    let sa = src_alpha as u32;
    if sa == 0 {
        return;
    }

    if sa == 255 {
        dst[0] = color.r;
        dst[1] = color.g;
        dst[2] = color.b;
        dst[3] = 255;
        return;
    }

    let da = dst[3] as u32;
    if da == 0 {
        dst[0] = color.r;
        dst[1] = color.g;
        dst[2] = color.b;
        dst[3] = src_alpha;
        return;
    }

    let inv_sa = 255 - sa;
    let out_a = sa + ((da * inv_sa + 127) / 255);
    if out_a == 0 {
        return;
    }

    let src_channels = [color.r, color.g, color.b];
    for channel in 0..3 {
        let src_term = src_channels[channel] as u32 * sa;
        let dst_term = (dst[channel] as u32 * da * inv_sa + 127) / 255;
        let out = (src_term + dst_term + out_a / 2) / out_a;
        dst[channel] = out as u8;
    }
    dst[3] = out_a as u8;
}

fn erase_pixel(dst: &mut [u8], erase_alpha: u8) {
    let da = dst[3] as u32;
    if da == 0 {
        return;
    }

    let inv = 255 - erase_alpha as u32;
    let out_a = (da * inv + 127) / 255;
    dst[3] = out_a as u8;
    if out_a == 0 {
        dst[0] = 0;
        dst[1] = 0;
        dst[2] = 0;
    }
}

fn build_dab_mask(diameter: u32, hardness: f32) -> DabMask {
    let radius = diameter as f32 / 2.0;
    let center = radius;
    let inner_radius = radius * clamp01(hardness);
    let mut alpha = vec![0u8; (diameter * diameter) as usize];

    for y in 0..diameter {
        for x in 0..diameter {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let sample = if distance >= radius {
                0.0
            } else if hardness >= 0.999 || distance <= inner_radius {
                1.0
            } else {
                ((radius - distance) / (radius - inner_radius)).clamp(0.0, 1.0)
            };

            alpha[(y * diameter + x) as usize] = (sample * 255.0).round() as u8;
        }
    }

    DabMask { diameter, alpha }
}

fn effective_size(base: f32, pressure_size: f32, pressure: f32) -> f32 {
    let influence = clamp01(pressure_size);
    base * ((1.0 - influence) + pressure.clamp(0.0, 1.0) * influence)
}

fn effective_opacity(base: f32, pressure_opacity: f32, pressure: f32) -> f32 {
    let influence = clamp01(pressure_opacity);
    clamp01(base) * ((1.0 - influence) + pressure.clamp(0.0, 1.0) * influence)
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_dab_paints_center_pixel() {
        let mut buffer = ImageBuffer::new_transparent(9, 9);
        let preset = BrushPreset {
            color: Rgba::new(255, 0, 0, 255),
            hardness: 1.0,
            size: 5.0,
            ..BrushPreset::default()
        };

        assert!(paint_stroke(
            &mut buffer,
            &preset,
            &[StrokePoint::new(4.0, 4.0, 1.0)],
        ));

        assert_eq!(buffer.get_pixel(4, 4), Rgba::new(255, 0, 0, 255));
        assert_eq!(buffer.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn soft_dab_falls_off_toward_edges() {
        let mut buffer = ImageBuffer::new_transparent(11, 11);
        let preset = BrushPreset {
            color: Rgba::new(255, 0, 0, 255),
            hardness: 0.0,
            size: 7.0,
            ..BrushPreset::default()
        };

        paint_stroke(&mut buffer, &preset, &[StrokePoint::new(5.0, 5.0, 1.0)]);

        let center = buffer.get_pixel(5, 5).a;
        let shoulder = buffer.get_pixel(7, 5).a;
        let edge = buffer.get_pixel(8, 5).a;

        assert!(center > shoulder);
        assert!(shoulder > edge);
    }

    #[test]
    fn eraser_reduces_alpha_without_touching_untouched_pixels() {
        let mut buffer = ImageBuffer::new_transparent(8, 8);
        buffer.fill(Rgba::new(40, 50, 60, 255));
        let preset = BrushPreset {
            tool: BrushTool::Erase,
            size: 5.0,
            hardness: 1.0,
            ..BrushPreset::default()
        };

        paint_stroke(&mut buffer, &preset, &[StrokePoint::new(4.0, 4.0, 1.0)]);

        assert!(buffer.get_pixel(4, 4).a < 255);
        assert_eq!(buffer.get_pixel(0, 0).a, 255);
    }

    #[test]
    fn pressure_changes_stroke_coverage() {
        let mut low = ImageBuffer::new_transparent(16, 16);
        let mut high = ImageBuffer::new_transparent(16, 16);
        let preset = BrushPreset {
            color: Rgba::new(0, 0, 0, 255),
            size: 10.0,
            pressure_size: 1.0,
            ..BrushPreset::default()
        };

        paint_stroke(&mut low, &preset, &[StrokePoint::new(8.0, 8.0, 0.2)]);
        paint_stroke(&mut high, &preset, &[StrokePoint::new(8.0, 8.0, 1.0)]);

        let low_pixels = low
            .data
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .count();
        let high_pixels = high
            .data
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .count();
        assert!(high_pixels > low_pixels);
    }

    #[test]
    fn stroke_spacing_places_multiple_dabs() {
        let mut buffer = ImageBuffer::new_transparent(48, 8);
        let preset = BrushPreset {
            color: Rgba::new(255, 255, 255, 255),
            size: 4.0,
            spacing: 0.5,
            ..BrushPreset::default()
        };

        paint_stroke(
            &mut buffer,
            &preset,
            &[
                StrokePoint::new(2.0, 4.0, 1.0),
                StrokePoint::new(42.0, 4.0, 1.0),
            ],
        );

        let painted_columns = (0..buffer.width)
            .filter(|&x| (0..buffer.height).any(|y| buffer.get_pixel(x, y).a > 0))
            .count();
        assert!(painted_columns > 10);
    }

    #[test]
    fn streaming_session_matches_batched_stroke() {
        let mut batched = ImageBuffer::new_transparent(96, 96);
        let mut streamed = ImageBuffer::new_transparent(96, 96);
        let preset = BrushPreset {
            color: Rgba::new(35, 79, 221, 255),
            flow: 0.75,
            hardness: 0.4,
            opacity: 0.9,
            pressure_opacity: 0.5,
            pressure_size: 1.0,
            size: 14.0,
            smoothing: 0.3,
            spacing: 0.28,
            ..BrushPreset::default()
        };
        let points = [
            StrokePoint::new(8.0, 10.0, 0.3),
            StrokePoint::new(18.0, 16.0, 0.5),
            StrokePoint::new(30.0, 28.0, 0.8),
            StrokePoint::new(42.0, 24.0, 1.0),
            StrokePoint::new(60.0, 40.0, 0.7),
            StrokePoint::new(82.0, 68.0, 0.9),
        ];

        assert!(paint_stroke(&mut batched, &preset, &points));

        let mut session = BrushStrokeSession::new(preset);
        assert!(session.apply_points(&mut streamed, &points[..2]));
        assert!(session.apply_points(&mut streamed, &points[2..4]));
        assert!(session.apply_points(&mut streamed, &points[4..]));

        assert_eq!(streamed.data, batched.data);
    }

    #[test]
    fn streaming_session_handles_sparse_short_pushes() {
        let mut buffer = ImageBuffer::new_transparent(32, 32);
        let preset = BrushPreset {
            color: Rgba::new(255, 255, 255, 255),
            size: 6.0,
            spacing: 0.45,
            smoothing: 0.2,
            ..BrushPreset::default()
        };
        let mut session = BrushStrokeSession::new(preset);

        assert!(session.apply_points(&mut buffer, &[StrokePoint::new(4.0, 4.0, 0.4)]));
        assert!(session.apply_points(&mut buffer, &[StrokePoint::new(6.0, 5.0, 0.5)]));
        assert!(session.apply_points(&mut buffer, &[StrokePoint::new(9.0, 7.0, 0.8)]));

        assert!(buffer.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }
}
