//! Transformed blitting — composite one image onto another with position,
//! anchor, flip, rotation, and opacity.
//!
//! The main function is [`blit_transformed`], which accepts a [`BlitParams`]
//! struct describing the transformation.  Rotation is restricted to 90-degree
//! increments ([`Rotation`]); arbitrary-angle rotation is in
//! [`crate::transform`].

use crate::buffer::ImageBuffer;

/// Anchor point for positioning a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Anchor {
    /// Anchor the layer by its top-left corner.
    TopLeft,
    /// Anchor the layer by its center.
    Center,
}

/// Rotation in 90-degree increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rotation {
    /// No rotation.
    None,
    /// 90 degrees clockwise rotation.
    Cw90,
    /// 180 degrees clockwise rotation.
    Cw180,
    /// 270 degrees clockwise rotation.
    Cw270,
}

impl Rotation {
    /// Snap an arbitrary degree value to the nearest 90-degree rotation.
    pub fn from_degrees(deg: f64) -> Self {
        let snapped = ((((deg / 90.0).round() as i32 * 90) % 360) + 360) % 360;
        match snapped {
            90 => Rotation::Cw90,
            180 => Rotation::Cw180,
            270 => Rotation::Cw270,
            _ => Rotation::None,
        }
    }
}

/// Parameters for a transformed blit operation.
#[non_exhaustive]
pub struct BlitParams {
    /// X offset according to the anchor.
    pub dx: i32,
    /// Y offset according to the anchor.
    pub dy: i32,
    /// The anchor point used for dx/dy positioning.
    pub anchor: Anchor,
    /// Whether to flip horizontally.
    pub flip_x: bool,
    /// Whether to flip vertically.
    pub flip_y: bool,
    /// Rotation in 90-degree increments.
    pub rotation: Rotation,
    /// Global opacity of the blit operation, 0.0 to 1.0.
    pub opacity: f64,
}

fn clamp_byte(n: f64) -> u8 {
    (n as i32).clamp(0, 255) as u8
}

