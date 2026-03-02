//! Geometric transforms: resize, crop, trim, and arbitrary-angle rotation.
//!
//! Three resize algorithms are provided at different quality/speed trade-offs:
//!
//! | Function | Algorithm | Best for |
//! |----------|-----------|----------|
//! | [`resize_nearest`] | Nearest-neighbor | Pixel art, fast previews |
//! | [`resize_bilinear`] | Bilinear interpolation | Photos, smooth upscaling |
//! | [`resize_lanczos3`] | Lanczos3 (separable) | High-quality photo downscaling |
//!
//! [`crop`] extracts a rectangular sub-region.  [`trim_alpha`] auto-crops to the
//! bounding box of non-transparent pixels.
//!
//! [`rotate_bilinear`] rotates by an arbitrary angle (degrees) with bilinear
//! interpolation; the output buffer is sized to contain the full rotated image.
//! For 90-degree-increment rotations prefer [`blit::Rotation`](crate::blit::Rotation)
//! which is lossless and faster.

use crate::buffer::ImageBuffer;
use fast_image_resize as fir;

/// Resize an image using nearest-neighbor sampling.
///
/// Fast and lossless for pixel art.  Returns a new buffer of size
/// `new_width × new_height`.  Returns a transparent buffer if either
/// source or destination dimension is zero.
pub fn resize_nearest(src: &ImageBuffer, new_width: u32, new_height: u32) -> ImageBuffer {
    let mut dst = ImageBuffer::new_transparent(new_width, new_height);
    if src.width == 0 || src.height == 0 || new_width == 0 || new_height == 0 {
        return dst;
    }
    let sw = src.width as f64;
    let sh = src.height as f64;
    let dw = new_width as usize;
    let nw = new_width as f64;
    let nh = new_height as f64;
    let src_w = src.width as usize;

    for y in 0..new_height as usize {
        let sy = ((y as f64 + 0.5) * sh / nh) as usize;
        let sy = sy.min(src.height as usize - 1);
        for x in 0..dw {
            let sx = ((x as f64 + 0.5) * sw / nw) as usize;
            let sx = sx.min(src.width as usize - 1);
            let si = (sy * src_w + sx) * 4;
            let di = (y * dw + x) * 4;
            dst.data[di..di + 4].copy_from_slice(&src.data[si..si + 4]);
        }
    }
    dst
}

/// Sample a pixel with bilinear interpolation at fractional coordinates.
fn sample_bilinear(src: &ImageBuffer, fx: f64, fy: f64) -> [f64; 4] {
    let w = src.width as usize;
    let h = src.height as usize;

    let x0 = (fx.floor() as isize).max(0).min(w as isize - 1) as usize;
    let y0 = (fy.floor() as isize).max(0).min(h as isize - 1) as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);

    let dx = fx - fx.floor();
    let dy = fy - fy.floor();

    let i00 = (y0 * w + x0) * 4;
    let i10 = (y0 * w + x1) * 4;
    let i01 = (y1 * w + x0) * 4;
    let i11 = (y1 * w + x1) * 4;

    let mut result = [0.0f64; 4];
    for (c, res) in result.iter_mut().enumerate() {
        let c00 = src.data[i00 + c] as f64;
        let c10 = src.data[i10 + c] as f64;
        let c01 = src.data[i01 + c] as f64;
        let c11 = src.data[i11 + c] as f64;
        *res = c00 * (1.0 - dx) * (1.0 - dy)
            + c10 * dx * (1.0 - dy)
            + c01 * (1.0 - dx) * dy
            + c11 * dx * dy;
    }
    result
}

/// Resize an image using bilinear interpolation.
///
/// Produces smoother results than nearest-neighbor for photos and continuous-
/// tone images.  Returns a new buffer of size `new_width × new_height`.
pub fn resize_bilinear(src: &ImageBuffer, new_width: u32, new_height: u32) -> ImageBuffer {
    if let Some(dst) = resize_with_fir(src, new_width, new_height, fir::FilterType::Bilinear) {
        return dst;
    }

    resize_bilinear_fallback(src, new_width, new_height)
}

