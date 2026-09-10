//! Browser-safe course appearance contracts.
//!
//! This module owns stable theme and presentation values only. Physical object
//! keys, checksums, upload metadata, signed URLs, and authorization records
//! belong to the object, persistence, and server layers.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The closed set of reviewed Course Theme palettes.
///
/// Each value selects one complete visual palette for a Course Appearance.
/// Palette values are design-system data rather than database identities. The
/// browser registry must exhaustively map every value and refuse an unknown
/// value.
/// @tsgen-runtime-values
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum CourseTheme {
    /// Muted grey-purple and moss green.
    Tundra,
    /// Deep green with a warm gold accent.
    Forest,
    /// Sand and clay tones.
    Desert,
    /// Roosevelt-inspired pale and vivid greens.
    #[default]
    Grass,
    /// Ice blue with a deep blue accent.
    Arctic,
    /// Pale ocean blue with two darker blue anchors.
    Ocean,
    /// Leaf green with a purple accent.
    Tropical,
    /// Teal with a coral-red accent.
    CoralReef,
    /// Olive and dark earth tones.
    Swamp,
    /// Stone grey with an orange accent.
    Underground,
    /// Blue-green with a brown accent.
    SaltMarsh,
    /// Muted green and blue.
    Wetland,
    /// Deep blue-grey and teal.
    SeaFloor,
    /// Warm ash with deep red and charcoal.
    Magma,
    /// Sand, sea blue, and warm brown.
    Beach,
}

impl CourseTheme {
    /// Every persisted and browser-visible theme, in authoring order.
    pub const ALL: [Self; 15] = [
        Self::Tundra,
        Self::Forest,
        Self::Desert,
        Self::Grass,
        Self::Arctic,
        Self::Ocean,
        Self::Tropical,
        Self::CoralReef,
        Self::Swamp,
        Self::Underground,
        Self::SaltMarsh,
        Self::Wetland,
        Self::SeaFloor,
        Self::Magma,
        Self::Beach,
    ];

    /// Returns the stable serialized palette value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tundra => "tundra",
            Self::Forest => "forest",
            Self::Desert => "desert",
            Self::Grass => "grass",
            Self::Arctic => "arctic",
            Self::Ocean => "ocean",
            Self::Tropical => "tropical",
            Self::CoralReef => "coral-reef",
            Self::Swamp => "swamp",
            Self::Underground => "underground",
            Self::SaltMarsh => "salt-marsh",
            Self::Wetland => "wetland",
            Self::SeaFloor => "sea-floor",
            Self::Magma => "magma",
            Self::Beach => "beach",
        }
    }
}

impl std::fmt::Display for CourseTheme {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CourseTheme {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.as_str() == value)
            .ok_or("unknown course theme")
    }
}

macro_rules! impl_banner_route_id {
    ($name:ident) => {
        impl $name {
            /// Wraps the opaque route identity read from trusted state.
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the UUID used by server and persistence boundaries.
            pub fn as_uuid(self) -> Uuid {
                self.0
            }

            /// Mints a fresh opaque route identity on the server.
            #[cfg(feature = "generate")]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

/// Stable same-origin reference for the current course banner.
///
/// This is a browser-safe delivery identity, not an object-store key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseBannerReference(Uuid);

impl_banner_route_id!(CourseBannerReference);

/// Opaque reference returned after an authorized Course Banner Upload.
///
/// The server binds it to the course, Account, and expiry before accepting
/// it in a Course Banner promotion. It reveals no physical storage identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseBannerUploadReference(Uuid);

impl_banner_route_id!(CourseBannerUploadReference);

/// A server-owned Course Banner delivery rendition.
///
/// This is deliberately closed: callers select neither a storage key nor an
/// arbitrary resize.  The delivery route chooses one of these fixed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CourseBannerRendition {
    /// Wide course-entry image, normalized to 1200 by 200 pixels.
    Hero,
    /// Course-card image, normalized to 1000 by 400 pixels.
    Card,
}

impl CourseBannerRendition {
    /// Stable storage and database value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hero => "hero",
            Self::Card => "card",
        }
    }

    /// Exact normalized pixel dimensions for this rendition.
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Hero => (1200, 200),
            Self::Card => (1000, 400),
        }
    }
}

/// Validated informative text for one course banner.
///
/// The wire value is a plain string. Empty or whitespace-only text is not an
/// informative Course Banner text, and the 160-scalar ceiling keeps the setting
/// short enough to review beside its preview.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseBannerInformativeText(String);

impl CourseBannerInformativeText {
    /// Returns the validated text without normalization or truncation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CourseBannerInformativeText {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let scalar_count = value.chars().count();
        if scalar_count == 0 || scalar_count > 160 || value.trim().is_empty() {
            return Err("informative course banner text must be 1 to 160 characters");
        }
        Ok(Self(value))
    }
}

impl From<CourseBannerInformativeText> for String {
    fn from(value: CourseBannerInformativeText) -> Self {
        value.0
    }
}