/// Blit `src` onto `dst` with position, anchor, flip, rotation, and opacity.
///
/// Pixels that map outside `dst`'s bounds are silently clipped.
/// Source pixels with `alpha == 0` are skipped.  Non-zero alpha uses
/// Porter-Duff source-over, scaled by `params.opacity`.
///
/// Ported from Spriteform compositorRenderMath.ts `blitTransformed`.
pub fn blit_transformed(dst: &mut ImageBuffer, src: &ImageBuffer, params: &BlitParams) {
    let w = src.width as i32;
    let h = src.height as i32;

    // After rotation, the bounding box may swap dimensions.
    let (rw, rh) = match params.rotation {
        Rotation::Cw90 | Rotation::Cw270 => (h, w),
        _ => (w, h),
    };

    let (x0, y0) = match params.anchor {
        Anchor::Center => (
            (params.dx as f64 - rw as f64 / 2.0).round() as i32,
            (params.dy as f64 - rh as f64 / 2.0).round() as i32,
        ),
        Anchor::TopLeft => (params.dx, params.dy),
    };

    let op = params.opacity.clamp(0.0, 1.0);

    let dw = dst.width as i32;
    let dh = dst.height as i32;

    for y in 0..rh {
        for x in 0..rw {
            // Unmap destination pixel back to source coordinates via rotation inverse.
            let (mut ux, mut uy) = match params.rotation {
                Rotation::None => (x, y),
                Rotation::Cw90 => (y, h - 1 - x),
                Rotation::Cw180 => (w - 1 - x, h - 1 - y),
                Rotation::Cw270 => (w - 1 - y, x),
            };

            if params.flip_x {
                ux = w - 1 - ux;
            }
            if params.flip_y {
                uy = h - 1 - uy;
            }

            if ux < 0 || uy < 0 || ux >= w || uy >= h {
                continue;
            }

            let si = ((uy * w + ux) * 4) as usize;
            let sa_byte = src.data[si + 3];
            if sa_byte == 0 {
                continue;
            }

            let tx = x0 + x;
            let ty = y0 + y;
            if tx < 0 || ty < 0 || tx >= dw || ty >= dh {
                continue;
            }

            let di = ((ty * dw + tx) * 4) as usize;
            let da = dst.data[di + 3] as f64 / 255.0;
            let s_alpha = (sa_byte as f64 / 255.0) * op;
            let out_a = s_alpha + da * (1.0 - s_alpha);
            if out_a <= 0.0 {
                continue;
            }

            let sr = src.data[si] as f64;
            let sg = src.data[si + 1] as f64;
            let sb = src.data[si + 2] as f64;
            let dr = dst.data[di] as f64;
            let dg = dst.data[di + 1] as f64;
            let db = dst.data[di + 2] as f64;

            let inv_sa = 1.0 - s_alpha;
            dst.data[di] = clamp_byte((sr * s_alpha + dr * da * inv_sa) / out_a);
            dst.data[di + 1] = clamp_byte((sg * s_alpha + dg * da * inv_sa) / out_a);
            dst.data[di + 2] = clamp_byte((sb * s_alpha + db * da * inv_sa) / out_a);
            dst.data[di + 3] = clamp_byte(out_a * 255.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba;

    fn red_2x2() -> ImageBuffer {
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.fill(Rgba::new(255, 0, 0, 255));
        buf
    }

    #[test]
    fn blit_topleft_no_transform() {
        let mut dst = ImageBuffer::new_transparent(4, 4);
        let src = red_2x2();
        blit_transformed(
            &mut dst,
            &src,
            &BlitParams {
                dx: 1,
                dy: 1,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
                opacity: 1.0,
            },
        );
        assert_eq!(dst.get_pixel(0, 0), Rgba::TRANSPARENT);
        assert_eq!(dst.get_pixel(1, 1), Rgba::new(255, 0, 0, 255));
        assert_eq!(dst.get_pixel(2, 2), Rgba::new(255, 0, 0, 255));
        assert_eq!(dst.get_pixel(3, 3), Rgba::TRANSPARENT);
    }

    #[test]
    fn blit_with_rotation_90() {
        // 2x1 horizontal bar → after 90° CW becomes 1x2 vertical bar
        let mut src = ImageBuffer::new_transparent(2, 1);
        src.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        src.set_pixel(1, 0, Rgba::new(0, 255, 0, 255));

        let mut dst = ImageBuffer::new_transparent(4, 4);
        blit_transformed(
            &mut dst,
            &src,
            &BlitParams {
                dx: 0,
                dy: 0,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::Cw90,
                opacity: 1.0,
            },
        );
        // After 90° CW rotation of [R, G] (2x1):
        // Output is 1x2: top=R, bottom=G
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(dst.get_pixel(0, 1), Rgba::new(0, 255, 0, 255));
    }

    #[test]
    fn blit_with_opacity() {
        let mut dst = ImageBuffer::new_transparent(1, 1);
        dst.set_pixel(0, 0, Rgba::new(0, 0, 255, 255));
        let mut src = ImageBuffer::new_transparent(1, 1);
        src.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        blit_transformed(
            &mut dst,
            &src,
            &BlitParams {
                dx: 0,
                dy: 0,
                anchor: Anchor::TopLeft,
                flip_x: false,
                flip_y: false,
                rotation: Rotation::None,
                opacity: 0.5,
            },
        );
        let p = dst.get_pixel(0, 0);
        // opacity=0.5: sAlpha=0.5, da=1.0, outA=0.5+0.5=1.0
        // outR = (255*0.5 + 0*0.5)/1.0 = 127.5 → 127
        assert!(p.r > 120 && p.r < 135, "r={}", p.r);
        assert!(p.b > 120 && p.b < 135, "b={}", p.b);
    }

    #[test]
    fn rotation_from_degrees() {
        assert_eq!(Rotation::from_degrees(0.0), Rotation::None);
        assert_eq!(Rotation::from_degrees(90.0), Rotation::Cw90);
        assert_eq!(Rotation::from_degrees(180.0), Rotation::Cw180);
        assert_eq!(Rotation::from_degrees(270.0), Rotation::Cw270);
        assert_eq!(Rotation::from_degrees(-90.0), Rotation::Cw270);
        assert_eq!(Rotation::from_degrees(360.0), Rotation::None);
        assert_eq!(Rotation::from_degrees(45.0), Rotation::Cw90); // snaps to 90 (round 0.5 → 1)
        assert_eq!(Rotation::from_degrees(44.0), Rotation::None); // snaps to 0
    }
}
