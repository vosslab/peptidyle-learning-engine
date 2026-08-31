//! Browser-safe shared-catalog metadata (MOD-API-CAT).

use serde::{Deserialize, Serialize};

use crate::taxonomy::{License, Tag, TaxonomyTerm};
use crate::{
    ActivityTimestamp, BackendCapabilities, CourseReference, DraftQuestionSource, QuestionMetadata,
    QuestionSource, QuestionVersionNumber,
};

pub use crate::catalog_facets::{
    CatalogAuthorship, CatalogBackendFacet, CatalogBylineFacet, CatalogCapabilityFacet,
    CatalogEvidenceAvailability, CatalogEvidenceFacet, CatalogLicenseFacet, CatalogLicenseValue,
    QuestionTypeFacet, CatalogSearchFacets, CatalogSearchFilter,
    CatalogSearchQuery, CatalogSearchQueryError, CatalogTagFacet, CatalogTaxonomyFacet,
    CatalogTaxonomyFilter, CatalogUsedInMyCourses, CatalogUsedInMyCoursesFacet,
    MAX_CATALOG_BACKEND_FACETS, MAX_CATALOG_BYLINE_FACETS, MAX_CATALOG_BYLINE_FILTERS,
    MAX_CATALOG_QUESTION_TYPE_FACETS, MAX_CATALOG_QUESTION_TYPE_FILTERS,
    MAX_CATALOG_TAG_FACETS, MAX_CATALOG_TAG_FILTERS,
};
pub use crate::response::QuestionType;

/// Maximum taxonomy facet values returned with one bounded catalog page.
pub const MAX_CATALOG_TAXONOMY_FACETS: usize = 64;

/// Maximum own-course rows included with one exact catalog usage detail.
///
/// The aggregate summary remains complete while the named course list stays a
/// compact, visible decision aid rather than an unbounded course inventory.
pub const MAX_CATALOG_OWN_COURSE_USAGES: usize = 100;

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

/// Copyable Question ID accepted by Instructor import and direct lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProblemDisplayRef {
    /// Stable human-facing question identity for authorized direct resolution.
    pub question_id: QuestionId,
}

impl std::fmt::Display for ProblemDisplayRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.question_id)
    }
}

impl std::str::FromStr for ProblemDisplayRef {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            question_id: value.parse()?,
        })
    }
}

/// Exact immutable publication evidence used by storage, delivery, grading,
/// replay, and audit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVersionReference {
    /// Stable Question lineage.
    pub question_id: QuestionId,
    /// Exact immutable version within that Question lineage.
    pub version_number: QuestionVersionNumber,
}

/// Current selection availability for an already published Question Version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "availability",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionVersionAvailability {
    /// Discoverable and eligible for ordinary new selection.
    Available,
    /// Discoverable historical content, ineligible for ordinary new selection,
    /// and retained for authorized stable-ID and exact-pin resolution.
    Archived {
        /// Original deprecation explanation retained for the record.
        reason: String,
    },
}

impl QuestionVersionAvailability {
    /// Whether catalog browsing should include the immutable publication.
    pub fn is_discoverable(&self) -> bool {
        matches!(
            self,
            Self::Available | Self::Archived { .. }
        )
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
    /// Published, deprecated, and archived publications remain resolvable by
    /// their stable identity. Resolution does not make non-Published content
    /// eligible for ordinary new selection.
    pub fn is_resolvable_by_stable_question_id(&self) -> bool {
        matches!(self, Self::Available | Self::Archived { .. })
    }
}

/// Question Backend without source paths or package identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionBackend {
    /// First-party Rust/WASM question.
    Native,
    /// WeBWorK PG question.
    Webwork,
    /// IMS QTI item.
    Qti,
    /// Ungraded H5P activity.
    H5p,
    /// iMathAS item served through a verified server-side broker.
    Imathas,
}

impl QuestionBackend {
    /// Every browser-safe Question Backend supported by this release.
    pub const ALL: [Self; 5] = [
        Self::Native,
        Self::Webwork,
        Self::Qti,
        Self::H5p,
        Self::Imathas,
    ];

    /// Canonical public wire value for this closed backend vocabulary.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Webwork => "webwork",
            Self::Qti => "qti",
            Self::H5p => "h5p",
            Self::Imathas => "imathas",
        }
    }
}

impl From<&QuestionSource> for QuestionBackend {
    fn from(source: &QuestionSource) -> Self {
        match source {
            QuestionSource::Native { .. } => Self::Native,
            QuestionSource::Webwork { .. } => Self::Webwork,
            QuestionSource::Qti { .. } => Self::Qti,
            QuestionSource::H5p { .. } => Self::H5p,
            QuestionSource::Imathas { .. } => Self::Imathas,
        }
    }
}

impl From<&DraftQuestionSource> for QuestionBackend {
    fn from(source: &DraftQuestionSource) -> Self {
        match source {
            DraftQuestionSource::Native { .. } => Self::Native,
            DraftQuestionSource::Webwork { .. } => Self::Webwork,
            DraftQuestionSource::Qti { .. } => Self::Qti,
            DraftQuestionSource::H5p { .. } => Self::H5p,
            DraftQuestionSource::Imathas { .. } => Self::Imathas,
        }
    }
}

