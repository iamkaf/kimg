//! Image encoding and decoding for multiple formats.
//!
//! Supports PNG, JPEG, WebP, GIF, and PSD (Adobe Photoshop).
//!
//! All decoders produce an RGBA8 [`ImageBuffer`].  All encoders accept one.
//!
//! | Function | Direction | Format |
//! |----------|-----------|--------|
//! | [`decode_png`] / [`encode_png`] | both | PNG (lossless) |
//! | [`decode_jpeg`] / [`encode_jpeg`] | both | JPEG (lossy) |
//! | [`decode_webp`] / [`encode_webp`] | both | WebP (lossless) |
//! | [`decode_gif`] | decode only | Animated GIF → frames |
//! | [`import_psd`] | decode only | PSD → per-layer buffers |
//! | [`decode_auto`] | decode only | Detect format from magic bytes |
//! | [`detect_format`] | — | Inspect magic bytes → [`ImageFormat`] |
//!
//! JPEG encoding strips the alpha channel (JPEG has no transparency support).

use crate::buffer::ImageBuffer;
use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

// Cap decoder-driven allocations for untrusted input. Larger images are not
// practical for the current WASM/JS targets and turn malformed headers into
// allocator aborts instead of ordinary decode errors.
const MAX_DECODED_IMAGE_BYTES: usize = 512 * 1024 * 1024;
const PSD_FILE_HEADER_LEN: usize = 26;

#[derive(Clone, Copy)]
struct PsdHeaderInfo {
    channel_count: usize,
    height: usize,
    width: usize,
}

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

fn checked_allocation_len(len: usize, what: &str) -> Result<usize, CodecError> {
    if len > MAX_DECODED_IMAGE_BYTES {
        return Err(CodecError::DecodingError(format!(
            "{what} is too large ({len} bytes > {MAX_DECODED_IMAGE_BYTES} byte limit)"
        )));
    }

    Ok(len)
}

fn checked_rgba_geometry(
    width: u32,
    height: u32,
    what: &str,
) -> Result<(usize, usize), CodecError> {
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| CodecError::DecodingError(format!("{what} dimensions overflow")))?;
    let rgba_len = pixel_count
        .checked_mul(4)
        .ok_or_else(|| CodecError::DecodingError(format!("{what} RGBA buffer size overflow")))?;

    checked_allocation_len(rgba_len, what)?;
    Ok((pixel_count, rgba_len))
}

fn checked_section_end(
    start: usize,
    len: usize,
    total_len: usize,
    what: &str,
) -> Result<usize, CodecError> {
    let end = start
        .checked_add(len)
        .ok_or_else(|| CodecError::DecodingError(format!("{what} length overflow")))?;
    if end > total_len {
        return Err(CodecError::DecodingError(format!(
            "{what} exceeds input size"
        )));
    }

    Ok(end)
}

fn read_psd_u16(data: &[u8], offset: usize, what: &str) -> Result<u16, CodecError> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| CodecError::DecodingError(format!("missing PSD {what}")))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_psd_u32(data: &[u8], offset: usize, what: &str) -> Result<u32, CodecError> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| CodecError::DecodingError(format!("missing PSD {what}")))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn parse_psd_header(data: &[u8]) -> Result<PsdHeaderInfo, CodecError> {
    if data.len() < PSD_FILE_HEADER_LEN {
        return Err(CodecError::DecodingError("PSD data is too short".into()));
    }
    if &data[..4] != b"8BPS" {
        return Err(CodecError::DecodingError("invalid PSD signature".into()));
    }
    if read_psd_u16(data, 4, "version")? != 1 {
        return Err(CodecError::DecodingError("unsupported PSD version".into()));
    }
    if data[6..12].iter().any(|&byte| byte != 0) {
        return Err(CodecError::DecodingError(
            "invalid PSD reserved header bytes".into(),
        ));
    }

    let channel_count = read_psd_u16(data, 12, "channel count")? as usize;
    if !(1..=56).contains(&channel_count) {
        return Err(CodecError::DecodingError(
            "PSD channel count out of range".into(),
        ));
    }

    let height = read_psd_u32(data, 14, "height")? as usize;
    let width = read_psd_u32(data, 18, "width")? as usize;
    if height == 0 || width == 0 {
        return Err(CodecError::DecodingError(
            "PSD dimensions must be non-zero".into(),
        ));
    }

    let depth = read_psd_u16(data, 22, "depth")?;
    if depth != 8 && depth != 16 {
        return Err(CodecError::DecodingError(format!(
            "unsupported PSD depth: {depth}"
        )));
    }

    Ok(PsdHeaderInfo {
        channel_count,
        height,
        width,
    })
}

