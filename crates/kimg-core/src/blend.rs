use crate::buffer::ImageBuffer;
use crate::color::{hsl_to_rgb, rgb_to_hsl};

/// Photoshop-compatible blend modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    /// Parse from a string name (case-insensitive). Returns Normal on unknown.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().replace(['-', '_', ' '], "").as_str() {
            "normal" => Self::Normal,
            "multiply" => Self::Multiply,
            "screen" => Self::Screen,
            "overlay" => Self::Overlay,
            "darken" => Self::Darken,
            "lighten" => Self::Lighten,
            "colordodge" => Self::ColorDodge,
            "colorburn" => Self::ColorBurn,
            "hardlight" => Self::HardLight,
            "softlight" => Self::SoftLight,
            "difference" => Self::Difference,
            "exclusion" => Self::Exclusion,
            "hue" => Self::Hue,
            "saturation" => Self::Saturation,
            "color" => Self::Color,
            "luminosity" => Self::Luminosity,
            _ => Self::Normal,
        }
    }

    /// Return the mode name as a lowercase string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::Darken => "darken",
            Self::Lighten => "lighten",
            Self::ColorDodge => "color-dodge",
            Self::ColorBurn => "color-burn",
            Self::HardLight => "hard-light",
            Self::SoftLight => "soft-light",
            Self::Difference => "difference",
            Self::Exclusion => "exclusion",
            Self::Hue => "hue",
            Self::Saturation => "saturation",
            Self::Color => "color",
            Self::Luminosity => "luminosity",
        }
    }
}

// ── Per-channel blend math (all inputs/outputs in 0.0..1.0) ──

fn blend_channel_multiply(b: f64, s: f64) -> f64 {
    b * s
}
fn blend_channel_screen(b: f64, s: f64) -> f64 {
    b + s - b * s
}
fn blend_channel_overlay(b: f64, s: f64) -> f64 {
    if b <= 0.5 {
        2.0 * b * s
    } else {
        1.0 - 2.0 * (1.0 - b) * (1.0 - s)
    }
}
fn blend_channel_darken(b: f64, s: f64) -> f64 {
    b.min(s)
}
fn blend_channel_lighten(b: f64, s: f64) -> f64 {
    b.max(s)
}
fn blend_channel_color_dodge(b: f64, s: f64) -> f64 {
    if b <= 0.0 {
        0.0
    } else if s >= 1.0 {
        1.0
    } else {
        (b / (1.0 - s)).min(1.0)
    }
}
fn blend_channel_color_burn(b: f64, s: f64) -> f64 {
    if b >= 1.0 {
        1.0
    } else if s <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - b) / s).min(1.0)
    }
}
fn blend_channel_hard_light(b: f64, s: f64) -> f64 {
    if s <= 0.5 {
        2.0 * b * s
    } else {
        1.0 - 2.0 * (1.0 - b) * (1.0 - s)
    }
}
fn blend_channel_soft_light(b: f64, s: f64) -> f64 {
    // W3C compositing spec formula
    if s <= 0.5 {
        b - (1.0 - 2.0 * s) * b * (1.0 - b)
    } else {
        let d = if b <= 0.25 {
            ((16.0 * b - 12.0) * b + 4.0) * b
        } else {
            b.sqrt()
        };
        b + (2.0 * s - 1.0) * (d - b)
    }
}
fn blend_channel_difference(b: f64, s: f64) -> f64 {
    (b - s).abs()
}
fn blend_channel_exclusion(b: f64, s: f64) -> f64 {
    b + s - 2.0 * b * s
}

