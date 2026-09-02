//! Browser-safe shared Question Library metadata (MOD-API-CAT).

use serde::{Deserialize, Serialize};

use crate::classification::{QuestionClassification, QuestionLicense, Tag};
use crate::{
    CourseInstanceReference, DraftQuestionBackendLocator, QuestionBackendCapabilities,
    QuestionBackendLocator, QuestionMetadata, QuestionRevisionNumber, Timestamp,
};

pub use crate::question_search::{
    MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS, MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS,
    MAX_QUESTION_SEARCH_BACKEND_FACETS, MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS,
    MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS, MAX_QUESTION_SEARCH_TAG_FACETS,
    MAX_QUESTION_SEARCH_TAG_FILTERS, QuestionSearchAuthorFacet, QuestionSearchAuthorship,
    QuestionSearchBackendFacet, QuestionSearchCapabilityFacet, QuestionSearchClassificationFacet,
    QuestionSearchClassificationFilter, QuestionSearchCourseUse, QuestionSearchCourseUseFacet,
    QuestionSearchFacets, QuestionSearchFilter, QuestionSearchQuestionLicenseFacet,
    QuestionSearchRequest, QuestionSearchRequestError, QuestionSearchTagFacet,
    QuestionStatisticsAvailability, QuestionStatisticsAvailabilityFacet, QuestionTypeFacet,
};
pub use crate::response::QuestionType;

/// Maximum Question Classification values returned with one bounded Question Search page.
pub const MAX_QUESTION_SEARCH_CLASSIFICATION_FACETS: usize = 64;

/// Maximum own-course rows included with one exact Question Library usage detail.
///
/// The aggregate summary remains complete while the named course list stays a
/// compact, visible decision aid rather than an unbounded course inventory.
pub const MAX_QUESTION_SEARCH_OWN_COURSE_USAGES: usize = 100;

/// Crockford Base32 alphabet used by the one human-facing Question ID.
pub const QUESTION_ID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Number of random identity characters before the validation character.
pub const QUESTION_ID_IDENTIFIER_LENGTH: usize = 6;

/// Total compact Question ID length, including its validation character.
pub const QUESTION_ID_COMPACT_LENGTH: usize = 7;

/// Product limit kept independent of the larger encoded namespace.
pub const MAX_QUESTION_ID_COUNT: u64 = 100_000_000;

/// One stable, non-sequential human-facing identity for an immutable published
/// question.
///
/// The canonical display is `AAA-BBBB`. Parsing accepts unhyphenated and
/// lowercase Crockford input plus the documented `O` to `0` and `I`/`L` to
/// `1` transcription aliases. This type validates syntax only; the server-held
/// HMAC secret validates the final character before resolution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionId(String);

impl QuestionId {
    /// Canonical seven-character storage value without the display hyphen.
    pub fn compact(&self) -> String {
        self.0
            .chars()
            .filter(|character| *character != '-')
            .collect()
    }

    /// Returns the six-character identity without allocating.
    pub fn identifier_compact(&self) -> String {
        self.compact()[..QUESTION_ID_IDENTIFIER_LENGTH].to_string()
    }

    /// Canonical validation character.
    pub fn validation_character(&self) -> char {
        self.0.as_bytes()[7] as char
    }

    /// Builds a canonical ID from server-generated canonical components.
    pub fn from_canonical_parts(identifier: &str, validation: char) -> Result<Self, &'static str> {
        if identifier.len() != QUESTION_ID_IDENTIFIER_LENGTH
            || !identifier
                .bytes()
                .all(|character| QUESTION_ID_ALPHABET.contains(&character))
            || !validation.is_ascii()
            || !QUESTION_ID_ALPHABET.contains(&(validation as u8))
        {
            return Err("question ID components are not canonical Crockford Base32");
        }
        Ok(Self(format!(
            "{}-{}{}",
            &identifier[..3],
            &identifier[3..],
            validation
        )))
    }
}

impl std::fmt::Display for QuestionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::str::FromStr for QuestionId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        if trimmed.contains('-')
            && (trimmed.chars().count() != 8
                || trimmed.chars().nth(3) != Some('-')
                || trimmed
                    .chars()
                    .filter(|character| *character == '-')
                    .count()
                    != 1)
        {
            return Err("question ID hyphen must use the canonical 3-4 grouping");
        }
        let normalized: String = trimmed
            .chars()
            .filter(|character| *character != '-')
            .map(|character| match character.to_ascii_uppercase() {
                'O' => '0',
                'I' | 'L' => '1',
                other => other,
            })
            .collect();
        if normalized.len() != QUESTION_ID_COMPACT_LENGTH
            || !normalized
                .bytes()
                .all(|character| QUESTION_ID_ALPHABET.contains(&character))
        {
            return Err("question ID must contain seven Crockford Base32 characters");
        }
        Self::from_canonical_parts(&normalized[..6], normalized.as_bytes()[6] as char)
    }
}

