//! Bounded, display-name-only co-instructor target discovery contracts.

use serde::{Deserialize, Serialize};

use crate::AccountReference;

use super::{TeachingDisplayLabel, TeachingOperationRevision, TeachingPageSize};

/// Smallest useful display-name fragment for co-instructor target discovery.
///
/// Requiring two characters prevents this endpoint from becoming an account
/// directory while still supporting ordinary name lookup.
pub const MIN_CO_INSTRUCTOR_TARGET_SEARCH_QUERY_UNICODE_SCALARS: usize = 2;
/// Discovery input is intentionally shorter than a display label.
pub const MAX_CO_INSTRUCTOR_TARGET_SEARCH_QUERY_UNICODE_SCALARS: usize = 100;

/// Safe current account projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingAccountView {
    pub reference: AccountReference,
    pub display: TeachingDisplayLabel,
}

/// Operator-owned approval projection, never course authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountApprovalView {
    pub state: InstructorApprovalStateView,
    pub revision: TeachingOperationRevision,
}

/// Closed non-authorizing account eligibility state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstructorApprovalStateView {
    Approved,
    Revoked,
}

/// Account eligible as a co-instructor invitation target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTargetView {
    pub account: TeachingAccountView,
    pub approval: AccountApprovalView,
}

/// A bounded, nonblank display-name fragment used only for co-instructor
/// target discovery. It is not an email address, account identifier, or
/// general account-search capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInvitationTargetSearchQuery(String);

impl CourseInvitationTargetSearchQuery {
    /// Returns the validated display-name fragment.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CourseInvitationTargetSearchQuery {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let length = value.chars().count();
        if value.trim() != value
            || !(MIN_CO_INSTRUCTOR_TARGET_SEARCH_QUERY_UNICODE_SCALARS
                ..=MAX_CO_INSTRUCTOR_TARGET_SEARCH_QUERY_UNICODE_SCALARS)
                .contains(&length)
        {
            return Err(
                "co-instructor target search query must be trimmed and contain 2 to 100 characters",
            );
        }
        Ok(Self(value))
    }
}

impl From<CourseInvitationTargetSearchQuery> for String {
    fn from(value: CourseInvitationTargetSearchQuery) -> Self {
        value.0
    }
}

/// Strict bounded request for safe co-instructor target discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTargetSearchRequest {
    pub query: CourseInvitationTargetSearchQuery,
    /// Server-issued opaque continuation token, or `null` for the first page.
    pub after: Option<String>,
    pub size: TeachingPageSize,
}

/// Authorized bounded co-instructor target search result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTargetSearchPage {
    pub targets: Vec<CourseInvitationTargetView>,
    pub next_cursor: Option<String>,
}

/// Sysadmin-only candidate for manual Instructor approval.
///
/// The reference is an opaque locator.  The projection deliberately omits
/// email, UUIDs, external-affiliation facts, and course relationships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SysadminInstructorCandidateView {
    pub account: TeachingAccountView,
    pub approval: SysadminInstructorApprovalView,
}

/// The only approval facts needed to decide whether to approve or revoke one
/// candidate.  A missing revision means no approval record exists yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SysadminInstructorApprovalView {
    pub state: SysadminInstructorApprovalStateView,
    pub revision: Option<TeachingOperationRevision>,
}

/// Closed approval states visible to a Sysadmin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SysadminInstructorApprovalStateView {
    Unapproved,
    Approved,
    Revoked,
}

/// Strict bounded request for display-name-only Sysadmin candidate discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SysadminInstructorCandidateSearchRequest {
    pub query: CourseInvitationTargetSearchQuery,
    /// Server-issued opaque continuation token, or `null` for the first page.
    pub after: Option<String>,
    pub size: TeachingPageSize,
}

/// Authorized bounded Sysadmin candidate search result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SysadminInstructorCandidateSearchPage {
    pub candidates: Vec<SysadminInstructorCandidateView>,
    pub next_cursor: Option<String>,
}
