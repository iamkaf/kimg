//! Matrix convolution filters.
//!
//! [`Kernel`] defines an NxN (odd dimensions) weight matrix.  A set of
//! ready-made presets is provided (Gaussian blur, sharpen, edge detect, emboss).
//! Apply a kernel to an [`ImageBuffer`] with [`convolve`], [`box_blur`], or
//! [`gaussian_blur`].
//!
//! Only RGB channels are modified; alpha is copied unchanged.

use crate::buffer::ImageBuffer;

/// A convolution kernel (NxN, odd dimensions).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Kernel {
    /// Dimension of the kernel (e.g. 3 for 3x3).
    pub size: usize,
    /// Flat row-major array of kernel weights.
    pub data: Vec<f64>,
}

impl Kernel {
    /// Create a kernel from a flat row-major array.
    ///
    /// # Panics
    ///
    /// Panics if `size` is even or if `data.len() != size * size`.
    pub fn new(size: usize, data: Vec<f64>) -> Self {
        assert!(size % 2 == 1, "kernel size must be odd");
        assert_eq!(data.len(), size * size, "data length must be size*size");
        Self { size, data }
    }

    /// 3x3 identity kernel (no-op).
    pub fn identity() -> Self {
        Self::new(3, vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0])
    }

    /// 3x3 box blur (average).
    pub fn box_blur_3x3() -> Self {
        let v = 1.0 / 9.0;
        Self::new(3, vec![v; 9])
    }

    /// 5x5 box blur.
    pub fn box_blur_5x5() -> Self {
        let v = 1.0 / 25.0;
        Self::new(5, vec![v; 25])
    }

    /// 3x3 Gaussian blur approximation.
    pub fn gaussian_blur_3x3() -> Self {
        Self::new(
            3,
            vec![
                1.0 / 16.0,
                2.0 / 16.0,
                1.0 / 16.0,
                2.0 / 16.0,
                4.0 / 16.0,
                2.0 / 16.0,
                1.0 / 16.0,
                2.0 / 16.0,
                1.0 / 16.0,
            ],
        )
    }

    /// 5x5 Gaussian blur approximation.
    pub fn gaussian_blur_5x5() -> Self {
        #[rustfmt::skip]
        let d = vec![
            1.0, 4.0,  6.0,  4.0,  1.0,
            4.0, 16.0, 24.0, 16.0, 4.0,
            6.0, 24.0, 36.0, 24.0, 6.0,
            4.0, 16.0, 24.0, 16.0, 4.0,
            1.0, 4.0,  6.0,  4.0,  1.0,
        ];
        let sum: f64 = d.iter().sum();
        Self::new(5, d.into_iter().map(|v| v / sum).collect())
    }

    /// 3x3 sharpen.
    pub fn sharpen() -> Self {
        Self::new(3, vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0])
    }

    /// 3x3 edge detection (Laplacian).
    pub fn edge_detect() -> Self {
        Self::new(3, vec![-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0])
    }

    /// 3x3 emboss.
    pub fn emboss() -> Self {
        Self::new(3, vec![-2.0, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 2.0])
    }
}

/// Apply a convolution kernel to an image buffer.
/// Only operates on RGB channels; alpha is preserved.
pub fn convolve(buf: &ImageBuffer, kernel: &Kernel) -> ImageBuffer {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let mut dst = buf.clone();
    let half = kernel.size / 2;

    for y in 0..h {
        for x in 0..w {
            let di = (y * w + x) * 4;
            if buf.data[di + 3] == 0 {
                continue;
            }

            let mut sum_r = 0.0f64;
            let mut sum_g = 0.0f64;
            let mut sum_b = 0.0f64;

            for ky in 0..kernel.size {
                for kx in 0..kernel.size {
                    let sy = (y as isize + ky as isize - half as isize)
                        .max(0)
                        .min(h as isize - 1) as usize;
                    let sx = (x as isize + kx as isize - half as isize)
                        .max(0)
                        .min(w as isize - 1) as usize;
                    let si = (sy * w + sx) * 4;
                    let kv = kernel.data[ky * kernel.size + kx];
                    sum_r += buf.data[si] as f64 * kv;
                    sum_g += buf.data[si + 1] as f64 * kv;
                    sum_b += buf.data[si + 2] as f64 * kv;
                }
            }

            dst.data[di] = sum_r.clamp(0.0, 255.0) as u8;
            dst.data[di + 1] = sum_g.clamp(0.0, 255.0) as u8;
            dst.data[di + 2] = sum_b.clamp(0.0, 255.0) as u8;
            // Alpha unchanged
        }
    }
    dst
}

