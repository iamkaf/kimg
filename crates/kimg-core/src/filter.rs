//! Non-destructive pixel filters.
//!
//! Two categories of filters are provided:
//!
//! **HSL/tone pipeline** — [`apply_hsl_filter`] applies a chain of adjustments
//! (hue, saturation, lightness, brightness, contrast, temperature, tint,
//! sharpen, alpha) configured via [`HslFilterConfig`].  All values default to
//! zero / no-op.
//!
//! **Pixel-level filters** — standalone functions that operate directly on an
//! [`ImageBuffer`] in place: [`invert`], [`posterize`], [`threshold`],
//! [`levels`], and [`gradient_map`].

use crate::buffer::ImageBuffer;
use crate::color::{hsl_to_rgb, rgb_to_hsl};

/// Configuration for the HSL/tone filter pipeline.
///
/// Ported from Spriteform compositorRenderMath.ts `applyHslFilter`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct HslFilterConfig {
    /// Hue offset in degrees.
    pub hue_deg: f64,
    /// Saturation delta, -1.0 to 1.0.
    pub saturation: f64,
    /// Lightness delta, -1.0 to 1.0.
    pub lightness: f64,
    /// Alpha delta, -1.0 to 1.0.
    pub alpha: f64,
    /// Brightness delta, -1.0 to 1.0.
    pub brightness: f64,
    /// Contrast delta, -1.0 to 1.0.
    pub contrast: f64,
    /// Temperature shift, -1.0 to 1.0 (cool to warm).
    pub temperature: f64,
    /// Tint shift, -1.0 to 1.0 (green to magenta).
    pub tint: f64,
    /// Sharpen strength, 0.0 to 1.0.
    pub sharpen: f64,
}

impl Default for HslFilterConfig {
    fn default() -> Self {
        Self {
            hue_deg: 0.0,
            saturation: 0.0,
            lightness: 0.0,
            alpha: 0.0,
            brightness: 0.0,
            contrast: 0.0,
            temperature: 0.0,
            tint: 0.0,
            sharpen: 0.0,
        }
    }
}

fn clamp_byte(n: f64) -> u8 {
    (n as i32).clamp(0, 255) as u8
}

