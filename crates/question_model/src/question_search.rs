//! Bounded browser-safe Question Type and facet contracts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::question_library::{QuestionBackend, QuestionId};
use crate::question_license::QuestionLicense;
use crate::response::QuestionType;
use crate::{BloomCognitiveProcess, BloomKnowledgeDimension, Capability};

/// Maximum Question Author name selections accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS: usize = 16;

/// Maximum free-form tag selections accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_TAG_FILTERS: usize = 64;

/// Maximum encoded continuation cursor length for Question Library search.
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

/// Account-bound course-use filter for Question Library discovery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionSearchCourseUse {
    /// Include every Published Question regardless of current Account use.
    #[default]
    Any,
    /// Include only publications used in at least one course visible to the Account.
    Used,
}

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
pub enum QuestionSearchSort {
    /// Order by Question Title, then stable public Question ID.
    #[default]
    TitleAscending,
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

/// Account-specific reverse-index count from the same Question Search query snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchCourseUseFacet {
    /// Publications used in one or more courses visible to the current Account.
    pub used: u64,
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

/// Strict, bounded Question Search request carried across the browser boundary.
///
/// The server normalizes this value before paging and aggregation. The cursor
/// is opaque and tied to that normalized query; positional paging is not
/// representable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QuestionSearchRequest {
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
    /// Optional global Discipline identity; unrestricted when absent.
    #[serde(default)]
    pub discipline_uuid: Option<Uuid>,
    /// Optional global Subject identity; requires a selected Discipline.
    #[serde(default)]
    pub subject_uuid: Option<Uuid>,
    /// Optional global Topic identity; requires a selected Subject.
    #[serde(default)]
    pub topic_uuid: Option<Uuid>,
    /// Optional global Subtopic identity; requires a selected Topic.
    #[serde(default)]
    pub subtopic_uuid: Option<Uuid>,
    /// Includes the selected Subject across Disciplines, omitting only Discipline equality.
    ///
    /// Requires Discipline and Subject. Text and name filters remain additional predicates.
    #[serde(default)]
    pub cross_discipline: bool,
    /// Optional exact Bloom Cognitive Process; unrestricted when absent.
    #[serde(default)]
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    /// Optional exact Bloom Knowledge Dimension; unrestricted when absent.
    #[serde(default)]
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
    /// Immutable Question Types; any supplied type may match.
    pub question_types: Vec<QuestionType>,
    /// Required adapter capabilities; every supplied capability must be present.
    pub capabilities: Vec<Capability>,
    /// Accepted exact Question Licenses; any supplied value may match.
    pub question_licenses: Vec<QuestionLicense>,
    /// Whether a current Account-visible course use is required.
    ///
    /// This closed filter carries no Course Instance ID, title, or identity.
    pub used_in_my_courses: QuestionSearchCourseUse,
    /// Whether immutable publication authorship by the current Account is required.
    ///
    /// This closed filter carries no browser-provided Account identity.
    pub authorship: QuestionSearchAuthorship,
    /// Visible deterministic result order.
    #[serde(default)]
    pub sort: QuestionSearchSort,
    /// Opaque continuation cursor from this exact normalized query.
    pub cursor: Option<String>,
    /// Requested bounded page size. `None` selects the server default.
    pub page_size: Option<u16>,
}

/// Normalized D1 filter and order meaning retained by a personal saved search.
///
/// Pagination is intentionally absent: running a saved search always starts a
/// fresh current-Question Search with a server-selected page size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QuestionSearchFilter {
    pub text: Option<String>,
    pub author_names: Vec<String>,
    pub backends: Vec<QuestionBackend>,
    pub tags: Vec<String>,
    pub subjects: Vec<String>,
    pub topics: Vec<String>,
    #[serde(default)]
    pub discipline_uuid: Option<Uuid>,
    #[serde(default)]
    pub subject_uuid: Option<Uuid>,
    #[serde(default)]
    pub topic_uuid: Option<Uuid>,
    #[serde(default)]
    pub subtopic_uuid: Option<Uuid>,
    #[serde(default)]
    pub cross_discipline: bool,
    #[serde(default)]
    pub bloom_cognitive_process: Option<BloomCognitiveProcess>,
    #[serde(default)]
    pub bloom_knowledge_dimension: Option<BloomKnowledgeDimension>,
    pub question_types: Vec<QuestionType>,
    pub capabilities: Vec<Capability>,
    pub question_licenses: Vec<QuestionLicense>,
    pub used_in_my_courses: QuestionSearchCourseUse,
    pub authorship: QuestionSearchAuthorship,
    #[serde(default)]
    pub sort: QuestionSearchSort,
}

