//! Strict browser/server contracts for teaching operations.
//!
//! The types here use only human route references and display labels.  They
//! deliberately exclude external-affiliation IDs, UUIDs, email, policy inputs, jobs, object
//! keys, recipient lists, Answer Key facts, and clock authority. A server maps
//! its authorized Store/domain result into these values after resolving Active Student
//! Course Membership and Effective Assessment Policy.

use std::num::{NonZeroU32, NonZeroU64};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    CourseInvitationId, LocalDateAndTime, MAX_ASSESSMENT_ATTEMPT_LIMIT,
    MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS, Timestamp,
};

pub use crate::preview_plane::HypotheticalStudentViewScenarioModifiers;

/// Maximum Unicode scalar count for an authorized account or membership label.
pub const MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS: usize = 200;
/// Maximum rows in one browser teaching-operations page.
pub const MAX_TEACHING_PAGE_SIZE: u32 = 100;

fn canonical_positive_postgres_bigint(value: &str) -> Result<NonZeroU64, &'static str> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("must be a canonical positive decimal string");
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value <= i64::MAX as u64)
        .and_then(NonZeroU64::new)
        .ok_or("must fit a positive PostgreSQL bigint")
}

/// One Course Invitation's current lifecycle-state precondition, represented as canonical decimal JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInvitationStatePrecondition(NonZeroU64);

impl CourseInvitationStatePrecondition {
    /// Creates a positive lifecycle-state precondition that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value <= i64::MAX as u64)
            .then(|| NonZeroU64::new(value))
            .flatten()
            .map(Self)
    }

    /// Returns the exact lifecycle-state precondition used in a strong conditional request.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for CourseInvitationStatePrecondition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

impl FromStr for CourseInvitationStatePrecondition {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        canonical_positive_postgres_bigint(value).map(Self)
    }
}

impl TryFrom<String> for CourseInvitationStatePrecondition {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<CourseInvitationStatePrecondition> for String {
    fn from(value: CourseInvitationStatePrecondition) -> Self {
        value.to_string()
    }
}

/// One Course Roster's current change number, represented as canonical decimal JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseRosterChangeNumber(NonZeroU64);

impl CourseRosterChangeNumber {
    /// Creates a positive change number that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value <= i64::MAX as u64)
            .then(|| NonZeroU64::new(value))
            .flatten()
            .map(Self)
    }

    /// Returns the exact change number used in a strong conditional request.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for CourseRosterChangeNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

impl FromStr for CourseRosterChangeNumber {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        canonical_positive_postgres_bigint(value).map(Self)
    }
}

impl TryFrom<String> for CourseRosterChangeNumber {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<CourseRosterChangeNumber> for String {
    fn from(value: CourseRosterChangeNumber) -> Self {
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

/// Closed modification behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccommodationApplicationRuleView {
    ExtendOnly,
    Replace,
}

/// Explicit adjustment state for a resolved time field; omitted fields never mean inherit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TeachingTimeFieldPatch {
    Inherit,
    Set { value: LocalDateAndTime },
    Unrestricted,
}

/// Explicit adjustment state for a resolved positive integer field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TeachingAssessmentAttemptTimeLimitFieldPatch {
    Inherit,
    Set {
        value: TeachingAssessmentAttemptTimeLimitSeconds,
    },
    Unrestricted,
}

/// Explicit adjustment state for an attempt limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TeachingAttemptLimitFieldPatch {
    Inherit,
    Set { value: TeachingAttemptLimit },
    Unrestricted,
}

/// Positive time limit that fits the PostgreSQL `INTEGER` policy column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct TeachingAssessmentAttemptTimeLimitSeconds(NonZeroU32);

impl TryFrom<u32> for TeachingAssessmentAttemptTimeLimitSeconds {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .filter(|limit| limit.get() <= MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS)
            .map(Self)
            .ok_or("time limit must fit the assessment policy bounds")
    }
}

impl From<TeachingAssessmentAttemptTimeLimitSeconds> for u32 {
    fn from(value: TeachingAssessmentAttemptTimeLimitSeconds) -> Self {
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
            .filter(|limit| limit.get() <= MAX_ASSESSMENT_ATTEMPT_LIMIT)
            .map(Self)
            .ok_or("attempt limit must fit the assessment policy bounds")
    }
}

