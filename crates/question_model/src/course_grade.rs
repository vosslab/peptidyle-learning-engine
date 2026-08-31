//! Closed, browser-safe configuration for course-grade aggregation.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_LABEL_LENGTH: usize = 32;
const MAX_CATEGORY_TITLE_LENGTH: usize = 200;
const TOTAL_WEIGHT_BASIS_POINTS: u32 = 10_000;

/// Stable category identifier within one course grade scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GradeCategoryId(Uuid);

impl GradeCategoryId {
    /// Wraps a UUID read from storage or an authenticated boundary.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    /// Returns the UUID used by storage and logging.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
    /// Mints a fresh server-owned identifier.
    #[cfg(feature = "generate")]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for GradeCategoryId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Human-readable, trimmed category title.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct GradeCategoryTitle(String);

impl GradeCategoryTitle {
    /// Builds a trimmed, bounded title.
    pub fn new(value: impl Into<String>) -> Result<Self, CourseGradeSchemeError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > MAX_CATEGORY_TITLE_LENGTH
            || trimmed != value
        {
            return Err(CourseGradeSchemeError::InvalidCategoryTitle);
        }
        Ok(Self(value))
    }
    /// Returns the title text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GradeCategoryTitle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Human-readable, trimmed letter-band label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct LetterBandLabel(String);

impl LetterBandLabel {
    /// Builds a trimmed, bounded label.
    pub fn new(value: impl Into<String>) -> Result<Self, CourseGradeSchemeError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_LABEL_LENGTH || trimmed != value {
            return Err(CourseGradeSchemeError::InvalidLetterBandLabel);
        }
        Ok(Self(value))
    }
    /// Returns the label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LetterBandLabel {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// The one rounding rule supported by the shipped course-grade contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseGradeRoundingRule {
    /// Round once to four decimal places, with .5 ties away from zero.
    #[default]
    FourDecimalPlacesHalfAwayFromZero,
}

/// A grade category with its exact contribution to a weighted course total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WeightedGradeCategory {
    /// Stable category identifier selected by included assignments.
    pub id: GradeCategoryId,
    /// Human-readable category title shown to the instructor.
    pub title: GradeCategoryTitle,
    /// Canonical zero-based display and tie-break order.
    pub position: u32,
    /// Exact percentage in basis points; all category weights sum to 10,000.
    pub weight_basis_points: u16,
    /// Number of lowest-scoring assignments removed before category aggregation.
    pub drop_lowest: u32,
}

/// A rounded-score threshold that awards a course letter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LetterBand {
    /// Letter shown when the rounded score reaches this threshold.
    pub label: LetterBandLabel,
    /// Inclusive threshold in basis points, from 0 through 10,000.
    pub minimum_basis_points: u16,
}

/// Closed list of shipped course-grade aggregation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseGradeMode {
    TotalPoints,
    WeightedCategories,
}

/// One course-owned assignment setting accepted from an authorized instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradeAssignmentSetting {
    /// Assignment identifier within the current course.
    pub assignment: crate::AssignmentId,
    /// Whether this assignment contributes to the total.
    pub included: bool,
    /// Weighted-category identity, when the mode uses one.
    pub category: Option<GradeCategoryId>,
    /// Canonical assignment order within that category.
    pub position: Option<u32>,
}

/// One current course assignment displayed to an authorized instructor.
///
/// The title is server-owned and intentionally has no write counterpart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradeAssignmentView {
    /// Assignment identifier within the current course.
    pub assignment: crate::AssignmentId,
    /// Current server-owned assignment title.
    pub title: String,
    /// Whether the assignment contributes to the total.
    pub included: bool,
    /// Weighted-category identity, when the mode uses one.
    pub category: Option<GradeCategoryId>,
    /// Canonical assignment order within that category.
    pub position: Option<u32>,
}

/// Browser-safe course-grade read representation. The revision travels only
/// in the strong HTTP ETag, never in a browser-supplied body field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradeSchemeView {
    /// Closed aggregation configuration.
    pub scheme: CourseGradeScheme,
    /// Current assignment titles and settings.
    pub assignments: Vec<CourseGradeAssignmentView>,
}

/// Strict title-free course-grade write representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradeSchemeUpdateView {
    /// Closed replacement aggregation configuration.
    pub scheme: CourseGradeScheme,
    /// Exact current assignment settings, excluding any title authority.
    pub assignments: Vec<CourseGradeAssignmentSetting>,
}

/// Exact reason an instructor total is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseGradeUnavailableReasonView {
    NoIncludedAssignments,
    Recalculating,
    Failed,
    EmptyAfterDrop,
    ZeroPossiblePoints,
}

