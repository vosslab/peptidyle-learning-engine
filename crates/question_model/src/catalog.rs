//! Browser-safe shared-catalog metadata (MOD-API-CAT).

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use crate::taxonomy::{License, Tag, TaxonomyTerm};
use crate::{
    ActivityTimestamp, BackendCapabilities, Capability, DraftQuestionSource, ProblemId,
    QuestionMetadata, QuestionSource, QuestionStatisticsView, UserId, VersionId,
};

/// Maximum taxonomy facet values returned with one bounded catalog page.
pub const MAX_CATALOG_TAXONOMY_FACETS: usize = 64;

/// Crockford Base32 alphabet used by the one human-facing Question ID.
pub const QUESTION_ID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Number of random identity characters before the validation character.
pub const QUESTION_ID_IDENTIFIER_LENGTH: usize = 6;

/// Total compact Question ID length, including its validation character.
pub const QUESTION_ID_COMPACT_LENGTH: usize = 7;

/// Product limit kept independent of the larger encoded namespace.
pub const MAX_QUESTION_ID_COUNT: u64 = 100_000_000;

/// One stable, non-sequential human-facing identity for a current question.
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

/// Largest value for either component of a copyable catalog locator.
///
/// The catalog is scoped to this product, rather than to every object ever
/// created anywhere. A positive 31-bit sequence provides more than two
/// billion stable problems and versions while remaining lossless in the
/// PostgreSQL `bigint` columns, Rust, JSON, and JavaScript's safe-integer
/// `number` representation.
pub const MAX_CATALOG_DISPLAY_NUMBER: u32 = i32::MAX as u32;

/// Copyable decimal identifier for one stable published problem.
///
/// This identifier is intentionally separate from [`ProblemId`]. It is safe
/// to display and search, but never carries authorization authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProblemPublicId(NonZeroU32);

impl ProblemPublicId {
    /// Builds a public identifier from its positive database value.
    pub fn new(value: u64) -> Option<Self> {
        u32::try_from(value)
            .ok()
            .filter(|value| *value <= MAX_CATALOG_DISPLAY_NUMBER)
            .and_then(NonZeroU32::new)
            .map(Self)
    }

    /// Returns the positive decimal value stored by PostgreSQL.
    pub fn value(self) -> u32 {
        self.0.get()
    }
}

impl std::fmt::Display for ProblemPublicId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "P-{}", self.value())
    }
}

impl std::str::FromStr for ProblemPublicId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.strip_prefix("P-").unwrap_or(value);
        if digits.is_empty()
            || digits.len() > 10
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("problem ID must look like P-123456");
        }
        digits
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or("problem ID must be a positive 31-bit decimal value")
    }
}

/// One-based display version within a stable published problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProblemVersionNumber(NonZeroU32);

impl ProblemVersionNumber {
    /// Builds a version number from its positive database value.
    pub fn new(value: u64) -> Option<Self> {
        u32::try_from(value)
            .ok()
            .filter(|value| *value <= MAX_CATALOG_DISPLAY_NUMBER)
            .and_then(NonZeroU32::new)
            .map(Self)
    }

    /// Returns the one-based version number.
    pub fn value(self) -> u32 {
        self.0.get()
    }
}

/// Copyable Question ID accepted by instructor import and direct lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProblemDisplayRef {
    /// Stable human-facing question identity. Hidden versions never enter it.
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

/// Exact immutable problem version used by lineage and assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemVersionRef {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Exact immutable version.
    pub version: VersionId,
}

/// Visibility of immutable published content.
///
/// Private content remains a tenant-owned draft and therefore has no variant
/// here and no `ProblemId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublicationScope {
    /// Discoverable only by the publishing institution.
    Institution,
    /// Discoverable across every tenant.
    Public,
}

/// Catalog state after publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CatalogLifecycle {
    /// Discoverable and eligible for new assignments.
    Published,
    /// Hidden from discovery but still eligible by exact reference.
    Deprecated {
        /// Why instructors should stop newly assigning this version.
        reason: String,
    },
    /// Historical content retained for exact resolution only.
    Archived {
        /// Original deprecation explanation retained for the record.
        reason: String,
    },
}

impl CatalogLifecycle {
    /// Whether catalog browsing should include the version.
    pub fn is_discoverable(&self) -> bool {
        matches!(self, Self::Published)
    }

    /// Whether a new assignment may reference the version.
    pub fn is_assignable(&self) -> bool {
        matches!(self, Self::Published | Self::Deprecated { .. })
    }
}

