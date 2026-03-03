//! SVG parsing and rasterization helpers.
//!
//! SVG layers remain source-backed in the document model and rasterize into the
//! compositor only when they need to be rendered.

use crate::buffer::ImageBuffer;
use roxmltree::Document as XmlDocument;
use usvg::{Options as UsvgOptions, Tree as UsvgTree};

/// Errors that can occur while parsing or rasterizing SVG input.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SvgError {
    /// The SVG data is not valid UTF-8 or SVGZ text.
    InvalidText,
    /// The SVG uses a feature intentionally out of scope for the current MVP.
    Unsupported(&'static str),
    /// The SVG could not be parsed into a render tree.
    Parse(String),
    /// The requested render size is invalid.
    InvalidSize,
    /// The SVG could not be rasterized.
    Render(String),
}

impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SvgError::InvalidText => write!(f, "invalid SVG text"),
            SvgError::Unsupported(reason) => write!(f, "unsupported SVG feature: {reason}"),
            SvgError::Parse(reason) => write!(f, "SVG parse failed: {reason}"),
            SvgError::InvalidSize => write!(f, "invalid SVG render size"),
            SvgError::Render(reason) => write!(f, "SVG render failed: {reason}"),
        }
    }
}

impl std::error::Error for SvgError {}

/// Validate that an SVG stays within the current MVP support envelope.
pub fn validate_svg(data: &[u8]) -> Result<(), SvgError> {
    let text = std::str::from_utf8(data).map_err(|_| SvgError::InvalidText)?;
    let doc = XmlDocument::parse(text).map_err(|_| SvgError::InvalidText)?;

    for node in doc.descendants().filter(|node| node.is_element()) {
        let tag = node.tag_name().name();
        if tag == "script" {
            return Err(SvgError::Unsupported("script elements"));
        }
        if tag == "set" || tag.starts_with("animate") {
            return Err(SvgError::Unsupported("animation elements"));
        }
        if tag == "image" {
            let href = node
                .attributes()
                .find(|attr| attr.name() == "href" || attr.name() == "xlink:href")
                .map(|attr| attr.value());
            if let Some(href) = href {
                let href = href.trim();
                if !href.is_empty() && !href.starts_with("data:") {
                    return Err(SvgError::Unsupported("external image references"));
                }
            }
        }
    }

    Ok(())
}

fn build_options() -> UsvgOptions<'static> {
    let mut options = UsvgOptions::default();
    let default_resolver = usvg::ImageHrefResolver::default();
    options.image_href_resolver = usvg::ImageHrefResolver {
        resolve_data: default_resolver.resolve_data,
        resolve_string: Box::new(|_, _| None),
    };
    if let Some(family) = crate::text::populate_runtime_fontdb(options.fontdb_mut()) {
        options.font_family = family;
    }
    options
}

fn parse_svg_tree(data: &[u8]) -> Result<UsvgTree, SvgError> {
    validate_svg(data)?;
    let options = build_options();
    UsvgTree::from_data(data, &options).map_err(|err| SvgError::Parse(err.to_string()))
}

fn pixmap_to_image_buffer(pixmap: &resvg::tiny_skia::Pixmap) -> ImageBuffer {
    let mut rgba = Vec::with_capacity((pixmap.width() * pixmap.height() * 4) as usize);
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        rgba.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    ImageBuffer::from_rgba(pixmap.width(), pixmap.height(), rgba)
        .expect("demultiplied SVG raster should match target dimensions")
}

/// Rasterize raw SVG input to a target RGBA buffer size.
pub fn rasterize_svg(data: &[u8], width: u32, height: u32) -> Result<ImageBuffer, SvgError> {
    if width == 0 || height == 0 {
        return Err(SvgError::InvalidSize);
    }

    let tree = parse_svg_tree(data)?;
    let source_size = tree.size();
    if source_size.width() <= 0.0 || source_size.height() <= 0.0 {
        return Err(SvgError::InvalidSize);
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or(SvgError::InvalidSize)?;
    let transform = resvg::tiny_skia::Transform::from_scale(
        width as f32 / source_size.width(),
        height as f32 / source_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap_to_image_buffer(&pixmap))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_SVG: &str = r##"
        <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
          <defs>
            <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stop-color="#d9482b"/>
              <stop offset="100%" stop-color="#2d55d7"/>
            </linearGradient>
          </defs>
          <rect x="4" y="4" width="56" height="56" rx="10" fill="url(#g)"/>
          <circle cx="20" cy="20" r="6" fill="#f2c94c"/>
        </svg>
    "##;

    #[test]
    fn rasterize_svg_renders_visible_pixels() {
        let rendered = rasterize_svg(SIMPLE_SVG.as_bytes(), 64, 64).unwrap();
        assert!(rendered.data.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn validate_svg_rejects_external_images() {
        let svg = br#"
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
              <image href="https://example.com/logo.png" width="16" height="16" />
            </svg>
        "#;
        let error = validate_svg(svg).unwrap_err();
        assert!(matches!(error, SvgError::Unsupported("external image references")));
    }
}
