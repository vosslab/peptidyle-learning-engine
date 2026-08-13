//! One authoritative hostile-input boundary for instructional raster images.
//!
//! An image's name and declared media type are untrusted.  This module accepts
//! only complete, single-container PNG, JPEG, and WebP files; rejects animation
//! and container trailing data; measures dimensions before allocating; and
//! performs a bounded full decode before callers create an immutable object.

use std::io::Cursor;

use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, Limits};

/// Maximum accepted original still-image byte length at every ingest path.
pub const MAX_STILL_IMAGE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum decoded pixels at every ingest path.
pub const MAX_STILL_IMAGE_DECODED_PIXELS: u64 = 20_000_000;

/// Exact media type established from the decoded bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StillImageMediaType {
    Jpeg,
    Png,
    WebP,
}

impl StillImageMediaType {
    #[must_use]
    pub const fn canonical_media_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
        }
    }
}

/// Facts measured from a fully decoded, still raster image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedStillImage {
    pub media_type: StillImageMediaType,
    /// Dimensions after EXIF orientation is applied.
    pub width: u32,
    pub height: u32,
}

/// Refusal reason safe for browser recovery guidance and import diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StillImageError {
    ByteLimit,
    UnsupportedMediaType,
    Animated,
    ZeroDimensions,
    DecodedPixelLimit,
    /// A valid container was followed by unowned bytes, so it could be a
    /// polyglot or an ambiguity between parsers.
    Polyglot,
    Malformed,
}

impl StillImageError {
    #[must_use]
    pub const fn user_message(self) -> &'static str {
        match self {
            Self::ByteLimit => "upload an image no larger than 8 MiB",
            Self::UnsupportedMediaType => "upload a PNG, JPEG, or WebP image",
            Self::Animated => "upload a still image rather than an animation",
            Self::ZeroDimensions => "upload an image with a non-zero width and height",
            Self::DecodedPixelLimit => "upload an image with at most 20 million pixels",
            Self::Polyglot | Self::Malformed => "upload a complete, readable image file",
        }
    }

    /// Stable answer-free wording for an import report.
    #[must_use]
    pub const fn import_detail(self) -> &'static str {
        match self {
            Self::ByteLimit => "image exceeds the 8 MiB per-image limit",
            Self::UnsupportedMediaType => "only still PNG, JPEG, and WebP images are allowed",
            Self::Animated => "animated images are not allowed",
            Self::ZeroDimensions => "image dimensions must be non-zero",
            Self::DecodedPixelLimit => "image exceeds the 20 million decoded-pixel limit",
            Self::Polyglot => "image has bytes after its declared container",
            Self::Malformed => "image is incomplete or malformed",
        }
    }
}

/// Validates one original raster image without rewriting its immutable bytes.
pub fn verify_still_image(bytes: &[u8]) -> Result<VerifiedStillImage, StillImageError> {
    if bytes.len() > MAX_STILL_IMAGE_BYTES {
        return Err(StillImageError::ByteLimit);
    }
    match image::guess_format(bytes).map_err(|_| StillImageError::UnsupportedMediaType)? {
        ImageFormat::Jpeg => {
            validate_jpeg_container(bytes)?;
            verify_decoder(
                StillImageMediaType::Jpeg,
                JpegDecoder::new(Cursor::new(bytes)).map_err(malformed)?,
            )
        }
        ImageFormat::Png => {
            validate_png_container(bytes)?;
            validate_dimensions(png_header_dimensions(bytes)?)?;
            let decoder = PngDecoder::new(Cursor::new(bytes)).map_err(malformed)?;
            if decoder.is_apng().map_err(malformed)? {
                return Err(StillImageError::Animated);
            }
            verify_decoder(StillImageMediaType::Png, decoder)
        }
        ImageFormat::WebP => {
            validate_webp_container(bytes)?;
            let decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(malformed)?;
            if decoder.has_animation() {
                return Err(StillImageError::Animated);
            }
            verify_decoder(StillImageMediaType::WebP, decoder)
        }
        _ => Err(StillImageError::UnsupportedMediaType),
    }
}

