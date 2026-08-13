//! Verification of the original instructional image used by native hotspots.
//!
//! This boundary intentionally does not normalize, transcode, or otherwise
//! rewrite upload bytes. Its descriptor records the browser-facing media type
//! and intrinsic dimensions that a later publication path can bind immutably.

#![allow(
    dead_code,
    reason = "the dependent hotspot-publication task has not yet attached this internal boundary"
)]

use std::io::Cursor;

use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, Limits};

pub(crate) const MAX_HOTSPOT_DECODED_PIXELS: u64 = 20_000_000;

/// Exact allowlisted media types determined from the uploaded bytes, never a
/// caller-provided `Content-Type` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotspotImageMediaType {
    Jpeg,
    Png,
    WebP,
}

impl HotspotImageMediaType {
    #[must_use]
    pub(crate) const fn canonical_media_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
        }
    }
}

/// Measured, post-orientation image facts safe to bind to a hotspot question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VerifiedHotspotImage {
    pub(crate) media_type: HotspotImageMediaType,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Upload failures that an HTTP boundary can map to precise recovery guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotspotImageError {
    UnsupportedMediaType,
    Animated,
    ZeroDimensions,
    DecodedPixelLimit,
    Malformed,
}

impl HotspotImageError {
    #[must_use]
    pub(crate) const fn user_message(self) -> &'static str {
        match self {
            Self::UnsupportedMediaType => "upload a PNG, JPEG, or WebP image",
            Self::Animated => "upload a still image rather than an animation",
            Self::ZeroDimensions => "upload an image with a non-zero width and height",
            Self::DecodedPixelLimit => "upload an image with at most 20 million pixels",
            Self::Malformed => "upload a complete, readable image file",
        }
    }
}

/// Sniffs, bounds, and fully decodes an original hotspot image without changing
/// its bytes. The returned dimensions reflect embedded orientation metadata.
pub(crate) fn verify_hotspot_image(
    bytes: &[u8],
) -> Result<VerifiedHotspotImage, HotspotImageError> {
    match image::guess_format(bytes).map_err(|_| HotspotImageError::UnsupportedMediaType)? {
        ImageFormat::Jpeg => verify_decoder(
            HotspotImageMediaType::Jpeg,
            JpegDecoder::new(Cursor::new(bytes)).map_err(malformed)?,
        ),
        ImageFormat::Png => {
            // PNG stores the intrinsic dimensions in its required first IHDR
            // chunk. Check them before asking the decoder to allocate or read
            // a complete payload; full decoding below still establishes that
            // this is a valid PNG rather than merely a PNG-shaped header.
            validate_dimensions(png_header_dimensions(bytes)?)?;
            let decoder = PngDecoder::new(Cursor::new(bytes)).map_err(malformed)?;
            if decoder.is_apng().map_err(malformed)? {
                return Err(HotspotImageError::Animated);
            }
            verify_decoder(HotspotImageMediaType::Png, decoder)
        }
        ImageFormat::WebP => {
            let decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(malformed)?;
            if decoder.has_animation() {
                return Err(HotspotImageError::Animated);
            }
            verify_decoder(HotspotImageMediaType::WebP, decoder)
        }
        _ => Err(HotspotImageError::UnsupportedMediaType),
    }
}

fn verify_decoder<D>(
    media_type: HotspotImageMediaType,
    mut decoder: D,
) -> Result<VerifiedHotspotImage, HotspotImageError>
where
    D: ImageDecoder,
{
    let (width, height) = decoder.dimensions();
    validate_dimensions((width, height))?;

    let orientation = decoder.orientation().map_err(malformed)?;
    let mut limits = Limits::default();
    // The enabled formats can expose up to 16-bit RGBA pixels. The allocation
    // limit remains below the decoder's allocation before the complete decode.
    limits.max_alloc = Some(
        MAX_HOTSPOT_DECODED_PIXELS
            .checked_mul(8)
            .ok_or(HotspotImageError::DecodedPixelLimit)?,
    );
    decoder.set_limits(limits).map_err(malformed)?;
    let _decoded = DynamicImage::from_decoder(decoder).map_err(malformed)?;

    let (width, height) = oriented_dimensions(width, height, orientation);
    Ok(VerifiedHotspotImage {
        media_type,
        width,
        height,
    })
}

fn png_header_dimensions(bytes: &[u8]) -> Result<(u32, u32), HotspotImageError> {
    if bytes.get(12..16) != Some(b"IHDR".as_slice()) {
        return Err(HotspotImageError::Malformed);
    }
    let dimensions = bytes.get(16..24).ok_or(HotspotImageError::Malformed)?;
    Ok((
        u32::from_be_bytes([dimensions[0], dimensions[1], dimensions[2], dimensions[3]]),
        u32::from_be_bytes([dimensions[4], dimensions[5], dimensions[6], dimensions[7]]),
    ))
}

