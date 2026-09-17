//! Profile composition applied to validated original pixels at the trusted boundary.

use std::io::Cursor;

use image::codecs::webp::WebPEncoder;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageReader, Limits};

use super::{
    MAX_NORMALIZED_STILL_IMAGE_BYTES, MAX_STILL_IMAGE_DECODED_PIXELS, StillImageError,
    VerifiedStillImage, malformed, verify_profile_still_image,
};

/// Integer crop controls over the post-orientation source. Positions are percentages
/// of available travel; zoom is 100..=400 percent. Source dimensions bind the preview
/// to the trusted orientation and prevent silently applying a different composition.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileImageCrop {
    pub source_width: u32,
    pub source_height: u32,
    pub horizontal: u32,
    pub vertical: u32,
    pub zoom_percent: u32,
}

impl ProfileImageCrop {
    /// Resolves integer pixel edges using the same floor rounding as the preview.
    fn rectangle(self, verified: VerifiedStillImage) -> Result<(u32, u32, u32), StillImageError> {
        // ASVS 2.2.1-3: validate related crop controls against trusted source facts.
        if self.source_width != verified.width
            || self.source_height != verified.height
            || self.horizontal > 100
            || self.vertical > 100
            || !(100..=400).contains(&self.zoom_percent)
        {
            return Err(StillImageError::Malformed);
        }
        let width = u64::from(verified.width);
        let height = u64::from(verified.height);
        let side = width.min(height) * 100 / u64::from(self.zoom_percent);
        let x = (width - side) * u64::from(self.horizontal) / 100;
        let y = (height - side) * u64::from(self.vertical) / 100;
        // The verified <=20M-pixel dimensions and bounded controls keep each value
        // within u32; use checked conversions to retain that invariant explicitly.
        Ok((
            u32::try_from(x).map_err(|_| StillImageError::Malformed)?,
            u32::try_from(y).map_err(|_| StillImageError::Malformed)?,
            u32::try_from(side).map_err(|_| StillImageError::Malformed)?,
        ))
    }
}

/// Validates the original, orients and crops it, then emits one 256-square WebP.
/// No caller-provided pixels or declared media type bypass original validation.
pub fn normalized_profile_image_webp(
    bytes: &[u8],
    crop: ProfileImageCrop,
) -> Result<Vec<u8>, StillImageError> {
    // ASVS 5.2.1, 5.2.2, 5.2.6: validate format, container, stillness, byte/pixel
    // limits and original minimum dimensions before any crop or scale occurs.
    let verified = verify_profile_still_image(bytes)?;
    let (x, y, side) = crop.rectangle(verified)?;
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| StillImageError::Malformed)?
        .into_decoder()
        .map_err(malformed)?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_STILL_IMAGE_DECODED_PIXELS * 8);
    decoder.set_limits(limits).map_err(malformed)?;
    let orientation = decoder.orientation().map_err(malformed)?;
    let mut source = DynamicImage::from_decoder(decoder).map_err(malformed)?;
    source.apply_orientation(orientation);
    let cropped = source.crop_imm(x, y, side, side);
    let rgba = cropped
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let mut output = Vec::new();
    WebPEncoder::new_lossless(&mut output)
        .encode(&rgba, 256, 256, ExtendedColorType::Rgba8)
        .map_err(malformed)?;
    if output.is_empty() || output.len() > MAX_NORMALIZED_STILL_IMAGE_BYTES {
        return Err(StillImageError::ByteLimit);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use image::{ImageEncoder, Rgb, RgbImage, codecs::jpeg::JpegEncoder};

    use super::*;
    use crate::image_validation::{StillImageMediaType, verify_still_image};

    #[test]
    fn profile_crop_has_integer_edges_and_rejects_unbounded_or_mismatched_geometry() {
        let verified = VerifiedStillImage {
            media_type: StillImageMediaType::Png,
            width: 515,
            height: 259,
        };
        let crop = ProfileImageCrop {
            source_width: 515,
            source_height: 259,
            horizontal: 37,
            vertical: 83,
            zoom_percent: 155,
        };
        assert_eq!(crop.rectangle(verified), Ok((128, 76, 167)));
        for invalid in [
            ProfileImageCrop {
                source_width: 259,
                ..crop
            },
            ProfileImageCrop {
                source_height: 515,
                ..crop
            },
            ProfileImageCrop {
                horizontal: 101,
                ..crop
            },
            ProfileImageCrop {
                vertical: u32::MAX,
                ..crop
            },
            ProfileImageCrop {
                zoom_percent: 0,
                ..crop
            },
            ProfileImageCrop {
                zoom_percent: 99,
                ..crop
            },
            ProfileImageCrop {
                zoom_percent: 401,
                ..crop
            },
            ProfileImageCrop {
                zoom_percent: u32::MAX,
                ..crop
            },
        ] {
            assert_eq!(invalid.rectangle(verified), Err(StillImageError::Malformed));
        }
        for zoom_percent in [100, 155, 200, 400] {
            for (horizontal, vertical) in [(0, 0), (100, 100)] {
                let (x, y, side) = ProfileImageCrop {
                    horizontal,
                    vertical,
                    zoom_percent,
                    ..crop
                }
                .rectangle(verified)
                .unwrap();
                assert!(side > 0);
                if horizontal == 100 {
                    assert_eq!((x + side, y + side), (verified.width, verified.height));
                } else {
                    assert_eq!((x, y), (0, 0));
                }
            }
        }
    }

    #[test]
    fn profile_crop_applies_every_exif_orientation_before_selecting_pixels() {
        let image = RgbImage::from_fn(128, 256, |_, y| {
            if y < 128 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 0, 255])
            }
        });
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 100)
            .write_image(image.as_raw(), 128, 256, ExtendedColorType::Rgb8)
            .unwrap();
        for (orientation, red) in [
            (1, true),
            (2, true),
            (3, false),
            (4, false),
            (5, false),
            (6, true),
            (7, true),
            (8, false),
        ] {
            let mut bytes = jpeg.clone();
            // One EXIF SHORT orientation entry; the encoded source pixels stay unchanged.
            bytes.splice(
                2..2,
                [
                    0xff,
                    0xe1,
                    0,
                    34,
                    b'E',
                    b'x',
                    b'i',
                    b'f',
                    0,
                    0,
                    b'I',
                    b'I',
                    42,
                    0,
                    8,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0x12,
                    0x01,
                    3,
                    0,
                    1,
                    0,
                    0,
                    0,
                    orientation,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ],
            );
            let (source_width, source_height) = if orientation >= 5 {
                (256, 128)
            } else {
                (128, 256)
            };
            let crop = ProfileImageCrop {
                source_width,
                source_height,
                horizontal: 100,
                vertical: 0,
                zoom_percent: 100,
            };
            let output = normalized_profile_image_webp(&bytes, crop).unwrap();
            let facts = verify_still_image(&output).unwrap();
            assert_eq!(facts.media_type, StillImageMediaType::WebP);
            assert_eq!((facts.width, facts.height), (256, 256));
            let rendered = image::load_from_memory(&output).unwrap().to_rgb8();
            let pixel = rendered.get_pixel(128, 128);
            assert_eq!(pixel[0] > pixel[2], red, "orientation {orientation}");
            assert_eq!(
                normalized_profile_image_webp(
                    &bytes,
                    ProfileImageCrop {
                        source_width: source_width + 1,
                        ..crop
                    }
                ),
                Err(StillImageError::Malformed)
            );
        }
    }
}
