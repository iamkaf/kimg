use crate::pixel::Rgba;

/// Owned RGBA image buffer stored as contiguous `Vec<u8>` in RGBA8 order.
/// 2D image buffer containing raw RGBA pixel data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBuffer {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// Flat RGBA pixel data, with size `width * height * 4`.
    pub data: Vec<u8>,
}

impl ImageBuffer {
    /// Create a new buffer filled with transparent black.
    pub fn new_transparent(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            data: vec![0; len],
        }
    }

    /// Create a buffer from existing RGBA data. Returns `None` if length doesn't match.
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if data.len() != expected {
            return None;
        }
        Some(Self {
            width,
            height,
            data,
        })
    }

    /// Get the pixel at (x, y). Panics if out of bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> Rgba {
        let i = self.pixel_index(x, y);
        Rgba {
            r: self.data[i],
            g: self.data[i + 1],
            b: self.data[i + 2],
            a: self.data[i + 3],
        }
    }

    /// Set the pixel at (x, y). Panics if out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, px: Rgba) {
        let i = self.pixel_index(x, y);
        self.data[i] = px.r;
        self.data[i + 1] = px.g;
        self.data[i + 2] = px.b;
        self.data[i + 3] = px.a;
    }

    /// Fill the entire buffer with a single color.
    pub fn fill(&mut self, px: Rgba) {
        for chunk in self.data.chunks_exact_mut(4) {
            chunk[0] = px.r;
            chunk[1] = px.g;
            chunk[2] = px.b;
            chunk[3] = px.a;
        }
    }

    #[inline]
    fn pixel_index(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width && y < self.height);
        ((y as usize) * (self.width as usize) + (x as usize)) * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_transparent_is_zeroed() {
        let buf = ImageBuffer::new_transparent(4, 4);
        assert_eq!(buf.data.len(), 64);
        assert!(buf.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn get_set_pixel_roundtrip() {
        let mut buf = ImageBuffer::new_transparent(2, 2);
        let red = Rgba::new(255, 0, 0, 255);
        buf.set_pixel(1, 0, red);
        assert_eq!(buf.get_pixel(1, 0), red);
        assert_eq!(buf.get_pixel(0, 0), Rgba::TRANSPARENT);
    }

    #[test]
    fn fill_sets_all_pixels() {
        let mut buf = ImageBuffer::new_transparent(3, 3);
        let color = Rgba::new(100, 150, 200, 255);
        buf.fill(color);
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(buf.get_pixel(x, y), color);
            }
        }
    }

    #[test]
    fn from_rgba_validates_length() {
        assert!(ImageBuffer::from_rgba(2, 2, vec![0; 16]).is_some());
        assert!(ImageBuffer::from_rgba(2, 2, vec![0; 15]).is_none());
    }
}
