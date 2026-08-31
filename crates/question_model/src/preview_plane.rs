//! Strict browser/server contracts for the non-mutating WP-INST-T3 preview plane.
//!
//! A route request owns any `M-` locator. The Store resolves and discards
//! those locators before returning the owned [`StudentViewScenario`].  That value is
//! immutable, self-contained, and identity-free; later preview evaluation only
//! borrows it and returns an owned closed projection.

use serde::{Deserialize, Serialize};

use crate::{
    AccommodationAdjustmentView, AccommodationApplicationRuleView, AssignmentDeadlineRule,
    AssignmentReference, CourseLocalDateAndTime, CourseMembershipReference, CourseTimeZone,
    LateWorkRule, MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    TeachingDisplayLabel, TeachingOperationRevision,
};

/// Bounded Instructor wall-clock input. The server resolves it in this exact course zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewSelectedMoment {
    pub value: CourseLocalDateAndTime,
    pub time_zone: CourseTimeZone,
}

/// Request to construct an identity-free hypothetical assignment preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudentViewScenarioRequest {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub modifiers: SyntheticPreviewModifiers,
}

/// Hypothetical modifier input: the server validates compatibility and it cannot assert
/// Assignment Access or an Assignment Policy Source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPreviewModifiers {
    pub mode: AccommodationApplicationRuleView,
    pub adjustment: AccommodationAdjustmentView,
}

/// Request-bound student locator used only to derive an identity-free subject.
///
/// The returned subject deliberately has no corresponding field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DerivedPreviewSubjectRequest {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub membership: CourseMembershipReference,
}

/// Closed, sanitized Assignment Policy Source kind labels. These never carry a membership or person locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssignmentPolicySourceKind {
    Base,
    Accommodation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewTimeField {
    pub value: Option<CourseLocalDateAndTime>,
    pub source: AssignmentPolicySourceKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewLimitField {
    pub value: Option<u32>,
    pub source: AssignmentPolicySourceKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewLateWorkRuleField {
    pub value: LateWorkRule,
    pub source: AssignmentPolicySourceKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewAssignmentDeadlineRuleField {
    pub value: AssignmentDeadlineRule,
    pub source: AssignmentPolicySourceKind,
}

/// Server-resolved values copied into a subject, never raw policy inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "PreviewResolvedPolicyWire",
    rename_all = "camelCase",
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

/// Prior-run fact; it is a count, not a run, attempt, or receipt reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct PreviewPriorRunCount(u32);

impl TryFrom<u32> for PreviewPriorRunCount {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<PreviewPriorRunCount> for u32 {
    fn from(value: PreviewPriorRunCount) -> Self {
        value.0
    }
}

/// Subject origin is descriptive only and never participates in authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StudentViewScenarioKind {
    Synthetic,
    Derived,
}

/// Immutable, portable, identity-free input for a hypothetical preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "StudentViewScenarioWire",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub struct StudentViewScenario {
    pub kind: StudentViewScenarioKind,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub policy: PreviewResolvedPolicy,
    pub prior_run_count: PreviewPriorRunCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StudentViewScenarioWire {
    kind: StudentViewScenarioKind,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    selected_moment: PreviewSelectedMoment,
    policy: PreviewResolvedPolicy,
    prior_run_count: PreviewPriorRunCount,
}

impl TryFrom<StudentViewScenarioWire> for StudentViewScenario {
    type Error = &'static str;
    fn try_from(value: StudentViewScenarioWire) -> Result<Self, Self::Error> {
        Self::new(
            value.kind,
            value.assignment,
            value.revision,
            value.selected_moment,
            value.policy,
            value.prior_run_count,
        )
    }
}

impl StudentViewScenario {
    /// Constructs a fully resolved subject after route authorization and Store resolution.
    pub fn new(
        kind: StudentViewScenarioKind,
        assignment: AssignmentReference,
        revision: TeachingOperationRevision,
        selected_moment: PreviewSelectedMoment,
        policy: PreviewResolvedPolicy,
        prior_run_count: PreviewPriorRunCount,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            kind,
            assignment,
            revision,
            selected_moment,
            policy,
            prior_run_count,
        })
    }
}

/// Safe Assignment Access outcome for an instructor-only effective_assignment_policy row or preview projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ActiveStudentCourseMembershipOutcome {
    Granted {
        reason: ActiveStudentCourseMembershipGrantReason,
    },
    Denied {
        reason: ActiveStudentCourseMembershipDenialReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveStudentCourseMembershipGrantReason {
    ActiveStudentCourseMembership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveStudentCourseMembershipDenialReason {
    NoActiveStudentCourseMembership,
}

/// The FERPA-authorized effective_assignment_policy table projection. This is not a StudentViewScenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
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

/// Canonical Instructor-only effective_assignment_policy page. Store paging owns cursor opacity and row bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorPreviewSchedulePage {
    pub revision: TeachingOperationRevision,
    pub rows: Vec<InstructorPreviewScheduleRow>,
    pub next_cursor: Option<String>,
}

/// Safe effective window and limits, reused in effective_assignment_policy and Before/After views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveAssignmentPolicyView {
    pub available_at: PreviewTimeField,
    pub due_at: PreviewTimeField,
    pub closes_at: PreviewTimeField,
    pub assignment_attempt_time_limit_seconds: PreviewLimitField,
    pub attempt_limit: PreviewLimitField,
    pub late_work_rule: PreviewLateWorkRuleField,
    pub assignment_deadline_rule: PreviewAssignmentDeadlineRuleField,
}

/// Accommodation effect compares two independently resolved safe projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewAccommodationComparison {
    pub before: EffectiveAssignmentPolicyView,
    pub after: EffectiveAssignmentPolicyView,
}

