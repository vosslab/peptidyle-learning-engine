//! One authoritative hostile-input boundary for instructional raster images.
//!
//! An image's name and declared media type are untrusted. This module accepts
//! complete, single-container PNG, JPEG, and WebP files. SVG is accepted only
//! by rewriting it into a WebP still image. Animation, container trailing data,
//! and pixel floods are rejected before callers create an immutable object.

use std::io::Cursor;

use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::codecs::webp::WebPEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageFormat, ImageReader, Limits};

mod profile_image;
mod svg_question_image;
pub use profile_image::{ProfileImageCrop, normalized_profile_image_webp};

/// Maximum accepted original still-image byte length at every ingest path.
pub const MAX_STILL_IMAGE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum decoded pixels at every ingest path.
pub const MAX_STILL_IMAGE_DECODED_PIXELS: u64 = 20_000_000;
/// Maximum encoded byte length for one normalized Course Banner rendition.
pub const MAX_COURSE_BANNER_RENDITION_BYTES: usize = 2 * 1024 * 1024;
/// Maximum encoded byte length for one normalized still-image rendition.
pub const MAX_NORMALIZED_STILL_IMAGE_BYTES: usize = MAX_COURSE_BANNER_RENDITION_BYTES;
/// Minimum post-orientation width and height accepted for a Profile image.
pub const MIN_PROFILE_IMAGE_SIDE_PIXELS: u32 = 128;

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
    /// A Course Banner must retain its whole 5:1 composition; accepting any
    /// other ratio would require a crop or padding to make its fixed delivery
    /// rendition.
    WrongCourseBannerAspectRatio,
    /// Both visual dimensions of a Profile image must meet the minimum.
    ProfileImageTooSmall,
    /// A valid container was followed by unowned bytes, so it could be a
    /// polyglot or an ambiguity between parsers.
    Polyglot,
    /// SVG text would disappear: the converter has no fonts, so accepting the
    /// file would store a picture without its labels.
    UnsupportedSvgText,
    Malformed,
}

/// Verifies the exact 5:1 visual container required for a Course Banner.
///
/// Dimensions are measured after EXIF orientation.  This deliberately has no
/// minimum pixel dimensions: a valid 5:1 source can be scaled up, while a
/// non-5:1 source is refused rather than silently cropped or padded.
pub fn verify_course_banner_still_image(
    bytes: &[u8],
) -> Result<VerifiedStillImage, StillImageError> {
    let verified = verify_still_image(bytes)?;
    if u64::from(verified.width) != 5 * u64::from(verified.height) {
        return Err(StillImageError::WrongCourseBannerAspectRatio);
    }
    Ok(verified)
}

/// Verifies a Profile image of any aspect ratio before storage or normalization.
// ASVS 2.2.2: enforce the input contract after trusted full decode and orientation.
pub fn verify_profile_still_image(bytes: &[u8]) -> Result<VerifiedStillImage, StillImageError> {
    let verified = verify_still_image(bytes)?;
    if verified.width < MIN_PROFILE_IMAGE_SIDE_PIXELS
        || verified.height < MIN_PROFILE_IMAGE_SIDE_PIXELS
    {
        return Err(StillImageError::ProfileImageTooSmall);
    }
    Ok(verified)
}