impl QuestionSearchFilter {
    /// Normalizes durable filter meaning through the D1 query normalizer.
    pub fn normalized(self) -> Result<Self, QuestionSearchRequestError> {
        Self::from_query(QuestionSearchRequest::from(self).normalized()?)
    }

    /// Drops cursor and page-size continuation state from one D1 query.
    pub fn from_query(query: QuestionSearchRequest) -> Result<Self, QuestionSearchRequestError> {
        let query = query.normalized()?;
        Ok(Self {
            text: query.text,
            author_names: query.author_names,
            backends: query.backends,
            tags: query.tags,
            subjects: query.subjects,
            topics: query.topics,
            discipline_uuid: query.discipline_uuid,
            subject_uuid: query.subject_uuid,
            topic_uuid: query.topic_uuid,
            subtopic_uuid: query.subtopic_uuid,
            cross_discipline: query.cross_discipline,
            bloom_cognitive_process: query.bloom_cognitive_process,
            bloom_knowledge_dimension: query.bloom_knowledge_dimension,
            question_types: query.question_types,
            capabilities: query.capabilities,
            question_licenses: query.question_licenses,
            used_in_my_courses: query.used_in_my_courses,
            authorship: query.authorship,
            sort: query.sort,
        })
    }

    /// Starts a fresh D1 query without continuation state.
    pub fn fresh_query(&self) -> QuestionSearchRequest {
        QuestionSearchRequest::from(self.clone())
    }
}

impl From<QuestionSearchFilter> for QuestionSearchRequest {
    fn from(filter: QuestionSearchFilter) -> Self {
        Self {
            text: filter.text,
            author_names: filter.author_names,
            backends: filter.backends,
            tags: filter.tags,
            subjects: filter.subjects,
            topics: filter.topics,
            discipline_uuid: filter.discipline_uuid,
            subject_uuid: filter.subject_uuid,
            topic_uuid: filter.topic_uuid,
            subtopic_uuid: filter.subtopic_uuid,
            cross_discipline: filter.cross_discipline,
            bloom_cognitive_process: filter.bloom_cognitive_process,
            bloom_knowledge_dimension: filter.bloom_knowledge_dimension,
            question_types: filter.question_types,
            capabilities: filter.capabilities,
            question_licenses: filter.question_licenses,
            used_in_my_courses: filter.used_in_my_courses,
            authorship: filter.authorship,
            sort: filter.sort,
            cursor: None,
            page_size: None,
        }
    }
}

impl Default for QuestionSearchRequest {
    fn default() -> Self {
        Self {
            text: None,
            author_names: Vec::new(),
            backends: Vec::new(),
            tags: Vec::new(),
            subjects: Vec::new(),
            topics: Vec::new(),
            discipline_uuid: None,
            subject_uuid: None,
            topic_uuid: None,
            subtopic_uuid: None,
            cross_discipline: false,
            bloom_cognitive_process: None,
            bloom_knowledge_dimension: None,
            question_types: Vec::new(),
            capabilities: Vec::new(),
            question_licenses: Vec::new(),
            used_in_my_courses: QuestionSearchCourseUse::Any,
            authorship: QuestionSearchAuthorship::Any,
            sort: QuestionSearchSort::TitleAscending,
            cursor: None,
            page_size: None,
        }
    }
}

/// Rejection reason for a Question Search request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionSearchRequestError {
    /// Text or a controlled-term component was blank after normalization.
    BlankFilter,
    /// A string field or filter values exceeded the bounded contract.
    TooLarge,
    /// An opaque continuation token was empty.
    EmptyCursor,
    /// A selected classification identity omitted its required parent selection.
    IncompleteClassificationChain,
    /// Cross-Discipline inclusion omitted Discipline or Subject.
    InvalidCrossDiscipline,
}

