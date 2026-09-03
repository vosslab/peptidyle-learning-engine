//! Bounded browser-safe Question Type and facet contracts.

use serde::{Deserialize, Serialize};

use crate::Capability;
use crate::question_library::{QuestionBackend, QuestionId};
use crate::question_license::QuestionLicense;
use crate::response::QuestionType;

/// Maximum Question Author name selections accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_AUTHOR_NAME_FILTERS: usize = 16;

/// Maximum free-form tag selections accepted in one Question Search query.
pub const MAX_QUESTION_SEARCH_TAG_FILTERS: usize = 64;

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

/// Server-computed count for one exact reviewed Question Author display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchAuthorFacet {
    /// Exact reviewed Question Author display value, never an Account Reference.
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

/// Availability filter for disclosed, validity-governed evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionStatisticsAvailability {
    /// Include results regardless of evidence availability.
    #[default]
    Any,
    /// Include only publications with disclosed independent student observations.
    Available,
    /// Include only publications without disclosed independent student observations.
    Unavailable,
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
    /// Immutable Question Types; any supplied type may match.
    pub question_types: Vec<QuestionType>,
    /// Required adapter capabilities; every supplied capability must be present.
    pub capabilities: Vec<Capability>,
    /// Accepted exact Question Licenses; any supplied value may match.
    pub question_licenses: Vec<QuestionLicense>,
    /// Whether disclosed independent student observations must be available.
    pub evidence: QuestionStatisticsAvailability,
    /// Whether a current Account-visible course use is required.
    ///
    /// This closed filter carries no course reference, title, or identity.
    pub used_in_my_courses: QuestionSearchCourseUse,
    /// Whether immutable publication authorship by the current Account is required.
    ///
    /// This closed filter carries no browser-provided Account identity.
    pub authorship: QuestionSearchAuthorship,
    /// Opaque continuation cursor from this exact normalized query.
    pub cursor: Option<String>,
    /// Requested bounded page size. `None` selects the server default.
    pub page_size: Option<u16>,
}

/// Normalized D1 filter meaning retained by a personal saved search.
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
    pub question_types: Vec<QuestionType>,
    pub capabilities: Vec<Capability>,
    pub question_licenses: Vec<QuestionLicense>,
    pub evidence: QuestionStatisticsAvailability,
    pub used_in_my_courses: QuestionSearchCourseUse,
    pub authorship: QuestionSearchAuthorship,
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
            question_types: query.question_types,
            capabilities: query.capabilities,
            question_licenses: query.question_licenses,
            evidence: query.evidence,
            used_in_my_courses: query.used_in_my_courses,
            authorship: query.authorship,
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
            question_types: filter.question_types,
            capabilities: filter.capabilities,
            question_licenses: filter.question_licenses,
            evidence: filter.evidence,
            used_in_my_courses: filter.used_in_my_courses,
            authorship: filter.authorship,
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
            question_types: Vec::new(),
            capabilities: Vec::new(),
            question_licenses: Vec::new(),
            evidence: QuestionStatisticsAvailability::Any,
            used_in_my_courses: QuestionSearchCourseUse::Any,
            authorship: QuestionSearchAuthorship::Any,
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
}

impl std::fmt::Display for QuestionSearchRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankFilter => formatter.write_str("Question Search filter must not be blank"),
            Self::TooLarge => {
                formatter.write_str("Question Search filter exceeds its bounded limit")
            }
            Self::EmptyCursor => formatter.write_str("Question Search cursor must not be empty"),
        }
    }
}

impl std::error::Error for QuestionSearchRequestError {}

impl QuestionSearchRequest {
    /// Returns the immutable-publication Question ID named in the text field.
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
    let normalized = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if normalized.chars().count() > maximum_characters {
        return Err(QuestionSearchRequestError::TooLarge);
    }
    Ok(normalized)
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

/// Disclosure-state counts from the same immutable query snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionStatisticsAvailabilityFacet {
    /// Publications with disclosed independent student observations.
    pub available: u64,
    /// Publications without disclosed independent student observations.
    pub unavailable: u64,
}

/// Aggregates computed from one normalized query snapshot, never a page sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuestionSearchFacets {
    /// Exact reviewed Question Author display-name counts.
    pub author_names: Vec<QuestionSearchAuthorFacet>,
    /// Closed Question Backend counts.
    pub backends: Vec<QuestionSearchBackendFacet>,
    /// Exact stored public tag counts.
    pub tags: Vec<QuestionSearchTagFacet>,
    /// Closed immutable Question Type counts.
    pub question_types: Vec<QuestionTypeFacet>,
    /// Adapter capability counts.
    pub capabilities: Vec<QuestionSearchCapabilityFacet>,
    /// Exact Question License counts.
    pub question_licenses: Vec<QuestionSearchQuestionLicenseFacet>,
    /// Validity-governed evidence availability counts.
    pub evidence: QuestionStatisticsAvailabilityFacet,
    /// Account-specific current-course-use count.
    pub used_in_my_courses: QuestionSearchCourseUseFacet,
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
    fn authored_scope_is_a_closed_account_bound_filter_and_survives_saved_search_conversion() {
        let query = QuestionSearchRequest {
            authorship: QuestionSearchAuthorship::AuthoredByCurrentAccount,
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
