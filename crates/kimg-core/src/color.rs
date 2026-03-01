/// Simple RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel (0-255).
    pub r: u8,
    /// Green channel (0-255).
    pub g: u8,
    /// Blue channel (0-255).
    pub b: u8,
}

/// HSL color with h in degrees [0, 360), s and l in [0, 1].
#[derive(Debug, Clone, Copy)]
pub struct Hsl {
    /// Hue in degrees [0, 360).
    pub h: f64,
    /// Saturation [0.0, 1.0].
    pub s: f64,
    /// Lightness [0.0, 1.0].
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
    (n as i32).clamp(0, 255) as u8
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

/// Parse `#rrggbb` or `#rgb` hex string to Rgb.
pub fn hex_to_rgb(hex: &str) -> Option<Rgb> {
    let s = hex.trim().trim_start_matches('#');
    match s.len() {
        6 => {
            let n = u32::from_str_radix(s, 16).ok()?;
            Some(Rgb {
                r: ((n >> 16) & 0xff) as u8,
                g: ((n >> 8) & 0xff) as u8,
                b: (n & 0xff) as u8,
            })
        }
        3 => {
            let n = u16::from_str_radix(s, 16).ok()?;
            let r = ((n >> 8) & 0xf) as u8;
            let g = ((n >> 4) & 0xf) as u8;
            let b = (n & 0xf) as u8;
            Some(Rgb {
                r: r << 4 | r,
                g: g << 4 | g,
                b: b << 4 | b,
            })
        }
        _ => None,
    }
}

/// Format Rgb as `#rrggbb`.
pub fn rgb_to_hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

/// Convert a single sRGB channel (0-255) to linear light (IEC 61966-2-1).
pub fn srgb_to_linear(u8_val: u8) -> f64 {
    let c = u8_val as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG 2.x relative luminance from a hex color string.
pub fn relative_luminance(hex: &str) -> Option<f64> {
    let rgb = hex_to_rgb(hex)?;
    let r = srgb_to_linear(rgb.r);
    let g = srgb_to_linear(rgb.g);
    let b = srgb_to_linear(rgb.b);
    Some((0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0))
}

/// WCAG 2.x contrast ratio between two hex colors.
pub fn contrast_ratio(a_hex: &str, b_hex: &str) -> Option<f64> {
    let la = relative_luminance(a_hex)?;
    let lb = relative_luminance(b_hex)?;
    let l1 = la.max(lb);
    let l2 = la.min(lb);
    Some((l1 + 0.05) / (l2 + 0.05))
}

/// Find the dominant color in an RGBA pixel buffer using a 5-bit histogram.
pub fn dominant_rgb_from_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    max_samples: u32,
) -> Option<Rgb> {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || data.len() < w * h * 4 {
        return None;
    }

    let step = ((w * h) as f64 / max_samples.max(1) as f64)
        .sqrt()
        .floor()
        .max(1.0) as usize;
    let mut bins = vec![0u32; 32 * 32 * 32];

    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let i = (w * y + x) * 4;
            if data[i + 3] > 0 {
                let idx = ((data[i] as usize >> 3) << 10)
                    | ((data[i + 1] as usize >> 3) << 5)
                    | (data[i + 2] as usize >> 3);
                bins[idx] += 1;
            }
            x += step;
        }
        y += step;
    }

    let mut best = 0usize;
    let mut best_count = 0u32;
    for (i, &c) in bins.iter().enumerate() {
        if c > best_count {
            best_count = c;
            best = i;
        }
    }

    if best_count == 0 {
        return None;
    }

    let ri = ((best >> 10) & 31) as u8;
    let gi = ((best >> 5) & 31) as u8;
    let bi = (best & 31) as u8;
    Some(Rgb {
        r: ri * 8 + 4,
        g: gi * 8 + 4,
        b: bi * 8 + 4,
    })
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

    #[test]
    fn hex_to_rgb_6digit() {
        assert_eq!(
            hex_to_rgb("#ff8000"),
            Some(Rgb {
                r: 255,
                g: 128,
                b: 0
            })
        );
        assert_eq!(
            hex_to_rgb("FF8000"),
            Some(Rgb {
                r: 255,
                g: 128,
                b: 0
            })
        );
    }

    #[test]
    fn hex_to_rgb_3digit() {
        assert_eq!(
            hex_to_rgb("#f80"),
            Some(Rgb {
                r: 255,
                g: 136,
                b: 0
            })
        );
    }

    #[test]
    fn hex_to_rgb_invalid() {
        assert_eq!(hex_to_rgb(""), None);
        assert_eq!(hex_to_rgb("#gg0000"), None);
        assert_eq!(hex_to_rgb("#12345"), None);
    }

    #[test]
    fn rgb_to_hex_roundtrip() {
        let hex = rgb_to_hex(Rgb {
            r: 255,
            g: 128,
            b: 0,
        });
        assert_eq!(hex, "#ff8000");
        assert_eq!(
            hex_to_rgb(&hex),
            Some(Rgb {
                r: 255,
                g: 128,
                b: 0
            })
        );
    }

    #[test]
    fn srgb_to_linear_bounds() {
        assert!((srgb_to_linear(0) - 0.0).abs() < 1e-10);
        assert!((srgb_to_linear(255) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn relative_luminance_black_white() {
        let black = relative_luminance("#000000").unwrap();
        let white = relative_luminance("#ffffff").unwrap();
        assert!(black < 0.001);
        assert!((white - 1.0).abs() < 0.001);
    }

    #[test]
    fn contrast_ratio_black_white() {
        let cr = contrast_ratio("#000000", "#ffffff").unwrap();
        assert!((cr - 21.0).abs() < 0.1);
    }

    #[test]
    fn dominant_rgb_solid_red() {
        let mut data = vec![0u8; 4 * 4 * 4];
        for chunk in data.chunks_exact_mut(4) {
            chunk[0] = 255;
            chunk[1] = 0;
            chunk[2] = 0;
            chunk[3] = 255;
        }
        let rgb = dominant_rgb_from_rgba(&data, 4, 4, 4096).unwrap();
        // 255 >> 3 = 31, 31*8+4 = 252
        assert_eq!(rgb.r, 252);
        assert_eq!(rgb.g, 4);
        assert_eq!(rgb.b, 4);
    }

    #[test]
    fn dominant_rgb_all_transparent() {
        let data = vec![0u8; 4 * 4 * 4];
        assert_eq!(dominant_rgb_from_rgba(&data, 4, 4, 4096), None);
    }

    #[test]
    fn readable_text_color_logic() {
        // Threshold used by JS readableTextColor: luminance > 0.179 → black text
        let threshold = 0.179;

        let white_lum = relative_luminance("#ffffff").unwrap();
        assert!(white_lum > threshold, "white bg should get black text");

        let black_lum = relative_luminance("#000000").unwrap();
        assert!(black_lum < threshold, "black bg should get white text");

        // Mid-gray (#808080): luminance ≈ 0.2159, should get black text
        let gray_lum = relative_luminance("#808080").unwrap();
        assert!(
            gray_lum > threshold,
            "mid-gray lum={gray_lum} should be above threshold"
        );

        // Dark blue (#1e3a5f): luminance should be below threshold → white text
        let dark_lum = relative_luminance("#1e3a5f").unwrap();
        assert!(
            dark_lum < threshold,
            "dark blue lum={dark_lum} should get white text"
        );
    }
}