/// Complete non-mutating preview response returned by the T3 route boundary.
///
/// The optional accommodation comparison is absent when a hypothetical subject
/// has no applicable accommodation effect. Its nested evaluation is a closed
/// union, so denied responses cannot carry policy or student_feedback_release data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewPlaneResponse {
    pub evaluation: PreviewEvaluation,
    pub accommodation: Option<PreviewAccommodationComparison>,
}

/// One requested student_feedback_release boundary. Missing due or close remains explicit rather than inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewDisclosureMoment {
    Now,
    Due,
    Close,
}

/// Five safe visibility flags; no feedback, solution, answer, or score content is transported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDisclosureFlags {
    pub score_shown: bool,
    pub correctness_shown: bool,
    pub feedback_shown: bool,
    pub solution_shown: bool,
    pub statistics_shown: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
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
#[serde(rename_all = "camelCase")]
pub enum PreviewDisclosureUnavailableReason {
    BoundaryMissing,
}

/// Closed denial with no subject, time, policy source, or student_feedback_release field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewDenialReason {
    ActiveStudentCourseMembershipRequired,
    StaleRevision,
}

/// Complete ready-state evaluation. A denied case intentionally cannot leak a subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)] // A boxed subject becomes an unresolved generic in tsgen output.
pub enum PreviewEvaluation {
    Allowed {
        student_view_scenario: StudentViewScenario,
        active_student_course_membership: ActiveStudentCourseMembershipGrantReason,
        effective_assignment_policy: EffectiveAssignmentPolicyView,
        student_feedback_release: Vec<StudentFeedbackReleaseView>,
    },
    Denied {
        reason: PreviewDenialReason,
    },
}

/// Typed names for later packages that have no executable implementation yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewDeferredCapability {
    CloneAndTermShift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PreviewFutureSeam {
    Unavailable {
        capability: PreviewDeferredCapability,
    },
}

#[cfg(test)]
mod direct_preview_tests {
    use super::*;

    #[test]
    fn synthetic_request_accepts_only_direct_preview_fields() {
        let request = serde_json::json!({
            "assignment": "A-1",
            "revision": "1",
            "selectedMoment": { "value": "2026-08-20T09:00:00.000", "timeZone": "America/Chicago" },
            "modifiers": { "mode": "extendOnly", "adjustment": {
                "availableAt": { "kind": "inherit" },
                "dueAt": { "kind": "inherit" },
                "closesAt": { "kind": "inherit" },
                "assignmentAttemptTimeLimitSeconds": { "kind": "inherit" },
                "attemptLimit": { "kind": "inherit" }
            } }
        });
        serde_json::from_value::<StudentViewScenarioRequest>(request)
            .expect("direct synthetic preview request");
        let retired = serde_json::json!({
            "assignment": "A-1",
            "revision": "1",
            "selectedMoment": { "value": "2026-08-20T09:00:00.000", "timeZone": "America/Chicago" },
            "groups": [],
            "modifiers": { "mode": "extendOnly", "adjustment": {
                "availableAt": { "kind": "inherit" },
                "dueAt": { "kind": "inherit" },
                "closesAt": { "kind": "inherit" },
                "assignmentAttemptTimeLimitSeconds": { "kind": "inherit" },
                "attemptLimit": { "kind": "inherit" }
            } }
        });
        assert!(serde_json::from_value::<StudentViewScenarioRequest>(retired).is_err());
    }

    #[test]
    fn preview_subject_serializes_without_membership_or_group_facts() {
        let subject = StudentViewScenario::new(
            StudentViewScenarioKind::Synthetic,
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
            PreviewPriorRunCount::try_from(0).expect("count"),
        )
        .expect("preview subject");
        let wire = serde_json::to_string(&subject).expect("wire");
        assert!(!wire.contains("groups"));
        assert!(!wire.contains("M-"));
    }
}
