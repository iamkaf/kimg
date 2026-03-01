use crate::buffer::ImageBuffer;
use std::io::Cursor;

/// Errors that can occur during image encoding/decoding.
#[derive(Debug)]
#[non_exhaustive]
pub enum CodecError {
    /// The input data could not be decoded.
    DecodingError(String),
    /// Encoding failed.
    EncodingError(String),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::DecodingError(msg) => write!(f, "decoding error: {msg}"),
            CodecError::EncodingError(msg) => write!(f, "encoding error: {msg}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// Detected image format based on magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFormat {
    /// Portable Network Graphics (.png)
    Png,
    /// Joint Photographic Experts Group (.jpg, .jpeg)
    Jpeg,
    /// Web Picture format (.webp)
    WebP,
    /// Graphics Interchange Format (.gif)
    Gif,
    /// Adobe Photoshop Document (.psd)
    Psd,
    /// Unrecognized format
    Unknown,
}

impl ImageFormat {
    /// Returns a string representation of the format (e.g. "png", "jpeg").
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::WebP => "webp",
            Self::Gif => "gif",
            Self::Psd => "psd",
            Self::Unknown => "unknown",
        }
    }
}

/// Detect image format by inspecting magic bytes.
pub fn detect_format(data: &[u8]) -> ImageFormat {
    if data.len() >= 8 && data[0..4] == [0x89, 0x50, 0x4E, 0x47] {
        ImageFormat::Png
    } else if data.len() >= 3 && data[0..3] == [0xFF, 0xD8, 0xFF] {
        ImageFormat::Jpeg
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        ImageFormat::WebP
    } else if data.len() >= 4 && &data[0..4] == b"GIF8" {
        ImageFormat::Gif
    } else if data.len() >= 4 && &data[0..4] == b"8BPS" {
        ImageFormat::Psd
    } else {
        ImageFormat::Unknown
    }
}

/// Auto-detect format and decode to an RGBA ImageBuffer.
pub fn decode_auto(data: &[u8]) -> Result<ImageBuffer, CodecError> {
    match detect_format(data) {
        ImageFormat::Png => decode_png(data),
        ImageFormat::Jpeg => decode_jpeg(data),
        ImageFormat::WebP => decode_webp(data),
        ImageFormat::Gif => {
            let frames = decode_gif(data)?;
            if frames.is_empty() {
                Err(CodecError::DecodingError("GIF has no frames".into()))
            } else {
                Ok(frames.into_iter().next().unwrap().buffer)
            }
        }
        ImageFormat::Psd => {
            let (w, h, layers) = import_psd(data)?;
            if layers.is_empty() {
                // Return empty canvas
                Ok(ImageBuffer::new_transparent(w, h))
            } else {
                Ok(layers.into_iter().next().unwrap().buffer)
            }
        }
        ImageFormat::Unknown => Err(CodecError::DecodingError("unknown image format".into())),
    }
}

/// Decode a PNG from raw bytes into an RGBA8 ImageBuffer.
pub fn decode_png(data: &[u8]) -> Result<ImageBuffer, CodecError> {
    let decoder = png::Decoder::new(Cursor::new(data));
    let mut reader = decoder
        .read_info()
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;

    let mut raw = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut raw)
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;
    raw.truncate(info.buffer_size());

    let width = info.width;
    let height = info.height;

    let rgba = match info.color_type {
        png::ColorType::Rgba => raw,
        png::ColorType::Rgb => {
            let pixel_count = (width as usize) * (height as usize);
            let mut out = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                out[i * 4] = raw[i * 3];
                out[i * 4 + 1] = raw[i * 3 + 1];
                out[i * 4 + 2] = raw[i * 3 + 2];
                out[i * 4 + 3] = 255;
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let pixel_count = (width as usize) * (height as usize);
            let mut out = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                let g = raw[i * 2];
                let a = raw[i * 2 + 1];
                out[i * 4] = g;
                out[i * 4 + 1] = g;
                out[i * 4 + 2] = g;
                out[i * 4 + 3] = a;
            }
            out
        }
        png::ColorType::Grayscale => {
            let pixel_count = (width as usize) * (height as usize);
            let mut out = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                let g = raw[i];
                out[i * 4] = g;
                out[i * 4 + 1] = g;
                out[i * 4 + 2] = g;
                out[i * 4 + 3] = 255;
            }
            out
        }
        other => {
            return Err(CodecError::DecodingError(format!(
                "unsupported color type: {other:?}"
            )));
        }
    };

    ImageBuffer::from_rgba(width, height, rgba)
        .ok_or_else(|| CodecError::DecodingError("buffer size mismatch".into()))
}

