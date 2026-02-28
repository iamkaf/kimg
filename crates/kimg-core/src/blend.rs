use crate::buffer::ImageBuffer;

/// Alpha-composite `src` onto `dst` using Porter-Duff source-over.
/// Both buffers must have the same dimensions.
///
/// Ported from Spriteform compositorRender.ts `blendPng`.
pub fn blend_normal(dst: &mut ImageBuffer, src: &ImageBuffer) {
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

            let sr = src.data[si] as f64;
            let sg = src.data[si + 1] as f64;
            let sb = src.data[si + 2] as f64;
            let dr = dst.data[di] as f64;
            let dg = dst.data[di + 1] as f64;
            let db = dst.data[di + 2] as f64;

            let inv_sa = 1.0 - sa;
            dst.data[di] = ((sr * sa + dr * da * inv_sa) / out_a + 0.5) as u8;
            dst.data[di + 1] = ((sg * sa + dg * da * inv_sa) / out_a + 0.5) as u8;
            dst.data[di + 2] = ((sb * sa + db * da * inv_sa) / out_a + 0.5) as u8;
            dst.data[di + 3] = (out_a * 255.0 + 0.5) as u8;
        }
    }
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
        // 128/255 ≈ 0.502 source alpha
        // out_a = 0.502 + 1.0 * 0.498 = 1.0
        // out_r = (255 * 0.502 + 0 * 0.498) / 1.0 ≈ 128
        assert!(p.r > 120 && p.r < 136, "r={}", p.r);
        assert!(p.b > 120 && p.b < 136, "b={}", p.b);
        assert_eq!(p.a, 255);
    }
}
