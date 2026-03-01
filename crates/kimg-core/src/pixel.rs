/// RGBA pixel with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

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