/// Encode an ImageBuffer as a PNG.
pub fn encode_png(buf: &ImageBuffer) -> Result<Vec<u8>, CodecError> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, buf.width, buf.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| CodecError::EncodingError(e.to_string()))?;
        writer
            .write_image_data(&buf.data)
            .map_err(|e| CodecError::EncodingError(e.to_string()))?;
    }
    Ok(output)
}

// ── JPEG ──

/// Decode a JPEG from raw bytes into an RGBA8 ImageBuffer.
pub fn decode_jpeg(data: &[u8]) -> Result<ImageBuffer, CodecError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    let pixels = decoder
        .decode()
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;
    let info = decoder
        .info()
        .ok_or_else(|| CodecError::DecodingError("missing JPEG info".into()))?;
    let width = info.width as u32;
    let height = info.height as u32;
    let pixel_count = (width as usize) * (height as usize);

    let rgba = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            let mut out = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                out[i * 4] = pixels[i * 3];
                out[i * 4 + 1] = pixels[i * 3 + 1];
                out[i * 4 + 2] = pixels[i * 3 + 2];
                out[i * 4 + 3] = 255;
            }
            out
        }
        jpeg_decoder::PixelFormat::L8 => {
            let mut out = vec![0u8; pixel_count * 4];
            for i in 0..pixel_count {
                let g = pixels[i];
                out[i * 4] = g;
                out[i * 4 + 1] = g;
                out[i * 4 + 2] = g;
                out[i * 4 + 3] = 255;
            }
            out
        }
        other => {
            return Err(CodecError::DecodingError(format!(
                "unsupported JPEG pixel format: {other:?}"
            )));
        }
    };

    ImageBuffer::from_rgba(width, height, rgba)
        .ok_or_else(|| CodecError::DecodingError("buffer size mismatch".into()))
}

/// Encode an ImageBuffer as JPEG with the given quality (1-100).
pub fn encode_jpeg(buf: &ImageBuffer, quality: u8) -> Result<Vec<u8>, CodecError> {
    let pixel_count = (buf.width as usize) * (buf.height as usize);
    let mut rgb = vec![0u8; pixel_count * 3];
    for i in 0..pixel_count {
        rgb[i * 3] = buf.data[i * 4];
        rgb[i * 3 + 1] = buf.data[i * 4 + 1];
        rgb[i * 3 + 2] = buf.data[i * 4 + 2];
    }

    let mut output = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut output, quality);
    encoder
        .encode(
            &rgb,
            buf.width as u16,
            buf.height as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .map_err(|e| CodecError::EncodingError(e.to_string()))?;
    Ok(output)
}

// ── WebP ──

/// Decode a WebP image from raw bytes into an RGBA8 ImageBuffer.
pub fn decode_webp(data: &[u8]) -> Result<ImageBuffer, CodecError> {
    use image_webp::WebPDecoder;

    let mut decoder = WebPDecoder::new(Cursor::new(data))
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;
    let (width, height) = decoder.dimensions();
    let has_alpha = decoder.has_alpha();
    let pixel_count = (width as usize) * (height as usize);

    if has_alpha {
        let mut rgba = vec![0u8; pixel_count * 4];
        decoder
            .read_image(&mut rgba)
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;
        ImageBuffer::from_rgba(width, height, rgba)
            .ok_or_else(|| CodecError::DecodingError("buffer size mismatch".into()))
    } else {
        let mut rgb = vec![0u8; pixel_count * 3];
        decoder
            .read_image(&mut rgb)
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;
        let mut rgba = vec![0u8; pixel_count * 4];
        for i in 0..pixel_count {
            rgba[i * 4] = rgb[i * 3];
            rgba[i * 4 + 1] = rgb[i * 3 + 1];
            rgba[i * 4 + 2] = rgb[i * 3 + 2];
            rgba[i * 4 + 3] = 255;
        }
        ImageBuffer::from_rgba(width, height, rgba)
            .ok_or_else(|| CodecError::DecodingError("buffer size mismatch".into()))
    }
}

/// Encode an ImageBuffer as lossless WebP.
pub fn encode_webp(buf: &ImageBuffer) -> Result<Vec<u8>, CodecError> {
    use image_webp::WebPEncoder;

    let mut output = Vec::new();
    let encoder = WebPEncoder::new(&mut output);
    encoder
        .encode(
            &buf.data,
            buf.width,
            buf.height,
            image_webp::ColorType::Rgba8,
        )
        .map_err(|e| CodecError::EncodingError(e.to_string()))?;
    Ok(output)
}

// ── GIF ──