/// Apply the full HSL + tone filter pipeline in-place on `buf`.
pub fn apply_hsl_filter(buf: &mut ImageBuffer, cfg: &HslFilterConfig) {
    let hue = cfg.hue_deg;
    let sat = cfg.saturation;
    let light = cfg.lightness;
    let a_delta = cfg.alpha;
    let brightness = cfg.brightness.clamp(-1.0, 1.0);
    let contrast = cfg.contrast.clamp(-1.0, 1.0);
    let temperature = cfg.temperature.clamp(-1.0, 1.0);
    let tint = cfg.tint.clamp(-1.0, 1.0);
    let sharpen = cfg.sharpen.clamp(0.0, 1.0);

    let has_hsl_adjust = hue != 0.0 || sat != 0.0 || light != 0.0;
    let has_alpha_adjust = a_delta != 0.0;
    let has_tone_adjust = brightness != 0.0 || contrast != 0.0 || temperature != 0.0 || tint != 0.0;
    let has_sharpen = sharpen > 0.0;

    if !(has_hsl_adjust || has_alpha_adjust || has_tone_adjust || has_sharpen) {
        return;
    }

    let pixel_count = (buf.width as usize) * (buf.height as usize);
    let mut rgb_base: Vec<u8> = if has_sharpen {
        vec![0; pixel_count * 3]
    } else {
        Vec::new()
    };

    if has_hsl_adjust || has_alpha_adjust {
        // Pass 1: HSL adjustment + alpha delta, plus optional sharpen source capture.
        for i in (0..buf.data.len()).step_by(4) {
            let a = buf.data[i + 3];
            if a == 0 {
                continue;
            }

            let mut nr = buf.data[i];
            let mut ng = buf.data[i + 1];
            let mut nb = buf.data[i + 2];

            if has_hsl_adjust {
                let hsl = rgb_to_hsl(nr, ng, nb);
                let nh = hsl.h + hue;
                let ns = (hsl.s + sat).clamp(0.0, 1.0);
                let nl = (hsl.l + light).clamp(0.0, 1.0);
                (nr, ng, nb) = hsl_to_rgb(nh, ns, nl);
                buf.data[i] = nr;
                buf.data[i + 1] = ng;
                buf.data[i + 2] = nb;
            }

            if has_alpha_adjust {
                let na = (a as f64 + (a_delta * 255.0).round()).clamp(0.0, 255.0) as u8;
                buf.data[i + 3] = na;
            }

            if has_sharpen {
                let pi = i / 4;
                let ri = pi * 3;
                rgb_base[ri] = nr;
                rgb_base[ri + 1] = ng;
                rgb_base[ri + 2] = nb;
            }
        }
    } else if has_sharpen {
        // Capture a stable RGB snapshot for the sharpen kernel without a second clone.
        for i in (0..buf.data.len()).step_by(4) {
            let pi = i / 4;
            let ri = pi * 3;
            rgb_base[ri] = buf.data[i];
            rgb_base[ri + 1] = buf.data[i + 1];
            rgb_base[ri + 2] = buf.data[i + 2];
        }
    } else if !has_tone_adjust {
        return;
    }

    if !has_tone_adjust && !has_sharpen {
        return;
    }

    let contrast_factor = if contrast >= 0.0 {
        1.0 + contrast * 3.0
    } else {
        1.0 / (1.0 + contrast.abs() * 3.0)
    };

    let w = buf.width as usize;
    let h = buf.height as usize;

    if !has_sharpen {
        for i in (0..buf.data.len()).step_by(4) {
            if buf.data[i + 3] == 0 {
                continue;
            }

            let mut r = buf.data[i] as f64;
            let mut g = buf.data[i + 1] as f64;
            let mut b = buf.data[i + 2] as f64;

            if brightness != 0.0 {
                let delta = brightness * 180.0;
                r += delta;
                g += delta;
                b += delta;
            }

            if contrast != 0.0 {
                r = (r - 128.0) * contrast_factor + 128.0;
                g = (g - 128.0) * contrast_factor + 128.0;
                b = (b - 128.0) * contrast_factor + 128.0;
            }

            if temperature != 0.0 {
                let t = temperature * 90.0;
                r += t;
                b -= t;
            }

            if tint != 0.0 {
                let t = tint * 80.0;
                g += t;
                r -= t * 0.35;
                b -= t * 0.35;
            }

            buf.data[i] = clamp_byte(r);
            buf.data[i + 1] = clamp_byte(g);
            buf.data[i + 2] = clamp_byte(b);
        }
        return;
    }

    // Pass 2: optional sharpen + tone adjustments.
    for y in 0..h {
        for x in 0..w {
            let pi = y * w + x;
            let di = pi * 4;
            if buf.data[di + 3] == 0 {
                continue;
            }
            let ri = pi * 3;

            let mut r = rgb_base[ri] as f64;
            let mut g = rgb_base[ri + 1] as f64;
            let mut b = rgb_base[ri + 2] as f64;

            let mut sum_r = 0.0;
            let mut sum_g = 0.0;
            let mut sum_b = 0.0;
            let mut weight = 0.0;
            for oy in -1i32..=1 {
                let ny = y as i32 + oy;
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                for ox in -1i32..=1 {
                    let nx = x as i32 + ox;
                    if nx < 0 || nx >= w as i32 {
                        continue;
                    }
                    let npi = ny as usize * w + nx as usize;
                    let ndi = npi * 4;
                    if buf.data[ndi + 3] == 0 {
                        continue;
                    }
                    let nri = npi * 3;
                    sum_r += rgb_base[nri] as f64;
                    sum_g += rgb_base[nri + 1] as f64;
                    sum_b += rgb_base[nri + 2] as f64;
                    weight += 1.0;
                }
            }
            if weight > 0.0 {
                let blur_r = sum_r / weight;
                let blur_g = sum_g / weight;
                let blur_b = sum_b / weight;
                let amount = sharpen * 1.2;
                r = (r + (r - blur_r) * amount).clamp(0.0, 255.0);
                g = (g + (g - blur_g) * amount).clamp(0.0, 255.0);
                b = (b + (b - blur_b) * amount).clamp(0.0, 255.0);
            }

            if brightness != 0.0 {
                let delta = brightness * 180.0;
                r += delta;
                g += delta;
                b += delta;
            }

            if contrast != 0.0 {
                r = (r - 128.0) * contrast_factor + 128.0;
                g = (g - 128.0) * contrast_factor + 128.0;
                b = (b - 128.0) * contrast_factor + 128.0;
            }

            if temperature != 0.0 {
                let t = temperature * 90.0;
                r += t;
                b -= t;
            }

            if tint != 0.0 {
                let t = tint * 80.0;
                g += t;
                r -= t * 0.35;
                b -= t * 0.35;
            }

            buf.data[di] = clamp_byte(r);
            buf.data[di + 1] = clamp_byte(g);
            buf.data[di + 2] = clamp_byte(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel-level filters (Phase 3)
// ---------------------------------------------------------------------------

/// Invert all RGB channels. Alpha is preserved.
pub fn invert(buf: &mut ImageBuffer) {
    for i in (0..buf.data.len()).step_by(4) {
        if buf.data[i + 3] == 0 {
            continue;
        }
        buf.data[i] = 255 - buf.data[i];
        buf.data[i + 1] = 255 - buf.data[i + 1];
        buf.data[i + 2] = 255 - buf.data[i + 2];
    }
}

/// Reduce color depth per channel. `levels` is the number of discrete
/// output levels per channel (2 = pure black/white per channel, 256 = no-op).
pub fn posterize(buf: &mut ImageBuffer, levels: u32) {
    if levels == 0 || levels >= 256 {
        return;
    }
    let levels_f = levels as f64;
    for i in (0..buf.data.len()).step_by(4) {
        if buf.data[i + 3] == 0 {
            continue;
        }
        for c in 0..3 {
            let v = buf.data[i + c] as f64 / 255.0;
            let q = (v * (levels_f - 1.0)).round() / (levels_f - 1.0);
            buf.data[i + c] = (q * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Convert to black/white based on luminance threshold (0–255).
pub fn threshold(buf: &mut ImageBuffer, thresh: u8) {
    for i in (0..buf.data.len()).step_by(4) {
        if buf.data[i + 3] == 0 {
            continue;
        }
        let lum = (0.299 * buf.data[i] as f64
            + 0.587 * buf.data[i + 1] as f64
            + 0.114 * buf.data[i + 2] as f64) as u8;
        let out = if lum >= thresh { 255 } else { 0 };
        buf.data[i] = out;
        buf.data[i + 1] = out;
        buf.data[i + 2] = out;
    }
}

/// Levels adjustment: remap input range [in_black, in_white] to output range
/// [out_black, out_white] with a gamma curve (midtones). Gamma 1.0 = linear.
pub fn levels(
    buf: &mut ImageBuffer,
    in_black: u8,
    in_white: u8,
    gamma: f64,
    out_black: u8,
    out_white: u8,
) {
    let in_range = (in_white as f64 - in_black as f64).max(1.0);
    let out_range = out_white as f64 - out_black as f64;
    let gamma_inv = if gamma > 0.0 { 1.0 / gamma } else { 1.0 };

    // Build lookup table for speed
    let mut lut = [0u8; 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        let clamped = (i as f64 - in_black as f64).clamp(0.0, in_range) / in_range;
        let curved = clamped.powf(gamma_inv);
        *entry = (out_black as f64 + curved * out_range).clamp(0.0, 255.0) as u8;
    }

    for i in (0..buf.data.len()).step_by(4) {
        if buf.data[i + 3] == 0 {
            continue;
        }
        buf.data[i] = lut[buf.data[i] as usize];
        buf.data[i + 1] = lut[buf.data[i + 1] as usize];
        buf.data[i + 2] = lut[buf.data[i + 2] as usize];
    }
}

/// Map pixel luminance to colors sampled from a gradient.
///
/// `stops` is a sorted list of `(position, color)` pairs where `position` is
/// in `[0.0, 1.0]`.  Each non-transparent pixel's luminance (BT.601) selects a
/// color by interpolating between the two surrounding stops.  Alpha is preserved.
///
/// Does nothing if fewer than 2 stops are provided.
pub fn gradient_map(buf: &mut ImageBuffer, stops: &[(f64, crate::pixel::Rgba)]) {
    if stops.len() < 2 {
        return;
    }

    // Build 256-entry LUT from gradient stops
    let mut lut = [(0u8, 0u8, 0u8); 256];
    for (i, entry) in lut.iter_mut().enumerate() {
        let t = i as f64 / 255.0;
        // Find the two surrounding stops
        let mut lo = 0;
        let mut hi = stops.len() - 1;
        for (j, stop) in stops.iter().enumerate() {
            if stop.0 <= t {
                lo = j;
            }
            if stop.0 >= t && j < hi {
                hi = j;
                break;
            }
        }
        if lo == hi {
            *entry = (stops[lo].1.r, stops[lo].1.g, stops[lo].1.b);
        } else {
            let range = stops[hi].0 - stops[lo].0;
            let frac = if range > 0.0 {
                (t - stops[lo].0) / range
            } else {
                0.0
            };
            let a = &stops[lo].1;
            let b = &stops[hi].1;
            *entry = (
                (a.r as f64 + (b.r as f64 - a.r as f64) * frac).clamp(0.0, 255.0) as u8,
                (a.g as f64 + (b.g as f64 - a.g as f64) * frac).clamp(0.0, 255.0) as u8,
                (a.b as f64 + (b.b as f64 - a.b as f64) * frac).clamp(0.0, 255.0) as u8,
            );
        }
    }

    for i in (0..buf.data.len()).step_by(4) {
        if buf.data[i + 3] == 0 {
            continue;
        }
        let lum = (0.299 * buf.data[i] as f64
            + 0.587 * buf.data[i + 1] as f64
            + 0.114 * buf.data[i + 2] as f64) as u8;
        let (r, g, b) = lut[lum as usize];
        buf.data[i] = r;
        buf.data[i + 1] = g;
        buf.data[i + 2] = b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba;

    #[test]
    fn identity_filter_is_noop() {
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.set_pixel(0, 0, Rgba::new(100, 150, 200, 255));
        buf.set_pixel(1, 0, Rgba::new(50, 100, 150, 128));
        let original = buf.clone();
        apply_hsl_filter(&mut buf, &HslFilterConfig::default());
        // HSL roundtrip may introduce ±1 rounding error per channel
        for (a, b) in buf.data.iter().zip(original.data.iter()) {
            assert!(
                (*a as i16 - *b as i16).abs() <= 1,
                "got {} expected {}",
                a,
                b
            );
        }
    }

    #[test]
    fn brightness_increases_channels() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(100, 100, 100, 255));
        apply_hsl_filter(
            &mut buf,
            &HslFilterConfig {
                brightness: 0.5,
                ..Default::default()
            },
        );
        let p = buf.get_pixel(0, 0);
        // brightness 0.5 → delta = 90, so 100 + 90 = 190
        assert!(p.r > 180 && p.r < 200, "r={}", p.r);
    }

    #[test]
    fn hue_shift_rotates_color() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255)); // pure red
        apply_hsl_filter(
            &mut buf,
            &HslFilterConfig {
                hue_deg: 120.0, // shift to green
                ..Default::default()
            },
        );
        let p = buf.get_pixel(0, 0);
        // Red shifted 120° → green-ish
        assert!(p.g > p.r, "expected green > red, got r={} g={}", p.r, p.g);
    }

    #[test]
    fn transparent_pixels_are_skipped() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        apply_hsl_filter(
            &mut buf,
            &HslFilterConfig {
                brightness: 1.0,
                hue_deg: 180.0,
                ..Default::default()
            },
        );
        assert_eq!(buf.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn alpha_delta_adjusts_opacity() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(100, 100, 100, 200));
        apply_hsl_filter(
            &mut buf,
            &HslFilterConfig {
                alpha: -0.5,
                ..Default::default()
            },
        );
        let p = buf.get_pixel(0, 0);
        // 200 + round(-0.5 * 255) = 200 - 128 = 72
        assert!(p.a < 80 && p.a > 60, "a={}", p.a);
    }

    #[test]
    fn invert_flips_channels() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(100, 150, 200, 255));
        invert(&mut buf);
        let p = buf.get_pixel(0, 0);
        assert_eq!(p, Rgba::new(155, 105, 55, 255));
    }

    #[test]
    fn invert_skips_transparent() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        invert(&mut buf);
        assert_eq!(buf.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn posterize_reduces_levels() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(100, 200, 50, 255));
        posterize(&mut buf, 2);
        let p = buf.get_pixel(0, 0);
        // 2 levels → each channel either 0 or 255
        assert!(p.r == 0 || p.r == 255);
        assert!(p.g == 0 || p.g == 255);
        assert!(p.b == 0 || p.b == 255);
    }

    #[test]
    fn threshold_produces_bw() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(200, 200, 200, 255));
        threshold(&mut buf, 128);
        let p = buf.get_pixel(0, 0);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 255);
        assert_eq!(p.b, 255);
    }

    #[test]
    fn threshold_dark_to_black() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(30, 30, 30, 255));
        threshold(&mut buf, 128);
        let p = buf.get_pixel(0, 0);
        assert_eq!(p.r, 0);
    }

    #[test]
    fn levels_remap() {
        let mut buf = ImageBuffer::new_transparent(1, 1);
        buf.set_pixel(0, 0, Rgba::new(128, 128, 128, 255));
        levels(&mut buf, 0, 255, 1.0, 0, 128);
        let p = buf.get_pixel(0, 0);
        // Linear remap: 128/255 * 128 ≈ 64
        assert!(p.r > 60 && p.r < 68, "r={}", p.r);
    }

    #[test]
    fn gradient_map_bw_to_color() {
        let mut buf = ImageBuffer::new_transparent(2, 1);
        buf.set_pixel(0, 0, Rgba::new(0, 0, 0, 255));
        buf.set_pixel(1, 0, Rgba::new(255, 255, 255, 255));
        let stops = vec![
            (0.0, Rgba::new(255, 0, 0, 255)),
            (1.0, Rgba::new(0, 0, 255, 255)),
        ];
        gradient_map(&mut buf, &stops);
        let dark = buf.get_pixel(0, 0);
        let light = buf.get_pixel(1, 0);
        assert_eq!(dark.r, 255); // black → red
        assert_eq!(dark.b, 0);
        assert_eq!(light.b, 255); // white → blue
        assert_eq!(light.r, 0);
    }
}
