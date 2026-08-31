//! Strict browser/server contracts for the non-mutating WP-INST-T3 preview plane.
//!
//! A route request owns any `M-` locator. The Store resolves and discards
//! those locators before returning the owned [`PreviewSubject`].  That value is
//! immutable, self-contained, and identity-free; later preview evaluation only
//! borrows it and returns an owned closed projection.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentDeadlineBehavior, AssignmentReference, CourseLocalDateTime, CourseMembershipReference,
    IanaTimeZone, LateSubmissionPolicy,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS, PolicyModificationModeView,
    PolicyPatchView, TeachingDisplayLabel, TeachingOperationRevision,
};

/// Bounded Instructor wall-clock input. The server resolves it in this exact course zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewSelectedMoment {
    pub value: CourseLocalDateTime,
    pub time_zone: IanaTimeZone,
}

/// Request to construct an identity-free hypothetical assignment preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPreviewSubjectRequest {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub modifiers: SyntheticPreviewModifiers,
}

/// Hypothetical modifier input: the server validates compatibility and it cannot assert entitlement or source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPreviewModifiers {
    pub mode: PolicyModificationModeView,
    pub patch: PolicyPatchView,
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

/// Closed, sanitized provenance labels. These never carry a membership or person locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewPolicySourceLayer {
    Base,
    Accommodation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewTimeField {
    pub value: Option<CourseLocalDateTime>,
    pub source: PreviewPolicySourceLayer,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewLimitField {
    pub value: Option<u32>,
    pub source: PreviewPolicySourceLayer,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewLateSubmissionField {
    pub value: LateSubmissionPolicy,
    pub source: PreviewPolicySourceLayer,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDeadlineBehaviorField {
    pub value: AssignmentDeadlineBehavior,
    pub source: PreviewPolicySourceLayer,
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
    time_limit_seconds: PreviewLimitField,
    attempt_limit: PreviewLimitField,
    late_submission: PreviewLateSubmissionField,
    deadline_behavior: PreviewDeadlineBehaviorField,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewResolvedPolicyWire {
    available_at: PreviewTimeField,
    due_at: PreviewTimeField,
    closes_at: PreviewTimeField,
    time_limit_seconds: PreviewLimitField,
    attempt_limit: PreviewLimitField,
    late_submission: PreviewLateSubmissionField,
    deadline_behavior: PreviewDeadlineBehaviorField,
}
impl TryFrom<PreviewResolvedPolicyWire> for PreviewResolvedPolicy {
    type Error = &'static str;
    fn try_from(v: PreviewResolvedPolicyWire) -> Result<Self, Self::Error> {
        Self::new(
            v.available_at,
            v.due_at,
            v.closes_at,
            v.time_limit_seconds,
            v.attempt_limit,
            v.late_submission,
            v.deadline_behavior,
        )
    }
}
impl PreviewResolvedPolicy {
    pub fn new(
        available_at: PreviewTimeField,
        due_at: PreviewTimeField,
        closes_at: PreviewTimeField,
        time_limit_seconds: PreviewLimitField,
        attempt_limit: PreviewLimitField,
        late_submission: PreviewLateSubmissionField,
        deadline_behavior: PreviewDeadlineBehaviorField,
    ) -> Result<Self, &'static str> {
        if time_limit_seconds
            .value
            .is_some_and(|v| v == 0 || v > MAX_ASSIGNMENT_TIME_LIMIT_SECONDS)
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
            return Err("preview schedule is out of order");
        }
        Ok(Self {
            available_at,
            due_at,
            closes_at,
            time_limit_seconds,
            attempt_limit,
            late_submission,
            deadline_behavior,
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
    pub fn time_limit_seconds(&self) -> &PreviewLimitField {
        &self.time_limit_seconds
    }
    /// Returns the validated effective attempt limit without exposing policy internals.
    pub fn attempt_limit(&self) -> &PreviewLimitField {
        &self.attempt_limit
    }
    /// Returns the validated effective late-submission policy without exposing policy internals.
    pub fn late_submission(&self) -> &PreviewLateSubmissionField {
        &self.late_submission
    }
    /// Returns the validated effective deadline behavior without exposing policy internals.
    pub fn deadline_behavior(&self) -> &PreviewDeadlineBehaviorField {
        &self.deadline_behavior
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
pub enum PreviewSubjectKind {
    Synthetic,
    Derived,
}

/// Immutable, portable, identity-free input for a hypothetical preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "PreviewSubjectWire",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub struct PreviewSubject {
    pub kind: PreviewSubjectKind,
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub policy: PreviewResolvedPolicy,
    pub prior_run_count: PreviewPriorRunCount,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewSubjectWire {
    kind: PreviewSubjectKind,
    assignment: AssignmentReference,
    revision: TeachingOperationRevision,
    selected_moment: PreviewSelectedMoment,
    policy: PreviewResolvedPolicy,
    prior_run_count: PreviewPriorRunCount,
}

impl TryFrom<PreviewSubjectWire> for PreviewSubject {
    type Error = &'static str;
    fn try_from(value: PreviewSubjectWire) -> Result<Self, Self::Error> {
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

impl PreviewSubject {
    /// Constructs a fully resolved subject after route authorization and Store resolution.
    pub fn new(
        kind: PreviewSubjectKind,
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

/// Safe entitlement outcome for an instructor-only schedule row or preview projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PreviewEntitlementOutcome {
    Granted {
        reason: PreviewEntitlementGrantReason,
    },
    Denied {
        reason: PreviewEntitlementDenialReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewEntitlementGrantReason {
    ActiveStudentCourseMembership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewEntitlementDenialReason {
    NotEntitled,
}

/// The FERPA-authorized schedule table projection. This is not a PreviewSubject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum InstructorPreviewScheduleRow {
    Granted {
        membership: CourseMembershipReference,
        display: TeachingDisplayLabel,
        entitlement: PreviewEntitlementGrantReason,
        schedule: PreviewScheduleProjection,
    },
    Denied {
        membership: CourseMembershipReference,
        display: TeachingDisplayLabel,
        reason: PreviewEntitlementDenialReason,
    },
}

/// Canonical Instructor-only schedule page. Store paging owns cursor opacity and row bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstructorPreviewSchedulePage {
    pub revision: TeachingOperationRevision,
    pub rows: Vec<InstructorPreviewScheduleRow>,
    pub next_cursor: Option<String>,
}

/// Safe effective window and limits, reused in schedule and Before/After views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewScheduleProjection {
    pub available_at: PreviewTimeField,
    pub due_at: PreviewTimeField,
    pub closes_at: PreviewTimeField,
    pub time_limit_seconds: PreviewLimitField,
    pub attempt_limit: PreviewLimitField,
    pub late_submission: PreviewLateSubmissionField,
    pub deadline_behavior: PreviewDeadlineBehaviorField,
}

/// Accommodation effect compares two independently resolved safe projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewAccommodationComparison {
    pub before: PreviewScheduleProjection,
    pub after: PreviewScheduleProjection,
}

/// Complete non-mutating preview response returned by the T3 route boundary.
///
/// The optional accommodation comparison is absent when a hypothetical subject
/// has no applicable accommodation effect. Its nested evaluation is a closed
/// union, so denied responses cannot carry policy or disclosure data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewPlaneResponse {
    pub evaluation: PreviewEvaluation,
    pub accommodation: Option<PreviewAccommodationComparison>,
}

/// One requested disclosure boundary. Missing due or close remains explicit rather than inferred.
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
pub enum PreviewDisclosureProjection {
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

/// Closed denial with no subject, time, policy, provenance, or disclosure field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewDenialReason {
    NotEntitled,
    StaleRevision,
}

/// Complete ready-family evaluation. A denied case intentionally cannot leak a subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)] // A boxed subject becomes an unresolved generic in tsgen output.
pub enum PreviewEvaluation {
    Allowed {
        subject: PreviewSubject,
        entitlement: PreviewEntitlementGrantReason,
        schedule: PreviewScheduleProjection,
        disclosure: Vec<PreviewDisclosureProjection>,
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
            "modifiers": { "mode": "extendOnly", "patch": {
                "availableAt": { "kind": "inherit" },
                "dueAt": { "kind": "inherit" },
                "closesAt": { "kind": "inherit" },
                "timeLimitSeconds": { "kind": "inherit" },
                "attemptLimit": { "kind": "inherit" }
            } }
        });
        serde_json::from_value::<SyntheticPreviewSubjectRequest>(request)
            .expect("direct synthetic preview request");
        let retired = serde_json::json!({
            "assignment": "A-1",
            "revision": "1",
            "selectedMoment": { "value": "2026-08-20T09:00:00.000", "timeZone": "America/Chicago" },
            "groups": [],
            "modifiers": { "mode": "extendOnly", "patch": {
                "availableAt": { "kind": "inherit" },
                "dueAt": { "kind": "inherit" },
                "closesAt": { "kind": "inherit" },
                "timeLimitSeconds": { "kind": "inherit" },
                "attemptLimit": { "kind": "inherit" }
            } }
        });
        assert!(serde_json::from_value::<SyntheticPreviewSubjectRequest>(retired).is_err());
    }

    #[test]
    fn preview_subject_serializes_without_membership_or_group_facts() {
        let subject = PreviewSubject::new(
            PreviewSubjectKind::Synthetic,
            AssignmentReference::new(1).expect("assignment reference"),
            TeachingOperationRevision::new(1).expect("revision"),
            PreviewSelectedMoment {
                value: CourseLocalDateTime::parse("2026-08-20T09:00:00.000").expect("moment"),
                time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
            },
            PreviewResolvedPolicy::new(
                PreviewTimeField { value: None, source: PreviewPolicySourceLayer::Base },
                PreviewTimeField { value: None, source: PreviewPolicySourceLayer::Base },
                PreviewTimeField { value: None, source: PreviewPolicySourceLayer::Base },
                PreviewLimitField { value: None, source: PreviewPolicySourceLayer::Base },
                PreviewLimitField { value: None, source: PreviewPolicySourceLayer::Base },
                PreviewLateSubmissionField { value: LateSubmissionPolicy::Accept, source: PreviewPolicySourceLayer::Base },
                PreviewDeadlineBehaviorField { value: AssignmentDeadlineBehavior::AutoSubmit, source: PreviewPolicySourceLayer::Base },
            ).expect("policy"),
            PreviewPriorRunCount::try_from(0).expect("count"),
        ).expect("preview subject");
        let wire = serde_json::to_string(&subject).expect("wire");
        assert!(!wire.contains("groups"));
        assert!(!wire.contains("M-"));
    }
}
