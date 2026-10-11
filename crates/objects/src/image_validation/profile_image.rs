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
