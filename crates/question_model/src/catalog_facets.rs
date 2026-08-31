//! Bounded browser-safe catalog response-family and facet contracts.

use serde::{Deserialize, Serialize};

use crate::Capability;
use crate::catalog::{MAX_CATALOG_TAXONOMY_FACETS, QuestionBackend, QuestionId};
use crate::response::ResponseDefinition;
use crate::taxonomy::{License, TaxonomyTerm};

/// Maximum public byline selections accepted in one catalog query.
pub const MAX_CATALOG_BYLINE_FILTERS: usize = 16;

/// Maximum free-form tag selections accepted in one catalog query.
pub const MAX_CATALOG_TAG_FILTERS: usize = 64;

/// Maximum reviewed public bylines returned in one catalog facet snapshot.
pub const MAX_CATALOG_BYLINE_FACETS: usize = 64;

/// Maximum backend values returned in one catalog facet snapshot.
pub const MAX_CATALOG_BACKEND_FACETS: usize = QuestionBackend::ALL.len();

/// Maximum free-form tags returned in one catalog facet snapshot.
pub const MAX_CATALOG_TAG_FACETS: usize = 64;

/// Maximum response-family values accepted in one catalog query.
pub const MAX_CATALOG_RESPONSE_FAMILY_FILTERS: usize = CatalogResponseFamily::ALL.len();

/// Maximum response-family values returned in one catalog facet snapshot.
pub const MAX_CATALOG_RESPONSE_FAMILY_FACETS: usize = CatalogResponseFamily::ALL.len();

/// Browser-safe immutable response family, derived at publication from the
/// exact [`ResponseDefinition`] rather than a backend or grading implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogResponseFamily {
    /// A numeric response.
    Numeric,
    /// One or more selected choices.
    MultipleChoice,
    /// One short free-text response.
    ShortText,
    /// Several named short-text responses.
    MultiBlank,
    /// Prompt-to-choice associations.
    Matching,
    /// An ordered sequence.
    Ordering,
    /// Point selection on an image-backed surface.
    Hotspot,
    /// A student file upload.
    FileUpload,
    /// A server-brokered external tool.
    ExternalTool,
}

impl CatalogResponseFamily {
    /// Every browser-safe response family supported by this release.
    pub const ALL: [Self; 9] = [
        Self::Numeric,
        Self::MultipleChoice,
        Self::ShortText,
        Self::MultiBlank,
        Self::Matching,
        Self::Ordering,
        Self::Hotspot,
        Self::FileUpload,
        Self::ExternalTool,
    ];
}

impl From<&ResponseDefinition> for CatalogResponseFamily {
    fn from(response: &ResponseDefinition) -> Self {
        match response {
            ResponseDefinition::Numeric { .. } => Self::Numeric,
            ResponseDefinition::MultipleChoice { .. } => Self::MultipleChoice,
            ResponseDefinition::ShortText { .. } => Self::ShortText,
            ResponseDefinition::MultiBlank { .. } => Self::MultiBlank,
            ResponseDefinition::Matching { .. } => Self::Matching,
            ResponseDefinition::Ordering { .. } => Self::Ordering,
            ResponseDefinition::Hotspot { .. } => Self::Hotspot,
            ResponseDefinition::FileUpload { .. } => Self::FileUpload,
            ResponseDefinition::ExternalTool { .. } => Self::ExternalTool,
        }
    }
}

/// Account-bound course-use filter for catalog discovery.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogUsedInMyCourses {
    /// Include every published item regardless of current Account use.
    #[default]
    Any,
    /// Include only publications used in at least one course visible to the Account.
    Used,
}

/// Account-bound authorship scope for catalog discovery.
///
/// The browser selects only this closed meaning. The active authenticated
/// session supplies the actual account identity at the trusted store boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogAuthorship {
    /// Include every published item regardless of authorship.
    #[default]
    Any,
    /// Include publications whose immutable author list contains the current Account.
    AuthoredByCurrentAccount,
}

