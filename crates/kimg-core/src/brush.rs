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
use ink_stroke_modeler_rs::{ModelerInput, ModelerInputEventType, ModelerParams, StrokeModeler};

/// Brush tool mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushTool {
    /// Paint colored dabs into the target raster.
    Paint,
    /// Remove alpha from the target raster using the brush tip as an eraser.
    Erase,
}

/// Brush tip shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrushTip {
    /// Standard smooth round/elliptical dab.
    Round,
    /// Grayscale grain tip for a textured brush edge.
    Grain,
}

/// Brush smoothing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushSmoothingMode {
    /// Lightweight in-house low-pass smoothing.
    Simple,
    /// `ink-stroke-modeler-rs` smoothing/modeling.
    Modeler,
}

/// Brush preset parameters for a stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushPreset {
    /// Brush tool mode.
    pub tool: BrushTool,
    /// Brush color.
    pub color: Rgba,
    /// Brush tip shape.
    pub tip: BrushTip,
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
    /// Smoothing strategy.
    pub smoothing_mode: BrushSmoothingMode,
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
            tip: BrushTip::Round,
            size: 12.0,
            opacity: 1.0,
            flow: 1.0,
            hardness: 1.0,
            spacing: 0.25,
            smoothing: 0.0,
            smoothing_mode: BrushSmoothingMode::Simple,
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
    /// Optional normalized tilt in the X axis, typically `[-1.0, 1.0]`.
    pub tilt_x: f32,
    /// Optional normalized tilt in the Y axis, typically `[-1.0, 1.0]`.
    pub tilt_y: f32,
    /// Input timestamp in milliseconds.
    pub time_ms: f32,
}

impl StrokePoint {
    /// Create a new stroke point.
    pub fn new(x: f32, y: f32, pressure: f32) -> Self {
        Self {
            x,
            y,
            pressure,
            tilt_x: 0.0,
            tilt_y: 0.0,
            time_ms: 0.0,
        }
    }

    /// Create a new stroke point with tilt and timestamp information.
    pub fn with_tilt_time(
        x: f32,
        y: f32,
        pressure: f32,
        tilt_x: f32,
        tilt_y: f32,
        time_ms: f32,
    ) -> Self {
        Self {
            x,
            y,
            pressure,
            tilt_x,
            tilt_y,
            time_ms,
        }
    }
}

/// Incremental brush stroke session for interactive painting.
///
/// This preserves smoothing, spacing, and tip-cache state across multiple
/// `apply_points` calls so hosts can stream pointer samples into a raster layer
/// without repainting the full stroke each time.
pub struct BrushStrokeSession {
    preset: BrushPreset,
    alpha_locked: bool,
    tip_cache: HashMap<DabMaskKey, DabMask>,
    pending: f32,
    last_output: Option<StrokePoint>,
    last_stamp: Option<StrokePoint>,
    last_raw: Option<StrokePoint>,
    smoother: StrokeSmoother,
}