impl From<TeachingAttemptLimit> for u32 {
    fn from(value: TeachingAttemptLimit) -> Self {
        value.0.get()
    }
}

/// Complete adjustment replacement: every adjustment state is explicit and closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AccommodationAdjustmentView {
    pub available_at: TeachingTimeFieldPatch,
    pub due_at: TeachingTimeFieldPatch,
    pub closes_at: TeachingTimeFieldPatch,
    pub assessment_attempt_time_limit_seconds: TeachingAssessmentAttemptTimeLimitFieldPatch,
    pub attempt_limit: TeachingAttemptLimitFieldPatch,
}

/// Pending account-owned invitation row. It intentionally contains no email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingCourseInvitationView {
    pub id: CourseInvitationId,
    pub course_label: TeachingDisplayLabel,
    pub state: CourseInvitationStateView,
    pub expires_at: Timestamp,
    /// Exact lifecycle-state precondition required by accept or decline `If-Match`.
    #[serde(rename = "state_precondition")]
    pub state_precondition: CourseInvitationStatePrecondition,
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
    /// Authenticated Instructor's own IANA zone for rendering this page's instants.
    pub display_time_zone: crate::AccountTimeZone,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_preconditions_and_labels_are_canonical_and_bounded() {
        assert!("01".parse::<CourseInvitationStatePrecondition>().is_err());
        assert!("0".parse::<CourseInvitationStatePrecondition>().is_err());
        assert_eq!(
            "42".parse::<CourseInvitationStatePrecondition>()
                .unwrap()
                .value(),
            42
        );
        assert!("01".parse::<CourseRosterChangeNumber>().is_err());
        assert!("0".parse::<CourseRosterChangeNumber>().is_err());
        assert_eq!(
            "42".parse::<CourseRosterChangeNumber>().unwrap().value(),
            42
        );
        assert!(TeachingDisplayLabel::try_from(" ".to_owned()).is_err());
    }

    #[test]
    fn mutation_bodies_are_value_only_and_state_preconditions_are_absent() {
        let action = serde_json::to_value(CourseInvitationTerminalActionRequest {
            action: CourseInvitationTerminalAction::Accept,
        })
        .unwrap();
        assert_eq!(action, serde_json::json!({"action":"accept"}));
    }

    #[test]
    fn validated_deserialization_rejects_bounds_and_duplicate_members() {
        assert!(
            serde_json::from_str::<AccommodationAdjustmentView>(concat!(
                r#"{"available_at":{"kind":"inherit"},"due_at":{"kind":"inherit"},"#,
                r#""closes_at":{"kind":"inherit"},"assessment_attempt_time_limit_seconds":{"kind":"set","#,
                r#""value":2147483648},"attempt_limit":{"kind":"inherit"}}"#
            ))
            .is_err()
        );
    }

    #[test]
    fn pending_invitation_serializes_server_owned_expiry_without_inviter() {
        let row = PendingCourseInvitationView {
            id: "00000000-0000-0000-0000-000000000004".parse().unwrap(),
            course_label: TeachingDisplayLabel::try_from("Biochemistry".to_owned()).unwrap(),
            state: CourseInvitationStateView::Pending,
            expires_at: Timestamp::from_unix_millis(2_592_000_000),
            state_precondition: "3".parse().unwrap(),
        };
        let value = serde_json::to_value(row).unwrap();
        assert_eq!(value["expiresAt"], 2_592_000_000_i64);
        assert!(value.get("invitedBy").is_none());
    }

    #[test]
    fn invitation_pages_keep_the_viewer_zone_at_the_page_envelope() {
        let display_time_zone: crate::AccountTimeZone = "America/New_York".parse().unwrap();
        let pending_page = PendingCourseInvitationsPage {
            display_time_zone,
            invitations: vec![],
            next_cursor: None,
        };

        let page = serde_json::to_value(pending_page).unwrap();
        assert_eq!(page["displayTimeZone"], "America/New_York");
        assert_eq!(page["invitations"], serde_json::json!([]));
    }
}