/// Hot catalog metadata returned by browse endpoints without loading payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogProblemSummary {
    /// Sole human-facing identity of this immutable published question.
    pub question_id: QuestionId,
    /// Question Backend, without private source-locator fields.
    pub backend: QuestionBackend,
    /// Immutable browser-safe Question Type derived at publication time.
    pub question_type: QuestionType,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Shared metadata used for title, taxonomy, license, and language facets.
    pub metadata: QuestionMetadata,
    /// Immutable reviewed publication attribution; never account authority.
    pub byline: crate::PublicByline,
    /// Current availability for ordinary new selection; publication itself is
    /// separate immutable history.
    pub availability: QuestionVersionAvailability,
    /// Database-authoritative publication time.
    pub published_at: ActivityTimestamp,
}

impl CatalogProblemSummary {
    /// Free-form tags for filtering without loading the question payload.
    pub fn tags(&self) -> &[Tag] {
        &self.metadata.tags
    }

    /// Controlled terms for taxonomy aggregation and filtering.
    pub fn taxonomy(&self) -> &[TaxonomyTerm] {
        &self.metadata.taxonomy
    }

    /// License facet for reuse decisions.
    pub fn license(&self) -> &License {
        &self.metadata.license
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
pub enum CatalogDiscoveryEvidence {
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
        evidence_at: ActivityTimestamp,
    },
}

/// One context-free search item. Search results contain immutable catalog
/// metadata and anonymous discovery evidence only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogDiscoveryItem {
    /// Exact immutable hot catalog metadata.
    pub summary: CatalogProblemSummary,
    /// Decomposed anonymous evidence suitable for search discovery.
    pub evidence: CatalogDiscoveryEvidence,
}

/// Global and Account-owned usage counts for one exact publication.
///
/// Global totals do not identify a course. Own counts describe only the
/// requesting instructor's visible course references and are expanded by
/// [`CatalogUsageDetail`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogUsageSummary {
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
pub struct CatalogOwnCourseUsage {
    /// Authorized public course locator; it is never authority by itself.
    pub course: CourseReference,
    /// Current course title visible to the requesting instructor.
    pub title: String,
    /// Number of current assignment uses in this course.
    pub assignment_count: u64,
}

/// Bounded exact-detail usage projection for a requesting instructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogUsageDetail {
    /// Scope-explicit aggregate counts for this exact publication.
    pub summary: CatalogUsageSummary,
    /// At most [`MAX_CATALOG_OWN_COURSE_USAGES`] Account-visible course rows.
    pub own_courses: Vec<CatalogOwnCourseUsage>,
    /// Whether additional Account-visible course rows remain beyond this bounded list.
    pub own_courses_truncated: bool,
}

/// Bounded search page with aggregates from the same catalog snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogSearchPage {
    /// At most the request's validated page size of context-free discovery rows.
    pub items: Vec<CatalogDiscoveryItem>,
    /// Opaque continuation token, bound to the normalized query.
    pub next_cursor: Option<String>,
    /// Server-side facet counts; clients must not infer them from `items`.
    pub facets: CatalogSearchFacets,
}

/// Safe immutable content projection for catalog detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CatalogPromptProjection {
    /// The immutable publication contains one fixed prompt.
    Static {
        /// Browser-safe prompt blocks in authored order.
        blocks: Vec<crate::envelope::ContentBlock>,
    },
    /// One deterministic, server-materialized example of a variable prompt.
    GeneratedExample {
        /// Browser-safe prompt blocks with every authored parameter resolved.
        blocks: Vec<crate::envelope::ContentBlock>,
    },
}