fn verify_decoder<D>(
    media_type: StillImageMediaType,
    mut decoder: D,
) -> Result<VerifiedStillImage, StillImageError>
where
    D: ImageDecoder,
{
    let (width, height) = decoder.dimensions();
    validate_dimensions((width, height))?;
    let orientation = decoder.orientation().map_err(malformed)?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(
        MAX_STILL_IMAGE_DECODED_PIXELS
            .checked_mul(8)
            .ok_or(StillImageError::DecodedPixelLimit)?,
    );
    decoder.set_limits(limits).map_err(malformed)?;
    let _decoded = DynamicImage::from_decoder(decoder).map_err(malformed)?;
    let (width, height) = oriented_dimensions(width, height, orientation);
    Ok(VerifiedStillImage {
        media_type,
        width,
        height,
    })
}

fn validate_jpeg_container(bytes: &[u8]) -> Result<(), StillImageError> {
    if bytes.get(..2) != Some([0xff, 0xd8].as_slice()) {
        return Err(StillImageError::Malformed);
    }

    // JPEG decoders commonly accept data after EOI.  That is useful for media
    // tooling, but not an acceptable upload boundary: the same object could
    // then be a ZIP, HTML, or another parser's input.  Walk JPEG marker syntax
    // ourselves so the one EOI that terminates the codestream is also the last
    // byte of the object.  The decoder below remains responsible for image
    // semantics and a bounded full decode.
    let mut offset = 2_usize;
    let mut pending_marker = None;
    loop {
        let marker = match pending_marker.take() {
            Some(marker) => marker,
            None => jpeg_marker(bytes, &mut offset)?,
        };
        match marker {
            0xd9 => {
                return if offset == bytes.len() {
                    Ok(())
                } else {
                    Err(StillImageError::Polyglot)
                };
            }
            // These markers have no length field.  SOI may occur only at the
            // beginning, and restart markers belong inside entropy-coded scan
            // data, where `jpeg_scan_marker` handles them.
            0xd8 | 0xd0..=0xd7 | 0x00 => return Err(StillImageError::Malformed),
            // TEM has no parameter bytes and is the one other standalone JPEG
            // marker permitted between segments.
            0x01 => {}
            marker if jpeg_marker_has_segment(marker) => {
                let segment_end = jpeg_segment_end(bytes, offset)?;
                if marker == 0xda {
                    offset = segment_end;
                    pending_marker = Some(jpeg_scan_marker(bytes, &mut offset)?);
                } else {
                    offset = segment_end;
                }
            }
            _ => return Err(StillImageError::Malformed),
        }
    }
}

fn jpeg_marker(bytes: &[u8], offset: &mut usize) -> Result<u8, StillImageError> {
    if bytes.get(*offset) != Some(&0xff) {
        return Err(StillImageError::Malformed);
    }
    while bytes.get(*offset) == Some(&0xff) {
        *offset = offset.checked_add(1).ok_or(StillImageError::Malformed)?;
    }
    let marker = *bytes.get(*offset).ok_or(StillImageError::Malformed)?;
    *offset = offset.checked_add(1).ok_or(StillImageError::Malformed)?;
    if marker == 0x00 {
        return Err(StillImageError::Malformed);
    }
    Ok(marker)
}

fn jpeg_marker_has_segment(marker: u8) -> bool {
    matches!(marker, 0xc0..=0xfe) && !matches!(marker, 0xd0..=0xd9)
}

fn jpeg_segment_end(bytes: &[u8], offset: usize) -> Result<usize, StillImageError> {
    let length = bytes
        .get(offset..offset.saturating_add(2))
        .map(|length| u16::from_be_bytes([length[0], length[1]]))
        .ok_or(StillImageError::Malformed)?;
    if length < 2 {
        return Err(StillImageError::Malformed);
    }
    offset
        .checked_add(usize::from(length))
        .filter(|end| *end <= bytes.len())
        .ok_or(StillImageError::Malformed)
}

