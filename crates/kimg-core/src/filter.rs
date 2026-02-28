use crate::buffer::ImageBuffer;
use crate::color::{hsl_to_rgb, rgb_to_hsl};

/// Configuration for the HSL/tone filter pipeline.
///
/// Ported from Spriteform compositorRenderMath.ts `applyHslFilter`.
#[derive(Debug, Clone)]
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
    (n as i32).max(0).min(255) as u8
}

/// Apply the full HSL + tone filter pipeline in-place on `buf`.
pub fn apply_hsl_filter(buf: &mut ImageBuffer, cfg: &HslFilterConfig) {
    let hue = cfg.hue_deg;
    let sat = cfg.saturation;
    let light = cfg.lightness;
    let a_delta = cfg.alpha;
    let brightness = cfg.brightness.max(-1.0).min(1.0);
    let contrast = cfg.contrast.max(-1.0).min(1.0);
    let temperature = cfg.temperature.max(-1.0).min(1.0);
    let tint = cfg.tint.max(-1.0).min(1.0);
    let sharpen = cfg.sharpen.max(0.0).min(1.0);

    let has_post_adjust =
        brightness != 0.0 || contrast != 0.0 || temperature != 0.0 || tint != 0.0 || sharpen > 0.0;

    let pixel_count = (buf.width as usize) * (buf.height as usize);
    let mut rgb_base: Vec<u8> = if has_post_adjust {
        vec![0; pixel_count * 3]
    } else {
        Vec::new()
    };
    let mut alpha_mask: Vec<u8> = if has_post_adjust {
        vec![0; pixel_count]
    } else {
        Vec::new()
    };

    // Pass 1: HSL adjustment + alpha delta
    for i in (0..buf.data.len()).step_by(4) {
        let a = buf.data[i + 3];
        if a == 0 {
            continue;
        }

        let r = buf.data[i];
        let g = buf.data[i + 1];
        let b = buf.data[i + 2];

        let hsl = rgb_to_hsl(r, g, b);
        let nh = hsl.h + hue;
        let ns = (hsl.s + sat).max(0.0).min(1.0);
        let nl = (hsl.l + light).max(0.0).min(1.0);
        let (nr, ng, nb) = hsl_to_rgb(nh, ns, nl);

        buf.data[i] = nr;
        buf.data[i + 1] = ng;
        buf.data[i + 2] = nb;

        let na = (a as f64 + (a_delta * 255.0).round()).max(0.0).min(255.0) as u8;
        buf.data[i + 3] = na;

        if has_post_adjust {
            let pi = i / 4;
            let ri = pi * 3;
            rgb_base[ri] = nr;
            rgb_base[ri + 1] = ng;
            rgb_base[ri + 2] = nb;
            alpha_mask[pi] = na;
        }
    }

    if !has_post_adjust {
        return;
    }

    // Pass 2: brightness, contrast, temperature, tint, sharpen
    let contrast_factor = if contrast >= 0.0 {
        1.0 + contrast * 3.0
    } else {
        1.0 / (1.0 + contrast.abs() * 3.0)
    };

    let sharpen_src = if sharpen > 0.0 {
        rgb_base.clone()
    } else {
        Vec::new()
    };

    let w = buf.width as usize;
    let h = buf.height as usize;

    for y in 0..h {
        for x in 0..w {
            let pi = y * w + x;
            if alpha_mask[pi] == 0 {
                continue;
            }
            let di = pi * 4;
            let ri = pi * 3;

            let mut r = rgb_base[ri] as f64;
            let mut g = rgb_base[ri + 1] as f64;
            let mut b = rgb_base[ri + 2] as f64;

            // Sharpen (unsharp mask via 3x3 box blur)
            if sharpen > 0.0 {
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
                        if alpha_mask[npi] == 0 {
                            continue;
                        }
                        let nri = npi * 3;
                        sum_r += sharpen_src[nri] as f64;
                        sum_g += sharpen_src[nri + 1] as f64;
                        sum_b += sharpen_src[nri + 2] as f64;
                        weight += 1.0;
                    }
                }
                if weight > 0.0 {
                    let blur_r = sum_r / weight;
                    let blur_g = sum_g / weight;
                    let blur_b = sum_b / weight;
                    let amount = sharpen * 1.2;
                    r = (r + (r - blur_r) * amount).max(0.0).min(255.0);
                    g = (g + (g - blur_g) * amount).max(0.0).min(255.0);
                    b = (b + (b - blur_b) * amount).max(0.0).min(255.0);
                }
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
            assert!((*a as i16 - *b as i16).abs() <= 1, "got {} expected {}", a, b);
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
}
