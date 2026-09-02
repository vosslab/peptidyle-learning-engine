//! Strict browser/server contracts for the WP-INST-T2 teaching operations.
//!
//! The types here use only human route references and display labels.  They
//! deliberately exclude external-affiliation IDs, UUIDs, email, policy inputs, jobs, object
//! keys, recipient lists, answer material, and clock authority.  A server maps
//! its authorized Store/domain result into these values after resolving S5/S3.

use std::num::{NonZeroU32, NonZeroU64};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    AccountReference, AssignmentDeadlineRule, CourseInvitationReference, CourseLocalDateAndTime,
    CourseMembershipReference, CourseTimeZone, LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS, Timestamp,
};

mod assignment_policy_source;
mod local_time;
mod modifier_revision_response;
mod student_memberships;
mod target_search;

pub use assignment_policy_source::AssignmentPolicySource;
pub use local_time::{convert_teaching_preview_time_field, resolve_teaching_local_time};
pub use modifier_revision_response::TeachingOperationRevisionResponse;
pub use student_memberships::CourseStudentMembershipsPage;
pub use target_search::{
    CourseInvitationTargetSearchPage, CourseInvitationTargetSearchRequest,
    CourseInvitationTargetView, MAX_TEACHING_ACCOUNT_SEARCH_QUERY_UNICODE_SCALARS,
    MIN_TEACHING_ACCOUNT_SEARCH_QUERY_UNICODE_SCALARS, TeachingAccountSearchQuery,
    TeachingAccountView,
};

/// Maximum Unicode scalar count for an authorized account or membership label.
pub const MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS: usize = 200;
/// Maximum rows in one browser teaching-operations page.
pub const MAX_TEACHING_PAGE_SIZE: u32 = 100;
/// Largest browser-requestable retention extension in whole days.
pub const MAX_RETENTION_EXTENSION_DAYS: u32 = 36_500;

/// A nonzero teaching-operations page size that cannot exceed the route limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct TeachingPageSize(NonZeroU32);

impl TeachingPageSize {
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for TeachingPageSize {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .filter(|size| size.get() <= MAX_TEACHING_PAGE_SIZE)
            .map(Self)
            .ok_or("teaching page size must be between 1 and 100")
    }
}

impl From<TeachingPageSize> for u32 {
    fn from(value: TeachingPageSize) -> Self {
        value.get()
    }
}

/// Positive PostgreSQL-safe strong revision represented as canonical decimal JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TeachingOperationRevision(NonZeroU64);

impl TeachingOperationRevision {
    /// Creates a positive revision that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value <= i64::MAX as u64)
            .then(|| NonZeroU64::new(value))
            .flatten()
            .map(Self)
    }

    /// Returns the exact revision used in a strong conditional request.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for TeachingOperationRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

impl FromStr for TeachingOperationRevision {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("teaching revision must be a canonical positive decimal string");
        }
        value
            .parse::<u64>()
            .ok()
            .and_then(Self::new)
            .ok_or("teaching revision must fit a positive PostgreSQL bigint")
    }
}

impl TryFrom<String> for TeachingOperationRevision {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<TeachingOperationRevision> for String {
    fn from(value: TeachingOperationRevision) -> Self {
        value.to_string()
    }
}

/// Validated, nonblank browser display label with no email semantics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TeachingDisplayLabel(String);

impl TeachingDisplayLabel {
    /// Returns the safe human label.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TeachingDisplayLabel {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty()
            || value.trim() != value
            || value.chars().count() > MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS
        {
            return Err("display label must be trimmed, nonblank, and at most 200 characters");
        }
        Ok(Self(value))
    }
}

impl From<TeachingDisplayLabel> for String {
    fn from(value: TeachingDisplayLabel) -> Self {
        value.0
    }
}

/// One opaque, bounded cursor request for teaching operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MembershipPageRequest {
    /// Server-issued opaque continuation token, or `null` for the first page.
    pub after: Option<String>,
    /// Required bounded page size.
    pub size: TeachingPageSize,
}

/// Browser-safe Student Membership View; email and UUID are absent by type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentMembershipView {
    /// Course-local membership locator.
    pub reference: CourseMembershipReference,
    /// Authorized display label.
    pub display: TeachingDisplayLabel,
    /// Closed course role.
    pub role: TeachingMembershipRole,
    /// Closed current membership status.
    pub status: TeachingMembershipStatus,
}

/// Closed browser role vocabulary for a current course membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeachingMembershipRole {
    Instructor,
    Student,
}

/// Closed browser current-status vocabulary for a course membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TeachingMembershipStatus {
    Active,
    Revoked,
}

