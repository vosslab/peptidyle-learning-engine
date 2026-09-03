//! Strict browser/server contracts for the non-mutating Student View Scenario preview plane.
//!
//! A route request owns any `M-` Course Membership Reference. The Store resolves and discards
//! that Reference before returning the owned [`StudentViewScenario`]. That value is
//! immutable, self-contained, and identity-free; later preview evaluation only
//! borrows it and returns an owned closed Student View Scenario.

use serde::{Deserialize, Serialize};

use crate::{
    AccommodationAdjustmentView, AccommodationApplicationRuleView, AssignmentDeadlineRule,
    AssignmentReference, CourseLocalDateAndTime, CourseMembershipReference, CourseTimeZone,
    LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    TeachingDisplayLabel, TeachingOperationRevision,
};

/// Bounded Instructor wall-clock input. The server resolves it in this exact course zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewSelectedMoment {
    pub value: CourseLocalDateAndTime,
    pub time_zone: CourseTimeZone,
}

/// Request to construct an identity-free Student View Scenario from direct modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HypotheticalStudentViewScenarioRequest {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub modifiers: HypotheticalStudentViewScenarioModifiers,
}

/// Identity-free direct modifiers for a Hypothetical Student View Scenario.
///
/// The server validates compatibility; this input cannot assert Assignment Access
/// or an Assignment Policy Source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HypotheticalStudentViewScenarioModifiers {
    pub mode: AccommodationApplicationRuleView,
    pub adjustment: AccommodationAdjustmentView,
}

/// Request-bound selected-Student Course Membership Reference used only to construct an
/// identity-free Student View Scenario.
///
/// The returned scenario deliberately has no corresponding field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SelectedStudentViewScenarioRequest {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub selected_student_membership: CourseMembershipReference,
}

