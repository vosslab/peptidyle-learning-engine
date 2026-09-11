//! Shared validation for public response collections.

use crate::answer::ResponseSelectionRule;

use super::builder::PresentationBuildError;
use super::model::PresentedHotspotRegion;

pub(super) fn validate_regions(
    regions: &[PresentedHotspotRegion],
) -> Result<(), PresentationBuildError> {
    const MAX: u32 = 10_000;
    if regions.is_empty() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "hotspot presentation has no accessible regions",
        ));
    }
    for region in regions {
        let right = u32::from(region.x) + u32::from(region.width);
        let bottom = u32::from(region.y) + u32::from(region.height);
        if region.width == 0
            || region.height == 0
            || right > MAX
            || bottom > MAX
            || region.label.is_empty()
        {
            return Err(PresentationBuildError::InvalidPublicContent(
                "hotspot region is outside the normalized surface",
            ));
        }
    }
    Ok(())
}

pub(super) fn selection_bounds(
    selection: ResponseSelectionRule,
    item_count: usize,
) -> Result<(u32, u32), PresentationBuildError> {
    let maximum = u32::try_from(item_count).map_err(|_| PresentationBuildError::TooManyItems)?;
    let bounds = match selection {
        ResponseSelectionRule::ExactlyOne => (1, 1),
        ResponseSelectionRule::Exactly { count } => (count, count),
        ResponseSelectionRule::AnyNumber => (0, maximum),
        ResponseSelectionRule::AtLeastOne => (1, maximum),
    };
    if bounds.0 > bounds.1 || bounds.1 > maximum {
        return Err(PresentationBuildError::InvalidPublicContent(
            "Response Selection Rule exceeds presented objects",
        ));
    }
    Ok(bounds)
}