/// Closed M3/M4 modification behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccommodationApplicationRuleView {
    ExtendOnly,
    Replace,
}

/// Explicit adjustment state for a resolved time field; omitted fields never mean inherit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TeachingTimeFieldPatch {
    Inherit,
    Set { value: CourseLocalDateAndTime },
    Unrestricted,
}

/// Explicit adjustment state for a resolved positive integer field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TeachingAssignmentAttemptTimeLimitFieldPatch {
    Inherit,
    Set {
        value: TeachingAssignmentAttemptTimeLimitSeconds,
    },
    Unrestricted,
}

/// Explicit adjustment state for an attempt limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TeachingAttemptLimitFieldPatch {
    Inherit,
    Set { value: TeachingAttemptLimit },
    Unrestricted,
}

/// Positive time limit that fits the PostgreSQL `INTEGER` policy column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct TeachingAssignmentAttemptTimeLimitSeconds(NonZeroU32);

impl TryFrom<u32> for TeachingAssignmentAttemptTimeLimitSeconds {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .filter(|limit| limit.get() <= MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
            .map(Self)
            .ok_or("time limit must fit the assignment policy bounds")
    }
}

impl From<TeachingAssignmentAttemptTimeLimitSeconds> for u32 {
    fn from(value: TeachingAssignmentAttemptTimeLimitSeconds) -> Self {
        value.0.get()
    }
}

/// Positive attempt limit that fits the PostgreSQL `INTEGER` policy column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct TeachingAttemptLimit(NonZeroU32);

impl TryFrom<u32> for TeachingAttemptLimit {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .filter(|limit| limit.get() <= MAX_ASSIGNMENT_ATTEMPT_LIMIT)
            .map(Self)
            .ok_or("attempt limit must fit the assignment policy bounds")
    }
}

impl From<TeachingAttemptLimit> for u32 {
    fn from(value: TeachingAttemptLimit) -> Self {
        value.0.get()
    }
}

/// Complete M3/M4 adjustment replacement: every adjustment state is explicit and closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccommodationAdjustmentView {
    pub available_at: TeachingTimeFieldPatch,
    pub due_at: TeachingTimeFieldPatch,
    pub closes_at: TeachingTimeFieldPatch,
    pub assignment_attempt_time_limit_seconds: TeachingAssignmentAttemptTimeLimitFieldPatch,
    pub attempt_limit: TeachingAttemptLimitFieldPatch,
}

/// Complete Assignment policy adjustment replacement used by a synthetic preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPreviewAccommodationAdjustmentRequest {
    pub mode: AccommodationApplicationRuleView,
    pub adjustment: AccommodationAdjustmentView,
}

/// Strict direct Student Accommodation update; the membership reference selects its scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccommodationAdjustmentUpdateRequest {
    pub mode: AccommodationApplicationRuleView,
    pub adjustment: AccommodationAdjustmentView,
}

/// Server-derived resolved time value and its safe source label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingPreviewTimeField {
    pub value: Option<CourseLocalDateAndTime>,
    pub source: AssignmentPolicySource,
}

/// Server-derived resolved positive limit and its safe source label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingPreviewLimitField {
    pub value: Option<NonZeroU32>,
    pub source: AssignmentPolicySource,
}

/// Closed safe reason for an S5 preview denial. It never exposes S3 inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TeachingPreviewDenialReason {
    ActiveStudentCourseMembershipRequired,
}

/// Closed S3 start verdict for a preview subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TeachingAssignmentStartDecision {
    MayStart { late: TeachingStudentLateWorkStatus },
    NotYetAvailable,
    Closed,
    AttemptLimitReached,
    LateWorkRefused,
}

/// Closed late status supplied only inside an allowed S3 start decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TeachingStudentLateWorkStatus {
    OnTime,
    AcceptedLate,
    MarkedLate,
}

/// Server-derived resolved late-submission behavior and its safe source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingPreviewLateWorkRuleField {
    pub value: LateWorkRule,
    pub source: AssignmentPolicySource,
}

/// Server-derived resolved deadline behavior and its safe source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingPreviewAssignmentDeadlineRuleField {
    pub value: AssignmentDeadlineRule,
    pub source: AssignmentPolicySource,
}