/// Safe immutable content projection for catalog detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogProblemDetail {
    /// Exact immutable hot metadata for this publication.
    pub summary: CatalogProblemSummary,
    /// Static content or one server-materialized example; source, response,
    /// randomization, grading, keys, and the preview seed are excluded.
    pub prompt: CatalogPromptProjection,
    /// Explainable anonymous evidence for this exact publication.
    pub evidence: CatalogDiscoveryEvidence,
    /// Bounded current-Account usage evidence for this exact publication.
    pub usage: CatalogUsageDetail,
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
        let reference: ProblemDisplayRef = "7k3m9qx".parse().expect("Question ID parses");
        assert_eq!(reference.question_id.to_string(), "7K3-M9QX");
        assert_eq!(reference.to_string(), "7K3-M9QX");

        for invalid in ["P-123456", "P-12-v3", "7K3-M9Q", "7K3-M9QU"] {
            assert!(invalid.parse::<ProblemDisplayRef>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn catalog_text_recognizes_one_human_question_id_without_a_version() {
        let exact = CatalogSearchQuery {
            text: Some(" 7k3-m9qx ".to_string()),
            ..CatalogSearchQuery::default()
        }
        .normalized()
        .expect("search text normalizes")
        .exact_question_id()
        .expect("Question ID is recognized");
        assert_eq!(exact.to_string(), "7K3-M9QX");

        let old_versioned = CatalogSearchQuery {
            text: Some("P-70-v1".to_string()),
            ..CatalogSearchQuery::default()
        }
        .normalized()
        .expect("search text normalizes");
        assert_eq!(old_versioned.exact_question_id(), None);
    }

    #[test]
    fn backend_summary_never_carries_private_source_locators() {
        assert_eq!(
            QuestionBackend::from(&QuestionSource::Webwork {
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
            ["native", "webwork", "qti", "h5p", "imathas"]
        );
    }

    #[test]
    fn only_available_content_is_eligible_for_ordinary_new_selection() {
        assert!(QuestionVersionAvailability::Available.is_discoverable());
        assert!(QuestionVersionAvailability::Available.is_eligible_for_ordinary_new_selection());
        assert!(QuestionVersionAvailability::Available.is_resolvable_by_stable_question_id());
        let archived = QuestionVersionAvailability::Archived {
            reason: "Historical".to_string(),
        };
        assert!(archived.is_discoverable());
        assert!(!archived.is_eligible_for_ordinary_new_selection());
        assert!(archived.is_resolvable_by_stable_question_id());
    }

    #[test]
    fn catalog_search_normalizes_equivalent_filters_and_bounds_hostile_input() {
        let query = CatalogSearchQuery {
            text: Some("  Peptide\tBond  ".to_string()),
            bylines: vec![
                "  Dr. Ada  Lovelace ".to_string(),
                "dr. ada lovelace".to_string(),
            ],
            backends: vec![QuestionBackend::Native, QuestionBackend::Native],
            tags: vec![
                " Protein   Structure ".to_string(),
                "protein structure".to_string(),
            ],
            question_types: vec![QuestionType::MultipleChoice; 2],
            taxonomy: vec![
                CatalogTaxonomyFilter {
                    scheme: "  discipline ".to_string(),
                    code: " core ".to_string(),
                },
                CatalogTaxonomyFilter {
                    scheme: "discipline".to_string(),
                    code: "core".to_string(),
                },
            ],
            capabilities: vec![Capability::Hints, Capability::Hints],
            licenses: vec![CatalogLicenseValue::CcBy, CatalogLicenseValue::CcBy],
            ..CatalogSearchQuery::default()
        }
        .normalized()
        .expect("equivalent filters normalize");
        assert_eq!(query.text.as_deref(), Some("peptide bond"));
        assert_eq!(query.bylines, vec!["dr. ada lovelace"]);
        assert_eq!(query.backends, vec![QuestionBackend::Native]);
        assert_eq!(query.tags, vec!["protein structure"]);
        assert_eq!(
            query.question_types,
            vec![QuestionType::MultipleChoice]
        );
        assert_eq!(query.taxonomy.len(), 1);
        assert_eq!(query.capabilities, vec![Capability::Hints]);
        assert_eq!(query.licenses, vec![CatalogLicenseValue::CcBy]);
        assert!(
            CatalogSearchQuery {
                text: Some("x".repeat(257)),
                ..CatalogSearchQuery::default()
            }
            .normalized()
            .is_err()
        );
    }

    #[test]
    fn catalog_detail_wire_shape_has_no_source_or_grading_fields() {
        let detail = CatalogProblemDetail {
            summary: CatalogProblemSummary {
                question_id: "7K3-M9QX".parse().expect("fixture Question ID parses"),
                backend: QuestionBackend::Native,
                question_type: QuestionType::MultipleChoice,
                capabilities: BackendCapabilities::none(),
                metadata: QuestionMetadata {
                    title: "Safe detail".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::Cc0,
                    language: "en".to_string(),
                },
                byline: crate::PublicByline::new(vec![
                    crate::PublicAuthorName::new("Fixture Author".to_string())
                        .expect("valid byline"),
                ])
                .expect("valid byline"),
                availability: QuestionVersionAvailability::Available,
                published_at: ActivityTimestamp::from_unix_millis(0),
            },
            prompt: CatalogPromptProjection::Static { blocks: Vec::new() },
            evidence: CatalogDiscoveryEvidence::InsufficientEvidence,
            usage: CatalogUsageDetail {
                summary: CatalogUsageSummary {
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
        assert!(wire.get("randomization").is_none());
        assert!(wire.get("seed").is_none());
        assert!(wire.get("grading").is_none());
        assert!(wire.get("answerKey").is_none());
    }

    #[test]
    fn discovery_evidence_serializes_as_a_closed_explainable_union() {
        assert_eq!(
            serde_json::to_value(CatalogDiscoveryEvidence::InsufficientEvidence)
                .expect("insufficient evidence serializes"),
            serde_json::json!({ "state": "insufficientEvidence" })
        );
        assert_eq!(
            serde_json::to_value(CatalogDiscoveryEvidence::Available {
                formula_version: 1,
                observed_course_count: 2,
                independent_learner_observation_count: 5,
                difficulty_index: 0.7,
                attempts_mean: 1.2,
                time_median_seconds_estimate: 30,
                discrimination_index: None,
                evidence_at: ActivityTimestamp::from_unix_millis(0),
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
