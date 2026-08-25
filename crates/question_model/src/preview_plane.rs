//! Strict browser/server contracts for the non-mutating WP-PROF-T3 preview plane.
//!
//! A route request owns any `M-` or `G-` locator.  The Store resolves and discards
//! those locators before returning the owned [`PreviewSubject`].  That value is
//! immutable, self-contained, and identity-free; later preview evaluation only
//! borrows it and returns an owned closed projection.

use serde::{Deserialize, Serialize};

use crate::{
    AssignmentDeadlineBehavior, AssignmentReference, CourseGroupPurpose, CourseGroupReference,
    CourseLocalDateTime, CourseMembershipReference, IanaTimeZone, LateSubmissionPolicy,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_TIME_LIMIT_SECONDS, PolicyModificationModeView,
    PolicyPatchView, TeachingDisplayLabel, TeachingOperationRevision,
};

/// A synthetic request may name at most one normal teaching-operations page of groups.
pub const MAX_PREVIEW_SUBJECT_GROUPS: usize = 100;

/// A bounded, canonical collection of course-local group locators at the request boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "Vec<CourseGroupReference>",
    into = "Vec<CourseGroupReference>"
)]
pub struct PreviewSyntheticGroupReferences(Vec<CourseGroupReference>);

impl PreviewSyntheticGroupReferences {
    pub fn as_slice(&self) -> &[CourseGroupReference] {
        &self.0
    }
}

impl TryFrom<Vec<CourseGroupReference>> for PreviewSyntheticGroupReferences {
    type Error = &'static str;

    fn try_from(mut groups: Vec<CourseGroupReference>) -> Result<Self, Self::Error> {
        if groups.len() > MAX_PREVIEW_SUBJECT_GROUPS {
            return Err("preview subject may contain at most 100 groups");
        }
        groups.sort_unstable();
        if groups.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("preview subject groups must be unique");
        }
        Ok(Self(groups))
    }
}

impl From<PreviewSyntheticGroupReferences> for Vec<CourseGroupReference> {
    fn from(value: PreviewSyntheticGroupReferences) -> Self {
        value.0
    }
}

/// Bounded professor wall-clock input. The server resolves it in this exact course zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewSelectedMoment {
    pub value: CourseLocalDateTime,
    pub time_zone: IanaTimeZone,
}

/// Request to construct a hypothetical subject from course-local group choices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPreviewSubjectRequest {
    pub assignment: AssignmentReference,
    pub revision: TeachingOperationRevision,
    pub selected_moment: PreviewSelectedMoment,
    pub groups: PreviewSyntheticGroupReferences,
    pub modifiers: SyntheticPreviewModifiers,
}

/// Hypothetical modifier input: the server validates compatibility and it cannot assert entitlement or source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPreviewModifiers {
    pub mode: PolicyModificationModeView,
    pub patch: PolicyPatchView,
}

/// Request-bound learner locator used only to derive an identity-free subject.
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

/// Safe role label, selected by the server from a resolved group purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewGroupRole {
    SectionMember,
    LabMember,
    CohortMember,
    AccommodationRecipient,
    WorkGroupMember,
}

/// Maps the only safe role label from a closed group purpose; callers cannot pair them freely.
pub const fn preview_group_role(purpose: CourseGroupPurpose) -> PreviewGroupRole {
    match purpose {
        CourseGroupPurpose::Section => PreviewGroupRole::SectionMember,
        CourseGroupPurpose::Lab => PreviewGroupRole::LabMember,
        CourseGroupPurpose::Cohort => PreviewGroupRole::CohortMember,
        CourseGroupPurpose::Accommodation => PreviewGroupRole::AccommodationRecipient,
        CourseGroupPurpose::Work => PreviewGroupRole::WorkGroupMember,
    }
}

/// Identity-free group fact preserved in a portable preview subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    try_from = "PreviewGroupFactWire",
    deny_unknown_fields
)]
pub struct PreviewGroupFact {
    role: PreviewGroupRole,
    purpose: CourseGroupPurpose,
}

impl PreviewGroupFact {
    /// Constructs the only role and purpose pair admitted by the closed mapping.
    pub const fn from_purpose(purpose: CourseGroupPurpose) -> Self {
        Self {
            role: preview_group_role(purpose),
            purpose,
        }
    }

    pub const fn role(&self) -> PreviewGroupRole {
        self.role
    }

