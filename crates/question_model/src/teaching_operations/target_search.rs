//! Bounded, display-name-only Instructor Course Invitation target discovery contracts.

use serde::{Deserialize, Serialize};

use crate::AccountReference;

use super::{TeachingDisplayLabel, TeachingPageSize};

/// Smallest useful display-name fragment for Instructor Course Invitation target discovery.
///
/// Requiring two characters prevents this endpoint from becoming an account
/// directory while still supporting ordinary name lookup.
pub const MIN_TEACHING_ACCOUNT_SEARCH_QUERY_UNICODE_SCALARS: usize = 2;
/// Discovery input is intentionally shorter than a display label.
pub const MAX_TEACHING_ACCOUNT_SEARCH_QUERY_UNICODE_SCALARS: usize = 100;

/// Safe current Teaching Account View.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingAccountView {
    pub reference: AccountReference,
    pub display: TeachingDisplayLabel,
}

/// Account eligible as an Instructor Course Invitation target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTargetView {
    pub account: TeachingAccountView,
}

/// A bounded, nonblank display-name fragment used only for Instructor Course Invitation
/// target discovery. It is not an email address, account identifier, or
/// general account-search capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TeachingAccountSearchQuery(String);

impl TeachingAccountSearchQuery {
    /// Returns the validated display-name fragment.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TeachingAccountSearchQuery {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let length = value.chars().count();
        if value.trim() != value
            || !(MIN_TEACHING_ACCOUNT_SEARCH_QUERY_UNICODE_SCALARS
                ..=MAX_TEACHING_ACCOUNT_SEARCH_QUERY_UNICODE_SCALARS)
                .contains(&length)
        {
            return Err(
                "Instructor Course Invitation target search query must be trimmed and contain 2 to 100 characters",
            );
        }
        Ok(Self(value))
    }
}

impl From<TeachingAccountSearchQuery> for String {
    fn from(value: TeachingAccountSearchQuery) -> Self {
        value.0
    }
}

/// Strict bounded request for safe Instructor Course Invitation target discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTargetSearchRequest {
    pub query: TeachingAccountSearchQuery,
    /// Server-issued opaque continuation token, or `null` for the first page.
    pub after: Option<String>,
    pub size: TeachingPageSize,
}

/// Authorized bounded Instructor Course Invitation target search result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTargetSearchPage {
    pub targets: Vec<CourseInvitationTargetView>,
    pub next_cursor: Option<String>,
}
