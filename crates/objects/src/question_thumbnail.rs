//! Validated lossy WebP bytes handed from capture to publication storage.

use crate::image_validation::{
    StillImageError, StillImageMediaType, VerifiedStillImage, verify_still_image,
};

/// Renderer-produced lossy WebP bytes accepted for immutable Question Thumbnail storage.
///
/// This type validates the bytes without decoding and re-encoding them. Revision and
/// object identity belong to [`crate::ObjectAddress`], supplied separately by the
/// publication owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedQuestionThumbnail {
    bytes: Vec<u8>,
    verified: VerifiedStillImage,
}
impl PreparedQuestionThumbnail {
    /// Validates renderer output and preserves its exact encoded bytes.
    pub fn new(bytes: Vec<u8>) -> Result<Self, StillImageError> {
        let verified = verify_still_image(&bytes)?;
        if verified.media_type != StillImageMediaType::WebP {
            return Err(StillImageError::UnsupportedMediaType);
        }
        if verified.width != 256 || verified.height != 256 {
            return Err(StillImageError::WrongQuestionThumbnailDimensions);
        }
        Ok(Self { bytes, verified })
    }

    /// Returns the original encoded bytes unchanged.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the verified media type and dimensions.
    #[must_use]
    pub const fn verified(&self) -> VerifiedStillImage {
        self.verified
    }

    /// Returns the measured output width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.verified.width
    }

    /// Returns the measured output height.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.verified.height
    }

    /// Transfers the original encoded bytes to the object store.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
