//! Compatibility boundary for native hotspots.
//!
//! The implementation lives in `objects::image_validation`, shared with QTI
//! extraction so every immutable instructional-image path enforces the same
//! hostile-input contract.

pub(crate) use objects::image_validation::{
    StillImageError as HotspotImageError, VerifiedStillImage as VerifiedHotspotImage,
};

pub(crate) fn verify_hotspot_image(
    bytes: &[u8],
) -> Result<VerifiedHotspotImage, HotspotImageError> {
    objects::image_validation::verify_still_image(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotspot_boundary_delegates_to_the_shared_contract() {
        assert_eq!(
            verify_hotspot_image(b"not an image"),
            Err(HotspotImageError::UnsupportedMediaType)
        );
    }
}