impl std::fmt::Display for QuestionSearchRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankFilter => formatter.write_str("Question Search filter must not be blank"),
            Self::TooLarge => {
                formatter.write_str("Question Search filter exceeds its bounded limit")
            }
            Self::EmptyCursor => formatter.write_str("Question Search cursor must not be empty"),
            Self::IncompleteClassificationChain => {
                formatter.write_str("Question Search classification requires its parent selections")
            }
            Self::InvalidCrossDiscipline => formatter.write_str(
                "Question Search cross-Discipline inclusion requires Discipline and Subject",
            ),
        }
    }
}

impl std::error::Error for QuestionSearchRequestError {}

impl QuestionSearchRequest {
    /// Returns the stable Published Question lineage ID named in the text field.
    pub fn exact_question_id(&self) -> Option<QuestionId> {
        self.text.as_deref()?.parse::<QuestionId>().ok()
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
    pub fn normalized(mut self) -> Result<Self, QuestionSearchRequestError> {
        // ASVS 2.2.1 and 2.2.3: validate the selection structure without inferring parents.
        // The trusted service must additionally validate vocabulary identities and associations.
        if (self.subject_uuid.is_some() && self.discipline_uuid.is_none())
            || (self.topic_uuid.is_some() && self.subject_uuid.is_none())
            || (self.subtopic_uuid.is_some() && self.topic_uuid.is_none())
        {
            return Err(QuestionSearchRequestError::IncompleteClassificationChain);
        }
        if self.cross_discipline && (self.discipline_uuid.is_none() || self.subject_uuid.is_none())
        {
            return Err(QuestionSearchRequestError::InvalidCrossDiscipline);
        }
        self.text = self
            .text
            .map(|text| normalize_text(text, 256))
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
        {
            return Err(QuestionSearchRequestError::TooLarge);
        }
        self.capabilities.sort();
        self.capabilities.dedup();
        self.question_licenses.sort();
        self.question_licenses.dedup();
        self.backends.sort();
        self.backends.dedup();
        self.question_types.sort();
        self.question_types.dedup();
        if self.cursor.as_ref().is_some_and(String::is_empty) {
            return Err(QuestionSearchRequestError::EmptyCursor);
        }
        Ok(self)
    }
}

fn normalize_text_filters(
    filters: &mut Vec<String>,
    maximum_filters: usize,
    maximum_characters: usize,
) -> Result<(), QuestionSearchRequestError> {
    if filters.len() > maximum_filters {
        return Err(QuestionSearchRequestError::TooLarge);
    }
    for filter in filters.iter_mut() {
        *filter = normalize_text(std::mem::take(filter), maximum_characters)?;
        if filter.is_empty() {
            return Err(QuestionSearchRequestError::BlankFilter);
        }
    }
    filters.sort();
    filters.dedup();
    Ok(())
}

fn normalize_text(
    value: String,
    maximum_characters: usize,
) -> Result<String, QuestionSearchRequestError> {
    let normalized = normalized_question_search_group_value(&value);
    if normalized.chars().count() > maximum_characters {
        return Err(QuestionSearchRequestError::TooLarge);
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
    /// Account-specific current-course-use count.
    pub used_in_my_courses: QuestionSearchCourseUseFacet,
    /// Complete closed Bloom Cognitive Process counts in teaching-guide order.
    pub bloom_cognitive_processes: Vec<QuestionSearchBloomCognitiveProcessFacet>,
    /// Complete closed Bloom Knowledge Dimension counts in teaching-guide order.
    pub bloom_knowledge_dimensions: Vec<QuestionSearchBloomKnowledgeDimensionFacet>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_type_is_separate_from_the_question_response_control() {
        let response = crate::response::QuestionResponseFormat::ImathasQuestionBackend {};
        assert_eq!(
            response.control(),
            crate::response::QuestionResponseControl::ImathasQuestionBackend
        );
        assert!(response.supports_question_type(QuestionType::Numeric));
        assert_eq!(
            QuestionType::ALL
                .into_iter()
                .map(|question_type| serde_json::to_value(question_type).expect("type serializes"))
                .collect::<Vec<_>>(),
            vec![
                serde_json::json!("multipleChoice"),
                serde_json::json!("multipleAnswer"),
                serde_json::json!("fillInBlank"),
                serde_json::json!("multipleFillInBlank"),
                serde_json::json!("numeric"),
                serde_json::json!("matching"),
                serde_json::json!("ordering"),
                serde_json::json!("hotspot"),
            ]
        );
    }

    #[test]
    fn authored_scope_and_visible_sort_survive_saved_search_conversion() {
        let query = QuestionSearchRequest {
            authorship: QuestionSearchAuthorship::AuthoredByCurrentAccount,
            sort: QuestionSearchSort::PublishedNewest,
            bloom_cognitive_process: Some(BloomCognitiveProcess::Analyze),
            bloom_knowledge_dimension: Some(BloomKnowledgeDimension::ProceduralKnowledge),
            ..QuestionSearchRequest::default()
        };
        assert_eq!(
            serde_json::to_value(&query).expect("query serializes")["authorship"],
            serde_json::json!("authoredByCurrentAccount")
        );
        let filter =
            QuestionSearchFilter::from_query(query).expect("filter produces normalized D1 meaning");
        assert_eq!(
            filter.authorship,
            QuestionSearchAuthorship::AuthoredByCurrentAccount
        );
        assert_eq!(
            filter.fresh_query().authorship,
            QuestionSearchAuthorship::AuthoredByCurrentAccount
        );
        assert_eq!(filter.sort, QuestionSearchSort::PublishedNewest);
        assert_eq!(
            filter.fresh_query().sort,
            QuestionSearchSort::PublishedNewest
        );
        assert_eq!(
            filter.fresh_query().bloom_cognitive_process,
            Some(BloomCognitiveProcess::Analyze)
        );
        assert_eq!(
            filter.fresh_query().bloom_knowledge_dimension,
            Some(BloomKnowledgeDimension::ProceduralKnowledge)
        );
    }

    #[test]
    fn classification_identity_survives_normalization_and_saved_search_round_trip() {
        let query = QuestionSearchRequest {
            discipline_uuid: Some(Uuid::from_u128(1)),
            subject_uuid: Some(Uuid::from_u128(2)),
            topic_uuid: Some(Uuid::from_u128(3)),
            subtopic_uuid: Some(Uuid::from_u128(4)),
            cross_discipline: true,
            text: Some("  subject:Genetics -review  ".to_string()),
            subjects: vec![" Genetics ".to_string()],
            topics: vec![" Inheritance ".to_string()],
            cursor: Some("continuation".to_string()),
            page_size: Some(25),
            ..QuestionSearchRequest::default()
        }
        .normalized()
        .expect("complete classification selection normalizes");
        let filter = QuestionSearchFilter::from_query(query.clone()).expect("selection retained");
        let restored: QuestionSearchFilter =
            serde_json::from_value(serde_json::to_value(&filter).expect("filter serializes"))
                .expect("filter deserializes");
        assert_eq!(restored.normalized().expect("filter normalizes"), filter);
        let mut fresh = query;
        fresh.cursor = None;
        fresh.page_size = None;
        assert_eq!(filter.fresh_query(), fresh);
        assert_eq!(fresh.text.as_deref(), Some("subject:genetics -review"));
        assert_eq!(fresh.subjects, vec!["genetics"]);
        assert_eq!(fresh.topics, vec!["inheritance"]);
    }

    #[test]
    fn classification_selection_requires_parent_chain_and_explicit_cross_subject() {
        for query in [
            QuestionSearchRequest {
                subject_uuid: Some(Uuid::from_u128(2)),
                ..QuestionSearchRequest::default()
            },
            QuestionSearchRequest {
                discipline_uuid: Some(Uuid::from_u128(1)),
                topic_uuid: Some(Uuid::from_u128(3)),
                ..QuestionSearchRequest::default()
            },
            QuestionSearchRequest {
                discipline_uuid: Some(Uuid::from_u128(1)),
                subject_uuid: Some(Uuid::from_u128(2)),
                subtopic_uuid: Some(Uuid::from_u128(4)),
                ..QuestionSearchRequest::default()
            },
        ] {
            assert_eq!(
                query.normalized(),
                Err(QuestionSearchRequestError::IncompleteClassificationChain)
            );
        }
        for discipline_uuid in [None, Some(Uuid::from_u128(1))] {
            assert_eq!(
                QuestionSearchRequest {
                    discipline_uuid,
                    cross_discipline: true,
                    ..QuestionSearchRequest::default()
                }
                .normalized(),
                Err(QuestionSearchRequestError::InvalidCrossDiscipline)
            );
        }
        for query in [
            QuestionSearchRequest::default(),
            QuestionSearchRequest {
                discipline_uuid: Some(Uuid::from_u128(1)),
                ..QuestionSearchRequest::default()
            },
            QuestionSearchRequest {
                discipline_uuid: Some(Uuid::from_u128(1)),
                subject_uuid: Some(Uuid::from_u128(2)),
                cross_discipline: true,
                ..QuestionSearchRequest::default()
            },
        ] {
            assert!(query.normalized().is_ok());
        }
    }

    #[test]
    fn question_search_roots_use_strict_snake_case_without_scope_or_paging_state() {
        let query = QuestionSearchRequest {
            question_types: vec![QuestionType::FillInBlank],
            used_in_my_courses: QuestionSearchCourseUse::Used,
            cursor: Some("opaque-cursor".to_string()),
            page_size: Some(25),
            ..QuestionSearchRequest::default()
        };
        let query_json = serde_json::to_value(&query).expect("query serializes");
        assert_eq!(
            query_json["question_types"],
            serde_json::json!(["fillInBlank"])
        );
        assert_eq!(query_json["used_in_my_courses"], serde_json::json!("used"));
        assert_eq!(query_json["page_size"], serde_json::json!(25));
        assert!(query_json.get("responseFamilies").is_none());
        assert!(query_json.get("usedInMyCourses").is_none());

        let filter =
            QuestionSearchFilter::from_query(query).expect("filter produces normalized D1 meaning");
        let filter_json = serde_json::to_value(&filter).expect("filter serializes");
        assert_eq!(
            filter_json["question_types"],
            serde_json::json!(["fillInBlank"])
        );
        assert!(filter_json.get("cursor").is_none());
        assert!(filter_json.get("page_size").is_none());
        assert_eq!(filter.fresh_query().cursor, None);
        assert_eq!(filter.fresh_query().page_size, None);

        let mut empty_selection_query = query_json.clone();
        let mut empty_selection_filter = filter_json.clone();
        for field in [
            "discipline_uuid",
            "subject_uuid",
            "topic_uuid",
            "subtopic_uuid",
            "cross_discipline",
            "bloom_cognitive_process",
            "bloom_knowledge_dimension",
        ] {
            empty_selection_query
                .as_object_mut()
                .expect("query object")
                .remove(field);
            empty_selection_filter
                .as_object_mut()
                .expect("filter object")
                .remove(field);
        }
        assert_eq!(
            serde_json::from_value::<QuestionSearchRequest>(empty_selection_query)
                .expect("omitted optional selection means unrestricted"),
            serde_json::from_value::<QuestionSearchRequest>(query_json.clone())
                .expect("explicit empty selection deserializes")
        );
        assert_eq!(
            serde_json::from_value::<QuestionSearchFilter>(empty_selection_filter)
                .expect("omitted optional selection means unrestricted"),
            filter
        );
        for (field, value) in [
            ("discipline_uuid", serde_json::json!("not-a-uuid")),
            ("cross_discipline", serde_json::json!("true")),
            ("subjectUuid", serde_json::json!(Uuid::from_u128(2))),
            ("bloom_cognitive_process", serde_json::json!("analyze")),
            (
                "bloomKnowledgeDimension",
                serde_json::json!("Factual Knowledge"),
            ),
        ] {
            let mut rejected_query = query_json.clone();
            rejected_query[field] = value.clone();
            assert!(serde_json::from_value::<QuestionSearchRequest>(rejected_query).is_err());
            let mut rejected_filter = filter_json.clone();
            rejected_filter[field] = value;
            assert!(serde_json::from_value::<QuestionSearchFilter>(rejected_filter).is_err());
        }

        for retired_field in ["publication_scopes", "publicationScopes"] {
            let mut rejected_query = query_json.clone();
            rejected_query[retired_field] = serde_json::json!(["public"]);
            assert!(
                serde_json::from_value::<QuestionSearchRequest>(rejected_query).is_err(),
                "query rejects retired {retired_field}"
            );

            let mut rejected_filter = filter_json.clone();
            rejected_filter[retired_field] = serde_json::json!(["public"]);
            assert!(
                serde_json::from_value::<QuestionSearchFilter>(rejected_filter).is_err(),
                "filter rejects retired {retired_field}"
            );
        }
    }
}
