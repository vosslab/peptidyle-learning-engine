//! Bounded browser-safe Question Type and facet contracts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::question_library::{PublishedQuestionId, QuestionBackend};
use crate::question_license::QuestionLicense;
use crate::response::QuestionType;
use crate::{
    AccountId, BloomCognitiveProcess, BloomKnowledgeDimension, Capability,
    LibrarySearchCategoryCounts, LibrarySearchKind, PublishedQuestionFilter,
};

/// Maximum rows in one broad Question, Pool, or Blueprint discovery page.
pub const MAX_DISCOVERY_PAGE_SIZE: u32 = 250;

/// Maximum Question Author name selections accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS: usize = 16;

/// Maximum free-form tag selections accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_TAG_FILTERS: usize = 64;

/// Maximum encoded continuation cursor length for Question Library facet choices.
///
/// The longest sort key must carry a maximum-length Question Title and public
/// Question ID; this is intentionally distinct from the smaller generic cursor.
pub const MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES: usize = 5_462;

/// Maximum reviewed Question Author names returned in one Question Search facet snapshot.
pub const MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS: usize = 64;

/// Maximum backend values returned in one Question Search facet snapshot.
pub const MAX_QUESTION_SEARCH_BACKEND_FACETS: usize = QuestionBackend::ALL.len();

/// Maximum free-form tags returned in one Question Search facet snapshot.
pub const MAX_QUESTION_SEARCH_TAG_FACETS: usize = 64;

/// Maximum Question Type values accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS: usize = QuestionType::ALL.len();

/// Maximum Question Type values returned in one Question Search facet snapshot.
pub const MAX_QUESTION_SEARCH_QUESTION_TYPE_FACETS: usize = QuestionType::ALL.len();

/// Account-bound authorship scope for Question Library discovery.
///
/// The browser selects only this closed meaning. The active authenticated
/// session supplies the actual account identity at the trusted store boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionSearchAuthorship {
    /// Include every Published Question regardless of authorship.
    #[default]
    Any,
    /// Include publications whose immutable author list contains the current Account.
    AuthoredByCurrentAccount,
}

/// Visible deterministic order for Question Library discovery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryObjectSearchSort {
    /// Order by Question Title, then stable public Question ID.
    #[default]
    TitleAscending,
    TitleDescending,
    KindAscending,
    KindDescending,
    DisciplineAscending,
    DisciplineDescending,
    QuestionTypeAscending,
    QuestionTypeDescending,
    /// Order newest publication first, then stable public Question ID.
    PublishedNewest,
}