/// Server-computed count for one exact reviewed public author display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogBylineFacet {
    /// Exact reviewed public display value, never an account identifier.
    pub byline: String,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one closed backend family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogBackendFacet {
    /// Exact public backend value.
    pub backend: QuestionBackend,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one exact stored tag display value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogTagFacet {
    /// Exact public tag display value; it is not lowercased for presentation.
    pub tag: String,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one closed response family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogResponseFamilyFacet {
    /// Exact public response-family value.
    pub response_family: CatalogResponseFamily,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Account-specific reverse-index count from the same catalog query snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogUsedInMyCoursesFacet {
    /// Publications used in one or more courses visible to the current Account.
    pub used: u64,
}

/// One exact controlled-vocabulary term selected in catalog search.
///
/// The label is intentionally absent. It is presentation metadata, while a
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
/// `Other` means the supported other-SPDX class, rather than accepting an
/// arbitrary browser-provided SPDX string as a query primitive.
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

    /// Classifies metadata without exposing an SPDX value as a facet key.
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

/// Availability filter for disclosed, validity-governed evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogEvidenceAvailability {
    /// Include results regardless of evidence availability.
    #[default]
    Any,
    /// Include only publications with disclosed independent student observations.
    Available,
    /// Include only publications without disclosed independent student observations.
    Unavailable,
}

/// Strict, bounded catalog-search request carried across the browser boundary.
///
/// The server normalizes this value before paging and aggregation. The cursor
/// is opaque and tied to that normalized query; positional paging is not
/// representable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CatalogSearchQuery {
    /// Optional full-text-like text query over hot catalog metadata.
    pub text: Option<String>,
    /// Reviewed public author names; any normalized name may match.
    pub bylines: Vec<String>,
    /// Accepted adapter families; any supplied backend may match.
    pub backends: Vec<QuestionBackend>,
    /// Free-form metadata tags; any normalized tag may match.
    pub tags: Vec<String>,
    /// Immutable response families; any supplied family may match.
    pub response_families: Vec<CatalogResponseFamily>,
    /// Exact controlled terms; every supplied term must be present.
    pub taxonomy: Vec<CatalogTaxonomyFilter>,
    /// Required adapter capabilities; every supplied capability must be present.
    pub capabilities: Vec<Capability>,
    /// Accepted license classes; any supplied value may match.
    pub licenses: Vec<CatalogLicenseValue>,
    /// Whether disclosed independent student observations must be available.
    pub evidence: CatalogEvidenceAvailability,
    /// Whether a current Account-visible course use is required.
    ///
    /// This closed filter carries no course reference, title, or identity.
    pub used_in_my_courses: CatalogUsedInMyCourses,
    /// Whether immutable publication authorship by the current Account is required.
    ///
    /// This closed filter carries no browser-provided Account identity.
    pub authorship: CatalogAuthorship,
    /// Opaque continuation cursor from this exact normalized query.
    pub cursor: Option<String>,
    /// Requested bounded page size. `None` selects the server default.
    pub page_size: Option<u16>,
}

/// Canonical D1 filter meaning retained by a personal saved search.
///
/// Pagination is intentionally absent: running a saved search always starts a
/// fresh current-catalog search with a server-selected page size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CatalogSearchFilter {
    pub text: Option<String>,
    pub bylines: Vec<String>,
    pub backends: Vec<QuestionBackend>,
    pub tags: Vec<String>,
    pub response_families: Vec<CatalogResponseFamily>,
    pub taxonomy: Vec<CatalogTaxonomyFilter>,
    pub capabilities: Vec<Capability>,
    pub licenses: Vec<CatalogLicenseValue>,
    pub evidence: CatalogEvidenceAvailability,
    pub used_in_my_courses: CatalogUsedInMyCourses,
    pub authorship: CatalogAuthorship,
}

impl CatalogSearchFilter {
    /// Normalizes durable filter meaning through the D1 query canonicalizer.
    pub fn normalized(self) -> Result<Self, CatalogSearchQueryError> {
        Self::from_query(CatalogSearchQuery::from(self).normalized()?)
    }

    /// Drops cursor and page-size continuation state from one D1 query.
    pub fn from_query(query: CatalogSearchQuery) -> Result<Self, CatalogSearchQueryError> {
        let query = query.normalized()?;
        Ok(Self {
            text: query.text,
            bylines: query.bylines,
            backends: query.backends,
            tags: query.tags,
            response_families: query.response_families,
            taxonomy: query.taxonomy,
            capabilities: query.capabilities,
            licenses: query.licenses,
            evidence: query.evidence,
            used_in_my_courses: query.used_in_my_courses,
            authorship: query.authorship,
        })
    }

    /// Starts a fresh D1 query without continuation state.
    pub fn fresh_query(&self) -> CatalogSearchQuery {
        CatalogSearchQuery::from(self.clone())
    }
}