fn resize_bilinear_fallback(src: &ImageBuffer, new_width: u32, new_height: u32) -> ImageBuffer {
    let mut dst = ImageBuffer::new_transparent(new_width, new_height);
    if src.width == 0 || src.height == 0 || new_width == 0 || new_height == 0 {
        return dst;
    }
    let sx = src.width as f64 / new_width as f64;
    let sy = src.height as f64 / new_height as f64;
    let dw = new_width as usize;

    for y in 0..new_height as usize {
        for x in 0..dw {
            let fx = (x as f64 + 0.5) * sx - 0.5;
            let fy = (y as f64 + 0.5) * sy - 0.5;
            let px = sample_bilinear(src, fx, fy);
            let di = (y * dw + x) * 4;
            dst.data[di] = px[0].clamp(0.0, 255.0) as u8;
            dst.data[di + 1] = px[1].clamp(0.0, 255.0) as u8;
            dst.data[di + 2] = px[2].clamp(0.0, 255.0) as u8;
            dst.data[di + 3] = px[3].clamp(0.0, 255.0) as u8;
        }
    }
    dst
}

/// Lanczos kernel with a=3.
fn lanczos3(x: f64) -> f64 {
    if x.abs() < 1e-10 {
        return 1.0;
    }
    if x.abs() >= 3.0 {
        return 0.0;
    }
    let pi_x = std::f64::consts::PI * x;
    let pi_x3 = pi_x / 3.0;
    (pi_x.sin() / pi_x) * (pi_x3.sin() / pi_x3)
}

/// Resize an image using Lanczos3 interpolation.
///
/// Two-pass separable (horizontal then vertical).  Highest quality for
/// downscaling photographs; slower than bilinear.
pub fn resize_lanczos3(src: &ImageBuffer, new_width: u32, new_height: u32) -> ImageBuffer {
    if let Some(dst) = resize_with_fir(src, new_width, new_height, fir::FilterType::Lanczos3) {
        return dst;
    }

    resize_lanczos3_fallback(src, new_width, new_height)
}

fn resize_lanczos3_fallback(src: &ImageBuffer, new_width: u32, new_height: u32) -> ImageBuffer {
    if src.width == 0 || src.height == 0 || new_width == 0 || new_height == 0 {
        return ImageBuffer::new_transparent(new_width, new_height);
    }

    // Two-pass separable: horizontal then vertical
    let tmp = resize_lanczos3_horizontal(src, new_width);
    resize_lanczos3_vertical(&tmp, new_height)
}

fn resize_with_fir(
    src: &ImageBuffer,
    new_width: u32,
    new_height: u32,
    filter: fir::FilterType,
) -> Option<ImageBuffer> {
    if src.width == 0 || src.height == 0 || new_width == 0 || new_height == 0 {
        return Some(ImageBuffer::new_transparent(new_width, new_height));
    }

    let src_image =
        fir::images::ImageRef::new(src.width, src.height, &src.data, fir::PixelType::U8x4).ok()?;
    let mut dst_image = fir::images::Image::new(new_width, new_height, fir::PixelType::U8x4);
    let options = fir::ResizeOptions::new()
        .resize_alg(fir::ResizeAlg::Convolution(filter))
        .use_alpha(true);

    fir::Resizer::new()
        .resize(&src_image, &mut dst_image, &options)
        .ok()?;

    ImageBuffer::from_rgba(new_width, new_height, dst_image.into_vec())
}

