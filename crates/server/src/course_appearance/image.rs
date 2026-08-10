//! Bounded raster normalization for course banners.

use std::io::Cursor;

use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::{WebPDecoder, WebPEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, Limits};
use learning_data_access::{COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH};

pub(super) const MAX_BANNER_DECODED_PIXELS: u64 = 20_000_000;

/// Stable upload media types accepted by the normalization boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BannerImageMediaType {
    Jpeg,
    Png,
    WebP,
}

impl BannerImageMediaType {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "image/jpeg" => Some(Self::Jpeg),
            "image/png" => Some(Self::Png),
            "image/webp" => Some(Self::WebP),
            _ => None,
        }
    }
}

/// Safe image-validation classes used by the HTTP boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BannerImageError {
    Animated,
    DecodedPixelLimit,
    Malformed,
    TooSmall,
}

/// Decodes one allowlisted raster, applies its embedded orientation, performs
/// a centered cover crop, and emits fresh metadata-free lossless WebP bytes.
pub(super) fn normalize_banner(
    media_type: BannerImageMediaType,
    bytes: &[u8],
) -> Result<Vec<u8>, BannerImageError> {
    let cursor = Cursor::new(bytes);
    let image = match media_type {
        BannerImageMediaType::Jpeg => decode(JpegDecoder::new(cursor).map_err(malformed)?)?,
        BannerImageMediaType::Png => {
            let decoder = PngDecoder::new(cursor).map_err(malformed)?;
            if decoder.is_apng().map_err(malformed)? {
                return Err(BannerImageError::Animated);
            }
            decode(decoder)?
        }
        BannerImageMediaType::WebP => {
            let decoder = WebPDecoder::new(cursor).map_err(malformed)?;
            if decoder.has_animation() {
                return Err(BannerImageError::Animated);
            }
            decode(decoder)?
        }
    };
    normalize_decoded(image)
}