/// Apply a box blur with the given radius.
///
/// Kernel size is `2 * radius + 1`.  Radius 0 returns a clone unchanged.
pub fn box_blur(buf: &ImageBuffer, radius: u32) -> ImageBuffer {
    if radius == 0 {
        return buf.clone();
    }
    let size = (radius * 2 + 1) as usize;
    let count = (size * size) as f64;
    let kernel = Kernel::new(size, vec![1.0 / count; size * size]);
    convolve(buf, &kernel)
}

/// Apply a Gaussian blur.
///
/// Radius 1 uses a 3×3 kernel; radius 2 uses a 5×5 kernel.  Larger radii are
/// approximated by repeated 3×3 passes.  Radius 0 returns a clone unchanged.
pub fn gaussian_blur(buf: &ImageBuffer, radius: u32) -> ImageBuffer {
    match radius {
        0 => buf.clone(),
        1 => convolve(buf, &Kernel::gaussian_blur_3x3()),
        2 => convolve(buf, &Kernel::gaussian_blur_5x5()),
        _ => {
            // For larger radii, use multiple passes of 3x3 Gaussian
            let k = Kernel::gaussian_blur_3x3();
            let mut result = buf.clone();
            for _ in 0..radius {
                result = convolve(&result, &k);
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba;

    fn test_buf() -> ImageBuffer {
        let mut buf = ImageBuffer::new_transparent(3, 3);
        buf.fill(Rgba::new(100, 150, 200, 255));
        buf.set_pixel(1, 1, Rgba::new(255, 255, 255, 255));
        buf
    }

    #[test]
    fn identity_kernel_is_noop() {
        let buf = test_buf();
        let result = convolve(&buf, &Kernel::identity());
        assert_eq!(result.get_pixel(1, 1), Rgba::new(255, 255, 255, 255));
        assert_eq!(result.get_pixel(0, 0), Rgba::new(100, 150, 200, 255));
    }

    #[test]
    fn box_blur_3x3_averages() {
        let buf = test_buf();
        let result = convolve(&buf, &Kernel::box_blur_3x3());
        let p = result.get_pixel(1, 1);
        // Center pixel: average of 8 * (100,150,200) + 1 * (255,255,255) / 9
        // r = (800 + 255) / 9 ≈ 117
        assert!(p.r > 110 && p.r < 125, "r={}", p.r);
    }

    #[test]
    fn sharpen_increases_contrast() {
        let buf = test_buf();
        let result = convolve(&buf, &Kernel::sharpen());
        let p = result.get_pixel(1, 1);
        // Sharpen should push the bright center pixel even brighter
        assert_eq!(p.r, 255); // clamped to 255
    }

    #[test]
    fn edge_detect_finds_edges() {
        let buf = test_buf();
        let result = convolve(&buf, &Kernel::edge_detect());
        let p = result.get_pixel(1, 1);
        // Edge detect on center pixel surrounded by darker pixels should be bright
        assert!(p.r > 200, "r={}", p.r);
    }

    #[test]
    fn box_blur_radius() {
        let buf = test_buf();
        let result = box_blur(&buf, 1);
        let p = result.get_pixel(1, 1);
        assert!(p.r > 110 && p.r < 125, "r={}", p.r);
    }

    #[test]
    fn gaussian_blur_radius_1() {
        let buf = test_buf();
        let result = gaussian_blur(&buf, 1);
        let p = result.get_pixel(1, 1);
        // Should be somewhat averaged
        assert!(p.r > 130 && p.r < 220, "r={}", p.r);
    }

    #[test]
    fn transparent_pixels_preserved() {
        let mut buf = ImageBuffer::new_transparent(3, 3);
        buf.set_pixel(1, 1, Rgba::new(255, 0, 0, 255));
        let result = convolve(&buf, &Kernel::box_blur_3x3());
        // Corners should still be transparent (alpha = 0)
        assert_eq!(result.get_pixel(0, 0).a, 0);
    }

    #[test]
    fn emboss_runs() {
        let buf = test_buf();
        let result = convolve(&buf, &Kernel::emboss());
        // Just verify it doesn't panic and produces valid output
        let p = result.get_pixel(1, 1);
        assert!(p.a > 0);
    }
}