/// Apply per-channel blend for simple (non-HSL) modes.
fn blend_channels(
    mode: BlendMode,
    br: f64,
    bg: f64,
    bb: f64,
    sr: f64,
    sg: f64,
    sb: f64,
) -> (f64, f64, f64) {
    match mode {
        BlendMode::Normal => (sr, sg, sb),
        BlendMode::Multiply => (
            blend_channel_multiply(br, sr),
            blend_channel_multiply(bg, sg),
            blend_channel_multiply(bb, sb),
        ),
        BlendMode::Screen => (
            blend_channel_screen(br, sr),
            blend_channel_screen(bg, sg),
            blend_channel_screen(bb, sb),
        ),
        BlendMode::Overlay => (
            blend_channel_overlay(br, sr),
            blend_channel_overlay(bg, sg),
            blend_channel_overlay(bb, sb),
        ),
        BlendMode::Darken => (
            blend_channel_darken(br, sr),
            blend_channel_darken(bg, sg),
            blend_channel_darken(bb, sb),
        ),
        BlendMode::Lighten => (
            blend_channel_lighten(br, sr),
            blend_channel_lighten(bg, sg),
            blend_channel_lighten(bb, sb),
        ),
        BlendMode::ColorDodge => (
            blend_channel_color_dodge(br, sr),
            blend_channel_color_dodge(bg, sg),
            blend_channel_color_dodge(bb, sb),
        ),
        BlendMode::ColorBurn => (
            blend_channel_color_burn(br, sr),
            blend_channel_color_burn(bg, sg),
            blend_channel_color_burn(bb, sb),
        ),
        BlendMode::HardLight => (
            blend_channel_hard_light(br, sr),
            blend_channel_hard_light(bg, sg),
            blend_channel_hard_light(bb, sb),
        ),
        BlendMode::SoftLight => (
            blend_channel_soft_light(br, sr),
            blend_channel_soft_light(bg, sg),
            blend_channel_soft_light(bb, sb),
        ),
        BlendMode::Difference => (
            blend_channel_difference(br, sr),
            blend_channel_difference(bg, sg),
            blend_channel_difference(bb, sb),
        ),
        BlendMode::Exclusion => (
            blend_channel_exclusion(br, sr),
            blend_channel_exclusion(bg, sg),
            blend_channel_exclusion(bb, sb),
        ),
        // HSL modes handled separately
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => {
            blend_hsl_mode(mode, br, bg, bb, sr, sg, sb)
        }
    }
}