/// Closed instructor-only total for one roster entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CourseGradeOutcomeView {
    /// A grade calculated exclusively by the server.
    Available {
        score: f64,
        letter: Option<String>,
        dropped_assignment_ids: Vec<crate::AssignmentId>,
        total_earned: Option<f64>,
        total_possible: Option<f64>,
    },
    /// A grade deliberately not calculated by the server.
    Unavailable {
        reason: CourseGradeUnavailableReasonView,
    },
}

/// Protected Instructor roster row. It intentionally excludes email, external
/// affiliation, account, Student, and enrollment identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradebookTotalViewRow {
    /// Course roster display name.
    pub display_name: String,
    /// Server-calculated result or a closed unavailable reason.
    pub outcome: CourseGradeOutcomeView,
}

/// Browser-safe instructor totals view. The browser never recomputes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradebookTotalsView {
    /// Aggregation mode actually used by every row.
    pub mode: CourseGradeMode,
    /// Explicit final rounding rule actually used by every row.
    pub rounding: CourseGradeRoundingRule,
    /// Protected active-student rows.
    pub rows: Vec<CourseGradebookTotalViewRow>,
}

/// Complete course-grade configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGradeScheme {
    /// Closed aggregation mode for this course.
    pub mode: CourseGradeMode,
    /// Explicit, shared final-score rounding rule.
    pub rounding: CourseGradeRoundingRule,
    /// Ordered categories; required and empty only for total-points mode.
    pub categories: Vec<WeightedGradeCategory>,
    /// Descending threshold bands applied after final rounding.
    pub letter_bands: Vec<LetterBand>,
}