/// Server-derived S5/S3 preview. Denied views cannot carry a schedule or an Assignment Policy Source.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "active_student_course_membership",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TeachingPreviewView {
    Denied {
        reason: TeachingPreviewDenialReason,
    },
    Allowed {
        /// Course-owned zone that gives every resolved local time its meaning.
        time_zone: CourseTimeZone,
        start: TeachingAssignmentStartDecision,
        available_at: TeachingPreviewTimeField,
        due_at: TeachingPreviewTimeField,
        closes_at: TeachingPreviewTimeField,
        assignment_attempt_time_limit_seconds: TeachingPreviewLimitField,
        attempt_limit: TeachingPreviewLimitField,
        late_work_rule: TeachingPreviewLateWorkRuleField,
        assignment_deadline_rule: TeachingPreviewAssignmentDeadlineRuleField,
    },
}

/// Strict Instructor Course Invitation creation request for one existing Account.
///
/// The target-discovery and teaching-team endpoints are exclusively for adding
/// an Instructor Course Membership. The generic `CourseInvitation` persistence
/// value carries the role for workflows that support other membership roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorCourseInvitationCreateRequest {
    pub target: AccountReference,
}

/// Course-authorized Instructor Course Invitation row with no email or raw identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorCourseInvitationView {
    pub reference: CourseInvitationReference,
    pub target: CourseInvitationTargetView,
    pub state: CourseInvitationStateView,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub revision: TeachingOperationRevision,
}

/// Bounded exact-course Instructor Course Invitation page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorCourseInvitationsPage {
    pub invitations: Vec<InstructorCourseInvitationView>,
    pub next_cursor: Option<String>,
}

/// Pending account-owned invitation row. It intentionally contains no email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingCourseInvitationView {
    pub reference: CourseInvitationReference,
    pub course_label: TeachingDisplayLabel,
    pub state: CourseInvitationStateView,
    pub expires_at: Timestamp,
    pub revision: TeachingOperationRevision,
}

/// Closed pending-invitation lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseInvitationStateView {
    Pending,
    Expired,
    Accepted,
    Declined,
    Revoked,
}

/// Bounded pending invitation page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingCourseInvitationsPage {
    pub invitations: Vec<PendingCourseInvitationView>,
    pub next_cursor: Option<String>,
}

/// Closed terminal action for the authenticated invitation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseInvitationTerminalAction {
    Accept,
    Decline,
}

/// Strict terminal pending-invitation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseInvitationTerminalActionRequest {
    pub action: CourseInvitationTerminalAction,
}

/// Direct Instructor course membership row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorMembershipView {
    pub membership: CourseMembershipReference,
    pub account: TeachingAccountView,
    pub status: TeachingMembershipStatus,
}

/// Bounded direct Instructor list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorMembershipsPage {
    pub instructors: Vec<InstructorMembershipView>,
    pub next_cursor: Option<String>,
    /// Exact roster revision required by direct-Instructor removal `If-Match`.
    pub roster_revision: TeachingOperationRevision,
}

/// Empty-body direct Instructor removal action; its revision is `If-Match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorMembershipRemovalRequest {}

/// Coarse server-owned retention state with no deadline, job, recipient, or policy input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionStateView {
    Active,
    NotificationDue,
    StudentRecordsArchived,
    StudentRecordsDeleted,
}

/// Closed Assignment Content disposition for a retention action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionDispositionView {
    Retain,
    Delete,
}

/// Validated whole-day retention extension accepted by the existing server boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct RetentionAdditionalDays(NonZeroU32);

impl RetentionAdditionalDays {
    /// Returns the bounded number of additional days.
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for RetentionAdditionalDays {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .filter(|days| days.get() <= MAX_RETENTION_EXTENSION_DAYS)
            .map(Self)
            .ok_or("retention extension must be between 1 and 36500 whole days")
    }
}

impl From<RetentionAdditionalDays> for u32 {
    fn from(value: RetentionAdditionalDays) -> Self {
        value.get()
    }
}

/// Browser-safe retention read matching the server's strong revision boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionReadView {
    pub state: RetentionStateView,
    pub assignment_content: RetentionDispositionView,
    pub revision: TeachingOperationRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<RetentionNotificationView>,
}

/// Authorized notification accompanying retention GET and extend responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionNotificationView {
    pub intent: RetentionNotificationIntentView,
    pub created_at: Timestamp,
    pub copy: String,
}

/// Closed Course Retention Notice intent from the existing retention read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionNotificationIntentView {
    Archive,
    Delete,
    Extend,
}

/// Exact JSON body for the existing retention archive endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionArchiveRequest {
    pub assignment_content: RetentionDispositionView,
}

/// Exact JSON body for the existing retention extend endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionExtendRequest {
    pub additional_days: RetentionAdditionalDays,
}

/// Replay-safe result of a server-owned retention request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionActionOutcomeView {
    Scheduled,
    InProgress,
    Completed,
}