impl From<CatalogSearchFilter> for CatalogSearchQuery {
    fn from(filter: CatalogSearchFilter) -> Self {
        Self {
            text: filter.text,
            bylines: filter.bylines,
            backends: filter.backends,
            tags: filter.tags,
            response_families: filter.response_families,
            taxonomy: filter.taxonomy,
            capabilities: filter.capabilities,
            licenses: filter.licenses,
            evidence: filter.evidence,
            used_in_my_courses: filter.used_in_my_courses,
            authorship: filter.authorship,
            cursor: None,
            page_size: None,
        }
    }
}

impl Default for CatalogSearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            bylines: Vec::new(),
            backends: Vec::new(),
            tags: Vec::new(),
            response_families: Vec::new(),
            taxonomy: Vec::new(),
            capabilities: Vec::new(),
            licenses: Vec::new(),
            evidence: CatalogEvidenceAvailability::Any,
            used_in_my_courses: CatalogUsedInMyCourses::Any,
            authorship: CatalogAuthorship::Any,
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
    /// Returns the immutable-publication Question ID named in the text field.
    pub fn exact_question_id(&self) -> Option<QuestionId> {
        self.text.as_deref()?.parse::<QuestionId>().ok()
    }

    /// Produces the canonical query used for both rows and facet aggregates.
    ///
    /// Text, public bylines, and tags use lowercased, whitespace-collapsed
    /// Unicode text. Controlled terms retain durable case after trimming.
    /// Text, every active metadata filter family, Account-bound filters, and
    /// `authorship`
    /// combine with every other active filter using AND. Within bylines,
    /// backends, tags, response families, and licenses, values combine using
    /// OR. Taxonomy and capabilities retain every-value-matches semantics.
    pub fn normalized(mut self) -> Result<Self, CatalogSearchQueryError> {
        self.text = self
            .text
            .map(|text| normalize_text(text, 256))
            .transpose()?
            .filter(|text| !text.is_empty());
        normalize_text_filters(&mut self.bylines, MAX_CATALOG_BYLINE_FILTERS, 120)?;
        normalize_text_filters(&mut self.tags, MAX_CATALOG_TAG_FILTERS, 256)?;
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
        if self.taxonomy.len() > MAX_CATALOG_TAXONOMY_FACETS
            || self.capabilities.len() > Capability::ALL.len()
            || self.licenses.len() > 6
            || self.backends.len() > QuestionBackend::ALL.len()
            || self.response_families.len() > MAX_CATALOG_RESPONSE_FAMILY_FILTERS
        {
            return Err(CatalogSearchQueryError::TooLarge);
        }
        self.taxonomy.sort();
        self.taxonomy.dedup();
        self.capabilities.sort();
        self.capabilities.dedup();
        self.licenses.sort();
        self.licenses.dedup();
        self.backends.sort();
        self.backends.dedup();
        self.response_families.sort();
        self.response_families.dedup();
        if self.cursor.as_ref().is_some_and(String::is_empty) {
            return Err(CatalogSearchQueryError::EmptyCursor);
        }
        Ok(self)
    }
}

fn normalize_text_filters(
    filters: &mut Vec<String>,
    maximum_filters: usize,
    maximum_characters: usize,
) -> Result<(), CatalogSearchQueryError> {
    if filters.len() > maximum_filters {
        return Err(CatalogSearchQueryError::TooLarge);
    }
    for filter in filters.iter_mut() {
        *filter = normalize_text(std::mem::take(filter), maximum_characters)?;
        if filter.is_empty() {
            return Err(CatalogSearchQueryError::BlankFilter);
        }
    }
    filters.sort();
    filters.dedup();
    Ok(())
}

fn normalize_text(
    value: String,
    maximum_characters: usize,
) -> Result<String, CatalogSearchQueryError> {
    let normalized = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if normalized.chars().count() > maximum_characters {
        return Err(CatalogSearchQueryError::TooLarge);
    }
    Ok(normalized)
}

/// Server-computed count for a controlled taxonomy value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogTaxonomyFacet {
    /// Controlled term identity and display label.
    pub term: TaxonomyTerm,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogCapabilityFacet {
    /// Capability represented by the count.
    pub capability: Capability,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Server-computed count for one license class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogLicenseFacet {
    /// License class represented by the count.
    pub license: CatalogLicenseValue,
    /// Number of matching discoverable publications in the query snapshot.
    pub count: u64,
}

/// Disclosure-state counts from the same immutable query snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogEvidenceFacet {
    /// Publications with disclosed independent student observations.
    pub available: u64,
    /// Publications without disclosed independent student observations.
    pub unavailable: u64,
}