impl CourseGradeScheme {
    /// Validates cross-field invariants that serde cannot express.
    pub fn validate(&self) -> Result<(), CourseGradeSchemeError> {
        match self.mode {
            CourseGradeMode::TotalPoints if !self.categories.is_empty() => {
                return Err(CourseGradeSchemeError::TotalPointsRequiresEmptyCategories);
            }
            CourseGradeMode::WeightedCategories if self.categories.is_empty() => {
                return Err(CourseGradeSchemeError::EmptyCategories);
            }
            CourseGradeMode::TotalPoints | CourseGradeMode::WeightedCategories => {}
        }
        let mut ids = HashSet::new();
        let mut total = 0_u32;
        for (expected_position, category) in self.categories.iter().enumerate() {
            if category.position != expected_position as u32 {
                return Err(CourseGradeSchemeError::NonCanonicalCategoryPosition {
                    expected: expected_position as u32,
                    actual: category.position,
                });
            }
            if category.weight_basis_points == 0 {
                return Err(CourseGradeSchemeError::ZeroCategoryWeight {
                    category: category.id,
                });
            }
            if !ids.insert(category.id) {
                return Err(CourseGradeSchemeError::DuplicateCategory {
                    category: category.id,
                });
            }
            total += u32::from(category.weight_basis_points);
        }
        if !self.categories.is_empty() && total != TOTAL_WEIGHT_BASIS_POINTS {
            return Err(CourseGradeSchemeError::CategoryWeightsMustSumToTenThousand { total });
        }
        let mut labels = HashSet::new();
        let mut previous = None;
        for band in &self.letter_bands {
            if band.minimum_basis_points > TOTAL_WEIGHT_BASIS_POINTS as u16 {
                return Err(CourseGradeSchemeError::LetterBandThresholdOutOfRange {
                    threshold: band.minimum_basis_points,
                });
            }
            if !labels.insert(band.label.clone()) {
                return Err(CourseGradeSchemeError::DuplicateLetterBand {
                    label: band.label.clone(),
                });
            }
            if let Some(previous) = previous
                && band.minimum_basis_points >= previous
            {
                return Err(CourseGradeSchemeError::LetterBandsMustDescend);
            }
            previous = Some(band.minimum_basis_points);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for CourseGradeScheme {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawCourseGradeScheme {
            mode: CourseGradeMode,
            rounding: CourseGradeRoundingRule,
            categories: Vec<WeightedGradeCategory>,
            letter_bands: Vec<LetterBand>,
        }
        let raw = RawCourseGradeScheme::deserialize(deserializer)?;
        let scheme = Self {
            mode: raw.mode,
            rounding: raw.rounding,
            categories: raw.categories,
            letter_bands: raw.letter_bands,
        };
        scheme.validate().map_err(serde::de::Error::custom)?;
        Ok(scheme)
    }
}

/// A scheme configuration refused before it can influence a grade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseGradeSchemeError {
    InvalidCategoryTitle,
    InvalidLetterBandLabel,
    TotalPointsRequiresEmptyCategories,
    EmptyCategories,
    ZeroCategoryWeight { category: GradeCategoryId },
    DuplicateCategory { category: GradeCategoryId },
    NonCanonicalCategoryPosition { expected: u32, actual: u32 },
    CategoryWeightsMustSumToTenThousand { total: u32 },
    DuplicateLetterBand { label: LetterBandLabel },
    LetterBandThresholdOutOfRange { threshold: u16 },
    LetterBandsMustDescend,
}

impl std::fmt::Display for CourseGradeSchemeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid course grade scheme: {self:?}")
    }
}
impl std::error::Error for CourseGradeSchemeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(id: u128, position: u32, weight_basis_points: u16) -> WeightedGradeCategory {
        WeightedGradeCategory {
            id: GradeCategoryId::from_uuid(Uuid::from_u128(id)),
            title: GradeCategoryTitle::new("Labs").expect("valid title"),
            position,
            weight_basis_points,
            drop_lowest: 0,
        }
    }

    #[test]
    fn serde_accepts_the_closed_flat_weighted_shape() {
        let scheme = CourseGradeScheme {
            mode: CourseGradeMode::WeightedCategories,
            rounding: CourseGradeRoundingRule::default(),
            categories: vec![category(1, 0, 10_000)],
            letter_bands: Vec::new(),
        };
        let json = serde_json::to_string(&scheme).expect("serialize scheme");
        let parsed = serde_json::from_str::<CourseGradeScheme>(&json).expect("parse scheme");
        assert_eq!(parsed, scheme);
    }

    #[test]
    fn serde_rejects_completion_unknown_and_nonflat_shapes() {
        let completion = r#"{"mode":"completionBased","rounding":"fourDecimalPlacesHalfAwayFromZero","categories":[],"letterBands":[]}"#;
        let unknown = r#"{"mode":"totalPoints","rounding":"fourDecimalPlacesHalfAwayFromZero","categories":[],"letterBands":[],"extra":true}"#;
        let nested = r#"{"mode":{"weightedCategories":{"categories":[]}},"rounding":"fourDecimalPlacesHalfAwayFromZero","categories":[],"letterBands":[]}"#;
        assert!(serde_json::from_str::<CourseGradeScheme>(completion).is_err());
        assert!(serde_json::from_str::<CourseGradeScheme>(unknown).is_err());
        assert!(serde_json::from_str::<CourseGradeScheme>(nested).is_err());
    }

    #[test]
    fn validation_requires_exact_category_and_band_contracts() {
        let mut scheme = CourseGradeScheme {
            mode: CourseGradeMode::WeightedCategories,
            rounding: CourseGradeRoundingRule::default(),
            categories: vec![category(1, 1, 9_999)],
            letter_bands: vec![LetterBand {
                label: LetterBandLabel::new("A").expect("valid label"),
                minimum_basis_points: 10_001,
            }],
        };
        assert!(matches!(
            scheme.validate(),
            Err(CourseGradeSchemeError::NonCanonicalCategoryPosition { .. })
        ));
        scheme.categories[0].position = 0;
        assert!(matches!(
            scheme.validate(),
            Err(CourseGradeSchemeError::CategoryWeightsMustSumToTenThousand { .. })
        ));
        scheme.categories[0].weight_basis_points = 10_000;
        assert!(matches!(
            scheme.validate(),
            Err(CourseGradeSchemeError::LetterBandThresholdOutOfRange { .. })
        ));
    }

    #[test]
    fn total_outcome_uses_closed_camel_case_browser_shape() {
        let view = CourseGradeOutcomeView::Available {
            score: 0.875,
            letter: Some("B+".to_string()),
            dropped_assignment_ids: Vec::new(),
            total_earned: Some(17.5),
            total_possible: Some(20.0),
        };
        let value = serde_json::to_value(view).expect("view serializes");
        assert_eq!(value["status"], "available");
        assert!(value.get("droppedAssignmentIds").is_some());
        assert!(value.get("totalEarned").is_some());
        assert!(value.get("totalPossible").is_some());
        let mut invalid = value;
        invalid["surprise"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CourseGradeOutcomeView>(invalid).is_err());
    }

    #[test]
    fn scheme_update_cannot_decode_read_only_assignment_titles() {
        let value = serde_json::json!({
            "scheme": {"mode":"totalPoints","rounding":"fourDecimalPlacesHalfAwayFromZero","categories":[],"letterBands":[]},
            "assignments": [{"assignment":"00000000-0000-0000-0000-000000000001","title":"Cannot write this","included":true,"category":null,"position":null}]
        });
        assert!(serde_json::from_value::<CourseGradeSchemeUpdateView>(value).is_err());
    }
}
