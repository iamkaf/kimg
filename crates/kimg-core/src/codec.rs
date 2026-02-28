use crate::buffer::ImageBuffer;
use std::io::Cursor;

/// Errors that can occur during image encoding/decoding.
#[derive(Debug)]
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
}