/// Closed, sanitized Assignment Policy Source kind labels. These never carry a membership or person Reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentPolicySourceKind {
    Base,
    Accommodation,
    HypotheticalStudentViewScenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewTimeField {
    pub value: Option<CourseLocalDateAndTime>,
    pub source: AssignmentPolicySourceKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewLimitField {
    pub value: Option<u32>,
    pub source: AssignmentPolicySourceKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewLateWorkRuleField {
    pub value: LateWorkRule,
    pub source: AssignmentPolicySourceKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewAssignmentDeadlineRuleField {
    pub value: AssignmentDeadlineRule,
    pub source: AssignmentPolicySourceKind,
}

/// Server-resolved values copied into a Student View Scenario, never raw policy inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "PreviewResolvedPolicyWire",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct PreviewResolvedPolicy {
    available_at: PreviewTimeField,
    due_at: PreviewTimeField,
    closes_at: PreviewTimeField,
    assignment_attempt_time_limit_seconds: PreviewLimitField,
    attempt_limit: PreviewLimitField,
    late_work_rule: PreviewLateWorkRuleField,
    assignment_deadline_rule: PreviewAssignmentDeadlineRuleField,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct PreviewResolvedPolicyWire {
    available_at: PreviewTimeField,
    due_at: PreviewTimeField,
    closes_at: PreviewTimeField,
    assignment_attempt_time_limit_seconds: PreviewLimitField,
    attempt_limit: PreviewLimitField,
    late_work_rule: PreviewLateWorkRuleField,
    assignment_deadline_rule: PreviewAssignmentDeadlineRuleField,
}
impl TryFrom<PreviewResolvedPolicyWire> for PreviewResolvedPolicy {
    type Error = &'static str;
    fn try_from(v: PreviewResolvedPolicyWire) -> Result<Self, Self::Error> {
        Self::new(
            v.available_at,
            v.due_at,
            v.closes_at,
            v.assignment_attempt_time_limit_seconds,
            v.attempt_limit,
            v.late_work_rule,
            v.assignment_deadline_rule,
        )
    }
}
impl PreviewResolvedPolicy {
    pub fn new(
        available_at: PreviewTimeField,
        due_at: PreviewTimeField,
        closes_at: PreviewTimeField,
        assignment_attempt_time_limit_seconds: PreviewLimitField,
        attempt_limit: PreviewLimitField,
        late_work_rule: PreviewLateWorkRuleField,
        assignment_deadline_rule: PreviewAssignmentDeadlineRuleField,
    ) -> Result<Self, &'static str> {
        if assignment_attempt_time_limit_seconds
            .value
            .is_some_and(|v| v == 0 || v > MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS)
            || attempt_limit
                .value
                .is_some_and(|v| v == 0 || v > MAX_ASSIGNMENT_ATTEMPT_LIMIT)
        {
            return Err("preview limit is outside assignment bounds");
        }
        if available_at
            .value
            .as_ref()
            .zip(due_at.value.as_ref())
            .is_some_and(|(a, b)| a > b)
            || available_at
                .value
                .as_ref()
                .zip(closes_at.value.as_ref())
                .is_some_and(|(a, b)| a > b)
            || due_at
                .value
                .as_ref()
                .zip(closes_at.value.as_ref())
                .is_some_and(|(a, b)| a > b)
        {
            return Err("preview effective_assignment_policy is out of order");
        }
        Ok(Self {
            available_at,
            due_at,
            closes_at,
            assignment_attempt_time_limit_seconds,
            attempt_limit,
            late_work_rule,
            assignment_deadline_rule,
        })
    }
    pub fn available_at(&self) -> &PreviewTimeField {
        &self.available_at
    }
    pub fn due_at(&self) -> &PreviewTimeField {
        &self.due_at
    }
    pub fn closes_at(&self) -> &PreviewTimeField {
        &self.closes_at
    }
    /// Returns the validated effective time limit without exposing policy internals.
    pub fn assignment_attempt_time_limit_seconds(&self) -> &PreviewLimitField {
        &self.assignment_attempt_time_limit_seconds
    }
    /// Returns the validated effective attempt limit without exposing policy internals.
    pub fn attempt_limit(&self) -> &PreviewLimitField {
        &self.attempt_limit
    }
    /// Returns the validated effective late-submission policy without exposing policy internals.
    pub fn late_work_rule(&self) -> &PreviewLateWorkRuleField {
        &self.late_work_rule
    }
    /// Returns the validated effective deadline behavior without exposing policy internals.
    pub fn assignment_deadline_rule(&self) -> &PreviewAssignmentDeadlineRuleField {
        &self.assignment_deadline_rule
    }
}

/// Prior Assignment Attempt fact; it is a count, not an attempt or receipt reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct PreviewPriorAssignmentAttemptCount(u32);

impl TryFrom<u32> for PreviewPriorAssignmentAttemptCount {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<PreviewPriorAssignmentAttemptCount> for u32 {
    fn from(value: PreviewPriorAssignmentAttemptCount) -> Self {
        value.0
    }
}

/// Student View Scenario origin is descriptive only and never participates in authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentViewScenarioOrigin {
    Hypothetical,
    SelectedStudent,
}

/// Identity-free admission fact paired with one Student View Scenario origin.
///
/// This describes why an already-authorized Instructor preview may return a
/// scenario. It neither identifies a Student nor grants any authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudentViewScenarioAdmission {
    SelectedStudentActiveStudentCourseMembership,
    HypotheticalStudentViewScenarioAdmission,
}

/// Immutable, portable, identity-free Student View Scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "StudentViewScenarioWire",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct StudentViewScenario {
    pub origin: StudentViewScenarioOrigin,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub policy: PreviewResolvedPolicy,
    pub prior_assignment_attempt_count: PreviewPriorAssignmentAttemptCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct StudentViewScenarioWire {
    origin: StudentViewScenarioOrigin,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    selected_moment: PreviewSelectedMoment,
    policy: PreviewResolvedPolicy,
    prior_assignment_attempt_count: PreviewPriorAssignmentAttemptCount,
}

impl TryFrom<StudentViewScenarioWire> for StudentViewScenario {
    type Error = &'static str;
    fn try_from(value: StudentViewScenarioWire) -> Result<Self, Self::Error> {
        Self::new(
            value.origin,
            value.assignment,
            value.revision,
            value.selected_moment,
            value.policy,
            value.prior_assignment_attempt_count,
        )
    }
}

impl StudentViewScenario {
    /// Constructs already-resolved scenario data; a future mounted route/Store boundary owns
    /// authorization and resolution.
    pub fn new(
        origin: StudentViewScenarioOrigin,
        assignment: AssignmentReference,
        revision: TeachingOperationRevision,
        selected_moment: PreviewSelectedMoment,
        policy: PreviewResolvedPolicy,
        prior_assignment_attempt_count: PreviewPriorAssignmentAttemptCount,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            origin,
            assignment,
            revision,
            selected_moment,
            policy,
            prior_assignment_attempt_count,
        })
    }
}