fn validate_dimensions((width, height): (u32, u32)) -> Result<(), HotspotImageError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(HotspotImageError::DecodedPixelLimit)?;
    if width == 0 || height == 0 {
        return Err(HotspotImageError::ZeroDimensions);
    }
    if pixels > MAX_HOTSPOT_DECODED_PIXELS {
        return Err(HotspotImageError::DecodedPixelLimit);
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

fn malformed(_error: image::ImageError) -> HotspotImageError {
    HotspotImageError::Malformed
}

#[cfg(test)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};

    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = RgbImage::from_pixel(width, height, Rgb([12, 34, 56]));
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(image.as_raw(), width, height, ExtendedColorType::Rgb8)
            .expect("PNG fixture encodes");
        bytes
    }

    fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let image = RgbImage::from_pixel(width, height, Rgb([12, 34, 56]));
        let mut bytes = Vec::new();
        JpegEncoder::new_with_quality(&mut bytes, 90)
            .write_image(image.as_raw(), width, height, ExtendedColorType::Rgb8)
            .expect("JPEG fixture encodes");
        bytes
    }

    fn exif_oriented_jpeg(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = jpeg(width, height);
        let exif = [
            b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1,
            0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ];
        let segment_length = u16::try_from(exif.len() + 2).expect("small APP1 fixture");
        let mut oriented = Vec::with_capacity(bytes.len() + exif.len() + 4);
        oriented.extend_from_slice(&bytes[..2]);
        oriented.extend_from_slice(&[0xff, 0xe1]);
        oriented.extend_from_slice(&segment_length.to_be_bytes());
        oriented.extend_from_slice(&exif);
        oriented.extend_from_slice(&bytes.split_off(2));
        oriented
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
        let source = png(2, 1);
        let data = [0, 0, 0, 2, 0, 0, 0, 0];
        let mut chunk_input = b"acTL".to_vec();
        chunk_input.extend_from_slice(&data);
        let mut animation_control = Vec::new();
        animation_control.extend_from_slice(&8_u32.to_be_bytes());
        animation_control.extend_from_slice(b"acTL");
        animation_control.extend_from_slice(&data);
        animation_control.extend_from_slice(&crc32(&chunk_input).to_be_bytes());
        // PNG signature plus IHDR is always the first 33 bytes.
        let mut output = source[..33].to_vec();
        output.extend_from_slice(&animation_control);
        output.extend_from_slice(&source[33..]);
        output
    }

    fn png_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = png(1, 1);
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        let mut ihdr = b"IHDR".to_vec();
        ihdr.extend_from_slice(&bytes[16..29]);
        bytes[29..33].copy_from_slice(&crc32(&ihdr).to_be_bytes());
        bytes
    }

    #[test]
    fn sniffs_png_bytes_instead_of_trusting_a_declared_content_type() {
        let bytes = png(3, 2);
        let original = bytes.clone();

        let verified = verify_hotspot_image(&bytes).expect("valid PNG is accepted");

        assert_eq!(verified.media_type, HotspotImageMediaType::Png);
        assert_eq!(verified.media_type.canonical_media_type(), "image/png");
        assert_eq!((verified.width, verified.height), (3, 2));
        assert_eq!(bytes, original, "verification never rewrites upload bytes");
    }

    #[test]
    fn jpeg_orientation_sets_post_orientation_intrinsic_dimensions() {
        let verified = verify_hotspot_image(&exif_oriented_jpeg(3, 2))
            .expect("EXIF-oriented JPEG is accepted");

        assert_eq!(verified.media_type, HotspotImageMediaType::Jpeg);
        assert_eq!((verified.width, verified.height), (2, 3));
    }

    #[test]
    fn apng_is_rejected_before_any_animation_decode() {
        assert_eq!(
            verify_hotspot_image(&apng()),
            Err(HotspotImageError::Animated)
        );
    }

    #[test]
    fn zero_and_excessive_dimensions_are_rejected_from_the_header() {
        assert_eq!(
            verify_hotspot_image(&png_with_dimensions(0, 1)),
            Err(HotspotImageError::ZeroDimensions)
        );
        assert_eq!(
            verify_hotspot_image(&png_with_dimensions(5_000, 5_000)),
            Err(HotspotImageError::DecodedPixelLimit)
        );
    }

    #[test]
    fn malformed_and_unsupported_uploads_have_distinct_recovery_errors() {
        assert_eq!(
            verify_hotspot_image(b"not an image"),
            Err(HotspotImageError::UnsupportedMediaType)
        );
        assert_eq!(
            verify_hotspot_image(&[0x89, b'P', b'N', b'G', 13, 10, 26, 10]),
            Err(HotspotImageError::Malformed)
        );
        assert_eq!(
            HotspotImageError::Animated.user_message(),
            "upload a still image rather than an animation"
        );
    }
}
