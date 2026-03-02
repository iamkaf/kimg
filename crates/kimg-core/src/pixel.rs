//! Core pixel type.
//!
//! [`Rgba`] is the fundamental color unit throughout kimg — every pixel in every
//! [`ImageBuffer`](crate::buffer::ImageBuffer) is one `Rgba` value stored as four
//! contiguous `u8` bytes in RGBA order.

/// A 32-bit RGBA color pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rgba {
    /// Red channel (0-255).
    pub r: u8,
    /// Green channel (0-255).
    pub g: u8,
    /// Blue channel (0-255).
    pub b: u8,
    /// Alpha channel (0-255). 0 is transparent, 255 is fully opaque.
    pub a: u8,
}

impl Default for Rgba {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl Rgba {
    /// A fully transparent pixel (0, 0, 0, 0).
    pub const TRANSPARENT: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// Create a new RGBA pixel.
    ///
    /// # Examples
    ///
    /// ```
    /// use kimg_core::pixel::Rgba;
    ///
    /// let red = Rgba::new(255, 0, 0, 255);
    /// let semi = Rgba::new(0, 128, 255, 128); // 50% transparent blue
    /// ```
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_transparent() {
        assert_eq!(Rgba::default(), Rgba::TRANSPARENT);
    }

    #[test]
    fn new_constructs_correctly() {
        let p = Rgba::new(255, 128, 0, 200);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 128);
        assert_eq!(p.b, 0);
        assert_eq!(p.a, 200);
    }
}