/// Browser-safe archive/delete response with its action outcome and new revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionActionResponse {
    pub state: RetentionStateView,
    pub assignment_content: RetentionDispositionView,
    pub revision: TeachingOperationRevision,
    pub outcome: RetentionActionOutcomeView,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revisions_and_labels_are_canonical_and_bounded() {
        assert!("01".parse::<TeachingOperationRevision>().is_err());
        assert!("0".parse::<TeachingOperationRevision>().is_err());
        assert_eq!(
            "42".parse::<TeachingOperationRevision>().unwrap().value(),
            42
        );
        assert!(TeachingDisplayLabel::try_from(" ".to_owned()).is_err());
    }

    #[test]
    fn mutation_bodies_are_value_only_and_revisions_are_absent() {
        let action = serde_json::to_value(CourseInvitationTerminalActionRequest {
            action: CourseInvitationTerminalAction::Accept,
        })
        .unwrap();
        assert_eq!(action, serde_json::json!({"action":"accept"}));
        assert_eq!(
            serde_json::to_value(InstructorMembershipRemovalRequest {}).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn validated_deserialization_rejects_bounds_and_duplicate_members() {
        assert!(
            serde_json::from_str::<MembershipPageRequest>(r#"{"after":null,"size":101}"#).is_err()
        );
        assert!(
            serde_json::from_str::<CourseInvitationTargetSearchRequest>(
                r#"{"query":"t","after":null,"size":10}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<CourseInvitationTargetSearchRequest>(
                r#"{"query":"  target","after":null,"size":10}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<CourseInvitationTargetSearchRequest>(
                r#"{"query":"target","after":null,"size":10,"email":"x@example.edu"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<AccommodationAdjustmentView>(concat!(
                r#"{"availableAt":{"kind":"inherit"},"dueAt":{"kind":"inherit"},"#,
                r#""closesAt":{"kind":"inherit"},"assignmentAttemptTimeLimitSeconds":{"kind":"set","#,
                r#""value":2147483648},"attemptLimit":{"kind":"inherit"}}"#
            ))
            .is_err()
        );
    }

    #[test]
    fn denied_preview_omits_every_resolved_schedule_key() {
        let denied = serde_json::to_value(TeachingPreviewView::Denied {
            reason: TeachingPreviewDenialReason::ActiveStudentCourseMembershipRequired,
        })
        .unwrap();
        assert_eq!(
            denied,
            serde_json::json!({"active_student_course_membership":"denied","reason":"activeStudentCourseMembershipRequired"})
        );
        for key in [
            "start",
            "availableAt",
            "dueAt",
            "closesAt",
            "source",
            "timeZone",
            "lateWorkRule",
        ] {
            assert!(denied.get(key).is_none());
        }
    }

    #[test]
    fn retention_endpoint_bodies_and_notification_are_exact() {
        assert_eq!(
            serde_json::to_value(RetentionArchiveRequest {
                assignment_content: RetentionDispositionView::Retain
            })
            .unwrap(),
            serde_json::json!({"assignmentContent":"retain"})
        );
        assert_eq!(
            serde_json::to_value(RetentionExtendRequest {
                additional_days: RetentionAdditionalDays::try_from(7).unwrap()
            })
            .unwrap(),
            serde_json::json!({"additionalDays":7})
        );
        assert!(
            serde_json::from_str::<RetentionExtendRequest>(r#"{"additionalDays":36501}"#).is_err()
        );
        let notification = RetentionReadView {
            state: RetentionStateView::NotificationDue,
            assignment_content: RetentionDispositionView::Retain,
            revision: "7".parse().unwrap(),
            notification: Some(RetentionNotificationView {
                intent: RetentionNotificationIntentView::Archive,
                created_at: Timestamp::from_unix_millis(1),
                copy: "Server-owned copy".to_owned(),
            }),
        };
        assert_eq!(
            serde_json::to_value(notification).unwrap()["notification"]["createdAt"],
            1
        );
    }

    #[test]
    fn pending_invitation_serializes_server_owned_expiry_without_inviter() {
        let row = PendingCourseInvitationView {
            reference: "CI-4".parse().unwrap(),
            course_label: TeachingDisplayLabel::try_from("Biochemistry".to_owned()).unwrap(),
            state: CourseInvitationStateView::Pending,
            expires_at: Timestamp::from_unix_millis(2_592_000_000),
            revision: "3".parse().unwrap(),
        };
        let value = serde_json::to_value(row).unwrap();
        assert_eq!(value["expiresAt"], 2_592_000_000_i64);
        assert!(value.get("invitedBy").is_none());
    }
}