/// Produces the exact, uncropped, lossless WebP rendition required by a Course
/// Banner delivery route. The source is decoded again at promotion time so a
/// staged-object substitution cannot bypass the ingest validation boundary.
// ASVS 5.2.1: decode and constrain hostile raster bytes before derived output.
pub fn normalized_course_banner_webp(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, StillImageError> {
    if width == 0 || height == 0 {
        return Err(StillImageError::ZeroDimensions);
    }
    verify_course_banner_still_image(bytes)?;
    if u64::from(width) != 5 * u64::from(height) {
        return Err(StillImageError::WrongCourseBannerAspectRatio);
    }
    // `load_from_memory` discards the decoder's EXIF orientation. Decode
    // through its decoder so the scale uses the same visual orientation that
    // `verify_still_image` measured.
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| StillImageError::Malformed)?
        .into_decoder()
        .map_err(malformed)?;
    let orientation = decoder.orientation().map_err(malformed)?;
    let mut source = DynamicImage::from_decoder(decoder).map_err(malformed)?;
    source.apply_orientation(orientation);
    // The verified source and output are both 5:1. `resize` is therefore an
    // aspect-preserving scale to the exact delivery dimensions, retaining both
    // edge pixels instead of using resize_to_fill's crop behavior.
    let scaled = source.resize(width, height, image::imageops::FilterType::Lanczos3);
    if scaled.width() != width || scaled.height() != height {
        return Err(StillImageError::Malformed);
    }
    let rgba = scaled.to_rgba8();
    let mut output = Vec::new();
    WebPEncoder::new_lossless(&mut output)
        .encode(&rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(malformed)?;
    if output.is_empty() || output.len() > MAX_COURSE_BANNER_RENDITION_BYTES {
        return Err(StillImageError::ByteLimit);
    }
    Ok(output)
}

/// Produces one exact centered lossless WebP rendition from verified still bytes.
// ASVS 5.2.1: decode and constrain hostile raster bytes before derived output.
pub fn normalized_still_image_webp(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, StillImageError> {
    if width == 0 || height == 0 {
        return Err(StillImageError::ZeroDimensions);
    }
    verify_still_image(bytes)?;
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| StillImageError::Malformed)?
        .into_decoder()
        .map_err(malformed)?;
    let orientation = decoder.orientation().map_err(malformed)?;
    let mut source = DynamicImage::from_decoder(decoder).map_err(malformed)?;
    source.apply_orientation(orientation);
    let cropped = source.resize_to_fill(width, height, image::imageops::FilterType::Lanczos3);
    let rgba = cropped.to_rgba8();
    let mut output = Vec::new();
    WebPEncoder::new_lossless(&mut output)
        .encode(&rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(malformed)?;
    if output.is_empty() || output.len() > MAX_NORMALIZED_STILL_IMAGE_BYTES {
        return Err(StillImageError::ByteLimit);
    }
    Ok(output)
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
            Self::WrongCourseBannerAspectRatio => {
                "upload an image with an exact 5:1 width-to-height ratio"
            }
            Self::ProfileImageTooSmall => "upload an image at least 128 pixels wide and tall",
            Self::Polyglot | Self::Malformed => "upload a complete, readable image file",
            Self::UnsupportedSvgText => {
                "remove text from the SVG, or upload a PNG, JPEG, or WebP image"
            }
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
            Self::WrongCourseBannerAspectRatio => {
                "Course Banner dimensions must have an exact 5:1 width-to-height ratio"
            }
            Self::ProfileImageTooSmall => {
                "Profile image dimensions must each be at least 128 pixels"
            }
            Self::Polyglot => "image has bytes after its declared container",
            Self::UnsupportedSvgText => {
                "SVG text is not stored because the image converter cannot keep those labels"
            }
            Self::Malformed => "image is incomplete or malformed",
        }
    }
}

/// Bytes retained for one Question image after the upload boundary.
#[derive(Debug, PartialEq, Eq)]
pub enum PreparedQuestionImage<'a> {
    /// PNG, JPEG, or WebP bytes that already match the declared type.
    Original(&'a [u8], VerifiedStillImage),
    /// SVG upload input rewritten into a WebP still image.
    Rewritten(Vec<u8>, VerifiedStillImage),
}

impl PreparedQuestionImage<'_> {
    /// Stored bytes. SVG input is never included.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Original(bytes, _) => bytes,
            Self::Rewritten(bytes, _) => bytes,
        }
    }

    /// Facts measured from the stored raster.
    #[must_use]
    pub fn verified(&self) -> VerifiedStillImage {
        match self {
            Self::Original(_, verified) | Self::Rewritten(_, verified) => *verified,
        }
    }
}

/// Accepts one Question image upload and retains only a verified raster.
// ASVS 2.2.1 and 5.2.2: allow PNG, JPEG, WebP, and SVG, then rewrite SVG.
pub fn prepare_question_image<'a>(
    declared_media_type: &str,
    bytes: &'a [u8],
) -> Result<PreparedQuestionImage<'a>, StillImageError> {
    match declared_media_type {
        "image/svg+xml" => {
            let webp = svg_question_image::rasterize_svg_question_image(bytes)?;
            let verified = verify_still_image(&webp)?;
            if verified.media_type != StillImageMediaType::WebP {
                return Err(StillImageError::Malformed);
            }
            Ok(PreparedQuestionImage::Rewritten(webp, verified))
        }
        "image/png" | "image/jpeg" | "image/webp" => {
            let verified = verify_still_image(bytes)?;
            if verified.media_type.canonical_media_type() != declared_media_type {
                return Err(StillImageError::UnsupportedMediaType);
            }
            Ok(PreparedQuestionImage::Original(bytes, verified))
        }
        _ => Err(StillImageError::UnsupportedMediaType),
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