fn decode<D>(mut decoder: D) -> Result<DynamicImage, BannerImageError>
where
    D: ImageDecoder,
{
    let (width, height) = decoder.dimensions();
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(BannerImageError::DecodedPixelLimit)?;
    if pixels > MAX_BANNER_DECODED_PIXELS {
        return Err(BannerImageError::DecodedPixelLimit);
    }
    let orientation = decoder.orientation().map_err(malformed)?;
    let mut limits = Limits::default();
    // Eight bytes per accepted pixel covers the largest enabled decoder's
    // 16-bit output while the explicit pixel ceiling remains authoritative.
    limits.max_alloc = Some(MAX_BANNER_DECODED_PIXELS * 8);
    decoder.set_limits(limits).map_err(malformed)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(malformed)?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn normalize_decoded(image: DynamicImage) -> Result<Vec<u8>, BannerImageError> {
    let width = image.width();
    let height = image.height();
    if width < COURSE_BANNER_WIDTH || height < COURSE_BANNER_HEIGHT {
        return Err(BannerImageError::TooSmall);
    }

    let source_is_wider = u64::from(width) * u64::from(COURSE_BANNER_HEIGHT)
        >= u64::from(COURSE_BANNER_WIDTH) * u64::from(height);
    let (resized_width, resized_height) = if source_is_wider {
        (
            ceil_ratio(width, COURSE_BANNER_HEIGHT, height)?,
            COURSE_BANNER_HEIGHT,
        )
    } else {
        (
            COURSE_BANNER_WIDTH,
            ceil_ratio(height, COURSE_BANNER_WIDTH, width)?,
        )
    };
    let resized = image.resize_exact(resized_width, resized_height, FilterType::Lanczos3);
    let x = (resized_width - COURSE_BANNER_WIDTH) / 2;
    let y = (resized_height - COURSE_BANNER_HEIGHT) / 2;
    let normalized = resized
        .crop_imm(x, y, COURSE_BANNER_WIDTH, COURSE_BANNER_HEIGHT)
        .to_rgba8();

    let mut output = Vec::new();
    WebPEncoder::new_lossless(&mut output)
        .write_image(
            normalized.as_raw(),
            COURSE_BANNER_WIDTH,
            COURSE_BANNER_HEIGHT,
            ExtendedColorType::Rgba8,
        )
        .map_err(malformed)?;
    Ok(output)
}

fn ceil_ratio(value: u32, numerator: u32, denominator: u32) -> Result<u32, BannerImageError> {
    let scaled = u64::from(value)
        .checked_mul(u64::from(numerator))
        .ok_or(BannerImageError::DecodedPixelLimit)?;
    let rounded = scaled
        .checked_add(u64::from(denominator) - 1)
        .ok_or(BannerImageError::DecodedPixelLimit)?
        / u64::from(denominator);
    u32::try_from(rounded).map_err(|_| BannerImageError::DecodedPixelLimit)
}

fn malformed(_error: image::ImageError) -> BannerImageError {
    BannerImageError::Malformed
}

#[cfg(test)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
    use image::{GenericImageView, ImageFormat, Rgb, RgbImage};

    use super::*;

    fn encode_png(image: &RgbImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, PngFilterType::Sub)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("PNG fixture should encode");
        bytes
    }

    fn decode_output(bytes: &[u8]) -> DynamicImage {
        image::load_from_memory_with_format(bytes, ImageFormat::WebP)
            .expect("normalized output should be WebP")
    }

    #[test]
    fn png_is_scaled_and_center_cropped_to_the_one_derivative() {
        let source = RgbImage::from_fn(1_600, 328, |x, _| {
            if x < 200 {
                Rgb([255, 0, 0])
            } else if x >= 1_400 {
                Rgb([0, 0, 255])
            } else {
                Rgb([0, 255, 0])
            }
        });
        let output = normalize_banner(BannerImageMediaType::Png, &encode_png(&source))
            .expect("wide PNG should normalize");
        let decoded = decode_output(&output).to_rgb8();

        assert_eq!(decoded.dimensions(), (1_200, 328));
        assert_eq!(decoded.get_pixel(0, 164), &Rgb([0, 255, 0]));
        assert_eq!(decoded.get_pixel(1_199, 164), &Rgb([0, 255, 0]));
    }

    #[test]
    fn orientation_is_applied_before_the_minimum_crop_check_and_metadata_is_stripped() {
        let portrait = RgbImage::from_pixel(328, 1_200, Rgb([12, 34, 56]));
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 90)
            .write_image(
                portrait.as_raw(),
                portrait.width(),
                portrait.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("JPEG fixture should encode");
        let oriented = with_exif_orientation(jpeg, 6);

        let output = normalize_banner(BannerImageMediaType::Jpeg, &oriented)
            .expect("oriented JPEG should normalize after rotation");

        assert_eq!(decode_output(&output).dimensions(), (1_200, 328));
        assert!(!output.windows(4).any(|window| window == b"Exif"));
    }

    #[test]
    fn undersized_and_animated_inputs_are_refused() {
        let too_narrow = RgbImage::from_pixel(1_199, 500, Rgb([1, 2, 3]));
        assert_eq!(
            normalize_banner(BannerImageMediaType::Png, &encode_png(&too_narrow)),
            Err(BannerImageError::TooSmall)
        );

        let static_png = encode_png(&RgbImage::from_pixel(1_200, 328, Rgb([1, 2, 3])));
        let animated_png = insert_apng_control(static_png);
        assert_eq!(
            normalize_banner(BannerImageMediaType::Png, &animated_png),
            Err(BannerImageError::Animated)
        );
    }

    fn with_exif_orientation(mut jpeg: Vec<u8>, orientation: u16) -> Vec<u8> {
        let mut payload = b"Exif\0\0II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
        payload.extend_from_slice(&orientation.to_le_bytes());
        payload.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        let length = u16::try_from(payload.len() + 2).expect("EXIF fixture is bounded");
        let mut output = Vec::with_capacity(jpeg.len() + payload.len() + 4);
        output.extend_from_slice(&jpeg[..2]);
        output.extend_from_slice(&[0xff, 0xe1]);
        output.extend_from_slice(&length.to_be_bytes());
        output.extend_from_slice(&payload);
        output.append(&mut jpeg.split_off(2));
        output
    }

    fn insert_apng_control(png: Vec<u8>) -> Vec<u8> {
        let insertion = 8 + 4 + 4 + 13 + 4;
        let data = [0, 0, 0, 1, 0, 0, 0, 0];
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(b"acTL");
        chunk.extend_from_slice(&data);
        chunk.extend_from_slice(&crc32(&[b"acTL".as_slice(), &data].concat()).to_be_bytes());
        let mut output = Vec::with_capacity(png.len() + chunk.len());
        output.extend_from_slice(&png[..insertion]);
        output.extend_from_slice(&chunk);
        output.extend_from_slice(&png[insertion..]);
        output
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = u32::MAX;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
            }
        }
        !crc
    }
}