/// Safe Assignment Access outcome for one Instructor Preview Schedule Row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum ActiveStudentCourseMembershipOutcome {
    Granted {
        reason: ActiveStudentCourseMembershipGrantReason,
    },
    Denied {
        reason: ActiveStudentCourseMembershipDenialReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveStudentCourseMembershipGrantReason {
    ActiveStudentCourseMembership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveStudentCourseMembershipDenialReason {
    NoActiveStudentCourseMembership,
}

/// FERPA-authorized Instructor Preview Schedule Row. This is not a Student View Scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum InstructorPreviewScheduleRow {
    Granted {
        membership: CourseMembershipReference,
        display: TeachingDisplayLabel,
        active_student_course_membership: ActiveStudentCourseMembershipGrantReason,
        effective_assignment_policy: EffectiveAssignmentPolicyView,
    },
    Denied {
        membership: CourseMembershipReference,
        display: TeachingDisplayLabel,
        reason: ActiveStudentCourseMembershipDenialReason,
    },
}

/// Instructor Preview page for effective_assignment_policy. Store paging owns cursor opacity and row bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstructorPreviewSchedulePage {
    pub revision: TeachingOperationRevision,
    pub rows: Vec<InstructorPreviewScheduleRow>,
    pub next_cursor: Option<String>,
}

/// Safe effective window and limits, reused in effective_assignment_policy and Before/After views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct EffectiveAssignmentPolicyView {
    pub available_at: PreviewTimeField,
    pub due_at: PreviewTimeField,
    pub closes_at: PreviewTimeField,
    pub assignment_attempt_time_limit_seconds: PreviewLimitField,
    pub attempt_limit: PreviewLimitField,
    pub late_work_rule: PreviewLateWorkRuleField,
    pub assignment_deadline_rule: PreviewAssignmentDeadlineRuleField,
}

/// Accommodation effect compares two independently resolved Effective Assignment Policy Views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewAccommodationComparison {
    pub before: EffectiveAssignmentPolicyView,
    pub after: EffectiveAssignmentPolicyView,
}

/// Complete non-mutating preview response returned by the Assignment Delivery Preview
/// contract boundary.
///
/// The optional accommodation comparison is absent when a Hypothetical Student View Scenario
/// has no applicable accommodation effect. Its nested evaluation is a closed
/// union, so denied responses cannot carry policy or student_feedback_release data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewPlaneResponse {
    pub evaluation: PreviewEvaluation,
    pub accommodation: Option<PreviewAccommodationComparison>,
}

/// One requested student_feedback_release boundary. Missing due or close remains explicit rather than inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewDisclosureMoment {
    Now,
    Due,
    Close,
}