fn resize_lanczos3_horizontal(src: &ImageBuffer, new_width: u32) -> ImageBuffer {
    let sw = src.width as usize;
    let sh = src.height as usize;
    let dw = new_width as usize;
    let ratio = sw as f64 / dw as f64;
    let mut dst = ImageBuffer::new_transparent(new_width, src.height);

    for y in 0..sh {
        for x in 0..dw {
            let center = (x as f64 + 0.5) * ratio - 0.5;
            let left = (center - 3.0).ceil() as isize;
            let right = (center + 3.0).floor() as isize;

            let mut sum = [0.0f64; 4];
            let mut weight_sum = 0.0;

            for i in left..=right {
                let si = i.max(0).min(sw as isize - 1) as usize;
                let w = lanczos3(center - i as f64);
                weight_sum += w;
                let idx = (y * sw + si) * 4;
                for (c, s) in sum.iter_mut().enumerate() {
                    *s += src.data[idx + c] as f64 * w;
                }
            }

            if weight_sum > 0.0 {
                let di = (y * dw + x) * 4;
                for (c, s) in sum.iter().enumerate() {
                    dst.data[di + c] = (*s / weight_sum).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    dst
}

fn resize_lanczos3_vertical(src: &ImageBuffer, new_height: u32) -> ImageBuffer {
    let sw = src.width as usize;
    let sh = src.height as usize;
    let dh = new_height as usize;
    let ratio = sh as f64 / dh as f64;
    let mut dst = ImageBuffer::new_transparent(src.width, new_height);

    for y in 0..dh {
        let center = (y as f64 + 0.5) * ratio - 0.5;
        let top = (center - 3.0).ceil() as isize;
        let bottom = (center + 3.0).floor() as isize;

        for x in 0..sw {
            let mut sum = [0.0f64; 4];
            let mut weight_sum = 0.0;

            for i in top..=bottom {
                let si = i.max(0).min(sh as isize - 1) as usize;
                let w = lanczos3(center - i as f64);
                weight_sum += w;
                let idx = (si * sw + x) * 4;
                for (c, s) in sum.iter_mut().enumerate() {
                    *s += src.data[idx + c] as f64 * w;
                }
            }

            if weight_sum > 0.0 {
                let di = (y * sw + x) * 4;
                for (c, s) in sum.iter().enumerate() {
                    dst.data[di + c] = (*s / weight_sum).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    dst
}

/// Crop a rectangular region from an image.
///
/// The region is clamped to the source bounds — rows and columns that fall
/// outside the source are left transparent.
pub fn crop(src: &ImageBuffer, x: u32, y: u32, width: u32, height: u32) -> ImageBuffer {
    let mut dst = ImageBuffer::new_transparent(width, height);
    let sw = src.width as usize;
    let dw = width as usize;

    for dy in 0..height as usize {
        let sy = y as usize + dy;
        if sy >= src.height as usize {
            break;
        }
        for dx in 0..dw {
            let sx = x as usize + dx;
            if sx >= sw {
                break;
            }
            let si = (sy * sw + sx) * 4;
            let di = (dy * dw + dx) * 4;
            dst.data[di..di + 4].copy_from_slice(&src.data[si..si + 4]);
        }
    }
    dst
}

/// Crop to the tight bounding box of non-transparent (alpha > 0) pixels.
///
/// Returns an empty 0×0 buffer if all pixels are fully transparent.
pub fn trim_alpha(src: &ImageBuffer) -> ImageBuffer {
    let w = src.width as usize;
    let h = src.height as usize;
    if w == 0 || h == 0 {
        return src.clone();
    }

    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0usize;
    let mut max_y = 0usize;

    for y in 0..h {
        for x in 0..w {
            let a = src.data[(y * w + x) * 4 + 3];
            if a > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x > max_x {
        // All transparent
        return ImageBuffer::new_transparent(0, 0);
    }

    let tw = max_x - min_x + 1;
    let th = max_y - min_y + 1;
    crop(src, min_x as u32, min_y as u32, tw as u32, th as u32)
}

/// Rotate an image by an arbitrary angle (clockwise, in degrees) with bilinear
/// interpolation.
///
/// The output buffer is sized to contain the complete rotated image with no
/// clipping.  Areas outside the source image are left transparent.
pub fn rotate_bilinear(src: &ImageBuffer, angle_deg: f64) -> ImageBuffer {
    if src.width == 0 || src.height == 0 {
        return src.clone();
    }

    let angle = angle_deg.to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let sw = src.width as f64;
    let sh = src.height as f64;
    let cx = sw / 2.0;
    let cy = sh / 2.0;

    // Compute bounding box of rotated image
    let corners = [(0.0, 0.0), (sw, 0.0), (0.0, sh), (sw, sh)];
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for (px, py) in &corners {
        let rx = (px - cx) * cos_a - (py - cy) * sin_a + cx;
        let ry = (px - cx) * sin_a + (py - cy) * cos_a + cy;
        min_x = min_x.min(rx);
        min_y = min_y.min(ry);
        max_x = max_x.max(rx);
        max_y = max_y.max(ry);
    }

    let dw = (max_x - min_x).ceil() as u32;
    let dh = (max_y - min_y).ceil() as u32;
    if dw == 0 || dh == 0 {
        return ImageBuffer::new_transparent(0, 0);
    }

    let mut dst = ImageBuffer::new_transparent(dw, dh);
    let dcx = dw as f64 / 2.0;
    let dcy = dh as f64 / 2.0;

    for dy in 0..dh as usize {
        for dx in 0..dw as usize {
            // Map destination back to source (inverse rotation)
            let rx = dx as f64 - dcx;
            let ry = dy as f64 - dcy;
            let sx = rx * cos_a + ry * sin_a + cx;
            let sy = -rx * sin_a + ry * cos_a + cy;

            if sx < -0.5 || sy < -0.5 || sx > sw - 0.5 || sy > sh - 0.5 {
                continue;
            }

            let px = sample_bilinear(src, sx, sy);
            let di = (dy * dw as usize + dx) * 4;
            dst.data[di] = px[0].clamp(0.0, 255.0) as u8;
            dst.data[di + 1] = px[1].clamp(0.0, 255.0) as u8;
            dst.data[di + 2] = px[2].clamp(0.0, 255.0) as u8;
            dst.data[di + 3] = px[3].clamp(0.0, 255.0) as u8;
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba;

    fn red_4x4() -> ImageBuffer {
        let mut buf = ImageBuffer::new_transparent(4, 4);
        buf.fill(Rgba::new(255, 0, 0, 255));
        buf
    }

    #[test]
    fn resize_nearest_upscale() {
        let src = red_4x4();
        let dst = resize_nearest(&src, 8, 8);
        assert_eq!(dst.width, 8);
        assert_eq!(dst.height, 8);
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(dst.get_pixel(7, 7), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn resize_nearest_downscale() {
        let src = red_4x4();
        let dst = resize_nearest(&src, 2, 2);
        assert_eq!(dst.width, 2);
        assert_eq!(dst.height, 2);
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn resize_bilinear_preserves_solid() {
        let src = red_4x4();
        let dst = resize_bilinear(&src, 8, 8);
        assert_eq!(dst.width, 8);
        let p = dst.get_pixel(4, 4);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 0);
    }

    #[test]
    fn resize_lanczos3_preserves_solid() {
        let src = red_4x4();
        let dst = resize_lanczos3(&src, 8, 8);
        assert_eq!(dst.width, 8);
        let p = dst.get_pixel(4, 4);
        assert!(p.r > 250, "r={}", p.r);
    }

    #[test]
    fn resize_fir_path_preserves_transparent_edges() {
        let mut src = ImageBuffer::new_transparent(4, 4);
        src.set_pixel(1, 1, Rgba::new(255, 128, 64, 255));
        let dst = resize_bilinear(&src, 8, 8);

        assert_eq!(dst.get_pixel(0, 0), Rgba::TRANSPARENT);
        assert!(dst.get_pixel(3, 3).a > 0);
    }

    #[test]
    fn crop_subregion() {
        let mut src = ImageBuffer::new_transparent(4, 4);
        src.set_pixel(1, 1, Rgba::new(255, 0, 0, 255));
        let cropped = crop(&src, 1, 1, 2, 2);
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        assert_eq!(cropped.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(cropped.get_pixel(1, 1), Rgba::TRANSPARENT);
    }

    #[test]
    fn trim_alpha_removes_transparent_border() {
        let mut src = ImageBuffer::new_transparent(4, 4);
        src.set_pixel(1, 1, Rgba::new(255, 0, 0, 255));
        src.set_pixel(2, 2, Rgba::new(0, 255, 0, 255));
        let trimmed = trim_alpha(&src);
        assert_eq!(trimmed.width, 2);
        assert_eq!(trimmed.height, 2);
        assert_eq!(trimmed.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(trimmed.get_pixel(1, 1), Rgba::new(0, 255, 0, 255));
    }

    #[test]
    fn trim_alpha_all_transparent() {
        let src = ImageBuffer::new_transparent(4, 4);
        let trimmed = trim_alpha(&src);
        assert_eq!(trimmed.width, 0);
        assert_eq!(trimmed.height, 0);
    }

    #[test]
    fn rotate_bilinear_0_degrees() {
        let src = red_4x4();
        let dst = rotate_bilinear(&src, 0.0);
        assert_eq!(dst.width, 4);
        assert_eq!(dst.height, 4);
        assert_eq!(dst.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
    }

    #[test]
    fn rotate_bilinear_90_degrees() {
        let mut src = ImageBuffer::new_transparent(2, 1);
        src.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        src.set_pixel(1, 0, Rgba::new(0, 0, 255, 255));
        let dst = rotate_bilinear(&src, 90.0);
        // 2x1 rotated 90° → roughly 1x2
        assert!(dst.width >= 1);
        assert!(dst.height >= 2);
    }

    #[test]
    fn rotate_bilinear_180_degrees() {
        let mut src = ImageBuffer::new_transparent(2, 2);
        src.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        let dst = rotate_bilinear(&src, 180.0);
        // The red pixel should be near (1,1) after 180° rotation
        assert!(dst.width >= 2);
        assert!(dst.height >= 2);
    }
}