/// Adapter family without source paths or package identifiers.
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
#[serde(rename_all = "camelCase")]
pub struct CatalogProblemSummary {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Copyable human-facing identity of the current question.
    pub question_id: QuestionId,
    /// Exact immutable version represented by this row.
    pub version: VersionId,
    /// Adapter family, without private source-locator fields.
    pub backend: QuestionBackend,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Shared metadata used for title, taxonomy, license, and language facets.
    pub metadata: QuestionMetadata,
    /// Institution-only or public visibility.
    pub scope: PublicationScope,
    /// Published, deprecated, or archived state.
    pub lifecycle: CatalogLifecycle,
    /// Ordered, nonempty author identifiers controlling the linear chain.
    pub authors: Vec<UserId>,
    /// Earlier version in the same single-writer chain, when this is a revision.
    pub previous_version: Option<VersionId>,
    /// Source version when this problem began as a third-party fork.
    pub derived_from: Option<ProblemVersionRef>,
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

/// One exact controlled-vocabulary term selected in catalog search.
///
/// The label is intentionally absent.  It is presentation metadata, while a
/// `(scheme, code)` pair is the durable term identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogTaxonomyFilter {
    /// Vocabulary namespace.
    pub scheme: String,
    /// Controlled term within the namespace.
    pub code: String,
}

/// Reuse-license values accepted by catalog search.
///
/// `Other` deliberately means the supported "other SPDX" class, rather than
/// accepting an arbitrary browser-provided SPDX string as a query primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogLicenseValue {
    /// All rights reserved.
    AllRightsReserved,
    /// Creative Commons Attribution.
    CcBy,
    /// Creative Commons Attribution-ShareAlike.
    CcBySa,
    /// Creative Commons Attribution-NonCommercial.
    CcByNc,
    /// Public-domain dedication.
    Cc0,
    /// A named SPDX license outside the common set.
    Other,
}

impl CatalogLicenseValue {
    /// Whether this value describes one metadata license.
    pub fn matches(self, license: &License) -> bool {
        matches!(
            (self, license),
            (Self::AllRightsReserved, License::AllRightsReserved)
                | (Self::CcBy, License::CcBy)
                | (Self::CcBySa, License::CcBySa)
                | (Self::CcByNc, License::CcByNc)
                | (Self::Cc0, License::Cc0)
                | (Self::Other, License::Other { .. })
        )
    }

    /// Classifies stored metadata without exposing an SPDX value as a facet key.
    pub fn from_license(license: &License) -> Self {
        match license {
            License::AllRightsReserved => Self::AllRightsReserved,
            License::CcBy => Self::CcBy,
            License::CcBySa => Self::CcBySa,
            License::CcByNc => Self::CcByNc,
            License::Cc0 => Self::Cc0,
            License::Other { .. } => Self::Other,
        }
    }
}

/// Availability filter for k-anonymous catalog statistics.
///
/// Statistics storage lands in MOD-STATS.  Until then every visible result is
/// honestly `Unavailable`; this enum keeps the later aggregate implementation
/// additive without pretending attempt history is catalog metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogStatisticsAvailability {
    /// Include results regardless of aggregate availability.
    #[default]
    Any,
    /// Include only versions with a releasable anonymous aggregate.
    Available,
    /// Include only versions without a releasable anonymous aggregate.
    Unavailable,
}

/// Strict, bounded catalog-search request carried across the browser boundary.
///
/// The server normalizes this value before both paging and aggregation.  The
/// cursor is opaque and tied to that normalized query by the catalog store;
/// positional paging is deliberately not representable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogSearchQuery {
    /// Optional full-text-like text query over hot catalog metadata.
    pub text: Option<String>,
    /// Exact controlled-term filters. Every supplied term must be present.
    pub taxonomy: Vec<CatalogTaxonomyFilter>,
    /// Required adapter capabilities. Every supplied capability must be present.
    pub capabilities: Vec<Capability>,
    /// Accepted license classes. Any supplied value may match.
    pub licenses: Vec<CatalogLicenseValue>,
    /// Whether anonymous aggregate statistics must be available.
    pub statistics: CatalogStatisticsAvailability,
    /// Opaque continuation cursor from this exact normalized query.
    pub cursor: Option<String>,
    /// Requested bounded page size. `None` selects the server default.
    pub page_size: Option<u16>,
}

impl Default for CatalogSearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            taxonomy: Vec::new(),
            capabilities: Vec::new(),
            licenses: Vec::new(),
            statistics: CatalogStatisticsAvailability::Any,
            cursor: None,
            page_size: None,
        }
    }
}