/// An individual animation frame.
#[derive(Debug, Clone)]
pub struct GifFrame {
    /// The image buffer for the frame.
    pub buffer: ImageBuffer,
    /// Delay before the next frame in milliseconds.
    pub delay_ms: u32,
}

/// Decode a GIF into individual frames. Each frame is composited onto a canvas
/// respecting disposal methods for correct animation.
pub fn decode_gif(data: &[u8]) -> Result<Vec<GifFrame>, CodecError> {
    use gif::{ColorOutput, DecodeOptions, DisposalMethod};

    let mut opts = DecodeOptions::new();
    opts.set_color_output(ColorOutput::RGBA);
    let mut decoder = opts
        .read_info(Cursor::new(data))
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;

    let canvas_w = decoder.width() as u32;
    let canvas_h = decoder.height() as u32;
    let mut canvas = ImageBuffer::new_transparent(canvas_w, canvas_h);
    let mut frames = Vec::new();

    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|e| CodecError::DecodingError(e.to_string()))?
    {
        let fw = frame.width as u32;
        let fh = frame.height as u32;
        let fx = frame.left as u32;
        let fy = frame.top as u32;
        let delay_ms = frame.delay as u32 * 10; // centiseconds → ms
        let dispose = frame.dispose;

        // Save canvas state before drawing for RestoreToPrevious
        let prev_canvas = canvas.clone();

        // Composite frame pixels onto canvas
        let frame_pixels = &frame.buffer;
        for row in 0..fh {
            for col in 0..fw {
                let cx = fx + col;
                let cy = fy + row;
                if cx < canvas_w && cy < canvas_h {
                    let src_i = ((row * fw + col) * 4) as usize;
                    let a = frame_pixels[src_i + 3];
                    if a > 0 {
                        let dst_i = ((cy * canvas_w + cx) * 4) as usize;
                        canvas.data[dst_i] = frame_pixels[src_i];
                        canvas.data[dst_i + 1] = frame_pixels[src_i + 1];
                        canvas.data[dst_i + 2] = frame_pixels[src_i + 2];
                        canvas.data[dst_i + 3] = frame_pixels[src_i + 3];
                    }
                }
            }
        }

        frames.push(GifFrame {
            buffer: canvas.clone(),
            delay_ms,
        });

        // Apply disposal method
        match dispose {
            DisposalMethod::Background => {
                // Clear the frame area to transparent
                for row in 0..fh {
                    for col in 0..fw {
                        let cx = fx + col;
                        let cy = fy + row;
                        if cx < canvas_w && cy < canvas_h {
                            let i = ((cy * canvas_w + cx) * 4) as usize;
                            canvas.data[i] = 0;
                            canvas.data[i + 1] = 0;
                            canvas.data[i + 2] = 0;
                            canvas.data[i + 3] = 0;
                        }
                    }
                }
            }
            DisposalMethod::Previous => {
                canvas = prev_canvas;
            }
            _ => {
                // Keep / Any — leave canvas as-is
            }
        }
    }

    Ok(frames)
}

// ── PSD ──

/// Parsed layer from PSD.
pub struct PsdLayer {
    /// Layer name extracted from the PSD.
    pub name: String,
    /// Pixel data for the layer.
    pub buffer: ImageBuffer,
    /// Left bound of the layer.
    pub x: i32,
    /// Top bound of the layer.
    pub y: i32,
    /// Global opacity of the layer [0.0, 1.0].
    pub opacity: f64,
    /// Whether the layer is visible.
    pub visible: bool,
}