impl TryFrom<String> for QuestionId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<QuestionId> for String {
    fn from(value: QuestionId) -> Self {
        value.0
    }
}

/// Exact immutable publication evidence used by storage, delivery, grading,
/// replay, and audit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRevisionReference {
    /// Stable Question lineage.
    pub question_id: QuestionId,
    /// Exact immutable version within that Question lineage.
    pub revision_number: QuestionRevisionNumber,
}

/// Current selection availability for an already published Question Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "availability",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionRevisionAvailability {
    /// Discoverable and eligible for ordinary new selection.
    Available,
    /// Discoverable historical content, ineligible for ordinary new selection,
    /// and retained for authorized stable-ID and exact-pin resolution.
    Archived {
        /// Archived-availability explanation retained for the record.
        reason: String,
    },
}

impl QuestionRevisionAvailability {
    /// Whether Question Library browsing should include the immutable publication.
    pub fn is_discoverable(&self) -> bool {
        matches!(self, Self::Available | Self::Archived { .. })
    }

    /// Whether this publication can create a new reference through ordinary selection.
    ///
    /// This does not govern resolution of an existing exact immutable pin.
    pub fn is_eligible_for_ordinary_new_selection(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Whether a stable Question ID can resolve this publication for an
    /// authorized read.
    ///
    /// Published Question Revisions remain resolvable by their stable identity.
    /// Resolution does not make an Archived Question Revision eligible for
    /// ordinary new selection.
    pub fn is_resolvable_by_stable_question_id(&self) -> bool {
        matches!(self, Self::Available | Self::Archived { .. })
    }
}

/// Question Backend without source paths or package identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionBackend {
    /// First-party Rust/WASM question.
    Ple,
    /// WeBWorK PG question.
    Webwork,
    /// IMS QTI item.
    Qti,
    /// Ungraded H5P activity.
    H5p,
    /// iMathAS item served through the verified iMathAS Question Backend Transport.
    Imathas,
}

impl QuestionBackend {
    /// Every browser-safe Question Backend supported by this release.
    pub const ALL: [Self; 5] = [
        Self::Ple,
        Self::Webwork,
        Self::Qti,
        Self::H5p,
        Self::Imathas,
    ];

    /// Canonical public wire value for this closed backend vocabulary.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ple => "ple",
            Self::Webwork => "webwork",
            Self::Qti => "qti",
            Self::H5p => "h5p",
            Self::Imathas => "imathas",
        }
    }
}

impl From<&QuestionBackendLocator> for QuestionBackend {
    fn from(source: &QuestionBackendLocator) -> Self {
        match source {
            QuestionBackendLocator::Ple => Self::Ple,
            QuestionBackendLocator::Webwork { .. } => Self::Webwork,
            QuestionBackendLocator::Qti { .. } => Self::Qti,
            QuestionBackendLocator::H5p { .. } => Self::H5p,
            QuestionBackendLocator::Imathas { .. } => Self::Imathas,
        }
    }
}

impl From<&DraftQuestionBackendLocator> for QuestionBackend {
    fn from(source: &DraftQuestionBackendLocator) -> Self {
        match source {
            DraftQuestionBackendLocator::Ple => Self::Ple,
            DraftQuestionBackendLocator::Webwork { .. } => Self::Webwork,
            DraftQuestionBackendLocator::Qti { .. } => Self::Qti,
            DraftQuestionBackendLocator::H5p { .. } => Self::H5p,
            DraftQuestionBackendLocator::Imathas { .. } => Self::Imathas,
        }
    }
}

/// Question Library summary metadata returned by browse endpoints without loading payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSummary {
    /// Sole human-facing identity of this immutable published question.
    pub question_id: QuestionId,
    /// Exact accepted Question Revision with the greatest revision number in
    /// this Question lineage. This is independent of selection availability.
    pub latest_question_revision: QuestionRevisionReference,
    /// Question Backend, without private source-locator fields.
    pub backend: QuestionBackend,
    /// Immutable browser-safe Question Type derived at publication time.
    pub question_type: QuestionType,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: QuestionBackendCapabilities,
    /// Shared metadata used for title, Question Classification, Question License, and language facets.
    pub metadata: QuestionMetadata,
    /// Immutable reviewed Question Authorship display snapshot; never Question Owner authority.
    pub authorship: crate::QuestionAuthorship,
    /// Current availability for ordinary new selection; publication itself is
    /// separate immutable history.
    pub availability: QuestionRevisionAvailability,
    /// Database-authoritative publication time.
    pub published_at: Timestamp,
}