impl BrushStrokeSession {
    /// Create a new incremental brush session for the given preset.
    pub fn new(preset: BrushPreset, alpha_locked: bool) -> Self {
        Self {
            preset,
            alpha_locked,
            tip_cache: HashMap::new(),
            pending: 0.0,
            last_output: None,
            last_stamp: None,
            last_raw: None,
            smoother: StrokeSmoother::new(preset),
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

        let spacing_factor = self.preset.spacing.max(0.01);
        let mut any = false;

        for raw_point in points.iter().copied() {
            let point = normalized_point(raw_point);
            let outputs = self.smoother.push(point);
            any |= self.apply_outputs(buffer, &outputs, spacing_factor);
            self.last_raw = Some(point);
        }

        any
    }

    /// Finish the stroke session and flush any remaining modeled output.
    pub fn finish(&mut self, buffer: &mut ImageBuffer) -> bool {
        let spacing_factor = self.preset.spacing.max(0.01);
        let outputs = self.smoother.finish(self.last_raw);
        self.apply_outputs(buffer, &outputs, spacing_factor)
    }

    fn apply_outputs(
        &mut self,
        buffer: &mut ImageBuffer,
        outputs: &[StrokePoint],
        spacing_factor: f32,
    ) -> bool {
        let mut any = false;
        for &output in outputs {
            let Some(previous) = self.last_output else {
                any |= stamp_dab(
                    buffer,
                    &self.preset,
                    output,
                    self.alpha_locked,
                    &mut self.tip_cache,
                );
                self.last_output = Some(output);
                self.last_stamp = Some(output);
                continue;
            };

            any |= self.apply_segment(buffer, previous, output, spacing_factor);
            any |= self.stamp_tail(buffer, output);
            self.last_output = Some(output);
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
                tilt_x: lerp(from.tilt_x, to.tilt_x, t).clamp(-1.0, 1.0),
                tilt_y: lerp(from.tilt_y, to.tilt_y, t).clamp(-1.0, 1.0),
                time_ms: lerp(from.time_ms, to.time_ms, t),
            };
            any |= stamp_dab(
                buffer,
                &self.preset,
                dab,
                self.alpha_locked,
                &mut self.tip_cache,
            );
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

        let stamped = stamp_dab(
            buffer,
            &self.preset,
            point,
            self.alpha_locked,
            &mut self.tip_cache,
        );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DabMaskKey {
    diameter: u32,
    hardness: u8,
    tip: BrushTip,
    aspect_ratio: u8,
    angle_bucket: u16,
}

impl DabMaskKey {
    fn new(preset: &BrushPreset, point: StrokePoint, diameter: u32) -> Self {
        let (aspect, angle) = effective_tilt_shape(point);
        Self {
            diameter,
            hardness: (clamp01(preset.hardness) * 255.0).round() as u8,
            tip: preset.tip,
            aspect_ratio: ((aspect - 1.0) * 64.0).round().clamp(0.0, 255.0) as u8,
            angle_bucket: ((normalize_angle_rad(angle) / std::f32::consts::TAU) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u16,
        }
    }
}

enum StrokeSmoother {
    Simple {
        alpha: f32,
        last: Option<StrokePoint>,
    },
    Modeler {
        engine: Box<StrokeModeler>,
        started: bool,
        last_time_ms: Option<f32>,
        sample_index: u64,
    },
}

impl StrokeSmoother {
    fn new(preset: BrushPreset) -> Self {
        match preset.smoothing_mode {
            BrushSmoothingMode::Simple => Self::Simple {
                alpha: 1.0 - clamp01(preset.smoothing).clamp(0.0, 0.95),
                last: None,
            },
            BrushSmoothingMode::Modeler => Self::Modeler {
                engine: Box::new(
                    StrokeModeler::new(modeler_params_for(preset))
                        .unwrap_or_else(|_| StrokeModeler::default()),
                ),
                started: false,
                last_time_ms: None,
                sample_index: 0,
            },
        }
    }

    fn push(&mut self, point: StrokePoint) -> Vec<StrokePoint> {
        match self {
            StrokeSmoother::Simple { alpha, last } => {
                let point = normalized_point(point);
                let smoothed = match *last {
                    None => point,
                    Some(_previous) if *alpha >= 0.999 => point,
                    Some(previous) => StrokePoint {
                        x: previous.x + (point.x - previous.x) * *alpha,
                        y: previous.y + (point.y - previous.y) * *alpha,
                        pressure: (previous.pressure
                            + (point.pressure - previous.pressure) * *alpha)
                            .clamp(0.0, 1.0),
                        tilt_x: previous.tilt_x + (point.tilt_x - previous.tilt_x) * *alpha,
                        tilt_y: previous.tilt_y + (point.tilt_y - previous.tilt_y) * *alpha,
                        time_ms: point.time_ms,
                    },
                };
                *last = Some(smoothed);
                vec![smoothed]
            }
            StrokeSmoother::Modeler {
                engine,
                started,
                last_time_ms,
                sample_index,
            } => {
                let point = normalized_point(point);
                let input = ModelerInput {
                    event_type: if *started {
                        ModelerInputEventType::Move
                    } else {
                        ModelerInputEventType::Down
                    },
                    pos: (point.x as f64, point.y as f64),
                    time: normalized_time_ms(point.time_ms, last_time_ms, sample_index) as f64
                        / 1000.0,
                    pressure: point.pressure as f64,
                };
                let outputs = engine
                    .update(input)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|result| StrokePoint {
                        x: result.pos.0 as f32,
                        y: result.pos.1 as f32,
                        pressure: result.pressure as f32,
                        tilt_x: point.tilt_x,
                        tilt_y: point.tilt_y,
                        time_ms: (result.time * 1000.0) as f32,
                    })
                    .collect::<Vec<_>>();
                *started = true;
                *last_time_ms = outputs
                    .last()
                    .map(|output| output.time_ms)
                    .or(Some(point.time_ms));
                *sample_index = sample_index.wrapping_add(1);
                if outputs.is_empty() {
                    vec![point]
                } else {
                    outputs
                }
            }
        }
    }

    fn finish(&mut self, last_raw: Option<StrokePoint>) -> Vec<StrokePoint> {
        match self {
            StrokeSmoother::Simple { .. } => Vec::new(),
            StrokeSmoother::Modeler {
                engine,
                started,
                last_time_ms,
                sample_index,
            } => {
                let Some(point) = last_raw.map(normalized_point) else {
                    return Vec::new();
                };
                if !*started {
                    return Vec::new();
                }
                let input = ModelerInput {
                    event_type: ModelerInputEventType::Up,
                    pos: (point.x as f64, point.y as f64),
                    time: normalized_time_ms(
                        if point.time_ms > 0.0 {
                            point.time_ms + 1.0
                        } else {
                            point.time_ms
                        },
                        last_time_ms,
                        sample_index,
                    ) as f64
                        / 1000.0,
                    pressure: point.pressure as f64,
                };
                let outputs = engine
                    .update(input)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|result| StrokePoint {
                        x: result.pos.0 as f32,
                        y: result.pos.1 as f32,
                        pressure: result.pressure as f32,
                        tilt_x: point.tilt_x,
                        tilt_y: point.tilt_y,
                        time_ms: (result.time * 1000.0) as f32,
                    })
                    .collect();
                *started = false;
                outputs
            }
        }
    }
}

/// Paint a batched stroke into a raster buffer.
///
/// Returns `true` when the stroke touched the buffer and `false` when nothing
/// was applied (for example, empty input or fully transparent brush settings).
pub fn paint_stroke(
    buffer: &mut ImageBuffer,
    preset: &BrushPreset,
    points: &[StrokePoint],
    alpha_locked: bool,
) -> bool {
    let mut session = BrushStrokeSession::new(*preset, alpha_locked);
    let mut any = session.apply_points(buffer, points);
    any |= session.finish(buffer);
    any
}

fn stamp_dab(
    buffer: &mut ImageBuffer,
    preset: &BrushPreset,
    point: StrokePoint,
    alpha_locked: bool,
    tip_cache: &mut HashMap<DabMaskKey, DabMask>,
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
    let mask_key = DabMaskKey::new(preset, point, diameter);
    let mask = tip_cache
        .entry(mask_key)
        .or_insert_with(|| build_dab_mask(mask_key));

    apply_mask(
        buffer,
        mask,
        point.x,
        point.y,
        preset,
        alpha_scale,
        alpha_locked,
    )
}

fn apply_mask(
    buffer: &mut ImageBuffer,
    mask: &DabMask,
    center_x: f32,
    center_y: f32,
    preset: &BrushPreset,
    alpha_scale: f32,
    alpha_locked: bool,
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
            if alpha_locked && buffer.data[pixel_index + 3] == 0 {
                continue;
            }
            match preset.tool {
                BrushTool::Paint => composite_paint_pixel(
                    &mut buffer.data[pixel_index..pixel_index + 4],
                    preset.color,
                    applied_alpha,
                ),
                BrushTool::Erase => {
                    if alpha_locked {
                        continue;
                    }
                    erase_pixel(
                        &mut buffer.data[pixel_index..pixel_index + 4],
                        applied_alpha,
                    )
                }
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

fn build_dab_mask(key: DabMaskKey) -> DabMask {
    let diameter = key.diameter;
    let radius = diameter as f32 / 2.0;
    let center = radius;
    let hardness = key.hardness as f32 / 255.0;
    let inner_radius = radius * clamp01(hardness);
    let aspect = 1.0 + (key.aspect_ratio as f32 / 64.0);
    let radius_x = radius * aspect;
    let radius_y = (radius / aspect.max(1.0)).max(0.5);
    let angle = key.angle_bucket as f32 / 255.0 * std::f32::consts::TAU;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let mut alpha = vec![0u8; (diameter * diameter) as usize];

    for y in 0..diameter {
        for x in 0..diameter {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let local_x = dx * cos_a + dy * sin_a;
            let local_y = -dx * sin_a + dy * cos_a;
            let distance = ((local_x / radius_x).powi(2) + (local_y / radius_y).powi(2)).sqrt();
            let sample = if distance >= 1.0 {
                0.0
            } else if hardness >= 0.999 || distance * radius <= inner_radius {
                1.0
            } else {
                ((1.0 - distance) / (1.0 - (inner_radius / radius.max(0.5)))).clamp(0.0, 1.0)
            };

            alpha[(y * diameter + x) as usize] =
                (sample * texture_factor(key.tip, x, y, diameter) * 255.0).round() as u8;
        }
    }

    DabMask { diameter, alpha }
}

fn texture_factor(tip: BrushTip, x: u32, y: u32, diameter: u32) -> f32 {
    match tip {
        BrushTip::Round => 1.0,
        BrushTip::Grain => {
            const GRAIN: [[u8; 8]; 8] = [
                [255, 196, 255, 140, 255, 176, 240, 120],
                [168, 248, 150, 232, 160, 255, 148, 224],
                [255, 132, 220, 176, 255, 128, 208, 160],
                [156, 236, 144, 255, 184, 255, 140, 232],
                [255, 176, 232, 128, 255, 188, 246, 150],
                [184, 255, 138, 224, 170, 255, 152, 244],
                [240, 154, 220, 182, 236, 136, 255, 166],
                [144, 228, 160, 255, 150, 236, 170, 255],
            ];
            let sample_x = ((x as f32 / diameter.max(1) as f32) * 7.0).round() as usize;
            let sample_y = ((y as f32 / diameter.max(1) as f32) * 7.0).round() as usize;
            0.55 + (GRAIN[sample_y.min(7)][sample_x.min(7)] as f32 / 255.0) * 0.45
        }
    }
}

fn effective_tilt_shape(point: StrokePoint) -> (f32, f32) {
    let tilt_x = point.tilt_x.clamp(-1.0, 1.0);
    let tilt_y = point.tilt_y.clamp(-1.0, 1.0);
    let magnitude = (tilt_x * tilt_x + tilt_y * tilt_y).sqrt().clamp(0.0, 1.0);
    if magnitude <= 0.01 {
        return (1.0, 0.0);
    }
    let aspect = 1.0 + magnitude * 0.75;
    let angle = tilt_y.atan2(tilt_x);
    (aspect, angle)
}

fn normalize_angle_rad(angle: f32) -> f32 {
    angle.rem_euclid(std::f32::consts::TAU)
}

fn modeler_params_for(preset: BrushPreset) -> ModelerParams {
    let smoothing = clamp01(preset.smoothing) as f64;
    let mut params = ModelerParams::suggested();
    params.wobble_smoother_timeout = 0.02 + smoothing * 0.05;
    params.position_modeler_drag_constant = 48.0 + smoothing * 48.0;
    params.position_modeler_spring_mass_constant = (11.0 / 32400.0) * (1.0 - smoothing * 0.35);
    params
}

fn normalized_time_ms(time_ms: f32, last_time_ms: &Option<f32>, sample_index: &u64) -> f32 {
    if time_ms.is_finite() && time_ms > 0.0 {
        return match last_time_ms {
            Some(last) if time_ms <= *last => *last + 1.0,
            _ => time_ms,
        };
    }

    match last_time_ms {
        Some(last) => *last + 16.0,
        None => *sample_index as f32 * 16.0,
    }
}

fn normalized_point(point: StrokePoint) -> StrokePoint {
    StrokePoint {
        pressure: point.pressure.clamp(0.0, 1.0),
        tilt_x: point.tilt_x.clamp(-1.0, 1.0),
        tilt_y: point.tilt_y.clamp(-1.0, 1.0),
        time_ms: if point.time_ms.is_finite() {
            point.time_ms.max(0.0)
        } else {
            0.0
        },
        ..point
    }
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
            false,
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

        paint_stroke(
            &mut buffer,
            &preset,
            &[StrokePoint::new(5.0, 5.0, 1.0)],
            false,
        );

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

        paint_stroke(
            &mut buffer,
            &preset,
            &[StrokePoint::new(4.0, 4.0, 1.0)],
            false,
        );

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

        paint_stroke(&mut low, &preset, &[StrokePoint::new(8.0, 8.0, 0.2)], false);
        paint_stroke(
            &mut high,
            &preset,
            &[StrokePoint::new(8.0, 8.0, 1.0)],
            false,
        );

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
            false,
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

        assert!(paint_stroke(&mut batched, &preset, &points, false));

        let mut session = BrushStrokeSession::new(preset, false);
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
        let mut session = BrushStrokeSession::new(preset, false);

        assert!(session.apply_points(&mut buffer, &[StrokePoint::new(4.0, 4.0, 0.4)]));
        assert!(session.apply_points(&mut buffer, &[StrokePoint::new(6.0, 5.0, 0.5)]));
        assert!(session.apply_points(&mut buffer, &[StrokePoint::new(9.0, 7.0, 0.8)]));

        assert!(buffer.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn grain_tip_leaves_textured_alpha_pattern() {
        let mut round = ImageBuffer::new_transparent(24, 24);
        let mut grain = ImageBuffer::new_transparent(24, 24);
        let base = BrushPreset {
            color: Rgba::new(255, 255, 255, 255),
            hardness: 0.6,
            size: 12.0,
            ..BrushPreset::default()
        };

        paint_stroke(
            &mut round,
            &base,
            &[StrokePoint::new(12.0, 12.0, 1.0)],
            false,
        );
        paint_stroke(
            &mut grain,
            &BrushPreset {
                tip: BrushTip::Grain,
                ..base
            },
            &[StrokePoint::new(12.0, 12.0, 1.0)],
            false,
        );

        assert_ne!(round.data, grain.data);
        assert!(grain
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[3] > 0 && pixel[3] < 255));
    }

    #[test]
    fn tilt_shapes_dab_into_ellipse() {
        let mut buffer = ImageBuffer::new_transparent(32, 32);
        let preset = BrushPreset {
            color: Rgba::new(255, 255, 255, 255),
            size: 12.0,
            ..BrushPreset::default()
        };

        paint_stroke(
            &mut buffer,
            &preset,
            &[StrokePoint::with_tilt_time(16.0, 16.0, 1.0, 1.0, 0.0, 0.0)],
            false,
        );

        let horizontal = (0..buffer.width)
            .filter(|&x| (0..buffer.height).any(|y| buffer.get_pixel(x, y).a > 0))
            .count();
        let vertical = (0..buffer.height)
            .filter(|&y| (0..buffer.width).any(|x| buffer.get_pixel(x, y).a > 0))
            .count();

        assert!(horizontal > vertical);
    }

    #[test]
    fn modeler_smoothing_paints_visible_stroke() {
        let mut buffer = ImageBuffer::new_transparent(48, 48);
        let preset = BrushPreset {
            color: Rgba::new(35, 79, 221, 255),
            size: 8.0,
            smoothing: 0.5,
            smoothing_mode: BrushSmoothingMode::Modeler,
            ..BrushPreset::default()
        };
        let points = [
            StrokePoint::with_tilt_time(6.0, 8.0, 0.4, 0.0, 0.0, 0.0),
            StrokePoint::with_tilt_time(18.0, 20.0, 0.8, 0.0, 0.0, 16.0),
            StrokePoint::with_tilt_time(36.0, 32.0, 1.0, 0.0, 0.0, 32.0),
        ];

        assert!(paint_stroke(&mut buffer, &preset, &points, false));
        assert!(buffer.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn alpha_locked_stroke_preserves_transparent_pixels() {
        let mut buffer = ImageBuffer::new_transparent(16, 16);
        buffer.set_pixel(8, 8, Rgba::new(40, 50, 60, 255));
        let preset = BrushPreset {
            color: Rgba::new(255, 0, 0, 255),
            size: 8.0,
            hardness: 1.0,
            ..BrushPreset::default()
        };

        assert!(paint_stroke(
            &mut buffer,
            &preset,
            &[StrokePoint::new(8.0, 8.0, 1.0)],
            true,
        ));

        assert_eq!(buffer.get_pixel(0, 0), Rgba::TRANSPARENT);
        assert_eq!(buffer.get_pixel(8, 8).a, 255);
        assert_eq!(buffer.get_pixel(8, 8).r, 255);
    }
}