/// Rejection reason for a catalog search request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSearchQueryError {
    /// Text or a controlled-term component was blank after normalization.
    BlankFilter,
    /// A string field or filter collection exceeded the bounded contract.
    TooLarge,
    /// An opaque continuation token was empty.
    EmptyCursor,
}

impl std::fmt::Display for CatalogSearchQueryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankFilter => formatter.write_str("catalog filter must not be blank"),
            Self::TooLarge => formatter.write_str("catalog filter exceeds its bounded limit"),
            Self::EmptyCursor => formatter.write_str("catalog cursor must not be empty"),
        }
    }
}

impl std::error::Error for CatalogSearchQueryError {}

impl CatalogSearchQuery {
    /// Returns the current question named by a human-facing Question ID in the
    /// text field. Catalog text remains case-insensitive, while the ID is
    /// canonicalized before it reaches a store. Hidden snapshot identity never
    /// appears in this search primitive.
    pub fn exact_question_id(&self) -> Option<QuestionId> {
        let text = self.text.as_deref()?;
        text.parse::<QuestionId>().ok()
    }

    /// Produces the canonical query used for both rows and facet aggregates.
    ///
    /// Text is Unicode-lowercased with internal whitespace collapsed.  Exact
    /// controlled terms retain their durable case but have surrounding
    /// whitespace removed.  Sets are sorted and deduplicated so a cursor is
    /// stable across equivalent browser requests.
    pub fn normalized(mut self) -> Result<Self, CatalogSearchQueryError> {
        self.text = self
            .text
            .map(|text| {
                text.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase()
            })
            .filter(|text| !text.is_empty());
        if self
            .text
            .as_ref()
            .is_some_and(|text| text.chars().count() > 256)
        {
            return Err(CatalogSearchQueryError::TooLarge);
        }
        for term in &mut self.taxonomy {
            term.scheme = term.scheme.trim().to_string();
            term.code = term.code.trim().to_string();
            if term.scheme.is_empty() || term.code.is_empty() {
                return Err(CatalogSearchQueryError::BlankFilter);
            }
            if term.scheme.chars().count() > 128 || term.code.chars().count() > 128 {
                return Err(CatalogSearchQueryError::TooLarge);
            }
        }
        if self.taxonomy.len() > 64
            || self.capabilities.len() > Capability::ALL.len()
            || self.licenses.len() > 6
        {
            return Err(CatalogSearchQueryError::TooLarge);
        }
        self.taxonomy.sort();
        self.taxonomy.dedup();
        self.capabilities.sort();
        self.capabilities.dedup();
        self.licenses.sort();
        self.licenses.dedup();
        if self.cursor.as_ref().is_some_and(String::is_empty) {
            return Err(CatalogSearchQueryError::EmptyCursor);
        }
        Ok(self)
    }
}

/// Server-computed count for a controlled taxonomy value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogTaxonomyFacet {
    /// Controlled term identity and display label.
    pub term: TaxonomyTerm,
    /// Number of matching discoverable versions in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCapabilityFacet {
    /// Capability represented by the count.
    pub capability: Capability,
    /// Number of matching discoverable versions in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one license class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogLicenseFacet {
    /// License class represented by the count.
    pub license: CatalogLicenseValue,
    /// Number of matching discoverable versions in the query snapshot.
    pub count: u64,
}

/// Anonymous-statistics availability counts, never attempt-derived values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStatisticsFacet {
    /// Versions whose aggregate is safely releasable. Zero before MOD-STATS.
    pub available: u64,
    /// Versions whose aggregate is suppressed or has not been computed.
    pub unavailable: u64,
}

/// Aggregates computed by the server from the same normalized search snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSearchFacets {
    /// At most [`MAX_CATALOG_TAXONOMY_FACETS`] controlled taxonomy counts,
    /// ordered by count descending and then `(scheme, code)`.
    pub taxonomy: Vec<CatalogTaxonomyFacet>,
    /// Adapter capability counts.
    pub capabilities: Vec<CatalogCapabilityFacet>,
    /// Reuse-license counts.
    pub licenses: Vec<CatalogLicenseFacet>,
    /// Honest k-anonymity availability counts.
    pub statistics: CatalogStatisticsFacet,
}

/// Bounded search page with aggregates from the same catalog snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSearchPage {
    /// At most the request's validated page size of hot metadata rows.
    pub items: Vec<CatalogProblemSummary>,
    /// Opaque continuation token, bound to the normalized query.
    pub next_cursor: Option<String>,
    /// Server-side facet counts; clients must not infer them from `items`.
    pub facets: CatalogSearchFacets,
}