/// Explicit accessibility treatment for a course banner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CourseBannerAlternativeText {
    /// The banner conveys no information beyond adjacent course text.
    Decorative,
    /// The banner conveys information described by the validated text.
    Informative {
        /// Concise equivalent information for non-visual use.
        text: CourseBannerInformativeText,
    },
}

/// Browser-safe current course banner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CourseBanner {
    /// Opaque reference resolved only through the same-origin asset route.
    pub reference: CourseBannerReference,
    /// Explicit decorative or informative treatment.
    pub alternative_text: CourseBannerAlternativeText,
}

/// Safe receipt returned after an authorized Course Banner Upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CourseBannerUploadReceipt {
    /// Opaque upload accepted by a later atomic appearance update.
    pub upload: CourseBannerUploadReference,
}

/// Strict request that promotes one already-authorized Course Banner Upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CourseBannerUpdate {
    /// Opaque upload staged by this same Account for this same Course.
    pub upload: CourseBannerUploadReference,
    /// Explicit accessibility treatment stored with the promoted banner.
    pub alternative_text: CourseBannerAlternativeText,
}

/// Browser-safe current Course Appearance View.
///
/// The reader shape contains only the authorized current values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// ASVS 1.5.2 and 2.2.1: allowlist the complete external reader shape.
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CourseAppearanceView {
    /// Reviewed theme selected for the complete course route scope.
    pub theme: CourseTheme,
    /// Current course banner, or no banner frame at all.
    pub banner: Option<CourseBanner>,
}

/// Strict body for one independent Course Theme update.
///
/// Course identity comes from the authenticated route; a theme write never
/// restates or changes the separately stored Course Banner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CourseThemeUpdate {
    /// Complete desired theme.
    pub theme: CourseTheme,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_are_closed_stable_and_have_one_default() {
        let wire_values = CourseTheme::ALL.map(|theme| theme.to_string());
        assert_eq!(
            wire_values,
            [
                "tundra",
                "forest",
                "desert",
                "grass",
                "arctic",
                "ocean",
                "tropical",
                "coral-reef",
                "swamp",
                "underground",
                "salt-marsh",
                "wetland",
                "sea-floor",
                "magma",
                "beach",
            ]
        );
        assert_eq!(CourseTheme::default(), CourseTheme::Grass);
        assert!("woodland".parse::<CourseTheme>().is_err());
        assert!(serde_json::from_str::<CourseTheme>(r#""unknown""#).is_err());
    }

    #[test]
    fn informative_banner_text_is_short_nonblank_and_unicode_aware() {
        let text = CourseBannerInformativeText::try_from("A peptide chain diagram".to_string())
            .expect("informative text should validate");
        assert_eq!(text.as_str(), "A peptide chain diagram");
        assert!(CourseBannerInformativeText::try_from("   ".to_string()).is_err());
        assert!(CourseBannerInformativeText::try_from("\u{03b2}".repeat(160)).is_ok());
        assert!(CourseBannerInformativeText::try_from("\u{03b2}".repeat(161)).is_err());
    }

    #[test]
    fn appearance_view_contains_only_safe_course_banner_data() {
        let appearance = CourseAppearanceView {
            theme: CourseTheme::Ocean,
            banner: Some(CourseBanner {
                reference: CourseBannerReference::from_uuid(Uuid::from_u128(7)),
                alternative_text: CourseBannerAlternativeText::Decorative,
            }),
        };

        assert_eq!(
            serde_json::to_value(appearance).expect("appearance should serialize"),
            serde_json::json!({
                "theme": "ocean",
                "banner": {
                    "reference": "00000000-0000-0000-0000-000000000007",
                    "alternativeText": { "kind": "decorative" }
                }
            })
        );
        assert!(
            serde_json::from_value::<CourseBanner>(serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000007",
                "alternativeText": { "kind": "decorative" }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CourseAppearanceView>(serde_json::json!({
                "theme": "ocean",
                "revision": "1",
                "banner": null,
                "privateAppearanceField": "must-not-be-accepted"
            }))
            .is_err()
        );
    }

    #[test]
    fn upload_receipt_exposes_only_the_route_bound_reference() {
        let receipt = CourseBannerUploadReceipt {
            upload: CourseBannerUploadReference::from_uuid(Uuid::from_u128(8)),
        };
        assert_eq!(
            serde_json::to_value(receipt).expect("upload receipt should serialize"),
            serde_json::json!({
                "upload": "00000000-0000-0000-0000-000000000008"
            })
        );
    }

    #[test]
    fn update_body_is_strict_and_route_bound() {
        let update = CourseThemeUpdate {
            theme: CourseTheme::Forest,
        };
        let value = serde_json::to_value(update).expect("update should serialize");
        assert_eq!(value["theme"], "forest");
        assert!(value.get("courseId").is_none());

        assert!(
            serde_json::from_value::<CourseThemeUpdate>(serde_json::json!({
                "theme": "ocean",
                "objectKey": "must-not-be-accepted"
            }))
            .is_err()
        );
    }
}