/// Returns the first marker that ends an entropy-coded scan.  Byte-stuffed
/// `FF 00`, restart markers, and marker fill bytes remain owned by the scan.
fn jpeg_scan_marker(bytes: &[u8], offset: &mut usize) -> Result<u8, StillImageError> {
    loop {
        let byte = *bytes.get(*offset).ok_or(StillImageError::Malformed)?;
        *offset = offset.checked_add(1).ok_or(StillImageError::Malformed)?;
        if byte != 0xff {
            continue;
        }

        while bytes.get(*offset) == Some(&0xff) {
            *offset = offset.checked_add(1).ok_or(StillImageError::Malformed)?;
        }
        let marker = *bytes.get(*offset).ok_or(StillImageError::Malformed)?;
        *offset = offset.checked_add(1).ok_or(StillImageError::Malformed)?;
        match marker {
            0x00 | 0xd0..=0xd7 => {}
            marker => return Ok(marker),
        }
    }
}

fn validate_webp_container(bytes: &[u8]) -> Result<(), StillImageError> {
    let Some(size) = bytes.get(4..8) else {
        return Err(StillImageError::Malformed);
    };
    let declared = usize::try_from(u32::from_le_bytes([size[0], size[1], size[2], size[3]]))
        .map_err(|_| StillImageError::Malformed)?;
    let expected = declared.checked_add(8).ok_or(StillImageError::Malformed)?;
    if expected < bytes.len() {
        return Err(StillImageError::Polyglot);
    }
    if expected != bytes.len() {
        return Err(StillImageError::Malformed);
    }
    Ok(())
}

fn validate_png_container(bytes: &[u8]) -> Result<(), StillImageError> {
    let mut offset = 8_usize;
    loop {
        let Some(header) = bytes.get(offset..offset.saturating_add(8)) else {
            return Err(StillImageError::Malformed);
        };
        let length = usize::try_from(u32::from_be_bytes([
            header[0], header[1], header[2], header[3],
        ]))
        .map_err(|_| StillImageError::Malformed)?;
        let chunk_end = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
            .ok_or(StillImageError::Malformed)?;
        if chunk_end > bytes.len() {
            return Err(StillImageError::Malformed);
        }
        if &header[4..8] == b"IEND" {
            if length != 0 {
                return Err(StillImageError::Malformed);
            }
            return if chunk_end == bytes.len() {
                Ok(())
            } else {
                Err(StillImageError::Polyglot)
            };
        }
        offset = chunk_end;
    }
}

fn png_header_dimensions(bytes: &[u8]) -> Result<(u32, u32), StillImageError> {
    if bytes.get(12..16) != Some(b"IHDR".as_slice()) {
        return Err(StillImageError::Malformed);
    }
    let dimensions = bytes.get(16..24).ok_or(StillImageError::Malformed)?;
    Ok((
        u32::from_be_bytes([dimensions[0], dimensions[1], dimensions[2], dimensions[3]]),
        u32::from_be_bytes([dimensions[4], dimensions[5], dimensions[6], dimensions[7]]),
    ))
}

fn validate_dimensions((width, height): (u32, u32)) -> Result<(), StillImageError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(StillImageError::DecodedPixelLimit)?;
    if width == 0 || height == 0 {
        return Err(StillImageError::ZeroDimensions);
    }
    if pixels > MAX_STILL_IMAGE_DECODED_PIXELS {
        return Err(StillImageError::DecodedPixelLimit);
    }
    Ok(())
}

const fn oriented_dimensions(width: u32, height: u32, orientation: Orientation) -> (u32, u32) {
    match orientation {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (height, width),
        Orientation::NoTransforms
        | Orientation::Rotate180
        | Orientation::FlipHorizontal
        | Orientation::FlipVertical => (width, height),
    }
}

fn malformed(_error: image::ImageError) -> StillImageError {
    StillImageError::Malformed
}