/// Server-computed count for one exact reviewed Question Author display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchAuthorFacet {
    /// Exact reviewed Question Author display value, never an Account ID.
    pub author_name: String,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one closed Question Backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchBackendFacet {
    /// Exact public backend value.
    pub backend: QuestionBackend,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one exact stored tag display value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchTagFacet {
    /// Exact public tag display value; it is not lowercased for presentation.
    pub tag: String,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one exact stored subject display value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchSubjectFacet {
    /// Exact public subject display value; it is not lowercased for presentation.
    pub subject: String,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one exact stored topic display value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchTopicFacet {
    /// Exact public topic display value; it is not lowercased for presentation.
    pub topic: String,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one closed Question Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionTypeFacet {
    /// Exact public Question Type value.
    pub question_type: QuestionType,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one closed Bloom Cognitive Process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchBloomCognitiveProcessFacet {
    /// Exact teaching-guide value in guide order.
    pub cognitive_process: BloomCognitiveProcess,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one closed Bloom Knowledge Dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchBloomKnowledgeDimensionFacet {
    /// Exact teaching-guide value in guide order.
    pub knowledge_dimension: BloomKnowledgeDimension,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Strict, bounded Library Object Search request carried across the browser boundary.
///
/// The server normalizes this value before paging and aggregation. `cursor`
/// remains reserved for separately paged facet-choice enumeration; collection
/// rows use `page_number`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LibraryObjectSearchRequest {
    /// Result kinds admitted before the global sort and page.
    #[serde(default)]
    pub kind: LibrarySearchKind,
    /// Restricts Question rows by membership in every Pool, including forks.
    #[serde(default)]
    pub questions: PublishedQuestionFilter,
    /// Current Question owner or immutable Pool owner.
    #[serde(default)]
    pub owner_account_id: Option<AccountId>,
    /// Optional full-text-like text query over Question Library summary metadata.
    pub text: Option<String>,
    /// Reviewed Question Author display names; any normalized name may match.
    pub author_names: Vec<String>,
    /// Accepted Question Backends; any supplied backend may match.
    pub backends: Vec<QuestionBackend>,
    /// Free-form metadata tags; any normalized tag may match.
    pub tags: Vec<String>,
    /// Structured subjects; any normalized exact subject may match.
    pub subjects: Vec<String>,
    /// Structured topics; any normalized exact topic may match.
    pub topics: Vec<String>,
    /// Global Discipline identities; any selected value may match.
    #[serde(default)]
    pub discipline_uuids: Vec<Uuid>,
    /// Global Subject identities; each requires a selected parent Discipline.
    #[serde(default)]
    pub subject_uuids: Vec<Uuid>,
    /// Global Topic identities; each requires a selected parent Subject.
    #[serde(default)]
    pub topic_uuids: Vec<Uuid>,
    /// Global Subtopic identities; each requires a selected parent Topic.
    #[serde(default)]
    pub subtopic_uuids: Vec<Uuid>,
    /// Includes the selected Subject across Disciplines, omitting only Discipline equality.
    ///
    /// Requires Discipline and Subject. Text and name filters remain additional predicates.
    #[serde(default)]
    pub cross_discipline: bool,
    /// Exact Bloom Cognitive Processes; any selected value may match.
    #[serde(default)]
    pub bloom_cognitive_processes: Vec<BloomCognitiveProcess>,
    /// Exact Bloom Knowledge Dimensions; any selected value may match.
    #[serde(default)]
    pub bloom_knowledge_dimensions: Vec<BloomKnowledgeDimension>,
    /// Immutable Question Types; any supplied type may match.
    pub question_types: Vec<QuestionType>,
    /// Required adapter capabilities; every supplied capability must be present.
    pub capabilities: Vec<Capability>,
    /// Accepted exact Question Licenses; any supplied value may match.
    pub question_licenses: Vec<QuestionLicense>,
    /// Whether immutable publication authorship by the current Account is required.
    ///
    /// This closed filter carries no browser-provided Account identity.
    pub authorship: QuestionSearchAuthorship,
    /// Visible deterministic result order.
    #[serde(default)]
    pub sort: LibraryObjectSearchSort,
    /// Exact Question IDs omitted from this complete collection before counting and paging.
    /// Dedicated editors use this only for their already-fixed Questions.
    #[serde(default)]
    pub excluded_question_ids: Vec<PublishedQuestionId>,
    /// Opaque continuation for a facet-choice request, never collection rows.
    pub cursor: Option<String>,
    /// One-based requested result page. `None` selects the first page.
    pub page_number: Option<u32>,
    /// Requested bounded page size. `None` selects the server default.
    pub page_size: Option<u16>,
}

/// Normalized D1 filter and order meaning for a Question Search query.
///
/// Pagination is intentionally absent: a new search starts at the first page
/// with a server-selected page size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LibraryObjectSearchFilter {
    /// Result kinds admitted before the global sort and page.
    #[serde(default)]
    pub kind: LibrarySearchKind,
    /// Restricts Question rows by membership in every Pool, including forks.
    #[serde(default)]
    pub questions: PublishedQuestionFilter,
    /// Current Question owner or immutable Pool owner.
    #[serde(default)]
    pub owner_account_id: Option<AccountId>,
    pub text: Option<String>,
    pub author_names: Vec<String>,
    pub backends: Vec<QuestionBackend>,
    pub tags: Vec<String>,
    pub subjects: Vec<String>,
    pub topics: Vec<String>,
    #[serde(default)]
    pub discipline_uuids: Vec<Uuid>,
    #[serde(default)]
    pub subject_uuids: Vec<Uuid>,
    #[serde(default)]
    pub topic_uuids: Vec<Uuid>,
    #[serde(default)]
    pub subtopic_uuids: Vec<Uuid>,
    #[serde(default)]
    pub cross_discipline: bool,
    #[serde(default)]
    pub bloom_cognitive_processes: Vec<BloomCognitiveProcess>,
    #[serde(default)]
    pub bloom_knowledge_dimensions: Vec<BloomKnowledgeDimension>,
    pub question_types: Vec<QuestionType>,
    pub capabilities: Vec<Capability>,
    pub question_licenses: Vec<QuestionLicense>,
    pub authorship: QuestionSearchAuthorship,
    #[serde(default)]
    pub sort: LibraryObjectSearchSort,
    #[serde(default)]
    pub excluded_question_ids: Vec<PublishedQuestionId>,
}

impl LibraryObjectSearchFilter {
    /// Normalizes durable filter meaning through the D1 query normalizer.
    pub fn normalized(self) -> Result<Self, LibraryObjectSearchRequestError> {
        Self::from_query(LibraryObjectSearchRequest::from(self).normalized()?)
    }

    /// Drops facet-choice and page selection state from one D1 query.
    pub fn from_query(
        query: LibraryObjectSearchRequest,
    ) -> Result<Self, LibraryObjectSearchRequestError> {
        let query = query.normalized()?;
        Ok(Self {
            kind: query.kind,
            questions: query.questions,
            owner_account_id: query.owner_account_id,
            text: query.text,
            author_names: query.author_names,
            backends: query.backends,
            tags: query.tags,
            subjects: query.subjects,
            topics: query.topics,
            discipline_uuids: query.discipline_uuids,
            subject_uuids: query.subject_uuids,
            topic_uuids: query.topic_uuids,
            subtopic_uuids: query.subtopic_uuids,
            cross_discipline: query.cross_discipline,
            bloom_cognitive_processes: query.bloom_cognitive_processes,
            bloom_knowledge_dimensions: query.bloom_knowledge_dimensions,
            question_types: query.question_types,
            capabilities: query.capabilities,
            question_licenses: query.question_licenses,
            authorship: query.authorship,
            sort: query.sort,
            excluded_question_ids: query.excluded_question_ids,
        })
    }

    /// Starts a fresh D1 query without continuation state.
    pub fn fresh_query(&self) -> LibraryObjectSearchRequest {
        LibraryObjectSearchRequest::from(self.clone())
    }
}

impl From<LibraryObjectSearchFilter> for LibraryObjectSearchRequest {
    fn from(filter: LibraryObjectSearchFilter) -> Self {
        Self {
            kind: filter.kind,
            questions: filter.questions,
            owner_account_id: filter.owner_account_id,
            text: filter.text,
            author_names: filter.author_names,
            backends: filter.backends,
            tags: filter.tags,
            subjects: filter.subjects,
            topics: filter.topics,
            discipline_uuids: filter.discipline_uuids,
            subject_uuids: filter.subject_uuids,
            topic_uuids: filter.topic_uuids,
            subtopic_uuids: filter.subtopic_uuids,
            cross_discipline: filter.cross_discipline,
            bloom_cognitive_processes: filter.bloom_cognitive_processes,
            bloom_knowledge_dimensions: filter.bloom_knowledge_dimensions,
            question_types: filter.question_types,
            capabilities: filter.capabilities,
            question_licenses: filter.question_licenses,
            authorship: filter.authorship,
            sort: filter.sort,
            excluded_question_ids: filter.excluded_question_ids,
            cursor: None,
            page_number: None,
            page_size: None,
        }
    }
}

impl Default for LibraryObjectSearchRequest {
    fn default() -> Self {
        Self {
            kind: LibrarySearchKind::Questions,
            questions: PublishedQuestionFilter::All,
            owner_account_id: None,
            text: None,
            author_names: Vec::new(),
            backends: Vec::new(),
            tags: Vec::new(),
            subjects: Vec::new(),
            topics: Vec::new(),
            discipline_uuids: Vec::new(),
            subject_uuids: Vec::new(),
            topic_uuids: Vec::new(),
            subtopic_uuids: Vec::new(),
            cross_discipline: false,
            bloom_cognitive_processes: Vec::new(),
            bloom_knowledge_dimensions: Vec::new(),
            question_types: Vec::new(),
            capabilities: Vec::new(),
            question_licenses: Vec::new(),
            authorship: QuestionSearchAuthorship::Any,
            sort: LibraryObjectSearchSort::TitleAscending,
            excluded_question_ids: Vec::new(),
            cursor: None,
            page_number: None,
            page_size: None,
        }
    }
}

/// Rejection reason for a Library Object search request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryObjectSearchRequestError {
    /// Text or a controlled-term component was blank after normalization.
    BlankFilter,
    /// A string field or filter values exceeded the bounded contract.
    TooLarge,
    /// A selected classification identity omitted its required parent selection.
    IncompleteClassificationChain,
    /// Cross-Discipline inclusion omitted Discipline or Subject.
    InvalidCrossDiscipline,
}

impl std::fmt::Display for LibraryObjectSearchRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankFilter => formatter.write_str("Question Search filter must not be blank"),
            Self::TooLarge => {
                formatter.write_str("Question Search filter exceeds its bounded limit")
            }
            Self::IncompleteClassificationChain => {
                formatter.write_str("Question Search classification requires its parent selections")
            }
            Self::InvalidCrossDiscipline => formatter.write_str(
                "Question Search cross-Discipline inclusion requires Discipline and Subject",
            ),
        }
    }
}

impl std::error::Error for LibraryObjectSearchRequestError {}

impl LibraryObjectSearchRequest {
    /// Returns the stable Published Question lineage ID named in the text field.
    pub fn exact_question_id(&self) -> Option<PublishedQuestionId> {
        self.text.as_deref()?.parse::<PublishedQuestionId>().ok()
    }

    /// Normalizes one D1 query for both rows and facet aggregates.
    ///
    /// Text, Question Author display names, and tags use lowercased, whitespace-collapsed
    /// Unicode text. Controlled terms retain durable case after trimming.
    /// Text, every active metadata filter, Account-bound filters, and
    /// `authorship`
    /// combine with every other active filter using AND. Within Question Author names,
    /// backends, tags, Question Types, and Question Licenses, values combine using
    /// OR. Capabilities retain every-value-matches semantics.
    pub fn normalized(mut self) -> Result<Self, LibraryObjectSearchRequestError> {
        // ASVS 2.2.1 and 2.2.3: validate the selection structure without inferring parents.
        // The trusted service must additionally validate vocabulary identities and associations.
        if (!self.subject_uuids.is_empty() && self.discipline_uuids.is_empty())
            || (!self.topic_uuids.is_empty() && self.subject_uuids.is_empty())
            || (!self.subtopic_uuids.is_empty() && self.topic_uuids.is_empty())
        {
            return Err(LibraryObjectSearchRequestError::IncompleteClassificationChain);
        }
        if self.cross_discipline
            && (self.discipline_uuids.is_empty() || self.subject_uuids.is_empty())
        {
            return Err(LibraryObjectSearchRequestError::InvalidCrossDiscipline);
        }
        self.text = self
            .text
            .map(|text| normalize_question_search_text(text, 256))
            .transpose()?
            .filter(|text| !text.is_empty());
        normalize_text_filters(
            &mut self.author_names,
            MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS,
            120,
        )?;
        normalize_text_filters(&mut self.tags, MAX_QUESTION_SEARCH_TAG_FILTERS, 256)?;
        normalize_text_filters(&mut self.subjects, MAX_QUESTION_SEARCH_TAG_FILTERS, 256)?;
        normalize_text_filters(&mut self.topics, MAX_QUESTION_SEARCH_TAG_FILTERS, 256)?;
        if self.capabilities.len() > Capability::ALL.len()
            || self.question_licenses.len() > 3
            || self.backends.len() > QuestionBackend::ALL.len()
            || self.question_types.len() > MAX_QUESTION_SEARCH_QUESTION_TYPE_FILTERS
            || self.excluded_question_ids.len() > 1024
        {
            return Err(LibraryObjectSearchRequestError::TooLarge);
        }
        // ASVS 2.2.1: bound each repeated exact selector before normalization.
        if self.discipline_uuids.len() > 32
            || self.subject_uuids.len() > 32
            || self.topic_uuids.len() > 32
            || self.subtopic_uuids.len() > 32
            || self.bloom_cognitive_processes.len() > BloomCognitiveProcess::ALL.len()
            || self.bloom_knowledge_dimensions.len() > BloomKnowledgeDimension::ALL.len()
        {
            return Err(LibraryObjectSearchRequestError::TooLarge);
        }
        self.discipline_uuids.sort();
        self.discipline_uuids.dedup();
        self.subject_uuids.sort();
        self.subject_uuids.dedup();
        self.topic_uuids.sort();
        self.topic_uuids.dedup();
        self.subtopic_uuids.sort();
        self.subtopic_uuids.dedup();
        self.bloom_cognitive_processes.sort();
        self.bloom_cognitive_processes.dedup();
        self.bloom_knowledge_dimensions.sort();
        self.bloom_knowledge_dimensions.dedup();
        self.capabilities.sort();
        self.capabilities.dedup();
        self.question_licenses.sort();
        self.question_licenses.dedup();
        self.backends.sort();
        self.backends.dedup();
        self.question_types.sort();
        self.question_types.dedup();
        self.excluded_question_ids.sort();
        self.excluded_question_ids.dedup();
        Ok(self)
    }
}

fn normalize_text_filters(
    filters: &mut Vec<String>,
    maximum_filters: usize,
    maximum_characters: usize,
) -> Result<(), LibraryObjectSearchRequestError> {
    if filters.len() > maximum_filters {
        return Err(LibraryObjectSearchRequestError::TooLarge);
    }
    for filter in filters.iter_mut() {
        *filter = normalize_text(std::mem::take(filter), maximum_characters)?;
        if filter.is_empty() {
            return Err(LibraryObjectSearchRequestError::BlankFilter);
        }
    }
    filters.sort();
    filters.dedup();
    Ok(())
}

fn normalize_text(
    value: String,
    maximum_characters: usize,
) -> Result<String, LibraryObjectSearchRequestError> {
    let normalized = normalized_question_search_group_value(&value);
    if normalized.chars().count() > maximum_characters {
        return Err(LibraryObjectSearchRequestError::TooLarge);
    }
    Ok(normalized)
}

fn normalize_question_search_text(
    value: String,
    maximum_characters: usize,
) -> Result<String, LibraryObjectSearchRequestError> {
    let whitespace_normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = if whitespace_normalized.parse::<PublishedQuestionId>().is_ok() {
        whitespace_normalized
    } else {
        whitespace_normalized.to_lowercase()
    };
    if normalized.chars().count() > maximum_characters {
        return Err(LibraryObjectSearchRequestError::TooLarge);
    }
    Ok(normalized)
}

/// Produces the one comparison key used by structured text filters and facets.
///
/// The server uses this same function when grouping stored display values, so
/// selecting a returned label applies to the group whose count was displayed.
pub fn normalized_question_search_group_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Server-computed count for one capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchCapabilityFacet {
    /// Capability represented by the count.
    pub capability: Capability,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one exact Question License.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchQuestionLicenseFacet {
    /// Exact Question License represented by the count.
    pub question_license: QuestionLicense,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Aggregates computed from one normalized query snapshot, never a page sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchFacets {
    /// Matching categories before kind and membership selection.
    pub categories: LibrarySearchCategoryCounts,
    /// Exact reviewed Question Author display-name counts.
    pub author_names: Vec<QuestionSearchAuthorFacet>,
    /// Whether additional matching Question Author names were omitted by the bound.
    pub author_names_truncated: bool,
    /// Closed Question Backend counts.
    pub backends: Vec<QuestionSearchBackendFacet>,
    /// Exact stored public tag counts.
    pub tags: Vec<QuestionSearchTagFacet>,
    /// Whether additional matching tags were omitted by the bound.
    pub tags_truncated: bool,
    /// Exact stored public subject counts.
    pub subjects: Vec<QuestionSearchSubjectFacet>,
    /// Whether additional matching subjects were omitted by the bound.
    pub subjects_truncated: bool,
    /// Exact stored public topic counts.
    pub topics: Vec<QuestionSearchTopicFacet>,
    /// Whether additional matching topics were omitted by the bound.
    pub topics_truncated: bool,
    /// Closed immutable Question Type counts.
    pub question_types: Vec<QuestionTypeFacet>,
    /// Adapter capability counts.
    pub capabilities: Vec<QuestionSearchCapabilityFacet>,
    /// Exact Question License counts.
    pub question_licenses: Vec<QuestionSearchQuestionLicenseFacet>,
    /// Complete closed Bloom Cognitive Process counts in teaching-guide order.
    pub bloom_cognitive_processes: Vec<QuestionSearchBloomCognitiveProcessFacet>,
    /// Complete closed Bloom Knowledge Dimension counts in teaching-guide order.
    pub bloom_knowledge_dimensions: Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
}

#[cfg(test)]
mod library_multi_filter_tests {
    use super::*;

    #[test]
    fn normalizes_repeated_exact_selections_and_requires_parents() {
        let discipline = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let another_discipline = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let subject = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let query = LibraryObjectSearchRequest {
            discipline_uuids: vec![another_discipline, discipline, discipline],
            subject_uuids: vec![subject, subject],
            bloom_cognitive_processes: vec![
                BloomCognitiveProcess::Analyze,
                BloomCognitiveProcess::Apply,
                BloomCognitiveProcess::Apply,
            ],
            ..LibraryObjectSearchRequest::default()
        };
        let normalized = query.normalized().unwrap();
        assert_eq!(
            normalized.discipline_uuids,
            vec![discipline, another_discipline]
        );
        assert_eq!(normalized.subject_uuids, vec![subject]);
        assert_eq!(
            normalized.bloom_cognitive_processes,
            vec![BloomCognitiveProcess::Apply, BloomCognitiveProcess::Analyze]
        );

        let orphan = LibraryObjectSearchRequest {
            subject_uuids: vec![subject],
            ..LibraryObjectSearchRequest::default()
        };
        assert_eq!(
            orphan.normalized(),
            Err(LibraryObjectSearchRequestError::IncompleteClassificationChain)
        );
    }
}