/// Six safe visibility flags; no feedback, answer, explanation, or score content is transported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PreviewDisclosureFlags {
    pub score_shown: bool,
    pub correctness_shown: bool,
    pub feedback_shown: bool,
    pub question_answer_shown: bool,
    pub question_answer_explanation_shown: bool,
    pub statistics_shown: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum StudentFeedbackReleaseView {
    Available {
        moment: PreviewDisclosureMoment,
        flags: PreviewDisclosureFlags,
    },
    Unavailable {
        moment: PreviewDisclosureMoment,
        reason: PreviewDisclosureUnavailableReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewDisclosureUnavailableReason {
    BoundaryMissing,
}

/// Closed denial with no Student View Scenario, time, policy source, or student_feedback_release field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewDenialReason {
    ActiveStudentCourseMembershipRequired,
    StaleRevision,
}

/// Complete ready-state evaluation. A denied case intentionally cannot leak a Student View Scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
#[allow(clippy::large_enum_variant)] // A boxed Student View Scenario becomes an unresolved generic in tsgen output.
pub enum PreviewEvaluation {
    Allowed {
        student_view_scenario: StudentViewScenario,
        student_view_scenario_admission: StudentViewScenarioAdmission,
        effective_assignment_policy: EffectiveAssignmentPolicyView,
        student_feedback_release: Vec<StudentFeedbackReleaseView>,
    },
    Denied {
        reason: PreviewDenialReason,
    },
}

/// Typed names for later packages that have no executable implementation yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewDeferredCapability {
    CloneAndTermShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum PreviewFutureSeam {
    Unavailable {
        capability: PreviewDeferredCapability,
    },
}

#[cfg(test)]
mod direct_preview_tests {
    use super::*;

    #[test]
    fn hypothetical_student_view_scenario_request_accepts_only_direct_preview_fields() {
        let request = serde_json::json!({
            "assignment": "A-1",
            "revision": "1",
            "selected_moment": { "value": "2026-08-20T09:00:00.000", "time_zone": "America/Chicago" },
            "modifiers": { "mode": "extend_only", "adjustment": {
                "available_at": { "kind": "inherit" },
                "due_at": { "kind": "inherit" },
                "closes_at": { "kind": "inherit" },
                "assignment_attempt_time_limit_seconds": { "kind": "inherit" },
                "attempt_limit": { "kind": "inherit" }
            } }
        });
        serde_json::from_value::<HypotheticalStudentViewScenarioRequest>(request)
            .expect("direct hypothetical Student View Scenario request");
        let retired = serde_json::json!({
            "assignment": "A-1",
            "revision": "1",
            "selectedMoment": { "value": "2026-08-20T09:00:00.000", "timeZone": "America/Chicago" },
            "modifiers": { "mode": "extend_only", "adjustment": {
                "available_at": { "kind": "inherit" },
                "due_at": { "kind": "inherit" },
                "closes_at": { "kind": "inherit" },
                "assignment_attempt_time_limit_seconds": { "kind": "inherit" },
                "attempt_limit": { "kind": "inherit" }
            } }
        });
        assert!(serde_json::from_value::<HypotheticalStudentViewScenarioRequest>(retired).is_err());
    }

    #[test]
    fn student_view_scenario_serializes_without_membership_or_group_facts() {
        let student_view_scenario = StudentViewScenario::new(
            StudentViewScenarioOrigin::Hypothetical,
            AssignmentReference::new(1).expect("assignment reference"),
            TeachingOperationRevision::new(1).expect("revision"),
            PreviewSelectedMoment {
                value: CourseLocalDateAndTime::parse("2026-08-20T09:00:00.000").expect("moment"),
                time_zone: CourseTimeZone::parse("America/Chicago").expect("zone"),
            },
            PreviewResolvedPolicy::new(
                PreviewTimeField {
                    value: None,
                    source: AssignmentPolicySourceKind::Base,
                },
                PreviewTimeField {
                    value: None,
                    source: AssignmentPolicySourceKind::Base,
                },
                PreviewTimeField {
                    value: None,
                    source: AssignmentPolicySourceKind::Base,
                },
                PreviewLimitField {
                    value: None,
                    source: AssignmentPolicySourceKind::Base,
                },
                PreviewLimitField {
                    value: None,
                    source: AssignmentPolicySourceKind::Base,
                },
                PreviewLateWorkRuleField {
                    value: LateWorkRule::Accept,
                    source: AssignmentPolicySourceKind::Base,
                },
                PreviewAssignmentDeadlineRuleField {
                    value: AssignmentDeadlineRule::AutoSubmit,
                    source: AssignmentPolicySourceKind::Base,
                },
            )
            .expect("policy"),
            PreviewPriorAssignmentAttemptCount::try_from(0).expect("count"),
        )
        .expect("Student View Scenario");
        let wire = serde_json::to_string(&student_view_scenario).expect("wire");
        assert!(!wire.contains("groups"));
        assert!(!wire.contains("M-"));
    }
}