#[cfg(test)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::PngEncoder;
    use image::codecs::webp::WebPEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};

    use super::*;

    fn rgb() -> RgbImage {
        RgbImage::from_pixel(3, 2, Rgb([12, 34, 56]))
    }

    fn png() -> Vec<u8> {
        let image = rgb();
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("PNG fixture encodes");
        bytes
    }

    fn jpeg() -> Vec<u8> {
        let image = rgb();
        let mut bytes = Vec::new();
        JpegEncoder::new_with_quality(&mut bytes, 90)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("JPEG fixture encodes");
        bytes
    }

    fn webp() -> Vec<u8> {
        let image = rgb();
        let mut bytes = Vec::new();
        WebPEncoder::new_lossless(&mut bytes)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("WebP fixture encodes");
        bytes
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn apng() -> Vec<u8> {
        let source = png();
        let data = [0, 0, 0, 2, 0, 0, 0, 0];
        let mut crc_input = b"acTL".to_vec();
        crc_input.extend_from_slice(&data);
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&8_u32.to_be_bytes());
        chunk.extend_from_slice(b"acTL");
        chunk.extend_from_slice(&data);
        chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());
        let mut output = source[..33].to_vec();
        output.extend_from_slice(&chunk);
        output.extend_from_slice(&source[33..]);
        output
    }

    fn png_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = png();
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        let mut ihdr = b"IHDR".to_vec();
        ihdr.extend_from_slice(&bytes[16..29]);
        bytes[29..33].copy_from_slice(&crc32(&ihdr).to_be_bytes());
        bytes
    }

    #[test]
    fn fully_decodes_each_supported_still_format() {
        for (bytes, media_type) in [
            (png(), StillImageMediaType::Png),
            (jpeg(), StillImageMediaType::Jpeg),
            (webp(), StillImageMediaType::WebP),
        ] {
            let verified = verify_still_image(&bytes).expect("valid still raster");
            assert_eq!(verified.media_type, media_type);
            assert_eq!((verified.width, verified.height), (3, 2));
        }
    }

    #[test]
    fn rejects_truncated_disguised_animated_and_polyglot_inputs() {
        assert_eq!(
            verify_still_image(b"GIF89a"),
            Err(StillImageError::UnsupportedMediaType)
        );
        assert_eq!(
            verify_still_image(&[0x89, b'P', b'N', b'G', 13, 10, 26, 10]),
            Err(StillImageError::Malformed)
        );
        assert_eq!(verify_still_image(&apng()), Err(StillImageError::Animated));
        let mut polyglot = png();
        polyglot.extend_from_slice(b"PK\\x03\\x04not-an-image");
        assert_eq!(
            verify_still_image(&polyglot),
            Err(StillImageError::Polyglot)
        );
    }

    #[test]
    fn rejects_jpeg_data_after_its_terminal_marker() {
        let mut zip_polyglot = jpeg();
        zip_polyglot.extend_from_slice(b"PK\x03\x04not-an-image\xff\xd9");
        assert_eq!(
            verify_still_image(&zip_polyglot),
            Err(StillImageError::Polyglot)
        );

        let mut html_polyglot = jpeg();
        html_polyglot.extend_from_slice(b"<script>alert(1)</script>");
        assert_eq!(
            verify_still_image(&html_polyglot),
            Err(StillImageError::Polyglot)
        );
    }

    #[test]
    fn accepts_marker_fill_before_terminal_jpeg_eoi() {
        let mut bytes = jpeg();
        let eoi = bytes.len() - 1;
        bytes.insert(eoi, 0xff);
        let verified = verify_still_image(&bytes).expect("marker fill remains part of JPEG syntax");
        assert_eq!(verified.media_type, StillImageMediaType::Jpeg);
    }

    #[test]
    fn rejects_jpeg_marker_segments_with_invalid_lengths() {
        let bytes = [0xff, 0xd8, 0xff, 0xe0, 0, 1, 0xff, 0xd9];
        assert_eq!(verify_still_image(&bytes), Err(StillImageError::Malformed));
    }

    #[test]
    fn rejects_bombs_before_full_decode_and_enforces_byte_limit() {
        assert_eq!(
            verify_still_image(&png_with_dimensions(5_000, 5_000)),
            Err(StillImageError::DecodedPixelLimit)
        );
        assert_eq!(
            verify_still_image(&vec![0_u8; MAX_STILL_IMAGE_BYTES + 1]),
            Err(StillImageError::ByteLimit)
        );
    }
}