fn validate_psd_image_data(bytes: &[u8], header: PsdHeaderInfo) -> Result<(), CodecError> {
    let compression = read_psd_u16(bytes, 0, "image data compression")?;

    match compression {
        0 => Ok(()),
        1 => {
            let row_count = header
                .channel_count
                .checked_mul(header.height)
                .ok_or_else(|| CodecError::DecodingError("PSD RLE row count overflow".into()))?;
            let row_table_len = row_count.checked_mul(2).ok_or_else(|| {
                CodecError::DecodingError("PSD RLE row table length overflow".into())
            })?;
            let channel_data_start = 2usize.checked_add(row_table_len).ok_or_else(|| {
                CodecError::DecodingError("PSD RLE channel data offset overflow".into())
            })?;
            if bytes.len() < channel_data_start {
                return Err(CodecError::DecodingError(
                    "PSD RLE row table is truncated".into(),
                ));
            }

            let mut offset = 2usize;
            let mut compressed_len = 0usize;
            for row_index in 0..row_count {
                let row_len = read_psd_u16(bytes, offset, "RLE row length")? as usize;
                offset += 2;
                compressed_len = compressed_len.checked_add(row_len).ok_or_else(|| {
                    CodecError::DecodingError(format!(
                        "PSD RLE compressed length overflow at row {row_index}"
                    ))
                })?;
            }

            let required_len = channel_data_start
                .checked_add(compressed_len)
                .ok_or_else(|| CodecError::DecodingError("PSD RLE data size overflow".into()))?;
            if bytes.len() < required_len {
                return Err(CodecError::DecodingError(
                    "PSD RLE image data is truncated".into(),
                ));
            }

            Ok(())
        }
        2 | 3 => Err(CodecError::DecodingError(
            "unsupported PSD ZIP compression".into(),
        )),
        other => Err(CodecError::DecodingError(format!(
            "invalid PSD compression mode: {other}"
        ))),
    }
}

fn validate_psd_layout(data: &[u8]) -> Result<(), CodecError> {
    let header = parse_psd_header(data)?;
    let _ = header
        .width
        .checked_mul(header.height)
        .ok_or_else(|| CodecError::DecodingError("PSD dimensions overflow".into()))?;

    let mut offset = PSD_FILE_HEADER_LEN;
    for (index, label) in ["color mode data", "image resources", "layer and mask"]
        .into_iter()
        .enumerate()
    {
        let len_bytes = data
            .get(offset..offset + 4)
            .ok_or_else(|| CodecError::DecodingError(format!("missing PSD {label} length")))?;
        let section_len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        let start = offset;
        offset = checked_section_end(
            start + 4,
            section_len,
            data.len(),
            &format!("PSD section {} ({label})", index + 1),
        )?;
    }

    let image_data = &data[offset..];
    validate_psd_image_data(image_data, header)?;

    Ok(())
}

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

/// Detect the image format by inspecting the first few bytes (magic bytes).
///
/// Returns [`ImageFormat::Unknown`] if the header doesn't match any known format.
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

