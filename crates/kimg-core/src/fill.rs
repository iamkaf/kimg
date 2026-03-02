//! Bucket-fill operations for RGBA image buffers.
//!
//! Matching is alpha-aware: tolerance is applied per channel across RGBA, so a
//! pixel only matches when each channel differs from the seed color by at most
//! `tolerance`.

use std::collections::VecDeque;

use crate::buffer::ImageBuffer;
use crate::pixel::Rgba;

/// Bucket-fill a buffer starting from `(x, y)`.
///
/// When `contiguous` is true, only the 4-connected region matching the seed
/// color is filled. When false, every matching pixel in the layer is replaced.
///
/// Returns `false` when the start point is out of bounds.
pub fn bucket_fill(
    buffer: &mut ImageBuffer,
    x: u32,
    y: u32,
    replacement: Rgba,
    contiguous: bool,
    tolerance: u8,
) -> bool {
    if x >= buffer.width || y >= buffer.height {
        return false;
    }

    let seed = buffer.get_pixel(x, y);
    if seed == replacement && tolerance == 0 {
        return true;
    }

    if contiguous {
        bucket_fill_contiguous(buffer, x, y, seed, replacement, tolerance);
    } else {
        bucket_fill_global(buffer, seed, replacement, tolerance);
    }

    true
}

fn bucket_fill_global(buffer: &mut ImageBuffer, seed: Rgba, replacement: Rgba, tolerance: u8) {
    for py in 0..buffer.height {
        for px in 0..buffer.width {
            if matches_seed(buffer.get_pixel(px, py), seed, tolerance) {
                buffer.set_pixel(px, py, replacement);
            }
        }
    }
}

fn bucket_fill_contiguous(
    buffer: &mut ImageBuffer,
    start_x: u32,
    start_y: u32,
    seed: Rgba,
    replacement: Rgba,
    tolerance: u8,
) {
    let mut queue = VecDeque::from([(start_x, start_y)]);
    let mut visited = vec![false; buffer.data.len() / 4];

    while let Some((x, y)) = queue.pop_front() {
        let index = (y as usize) * (buffer.width as usize) + (x as usize);
        if visited[index] {
            continue;
        }
        visited[index] = true;

        if !matches_seed(buffer.get_pixel(x, y), seed, tolerance) {
            continue;
        }

        buffer.set_pixel(x, y, replacement);

        if x > 0 {
            queue.push_back((x - 1, y));
        }
        if x + 1 < buffer.width {
            queue.push_back((x + 1, y));
        }
        if y > 0 {
            queue.push_back((x, y - 1));
        }
        if y + 1 < buffer.height {
            queue.push_back((x, y + 1));
        }
    }
}

fn matches_seed(pixel: Rgba, seed: Rgba, tolerance: u8) -> bool {
    pixel.r.abs_diff(seed.r) <= tolerance
        && pixel.g.abs_diff(seed.g) <= tolerance
        && pixel.b.abs_diff(seed.b) <= tolerance
        && pixel.a.abs_diff(seed.a) <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contiguous_fill_stays_within_region() {
        let mut buffer = ImageBuffer::new_transparent(3, 3);
        buffer.fill(Rgba::new(10, 10, 10, 255));
        buffer.set_pixel(1, 1, Rgba::new(200, 0, 0, 255));

        assert!(bucket_fill(
            &mut buffer,
            0,
            0,
            Rgba::new(0, 255, 0, 255),
            true,
            0,
        ));

        assert_eq!(buffer.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
        assert_eq!(buffer.get_pixel(2, 2), Rgba::new(0, 255, 0, 255));
        assert_eq!(buffer.get_pixel(1, 1), Rgba::new(200, 0, 0, 255));
    }

    #[test]
    fn non_contiguous_fill_reaches_disconnected_matches() {
        let mut buffer = ImageBuffer::new_transparent(3, 1);
        buffer.set_pixel(0, 0, Rgba::new(100, 0, 0, 255));
        buffer.set_pixel(1, 0, Rgba::new(0, 0, 0, 255));
        buffer.set_pixel(2, 0, Rgba::new(100, 0, 0, 255));

        assert!(bucket_fill(
            &mut buffer,
            0,
            0,
            Rgba::new(0, 255, 0, 255),
            false,
            0,
        ));

        assert_eq!(buffer.get_pixel(0, 0), Rgba::new(0, 255, 0, 255));
        assert_eq!(buffer.get_pixel(1, 0), Rgba::new(0, 0, 0, 255));
        assert_eq!(buffer.get_pixel(2, 0), Rgba::new(0, 255, 0, 255));
    }

    #[test]
    fn tolerance_is_alpha_aware() {
        let mut buffer = ImageBuffer::new_transparent(2, 1);
        buffer.set_pixel(0, 0, Rgba::new(100, 100, 100, 128));
        buffer.set_pixel(1, 0, Rgba::new(100, 100, 100, 140));

        assert!(bucket_fill(
            &mut buffer,
            0,
            0,
            Rgba::new(255, 0, 0, 255),
            false,
            8,
        ));
        assert_eq!(buffer.get_pixel(0, 0), Rgba::new(255, 0, 0, 255));
        assert_eq!(buffer.get_pixel(1, 0), Rgba::new(100, 100, 100, 140));

        assert!(bucket_fill(
            &mut buffer,
            1,
            0,
            Rgba::new(0, 255, 0, 255),
            false,
            20,
        ));
        assert_eq!(buffer.get_pixel(1, 0), Rgba::new(0, 255, 0, 255));
    }
}