/// Aggregates computed from one normalized query snapshot, never a page sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogSearchFacets {
    /// Exact reviewed public byline counts.
    pub bylines: Vec<CatalogBylineFacet>,
    /// Closed adapter-family counts.
    pub backends: Vec<CatalogBackendFacet>,
    /// Exact stored public tag counts.
    pub tags: Vec<CatalogTagFacet>,
    /// Closed immutable response-family counts.
    pub response_families: Vec<CatalogResponseFamilyFacet>,
    /// Controlled taxonomy counts.
    pub taxonomy: Vec<CatalogTaxonomyFacet>,
    /// Adapter capability counts.
    pub capabilities: Vec<CatalogCapabilityFacet>,
    /// Reuse-license counts.
    pub licenses: Vec<CatalogLicenseFacet>,
    /// Validity-governed evidence availability counts.
    pub evidence: CatalogEvidenceFacet,
    /// Account-specific current-course-use count.
    pub used_in_my_courses: CatalogUsedInMyCoursesFacet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_family_is_derived_from_the_immutable_response_definition() {
        let response = ResponseDefinition::ExternalTool {};
        assert_eq!(
            CatalogResponseFamily::from(&response),
            CatalogResponseFamily::ExternalTool
        );
        assert_eq!(
            CatalogResponseFamily::ALL
                .into_iter()
                .map(|family| serde_json::to_value(family).expect("family serializes"))
                .collect::<Vec<_>>(),
            vec![
                serde_json::json!("numeric"),
                serde_json::json!("multipleChoice"),
                serde_json::json!("shortText"),
                serde_json::json!("multiBlank"),
                serde_json::json!("matching"),
                serde_json::json!("ordering"),
                serde_json::json!("hotspot"),
                serde_json::json!("fileUpload"),
                serde_json::json!("externalTool"),
            ]
        );
    }

    #[test]
    fn authored_scope_is_a_closed_account_bound_filter_and_survives_saved_search_conversion() {
        let query = CatalogSearchQuery {
            authorship: CatalogAuthorship::AuthoredByCurrentAccount,
            ..CatalogSearchQuery::default()
        };
        assert_eq!(
            serde_json::to_value(&query).expect("query serializes")["authorship"],
            serde_json::json!("authoredByCurrentAccount")
        );
        let filter = CatalogSearchFilter::from_query(query).expect("filter normalizes");
        assert_eq!(
            filter.authorship,
            CatalogAuthorship::AuthoredByCurrentAccount
        );
        assert_eq!(
            filter.fresh_query().authorship,
            CatalogAuthorship::AuthoredByCurrentAccount
        );
    }

    #[test]
    fn catalog_search_roots_use_strict_snake_case_without_scope_or_paging_state() {
        let query = CatalogSearchQuery {
            response_families: vec![CatalogResponseFamily::ShortText],
            used_in_my_courses: CatalogUsedInMyCourses::Used,
            cursor: Some("opaque-cursor".to_string()),
            page_size: Some(25),
            ..CatalogSearchQuery::default()
        };
        let query_json = serde_json::to_value(&query).expect("query serializes");
        assert_eq!(
            query_json["response_families"],
            serde_json::json!(["shortText"])
        );
        assert_eq!(query_json["used_in_my_courses"], serde_json::json!("used"));
        assert_eq!(query_json["page_size"], serde_json::json!(25));
        assert!(query_json.get("responseFamilies").is_none());
        assert!(query_json.get("usedInMyCourses").is_none());

        let filter = CatalogSearchFilter::from_query(query).expect("filter normalizes");
        let filter_json = serde_json::to_value(&filter).expect("filter serializes");
        assert_eq!(
            filter_json["response_families"],
            serde_json::json!(["shortText"])
        );
        assert!(filter_json.get("cursor").is_none());
        assert!(filter_json.get("page_size").is_none());
        assert_eq!(filter.fresh_query().cursor, None);
        assert_eq!(filter.fresh_query().page_size, None);

        for retired_field in ["publication_scopes", "publicationScopes"] {
            let mut rejected_query = query_json.clone();
            rejected_query[retired_field] = serde_json::json!(["public"]);
            assert!(
                serde_json::from_value::<CatalogSearchQuery>(rejected_query).is_err(),
                "query rejects retired {retired_field}"
            );

            let mut rejected_filter = filter_json.clone();
            rejected_filter[retired_field] = serde_json::json!(["public"]);
            assert!(
                serde_json::from_value::<CatalogSearchFilter>(rejected_filter).is_err(),
                "filter rejects retired {retired_field}"
            );
        }
    }
}