/// Detect the image format automatically and decode to an RGBA8 [`ImageBuffer`].
///
/// For animated GIFs only the first frame is returned.
/// For PSDs only the first layer is returned.
///
/// # Errors
///
/// Returns [`CodecError::DecodingError`] if the format is unknown or the data
/// is malformed.
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

    let header = reader.info();
    let raw_len = reader
        .output_line_size(header.width)
        .checked_mul(header.height as usize)
        .ok_or_else(|| CodecError::DecodingError("PNG decode buffer size overflow".into()))
        .and_then(|len| checked_allocation_len(len, "PNG decode buffer"))?;

    let mut raw = vec![0u8; raw_len];
    let info = reader
        .next_frame(&mut raw)
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;
    raw.truncate(info.buffer_size());

    let width = info.width;
    let height = info.height;

    let rgba = match info.color_type {
        png::ColorType::Rgba => raw,
        png::ColorType::Rgb => {
            let (pixel_count, rgba_len) = checked_rgba_geometry(width, height, "PNG image")?;
            let mut out = vec![0u8; rgba_len];
            for i in 0..pixel_count {
                out[i * 4] = raw[i * 3];
                out[i * 4 + 1] = raw[i * 3 + 1];
                out[i * 4 + 2] = raw[i * 3 + 2];
                out[i * 4 + 3] = 255;
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let (pixel_count, rgba_len) = checked_rgba_geometry(width, height, "PNG image")?;
            let mut out = vec![0u8; rgba_len];
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
            let (pixel_count, rgba_len) = checked_rgba_geometry(width, height, "PNG image")?;
            let mut out = vec![0u8; rgba_len];
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

/// Encode an [`ImageBuffer`] as a lossless PNG with full alpha.
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
    let (pixel_count, rgba_len) = checked_rgba_geometry(width, height, "JPEG image")?;

    let rgba = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            let mut out = vec![0u8; rgba_len];
            for i in 0..pixel_count {
                out[i * 4] = pixels[i * 3];
                out[i * 4 + 1] = pixels[i * 3 + 1];
                out[i * 4 + 2] = pixels[i * 3 + 2];
                out[i * 4 + 3] = 255;
            }
            out
        }
        jpeg_decoder::PixelFormat::L8 => {
            let mut out = vec![0u8; rgba_len];
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

/// Encode an [`ImageBuffer`] as JPEG with the given quality (1–100).
///
/// Alpha is stripped (JPEG has no transparency support).
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
    let (pixel_count, rgba_len) = checked_rgba_geometry(width, height, "WebP image")?;

    if has_alpha {
        let mut rgba = vec![0u8; rgba_len];
        decoder
            .read_image(&mut rgba)
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;
        ImageBuffer::from_rgba(width, height, rgba)
            .ok_or_else(|| CodecError::DecodingError("buffer size mismatch".into()))
    } else {
        let rgb_len = checked_allocation_len(
            pixel_count
                .checked_mul(3)
                .ok_or_else(|| CodecError::DecodingError("WebP RGB buffer size overflow".into()))?,
            "WebP decode buffer",
        )?;
        let mut rgb = vec![0u8; rgb_len];
        decoder
            .read_image(&mut rgb)
            .map_err(|e| CodecError::DecodingError(e.to_string()))?;
        let mut rgba = vec![0u8; rgba_len];
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

/// Encode an [`ImageBuffer`] as lossless WebP with full alpha.
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
#[non_exhaustive]
pub struct GifFrame {
    /// The image buffer for the frame.
    pub buffer: ImageBuffer,
    /// Delay before the next frame in milliseconds.
    pub delay_ms: u32,
}

/// Decode a GIF into individual animation frames.
///
/// Each frame is composited onto a running canvas using the GIF disposal method
/// (`Background`, `Previous`, or `Keep`), producing the correct image for that
/// point in the animation.
pub fn decode_gif(data: &[u8]) -> Result<Vec<GifFrame>, CodecError> {
    use gif::{ColorOutput, DecodeOptions, DisposalMethod};

    let mut opts = DecodeOptions::new();
    opts.set_color_output(ColorOutput::RGBA);
    let mut decoder = opts
        .read_info(Cursor::new(data))
        .map_err(|e| CodecError::DecodingError(e.to_string()))?;

    let canvas_w = decoder.width() as u32;
    let canvas_h = decoder.height() as u32;
    checked_rgba_geometry(canvas_w, canvas_h, "GIF canvas")?;
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
#[non_exhaustive]
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

/// Import layers from an Adobe Photoshop (PSD) file.
///
/// Returns `(canvas_width, canvas_height, layers)`.  Each [`PsdLayer`] contains
/// the layer's name, pixel buffer, position, opacity, and visibility.
/// Layers with zero width or height are skipped.
pub fn import_psd(data: &[u8]) -> Result<(u32, u32, Vec<PsdLayer>), CodecError> {
    validate_psd_layout(data)?;

    catch_unwind(AssertUnwindSafe(|| {
        let psd =
            psd::Psd::from_bytes(data).map_err(|e| CodecError::DecodingError(e.to_string()))?;
        let canvas_w = psd.width();
        let canvas_h = psd.height();

        let mut layers = Vec::new();
        for layer in psd.layers() {
            let w = layer.width() as u32;
            let h = layer.height() as u32;
            if w == 0 || h == 0 {
                continue;
            }
            checked_rgba_geometry(w, h, "PSD layer")?;
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
    }))
    .map_err(|_| CodecError::DecodingError("PSD parser panicked".into()))?
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

    #[test]
    fn decode_png_rejects_oversized_allocation() {
        let data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x27, 0x49, 0x48,
            0x44, 0x52, 0x10, 0x19, 0xE6, 0xE6, 0xE6, 0xE6, 0x2B, 0xE6, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x25, 0x00, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x51, 0x44, 0x48,
            0x49, 0x00, 0x00, 0x04, 0xA8, 0x00, 0x00, 0x44, 0x19, 0x09, 0x19, 0x19, 0x0C, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x49, 0x44, 0x41, 0x54, 0x00, 0x54, 0xFF,
        ];

        assert!(decode_png(&data).is_err());
    }

    #[test]
    fn import_psd_rejects_invalid_section_lengths() {
        let data = [
            0x38, 0x42, 0x50, 0x53, 0x60, 0x11, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4,
            0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4,
            0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4,
            0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4, 0xE4,
        ];

        assert!(import_psd(&data).is_err());
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