/// HSL-based blend modes: Hue, Saturation, Color, Luminosity.
fn blend_hsl_mode(
    mode: BlendMode,
    br: f64,
    bg: f64,
    bb: f64,
    sr: f64,
    sg: f64,
    sb: f64,
) -> (f64, f64, f64) {
    let base_hsl = rgb_to_hsl((br * 255.0) as u8, (bg * 255.0) as u8, (bb * 255.0) as u8);
    let src_hsl = rgb_to_hsl((sr * 255.0) as u8, (sg * 255.0) as u8, (sb * 255.0) as u8);

    let (h, s, l) = match mode {
        BlendMode::Hue => (src_hsl.h, base_hsl.s, base_hsl.l),
        BlendMode::Saturation => (base_hsl.h, src_hsl.s, base_hsl.l),
        BlendMode::Color => (src_hsl.h, src_hsl.s, base_hsl.l),
        BlendMode::Luminosity => (base_hsl.h, base_hsl.s, src_hsl.l),
        _ => unreachable!(),
    };

    let (r, g, b) = hsl_to_rgb(h, s, l);
    (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

fn clamp_byte(n: f64) -> u8 {
    (n + 0.5) as u8 // n is already in 0..255 range after clamping
}

/// Alpha-composite `src` onto `dst` using the given blend mode.
/// Both buffers must have the same dimensions.
pub fn blend(dst: &mut ImageBuffer, src: &ImageBuffer, mode: BlendMode) {
    let w = dst.width.min(src.width) as usize;
    let h = dst.height.min(src.height) as usize;
    let dst_stride = dst.width as usize;
    let src_stride = src.width as usize;

    for y in 0..h {
        for x in 0..w {
            let si = (y * src_stride + x) * 4;
            let sa_byte = src.data[si + 3];
            if sa_byte == 0 {
                continue;
            }
            let sa = sa_byte as f64 / 255.0;

            let di = (y * dst_stride + x) * 4;
            let da = dst.data[di + 3] as f64 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 {
                continue;
            }

            let sr = src.data[si] as f64 / 255.0;
            let sg = src.data[si + 1] as f64 / 255.0;
            let sb = src.data[si + 2] as f64 / 255.0;
            let dr = dst.data[di] as f64 / 255.0;
            let dg = dst.data[di + 1] as f64 / 255.0;
            let db = dst.data[di + 2] as f64 / 255.0;

            // Compute blended color (in premultiply-aware composite)
            let (cr, cg, cb) = blend_channels(mode, dr, dg, db, sr, sg, sb);

            // Porter-Duff source-over with blended color
            let inv_sa = 1.0 - sa;
            let or = (cr * sa + dr * da * inv_sa) / out_a;
            let og = (cg * sa + dg * da * inv_sa) / out_a;
            let ob = (cb * sa + db * da * inv_sa) / out_a;

            dst.data[di] = clamp_byte(or.clamp(0.0, 1.0) * 255.0);
            dst.data[di + 1] = clamp_byte(og.clamp(0.0, 1.0) * 255.0);
            dst.data[di + 2] = clamp_byte(ob.clamp(0.0, 1.0) * 255.0);
            dst.data[di + 3] = clamp_byte(out_a.clamp(0.0, 1.0) * 255.0);
        }
    }
}

/// Convenience: alpha-composite using Normal blend mode.
pub fn blend_normal(dst: &mut ImageBuffer, src: &ImageBuffer) {
    blend(dst, src, BlendMode::Normal);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba;

    #[test]
    fn blend_opaque_over_transparent() {
        let mut dst = ImageBuffer::new_transparent(2, 2);
        let mut src = ImageBuffer::new_transparent(2, 2);
        src.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        blend_normal(&mut dst, &src);
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(dst.get_pixel(1, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn blend_transparent_over_opaque_is_noop() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(0, 255, 0, 255));
        let src = ImageBuffer::new_transparent(1, 1);
        blend_normal(&mut dst, &src);
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
    }

    #[test]
    fn blend_semi_transparent() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(0, 0, 255, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(255, 0, 0, 128));
        blend_normal(&mut dst, &src);
        let p = dst.get_pixel(0, 0);
        assert!(p.r > 120 && p.r < 136, "r={}", p.r);
        assert!(p.b > 120 && p.b < 136, "b={}", p.b);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn blend_mode_multiply() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(200, 100, 50, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(128, 128, 128, 255));
        blend(&mut dst, &src, BlendMode::Multiply);
        let p = dst.get_pixel(0, 0);
        // multiply: 200/255 * 128/255 * 255 ≈ 100
        assert!(p.r > 95 && p.r < 105, "r={}", p.r);
    }

    #[test]
    fn blend_mode_screen() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(100, 100, 100, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(100, 100, 100, 255));
        blend(&mut dst, &src, BlendMode::Screen);
        let p = dst.get_pixel(0, 0);
        // screen: 100/255 + 100/255 - (100/255)^2 ≈ 0.631 * 255 ≈ 161
        assert!(p.r > 155 && p.r < 170, "r={}", p.r);
    }

    #[test]
    fn blend_mode_overlay_dark_base() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(50, 50, 50, 255)); // dark base
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(200, 200, 200, 255));
        blend(&mut dst, &src, BlendMode::Overlay);
        let p = dst.get_pixel(0, 0);
        // overlay with dark base (<=0.5): 2*b*s
        // 2 * (50/255) * (200/255) * 255 ≈ 78
        assert!(p.r > 73 && p.r < 83, "r={}", p.r);
    }

    #[test]
    fn blend_mode_difference() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(200, 100, 50, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(100, 200, 150, 255));
        blend(&mut dst, &src, BlendMode::Difference);
        let p = dst.get_pixel(0, 0);
        // |200-100|=100, |100-200|=100, |50-150|=100
        assert!(p.r > 95 && p.r < 105, "r={}", p.r);
        assert!(p.g > 95 && p.g < 105, "g={}", p.g);
        assert!(p.b > 95 && p.b < 105, "b={}", p.b);
    }

    #[test]
    fn blend_mode_from_str() {
        assert_eq!(BlendMode::from_str_lossy("multiply"), BlendMode::Multiply);
        assert_eq!(
            BlendMode::from_str_lossy("color-dodge"),
            BlendMode::ColorDodge
        );
        assert_eq!(
            BlendMode::from_str_lossy("Color Dodge"),
            BlendMode::ColorDodge
        );
        assert_eq!(
            BlendMode::from_str_lossy("HARD_LIGHT"),
            BlendMode::HardLight
        );
        assert_eq!(BlendMode::from_str_lossy("unknown"), BlendMode::Normal);
    }

    #[test]
    fn blend_mode_roundtrip_str() {
        let modes = [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::ColorDodge,
            BlendMode::ColorBurn,
            BlendMode::HardLight,
            BlendMode::SoftLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Hue,
            BlendMode::Saturation,
            BlendMode::Color,
            BlendMode::Luminosity,
        ];
        for mode in modes {
            assert_eq!(BlendMode::from_str_lossy(mode.as_str()), mode);
        }
    }

    #[test]
    fn blend_mode_darken_lighten() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(200, 50, 100, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(100, 200, 100, 255));

        let mut d1 = dst.clone();
        blend(&mut d1, &src, BlendMode::Darken);
        let p = d1.get_pixel(0, 0);
        assert_eq!(p.r, 100);
        assert_eq!(p.g, 50);
        assert_eq!(p.b, 100);

        let mut d2 = dst.clone();
        blend(&mut d2, &src, BlendMode::Lighten);
        let p = d2.get_pixel(0, 0);
        assert_eq!(p.r, 200);
        assert_eq!(p.g, 200);
        assert_eq!(p.b, 100);
    }

    #[test]
    fn blend_mode_exclusion() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(255, 0, 128, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(255, 255, 128, 255));
        blend(&mut dst, &src, BlendMode::Exclusion);
        let p = dst.get_pixel(0, 0);
        // exclusion(1.0, 1.0) = 1+1-2*1*1 = 0
        assert!(p.r < 5, "r={}", p.r);
        // exclusion(0, 1) = 0+1-0 = 1 → 255
        assert!(p.g > 250, "g={}", p.g);
    }
}