    pub const fn purpose(&self) -> CourseGroupPurpose {
        self.purpose
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewGroupFactWire {
    role: PreviewGroupRole,
    purpose: CourseGroupPurpose,
}

impl TryFrom<PreviewGroupFactWire> for PreviewGroupFact {
    type Error = &'static str;

    fn try_from(value: PreviewGroupFactWire) -> Result<Self, Self::Error> {
        // ASVS 1.5.2 and 2.2.3: deserialize through a closed shape and validate the pair.
        let fact = Self::from_purpose(value.purpose);
        if value.role != fact.role {
            return Err("preview group role must match its purpose");
        }
        Ok(fact)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<PreviewGroupFact>", into = "Vec<PreviewGroupFact>")]
pub struct PreviewGroupFacts(Vec<PreviewGroupFact>);

impl PreviewGroupFacts {
    pub fn as_slice(&self) -> &[PreviewGroupFact] {
        &self.0
    }
}
impl TryFrom<Vec<PreviewGroupFact>> for PreviewGroupFacts {
    type Error = &'static str;
    fn try_from(mut values: Vec<PreviewGroupFact>) -> Result<Self, Self::Error> {
        if values.len() > MAX_PREVIEW_SUBJECT_GROUPS {
            return Err("preview subject may contain at most 100 group facts");
        }
        values.sort_unstable_by_key(|fact| fact.purpose);
        if values.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("preview subject group facts must be unique");
        }
        Ok(Self(values))
    }
}
impl From<PreviewGroupFacts> for Vec<PreviewGroupFact> {
    fn from(value: PreviewGroupFacts) -> Self {
        value.0
    }
}

/// Closed, sanitized provenance labels. These never carry a group, membership, or person locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewPolicySourceLayer {
    Base,
    GroupSchedule,
    GroupAccommodation,
    IndividualException,
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
    groups: PreviewGroupFacts,
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
    groups: Vec<PreviewGroupFact>,
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
            value.groups,
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
        groups: Vec<PreviewGroupFact>,
        policy: PreviewResolvedPolicy,
        prior_run_count: PreviewPriorRunCount,
    ) -> Result<Self, &'static str> {
        let groups = PreviewGroupFacts::try_from(groups)?;
        Ok(Self {
            kind,
            assignment,
            revision,
            selected_moment,
            groups,
            policy,
            prior_run_count,
        })
    }
    pub fn groups(&self) -> &[PreviewGroupFact] {
        self.groups.as_slice()
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
    CourseWide,
    GroupAudience,
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
mod tests {
    use super::*;

    fn selected() -> PreviewSelectedMoment {
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-08-20T09:00:00.000").unwrap(),
            time_zone: IanaTimeZone::parse("America/Chicago").unwrap(),
        }
    }

    #[test]
    fn synthetic_request_is_strict_and_group_refs_are_deduplicated() {
        let value = serde_json::json!({"assignment":"A-9","revision":"4","selectedMoment":{"value":"2026-08-20T09:00:00.000","timeZone":"America/Chicago"},"groups":["G-2","G-1"],"modifiers":{"mode":"extendOnly","patch":{"availableAt":{"kind":"inherit"},"dueAt":{"kind":"inherit"},"closesAt":{"kind":"inherit"},"timeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}}});
        let request: SyntheticPreviewSubjectRequest = serde_json::from_value(value).unwrap();
        assert_eq!(request.groups.as_slice()[0].to_string(), "G-1");
        let mut invalid = serde_json::to_value(&request).unwrap();
        invalid["email"] = serde_json::json!("nope");
        assert!(serde_json::from_value::<SyntheticPreviewSubjectRequest>(invalid).is_err());
        assert!(
            PreviewSyntheticGroupReferences::try_from(vec![
                "G-1".parse().unwrap(),
                "G-1".parse().unwrap()
            ])
            .is_err()
        );
    }

    #[test]
    fn derived_request_is_boundary_only_and_subject_json_has_no_identity_shapes() {
        let request: DerivedPreviewSubjectRequest = serde_json::from_value(serde_json::json!({"assignment":"A-9","revision":"4","selectedMoment":{"value":"2026-08-20T09:00:00.000","timeZone":"America/Chicago"},"membership":"M-2"})).unwrap();
        assert_eq!(request.membership.to_string(), "M-2");
        let field = PreviewPolicySourceLayer::Base;
        let policy = PreviewResolvedPolicy::new(
            PreviewTimeField {
                value: None,
                source: field,
            },
            PreviewTimeField {
                value: None,
                source: field,
            },
            PreviewTimeField {
                value: None,
                source: field,
            },
            PreviewLimitField {
                value: None,
                source: field,
            },
            PreviewLimitField {
                value: None,
                source: field,
            },
            PreviewLateSubmissionField {
                value: LateSubmissionPolicy::Accept,
                source: PreviewPolicySourceLayer::IndividualException,
            },
            PreviewDeadlineBehaviorField {
                value: AssignmentDeadlineBehavior::AutoSubmit,
                source: field,
            },
        )
        .unwrap();
        let subject = PreviewSubject::new(
            PreviewSubjectKind::Derived,
            "A-9".parse().unwrap(),
            TeachingOperationRevision::new(4).unwrap(),
            selected(),
            vec![PreviewGroupFact::from_purpose(CourseGroupPurpose::Lab)],
            policy,
            PreviewPriorRunCount::try_from(0).unwrap(),
        )
        .unwrap();
        let wire = serde_json::to_string(&subject).unwrap();
        for forbidden in [
            "M-", "G-", "U-", "CI-", "PV-", "email", "name", "uuid", "answer", "score", "audit",
        ] {
            assert!(!wire.contains(forbidden), "{forbidden} leaked: {wire}");
        }
        let mut malformed: serde_json::Value = serde_json::from_str(&wire).unwrap();
        malformed["groups"] = serde_json::json!([
            {"role":"labMember", "purpose":"lab"},
            {"role":"labMember", "purpose":"lab"}
        ]);
        assert!(serde_json::from_value::<PreviewSubject>(malformed).is_err());
        assert_eq!(
            preview_group_role(CourseGroupPurpose::Work),
            PreviewGroupRole::WorkGroupMember
        );
    }

    #[test]
    fn group_fact_wire_accepts_only_the_closed_role_and_purpose_pairs() {
        let pairs = [
            (CourseGroupPurpose::Section, PreviewGroupRole::SectionMember),
            (CourseGroupPurpose::Lab, PreviewGroupRole::LabMember),
            (CourseGroupPurpose::Cohort, PreviewGroupRole::CohortMember),
            (
                CourseGroupPurpose::Accommodation,
                PreviewGroupRole::AccommodationRecipient,
            ),
            (CourseGroupPurpose::Work, PreviewGroupRole::WorkGroupMember),
        ];
        for (purpose, role) in pairs {
            let fact = PreviewGroupFact::from_purpose(purpose);
            assert_eq!(fact.role(), role);
            assert_eq!(fact.purpose(), purpose);
            let wire = serde_json::to_value(fact).unwrap();
            assert_eq!(
                serde_json::from_value::<PreviewGroupFact>(wire).unwrap(),
                fact
            );
        }
        assert_eq!(
            serde_json::to_value(PreviewGroupFact::from_purpose(CourseGroupPurpose::Lab)).unwrap(),
            serde_json::json!({"role":"labMember", "purpose":"lab"})
        );
        let mismatched = serde_json::json!({"role":"sectionMember", "purpose":"lab"});
        assert!(serde_json::from_value::<PreviewGroupFact>(mismatched).is_err());
    }

    #[test]
    fn resolved_policy_rejects_bad_limits_and_schedule_but_accepts_equal_boundaries() {
        let zone = PreviewPolicySourceLayer::Base;
        let at = |value| PreviewTimeField {
            value: Some(CourseLocalDateTime::parse(value).unwrap()),
            source: zone,
        };
        let limit = |value| PreviewLimitField {
            value,
            source: zone,
        };
        let late = PreviewLateSubmissionField {
            value: LateSubmissionPolicy::Accept,
            source: zone,
        };
        let deadline = PreviewDeadlineBehaviorField {
            value: AssignmentDeadlineBehavior::AutoSubmit,
            source: zone,
        };
        assert!(
            PreviewResolvedPolicy::new(
                at("2026-08-20T09:00:00.000"),
                at("2026-08-20T09:00:00.000"),
                at("2026-08-20T09:00:00.000"),
                limit(Some(1)),
                limit(Some(1)),
                late.clone(),
                deadline.clone()
            )
            .is_ok()
        );
        assert!(
            PreviewResolvedPolicy::new(
                at("2026-08-20T10:00:00.000"),
                at("2026-08-20T09:00:00.000"),
                PreviewTimeField {
                    value: None,
                    source: zone
                },
                limit(None),
                limit(None),
                late.clone(),
                deadline.clone()
            )
            .is_err()
        );
        assert!(
            PreviewResolvedPolicy::new(
                PreviewTimeField {
                    value: None,
                    source: zone
                },
                PreviewTimeField {
                    value: None,
                    source: zone
                },
                PreviewTimeField {
                    value: None,
                    source: zone
                },
                limit(Some(0)),
                limit(None),
                late,
                deadline
            )
            .is_err()
        );
    }

    #[test]
    fn external_consumer_reads_every_validated_policy_field_without_serialization() {
        let policy = PreviewResolvedPolicy::new(
            PreviewTimeField {
                value: Some(CourseLocalDateTime::parse("2026-08-20T09:00:00.000").unwrap()),
                source: PreviewPolicySourceLayer::Base,
            },
            PreviewTimeField {
                value: Some(CourseLocalDateTime::parse("2026-08-20T10:00:00.000").unwrap()),
                source: PreviewPolicySourceLayer::GroupSchedule,
            },
            PreviewTimeField {
                value: Some(CourseLocalDateTime::parse("2026-08-20T11:00:00.000").unwrap()),
                source: PreviewPolicySourceLayer::GroupAccommodation,
            },
            PreviewLimitField {
                value: Some(1_200),
                source: PreviewPolicySourceLayer::IndividualException,
            },
            PreviewLimitField {
                value: Some(3),
                source: PreviewPolicySourceLayer::Base,
            },
            PreviewLateSubmissionField {
                value: LateSubmissionPolicy::Accept,
                source: PreviewPolicySourceLayer::Base,
            },
            PreviewDeadlineBehaviorField {
                value: AssignmentDeadlineBehavior::AutoSubmit,
                source: PreviewPolicySourceLayer::Base,
            },
        )
        .unwrap();

        assert_eq!(
            policy.available_at().value.as_ref().unwrap().as_str(),
            "2026-08-20T09:00:00.000"
        );
        assert_eq!(
            policy.due_at().value.as_ref().unwrap().as_str(),
            "2026-08-20T10:00:00.000"
        );
        assert_eq!(
            policy.closes_at().value.as_ref().unwrap().as_str(),
            "2026-08-20T11:00:00.000"
        );
        assert_eq!(policy.time_limit_seconds().value, Some(1_200));
        assert_eq!(policy.attempt_limit().value, Some(3));
        assert_eq!(policy.late_submission().value, LateSubmissionPolicy::Accept);
        assert_eq!(
            policy.deadline_behavior().value,
            AssignmentDeadlineBehavior::AutoSubmit
        );
    }

    #[test]
    fn remaining_future_seam_and_denial_wires_stay_minimal() {
        assert_eq!(
            serde_json::to_value(PreviewFutureSeam::Unavailable {
                capability: PreviewDeferredCapability::CloneAndTermShift
            })
            .unwrap(),
            serde_json::json!({"kind":"unavailable","capability":"cloneAndTermShift"})
        );
        assert_eq!(
            serde_json::to_value(PreviewFutureSeam::Unavailable {
                capability: PreviewDeferredCapability::CloneAndTermShift
            })
            .unwrap(),
            serde_json::json!({"kind":"unavailable","capability":"cloneAndTermShift"})
        );
        assert_eq!(
            serde_json::to_value(PreviewEvaluation::Denied {
                reason: PreviewDenialReason::NotEntitled
            })
            .unwrap(),
            serde_json::json!({"kind":"denied","reason":"notEntitled"})
        );
        let response = PreviewPlaneResponse {
            evaluation: PreviewEvaluation::Denied {
                reason: PreviewDenialReason::NotEntitled,
            },
            accommodation: None,
        };
        let response_wire = serde_json::to_value(&response).unwrap();
        assert_eq!(
            response_wire,
            serde_json::json!({
                "evaluation":{"kind":"denied","reason":"notEntitled"},
                "accommodation":null
            })
        );
        let mut extra_field = response_wire;
        extra_field["audit"] = serde_json::json!("forbidden");
        assert!(serde_json::from_value::<PreviewPlaneResponse>(extra_field).is_err());
        let schedule_row = InstructorPreviewScheduleRow::Denied {
            membership: "M-9".parse().unwrap(),
            display: TeachingDisplayLabel::try_from("Learner 9".to_owned()).unwrap(),
            reason: PreviewEntitlementDenialReason::NotEntitled,
        };
        let schedule_wire = serde_json::to_value(schedule_row).unwrap();
        assert_eq!(
            schedule_wire,
            serde_json::json!({
                "kind":"denied",
                "membership":"M-9",
                "display":"Learner 9",
                "reason":"notEntitled"
            })
        );
        let denied_wire = serde_json::to_string(&PreviewEvaluation::Denied {
            reason: PreviewDenialReason::NotEntitled,
        })
        .unwrap();
        let schedule_wire = serde_json::to_string(&schedule_wire).unwrap();
        for forbidden in [
            "subject",
            "schedule",
            "resolved",
            "provenance",
            "disclosure",
            "answer",
            "score",
            "audit",
        ] {
            assert!(
                !denied_wire.contains(forbidden),
                "{forbidden} leaked: {denied_wire}"
            );
            assert!(
                !schedule_wire.contains(forbidden),
                "{forbidden} leaked: {schedule_wire}"
            );
        }
        for purpose in [
            CourseGroupPurpose::Section,
            CourseGroupPurpose::Lab,
            CourseGroupPurpose::Cohort,
            CourseGroupPurpose::Accommodation,
            CourseGroupPurpose::Work,
        ] {
            assert!(
                !serde_json::to_string(&preview_group_role(purpose))
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
