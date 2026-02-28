/// HSL color with h in degrees [0, 360), s and l in [0, 1].
#[derive(Debug, Clone, Copy)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

/// Convert RGB (0-255) to HSL.
/// Ported from Spriteform compositorRenderMath.ts `rgbToHsl`.
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> Hsl {
    let rr = r as f64 / 255.0;
    let gg = g as f64 / 255.0;
    let bb = b as f64 / 255.0;
    let max = rr.max(gg).max(bb);
    let min = rr.min(gg).min(bb);
    let d = max - min;
    let l = (max + min) / 2.0;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    let mut h = if d == 0.0 {
        0.0
    } else if max == rr {
        ((gg - bb) / d) % 6.0
    } else if max == gg {
        (bb - rr) / d + 2.0
    } else {
        (rr - gg) / d + 4.0
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    Hsl { h, s, l }
}

fn clamp_byte(n: f64) -> u8 {
    (n as i32).max(0).min(255) as u8
}

/// Convert HSL to RGB (0-255).
/// Ported from Spriteform compositorRenderMath.ts `hslToRgb`.
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hh = (((h % 360.0) + 360.0) % 360.0) / 60.0;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if hh < 1.0 {
        (c, x, 0.0)
    } else if hh < 2.0 {
        (x, c, 0.0)
    } else if hh < 3.0 {
        (0.0, c, x)
    } else if hh < 4.0 {
        (0.0, x, c)
    } else if hh < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    (
        clamp_byte((r1 + m) * 255.0),
        clamp_byte((g1 + m) * 255.0),
        clamp_byte((b1 + m) * 255.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_hsl_pure_red() {
        let hsl = rgb_to_hsl(255, 0, 0);
        assert!((hsl.h - 0.0).abs() < 1.0);
        assert!((hsl.s - 1.0).abs() < 0.01);
        assert!((hsl.l - 0.5).abs() < 0.01);
    }

    #[test]
    fn rgb_to_hsl_white() {
        let hsl = rgb_to_hsl(255, 255, 255);
        assert!((hsl.s - 0.0).abs() < 0.01);
        assert!((hsl.l - 1.0).abs() < 0.01);
    }

    #[test]
    fn hsl_to_rgb_roundtrip() {
        let original = (180u8, 100, 50);
        let hsl = rgb_to_hsl(original.0, original.1, original.2);
        let (r, g, b) = hsl_to_rgb(hsl.h, hsl.s, hsl.l);
        assert!((r as i16 - original.0 as i16).abs() <= 1);
        assert!((g as i16 - original.1 as i16).abs() <= 1);
        assert!((b as i16 - original.2 as i16).abs() <= 1);
    }

    #[test]
    fn hsl_to_rgb_black() {
        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 0.0);
        assert_eq!((r, g, b), (0, 0, 0));
    }
}