impl QuestionSummary {
    /// Free-form tags for filtering without loading the question payload.
    pub fn tags(&self) -> &[Tag] {
        &self.metadata.tags
    }

    /// Exact Question Classifications for aggregation and filtering.
    pub fn classifications(&self) -> &[QuestionClassification] {
        &self.metadata.classifications
    }

    /// Question License facet for reuse decisions.
    pub fn question_license(&self) -> Option<&QuestionLicense> {
        self.metadata.question_license.as_ref()
    }
}

/// Explainable, privacy-governed discovery evidence for one exact publication.
///
/// The evidence contains only disclosed aggregate observations. It never
/// exposes a ranking contribution, student work, or a course identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum QuestionStatistics {
    /// The publication has no valid disclosed aggregate yet. This is neither
    /// a favorable nor unfavorable quality judgment.
    InsufficientEvidence,
    /// A versioned formula produced a safely disclosed aggregate observation.
    Available {
        /// Server-owned version of the disclosed evidence formula.
        formula_version: u16,
        /// Number of anonymous courses observed by the formula.
        observed_course_count: u64,
        /// Independent first-valid student observations for this exact publication.
        independent_learner_observation_count: u64,
        /// Mean normalized score, in the inclusive range `0.0..=1.0`.
        difficulty_index: f64,
        /// Mean submitted attempts represented by one first-run observation.
        attempts_mean: f64,
        /// Fixed-histogram estimate of the median response duration in seconds.
        time_median_seconds_estimate: u64,
        /// Correlation of question score with rest-of-run score when valid.
        #[serde(skip_serializing_if = "Option::is_none")]
        discrimination_index: Option<f64>,
        /// Database-authoritative time at which this evidence was computed.
        evidence_at: Timestamp,
    },
}

/// One context-free search item. Search results contain immutable Question Library
/// metadata and anonymous discovery evidence only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchResult {
    /// Exact immutable Question Library summary metadata.
    pub summary: QuestionSummary,
    /// Decomposed anonymous evidence suitable for search discovery.
    pub evidence: QuestionStatistics,
}

/// Global and Account-owned usage counts for one exact publication.
///
/// Global totals do not identify a course. Own counts describe only the
/// requesting instructor's visible course references and are expanded by
/// [`QuestionUseDetails`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionUseSummary {
    /// Installation-wide number of distinct courses with an assignment use.
    pub global_course_count: u64,
    /// Installation-wide number of assignment uses.
    pub global_assignment_count: u64,
    /// Current Account's distinct courses that use this publication.
    pub own_course_count: u64,
    /// Current Account's assignment uses across their visible courses.
    pub own_assignment_count: u64,
}

/// One current Account-visible course using an exact publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseQuestionUse {
    /// Authorized public course locator; it is never authority by itself.
    pub course: CourseInstanceReference,
    /// Current course title visible to the requesting instructor.
    pub title: String,
    /// Number of current assignment uses in this course.
    pub assignment_count: u64,
}

/// Bounded exact-detail usage projection for a requesting instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionUseDetails {
    /// Scope-explicit aggregate counts for this exact publication.
    pub summary: QuestionUseSummary,
    /// At most [`MAX_QUESTION_SEARCH_OWN_COURSE_USAGES`] Account-visible course rows.
    pub own_courses: Vec<CourseQuestionUse>,
    /// Whether additional Account-visible course rows remain beyond this bounded list.
    pub own_courses_truncated: bool,
}

/// Bounded search page with aggregates from the same Question Search snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchPage {
    /// At most the request's validated page size of context-free discovery rows.
    pub items: Vec<QuestionSearchResult>,
    /// Opaque continuation token, bound to the normalized query.
    pub next_cursor: Option<String>,
    /// Server-side facet counts; clients must not infer them from `items`.
    pub facets: QuestionSearchFacets,
}

/// Safe immutable content projection for Question Library detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum QuestionPromptProjection {
    /// The immutable publication contains one fixed prompt.
    Static {
        /// Browser-safe prompt blocks in authored order.
        blocks: Vec<crate::envelope::QuestionContentBlock>,
    },
    /// One deterministic, server-generated example of a variable prompt.
    GeneratedExample {
        /// Browser-safe prompt blocks with every authored parameter resolved.
        blocks: Vec<crate::envelope::QuestionContentBlock>,
    },
}