/// Import layers from a PSD file. Returns (canvas_width, canvas_height, layers).
pub fn import_psd(data: &[u8]) -> Result<(u32, u32, Vec<PsdLayer>), CodecError> {
    let psd = psd::Psd::from_bytes(data).map_err(|e| CodecError::DecodingError(e.to_string()))?;
    let canvas_w = psd.width();
    let canvas_h = psd.height();

    let mut layers = Vec::new();
    for layer in psd.layers() {
        let w = layer.width() as u32;
        let h = layer.height() as u32;
        if w == 0 || h == 0 {
            continue;
        }
        let rgba = layer.rgba();
        let expected = (w as usize) * (h as usize) * 4;
        if rgba.len() != expected {
            continue;
        }
        let buffer = match ImageBuffer::from_rgba(w, h, rgba) {
            Some(b) => b,
            None => continue,
        };
        layers.push(PsdLayer {
            name: layer.name().to_string(),
            buffer,
            x: layer.layer_left(),
            y: layer.layer_top(),
            opacity: layer.opacity() as f64 / 255.0,
            visible: layer.visible(),
        });
    }

    Ok((canvas_w, canvas_h, layers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba;

    #[test]
    fn png_roundtrip() {
        let mut buf = ImageBuffer::new_transparent(4, 4);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        buf.set_pixel(1, 1, Rgba::new(0, 255, 0, 128));
        buf.set_pixel(3, 3, Rgba::new(0, 0, 255, 255));

        let encoded = encode_png(&buf).expect("encode failed");
        let decoded = decode_png(&encoded).expect("decode failed");

        assert_eq!(decoded.width, buf.width);
        assert_eq!(decoded.height, buf.height);
        assert_eq!(decoded.data, buf.data);
    }

    #[test]
    fn decode_invalid_data() {
        assert!(decode_png(&[0, 1, 2, 3]).is_err());
    }

    // ── Format detection ──

    #[test]
    fn detect_format_png() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_format(&data), ImageFormat::Png);
    }

    #[test]
    fn detect_format_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_format(&data), ImageFormat::Jpeg);
    }

    #[test]
    fn detect_format_webp() {
        let data = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(detect_format(data), ImageFormat::WebP);
    }

    #[test]
    fn detect_format_gif() {
        let data = b"GIF89a";
        assert_eq!(detect_format(data), ImageFormat::Gif);
    }

    #[test]
    fn detect_format_psd() {
        let data = b"8BPS\x00\x01";
        assert_eq!(detect_format(data), ImageFormat::Psd);
    }

    #[test]
    fn detect_format_unknown() {
        assert_eq!(detect_format(&[0, 0, 0, 0]), ImageFormat::Unknown);
    }

    // ── JPEG ──

    #[test]
    fn jpeg_roundtrip() {
        let mut buf = ImageBuffer::new_transparent(4, 4);
        buf.fill(Rgba::new(255, 0, 0, 255));

        let encoded = encode_jpeg(&buf, 90).expect("encode failed");
        let decoded = decode_jpeg(&encoded).expect("decode failed");

        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        // JPEG is lossy, so check approximate values
        for i in (0..decoded.data.len()).step_by(4) {
            assert!(decoded.data[i] > 200, "r={}", decoded.data[i]); // red channel high
            assert!(decoded.data[i + 3] == 255); // alpha always 255 for JPEG
        }
    }

    // ── WebP ──

    #[test]
    fn webp_lossless_roundtrip() {
        let mut buf = ImageBuffer::new_transparent(4, 4);
        buf.set_pixel(0, 0, Rgba::new(255, 0, 0, 255));
        buf.set_pixel(1, 1, Rgba::new(0, 255, 0, 128));
        buf.set_pixel(3, 3, Rgba::new(0, 0, 255, 255));

        let encoded = encode_webp(&buf).expect("encode failed");
        let decoded = decode_webp(&encoded).expect("decode failed");

        assert_eq!(decoded.width, buf.width);
        assert_eq!(decoded.height, buf.height);
        assert_eq!(decoded.data, buf.data);
    }

    // ── GIF ──

    #[test]
    fn gif_decode_single_frame() {
        // Minimal valid GIF89a: 1x1 red pixel
        let gif_data: Vec<u8> = {
            let mut buf = Vec::new();
            use gif::Encoder;
            let mut encoder = Encoder::new(&mut buf, 1, 1, &[]).unwrap();
            use gif::Frame;
            let mut frame = Frame {
                width: 1,
                height: 1,
                buffer: std::borrow::Cow::Owned(vec![0]),
                ..Frame::default()
            };
            let palette = vec![255, 0, 0]; // red
            frame.palette = Some(palette);
            encoder.write_frame(&frame).unwrap();
            drop(encoder);
            buf
        };

        let frames = decode_gif(&gif_data).expect("decode failed");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].buffer.width, 1);
        assert_eq!(frames[0].buffer.height, 1);
        // Check that the pixel is red
        let p = frames[0].buffer.get_pixel(0, 0);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 0);
        assert_eq!(p.b, 0);
        assert_eq!(p.a, 255);
    }

    // ── Auto-detect ──

    #[test]
    fn decode_auto_delegates_png() {
        let buf = ImageBuffer::new_transparent(2, 2);
        let png_bytes = encode_png(&buf).unwrap();
        let decoded = decode_auto(&png_bytes).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    #[test]
    fn decode_auto_delegates_jpeg() {
        let mut buf = ImageBuffer::new_transparent(2, 2);
        buf.fill(Rgba::new(128, 128, 128, 255));
        let jpeg_bytes = encode_jpeg(&buf, 80).unwrap();
        let decoded = decode_auto(&jpeg_bytes).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
    }

    #[test]
    fn decode_auto_unknown_format() {
        assert!(decode_auto(&[0, 0, 0, 0]).is_err());
    }
}