/// Explicit anonymous-statistics status for an exact catalog detail page.
///
/// `Unavailable` preserves the established scalar wire value and deliberately
/// does not distinguish a below-k cohort from an aggregate not yet populated.
/// The available variant carries only the k-anonymity-gated safe projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogStatisticsStatus {
    /// No releasable anonymous aggregate is available yet or the cohort is
    /// below the disclosure threshold.
    Unavailable,
    /// A k-anonymity-gated shared aggregate is safe to disclose.
    Available(QuestionStatisticsView),
}

/// Safe immutable content projection for catalog detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProblemDetail {
    /// Exact immutable hot metadata and lineage.
    pub summary: CatalogProblemSummary,
    /// Authored display content only; source, response, grading, and keys are excluded.
    pub prompt: Vec<crate::envelope::ContentBlock>,
    /// Anonymous-statistics state, intentionally unavailable before MOD-STATS.
    pub statistics: CatalogStatisticsStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn hidden_numeric_storage_identifiers_stay_within_the_lossless_cross_layer_range() {
        let first_out_of_range = u64::from(MAX_CATALOG_DISPLAY_NUMBER) + 1;
        assert!(ProblemPublicId::new(first_out_of_range).is_none());
        assert!(ProblemVersionNumber::new(first_out_of_range).is_none());

        assert_eq!(
            serde_json::to_value(
                ProblemPublicId::new(u64::from(MAX_CATALOG_DISPLAY_NUMBER))
                    .expect("maximum public ID is valid"),
            )
            .expect("public ID serializes"),
            serde_json::json!(MAX_CATALOG_DISPLAY_NUMBER)
        );
        assert_eq!(
            serde_json::to_value(
                ProblemVersionNumber::new(u64::from(MAX_CATALOG_DISPLAY_NUMBER))
                    .expect("maximum version number is valid"),
            )
            .expect("version number serializes"),
            serde_json::json!(MAX_CATALOG_DISPLAY_NUMBER)
        );
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
    }

    #[test]
    fn deprecation_hides_discovery_while_archival_blocks_assignment() {
        assert!(CatalogLifecycle::Published.is_discoverable());
        assert!(CatalogLifecycle::Published.is_assignable());
        let deprecated = CatalogLifecycle::Deprecated {
            reason: "Correction available".to_string(),
        };
        assert!(!deprecated.is_discoverable());
        assert!(deprecated.is_assignable());
        assert!(
            !CatalogLifecycle::Archived {
                reason: "Historical".to_string(),
            }
            .is_assignable()
        );
    }

    #[test]
    fn catalog_search_normalizes_equivalent_filters_and_bounds_hostile_input() {
        let query = CatalogSearchQuery {
            text: Some("  Peptide\tBond  ".to_string()),
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
                problem: ProblemId::from_uuid(uuid::Uuid::from_u128(1)),
                question_id: "7K3-M9QX".parse().expect("fixture Question ID parses"),
                version: VersionId::from_uuid(uuid::Uuid::from_u128(2)),
                backend: QuestionBackend::Native,
                capabilities: BackendCapabilities::none(),
                metadata: QuestionMetadata {
                    title: "Safe detail".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::Cc0,
                    language: "en".to_string(),
                },
                scope: PublicationScope::Public,
                lifecycle: CatalogLifecycle::Published,
                authors: vec![UserId::from_uuid(uuid::Uuid::from_u128(3))],
                previous_version: None,
                derived_from: None,
                published_at: ActivityTimestamp::from_unix_millis(0),
            },
            prompt: Vec::new(),
            statistics: CatalogStatisticsStatus::Unavailable,
        };
        let wire = serde_json::to_value(detail).expect("detail serializes");
        assert!(wire.get("source").is_none());
        assert!(wire.get("response").is_none());
        assert!(wire.get("grading").is_none());
        assert!(wire.get("answerKey").is_none());
    }

    #[test]
    fn statistics_status_preserves_unavailable_wire_and_adds_available() {
        assert_eq!(
            serde_json::to_value(CatalogStatisticsStatus::Unavailable)
                .expect("unavailable status serializes"),
            serde_json::json!("unavailable")
        );
        assert_eq!(
            serde_json::to_value(CatalogStatisticsStatus::Available(QuestionStatisticsView {
                cohort_size: 5,
                difficulty_index: 0.7,
                attempts_mean: 1.2,
                time_median_seconds_estimate: 30,
                discrimination_index: None,
            }))
            .expect("available status serializes"),
            serde_json::json!({
                "available": {
                    "cohortSize": 5,
                    "difficultyIndex": 0.7,
                    "attemptsMean": 1.2,
                    "timeMedianSecondsEstimate": 30
                }
            })
        );
    }
}