/// Safe immutable content projection for Question Library detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionDetails {
    /// Exact immutable hot metadata for this publication.
    pub summary: QuestionSummary,
    /// Static content or one server-generated example; source, response,
    /// Question Variation Definition, grading, keys, and preview seed are excluded.
    pub prompt: QuestionPromptProjection,
    /// Explainable anonymous evidence for this exact publication.
    pub evidence: QuestionStatistics,
    /// Bounded current-Account usage evidence for this exact publication.
    pub usage: QuestionUseDetails,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Capability;

    #[test]
    fn question_ids_normalize_forgiving_input_without_accepting_other_characters() {
        let canonical: QuestionId = "7K3-M9QX".parse().expect("canonical ID parses");
        assert_eq!(canonical.to_string(), "7K3-M9QX");
        assert_eq!(canonical.compact(), "7K3M9QX");
        assert_eq!(canonical.identifier_compact(), "7K3M9Q");
        assert_eq!(canonical.validation_character(), 'X');
        assert_eq!(
            "7k3m9qx".parse::<QuestionId>().expect("lowercase parses"),
            canonical
        );
        assert_eq!(
            "o11-1lix"
                .parse::<QuestionId>()
                .expect("aliases parse")
                .to_string(),
            "011-111X"
        );
        for invalid in ["7K3-M9Q", "7K3-M9QXX", "7K3-M9QU", "7K3 M9QX"] {
            assert!(invalid.parse::<QuestionId>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn question_id_wire_uses_the_canonical_display_form() {
        let identifier: QuestionId = "7k3m9qx".parse().expect("ID parses");
        assert_eq!(
            serde_json::to_value(&identifier).expect("ID serializes"),
            serde_json::json!("7K3-M9QX")
        );
        assert_eq!(
            serde_json::from_value::<QuestionId>(serde_json::json!("7k3-m9qx"))
                .expect("wire aliases normalize"),
            identifier
        );
    }

    #[test]
    fn human_question_references_are_unambiguous_and_version_free() {
        let question_id: QuestionId = "7k3m9qx".parse().expect("Question ID parses");
        assert_eq!(question_id.to_string(), "7K3-M9QX");

        for invalid in ["P-123456", "P-12-v3", "7K3-M9Q", "7K3-M9QU"] {
            assert!(invalid.parse::<QuestionId>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn question_id_text_recognizes_one_human_question_id_without_a_revision() {
        let exact = QuestionSearchRequest {
            text: Some(" 7k3-m9qx ".to_string()),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("search text normalizes")
        .exact_question_id()
        .expect("Question ID is recognized");
        assert_eq!(exact.to_string(), "7K3-M9QX");

        let old_versioned = QuestionSearchRequest {
            text: Some("P-70-v1".to_string()),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("search text normalizes");
        assert_eq!(old_versioned.exact_question_id(), None);
    }

    #[test]
    fn backend_summary_never_carries_private_source_locators() {
        assert_eq!(
            QuestionBackend::from(&QuestionBackendLocator::Webwork {
                pg_path: "OpenProblemLibrary/private/path.pg".to_string(),
            }),
            QuestionBackend::Webwork
        );
        assert_eq!(
            serde_json::to_string(&QuestionBackend::Webwork).expect("backend serializes"),
            "\"webwork\""
        );
        assert_eq!(
            QuestionBackend::ALL.map(QuestionBackend::as_str),
            ["ple", "webwork", "qti", "h5p", "imathas"]
        );
    }

    #[test]
    fn only_available_content_is_eligible_for_ordinary_new_selection() {
        assert!(QuestionRevisionAvailability::Available.is_discoverable());
        assert!(QuestionRevisionAvailability::Available.is_eligible_for_ordinary_new_selection());
        assert!(QuestionRevisionAvailability::Available.is_resolvable_by_stable_question_id());
        let archived = QuestionRevisionAvailability::Archived {
            reason: "Historical".to_string(),
        };
        assert!(archived.is_discoverable());
        assert!(!archived.is_eligible_for_ordinary_new_selection());
        assert!(archived.is_resolvable_by_stable_question_id());
    }

    #[test]
    fn question_search_normalizes_equivalent_filters_and_bounds_hostile_input() {
        let query = QuestionSearchRequest {
            text: Some("  Peptide\tBond  ".to_string()),
            author_names: vec![
                "  Dr. Ada  Lovelace ".to_string(),
                "dr. ada lovelace".to_string(),
            ],
            backends: vec![QuestionBackend::Ple, QuestionBackend::Ple],
            tags: vec![
                " Protein   Structure ".to_string(),
                "protein structure".to_string(),
            ],
            question_types: vec![QuestionType::MultipleChoice; 2],
            classifications: vec![
                QuestionSearchClassificationFilter {
                    system: "  discipline ".to_string(),
                    code: " core ".to_string(),
                },
                QuestionSearchClassificationFilter {
                    system: "discipline".to_string(),
                    code: "core".to_string(),
                },
            ],
            capabilities: vec![Capability::Hints, Capability::Hints],
            question_licenses: vec![QuestionLicense::CcBy4_0, QuestionLicense::CcBy4_0],
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("equivalent filters normalize");
        assert_eq!(query.text.as_deref(), Some("peptide bond"));
        assert_eq!(query.author_names, vec!["dr. ada lovelace"]);
        assert_eq!(query.backends, vec![QuestionBackend::Ple]);
        assert_eq!(query.tags, vec!["protein structure"]);
        assert_eq!(query.question_types, vec![QuestionType::MultipleChoice]);
        assert_eq!(query.classifications.len(), 1);
        assert_eq!(query.capabilities, vec![Capability::Hints]);
        assert_eq!(query.question_licenses, vec![QuestionLicense::CcBy4_0]);
        assert!(
            QuestionSearchRequest {
                text: Some("x".repeat(257)),
                ..QuestionSearchRequest::default()
            }
            .normalized()
            .is_err()
        );
    }

    #[test]
    fn question_library_detail_wire_shape_has_no_source_or_grading_fields() {
        let detail = QuestionDetails {
            summary: QuestionSummary {
                question_id: "7K3-M9QX".parse().expect("fixture Question ID parses"),
                latest_question_revision: QuestionRevisionReference {
                    question_id: "7K3-M9QX".parse().expect("fixture Question ID parses"),
                    revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
                },
                backend: QuestionBackend::Ple,
                question_type: QuestionType::MultipleChoice,
                capabilities: QuestionBackendCapabilities::none(),
                metadata: QuestionMetadata {
                    title: "Safe detail".to_string(),
                    question_description: "Instructor-facing safe detail fixture summary."
                        .to_string(),
                    tags: Vec::new(),
                    classifications: Vec::new(),
                    question_license: Some(QuestionLicense::Cc0_1_0),
                    question_citation: None,
                    language: "en".to_string(),
                },
                authorship: crate::QuestionAuthorship::new(vec![crate::QuestionAuthor {
                    display_name: crate::QuestionAuthorDisplayName::new(
                        "Fixture Author".to_string(),
                    )
                    .expect("valid Question Author"),
                }])
                .expect("valid Question Authorship"),
                availability: QuestionRevisionAvailability::Available,
                published_at: Timestamp::from_unix_millis(0),
            },
            prompt: QuestionPromptProjection::Static { blocks: Vec::new() },
            evidence: QuestionStatistics::InsufficientEvidence,
            usage: QuestionUseDetails {
                summary: QuestionUseSummary {
                    global_course_count: 0,
                    global_assignment_count: 0,
                    own_course_count: 0,
                    own_assignment_count: 0,
                },
                own_courses: Vec::new(),
                own_courses_truncated: false,
            },
        };
        let wire = serde_json::to_value(detail).expect("detail serializes");
        assert!(wire.get("source").is_none());
        assert!(wire.get("response").is_none());
        assert!(wire.get("questionVariationRule").is_none());
        assert!(wire.get("seed").is_none());
        assert!(wire.get("grading").is_none());
        assert!(wire.get("answerKey").is_none());
    }

    #[test]
    fn discovery_evidence_serializes_as_a_closed_explainable_union() {
        assert_eq!(
            serde_json::to_value(QuestionStatistics::InsufficientEvidence)
                .expect("insufficient evidence serializes"),
            serde_json::json!({ "state": "insufficientEvidence" })
        );
        assert_eq!(
            serde_json::to_value(QuestionStatistics::Available {
                formula_version: 1,
                observed_course_count: 2,
                independent_learner_observation_count: 5,
                difficulty_index: 0.7,
                attempts_mean: 1.2,
                time_median_seconds_estimate: 30,
                discrimination_index: None,
                evidence_at: Timestamp::from_unix_millis(0),
            })
            .expect("available evidence serializes"),
            serde_json::json!({
                "state": "available",
                "formulaVersion": 1,
                "observedCourseCount": 2,
                "independentLearnerObservationCount": 5,
                "difficultyIndex": 0.7,
                "attemptsMean": 1.2,
                "timeMedianSecondsEstimate": 30,
                "evidenceAt": 0
            })
        );
    }
}
