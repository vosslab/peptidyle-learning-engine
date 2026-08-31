//! Pure composition of S5 entitlement, S3 policy, and S4 disclosure for WP-INST-T3.
//!
//! The Store owns route locators and authorization. It resolves them, discards all
//! identity-bearing values, owns the resulting `PreviewSubject`, and passes this
//! module only the already-resolved S5/S3 facts. Evaluation borrows those facts
//! and returns an owned, closed browser projection.

use question_model::{
    ActivityTimestamp, AssignmentTeachingSettingsField, CourseLocalDateTime, CourseTerm,
    PreviewDeadlineBehaviorField, PreviewDenialReason, PreviewDisclosureFlags,
    PreviewDisclosureMoment, PreviewDisclosureProjection, PreviewDisclosureUnavailableReason,
    PreviewEntitlementDenialReason, PreviewEntitlementGrantReason, PreviewEntitlementOutcome,
    PreviewLateSubmissionField, PreviewLimitField, PreviewPolicySourceLayer, PreviewResolvedPolicy,
    PreviewScheduleProjection, PreviewTimeField, StudentDisclosurePolicy,
};

use crate::{
    disclosure_policy::evaluate_student_disclosure,
    effective_assignment_policy::{
        EffectiveAssignmentPolicy, EffectivePolicyDecision, PolicySource,
    },
    entitlement::EntitlementDecision,
};

/// Maps an internal S3 source to a closed label, discarding every source identifier.
pub fn preview_source_layer(source: &PolicySource) -> PreviewPolicySourceLayer {
    match source {
        PolicySource::Base => PreviewPolicySourceLayer::Base,
        PolicySource::Accommodation(_) | PolicySource::HypotheticalAccommodation => {
            PreviewPolicySourceLayer::Accommodation
        }
    }
}

/// Copies an effective S3 policy into its identity-free preview representation.
pub fn project_preview_policy(
    policy: &EffectiveAssignmentPolicy,
    term: &CourseTerm,
) -> Result<PreviewResolvedPolicy, &'static str> {
    PreviewResolvedPolicy::new(
        time(
            &policy.available_at,
            term,
            AssignmentTeachingSettingsField::AvailableAt,
        )
        .map_err(|_| "invalid local preview time")?,
        time(&policy.due_at, term, AssignmentTeachingSettingsField::DueAt)
            .map_err(|_| "invalid local preview time")?,
        time(
            &policy.closes_at,
            term,
            AssignmentTeachingSettingsField::ClosesAt,
        )
        .map_err(|_| "invalid local preview time")?,
        limit(&policy.time_limit_seconds),
        limit(&policy.attempt_limit),
        late(&policy.late_submission),
        deadline(&policy.deadline_behavior),
    )
}
fn time(
    field: &crate::effective_assignment_policy::ResolvedField<Option<ActivityTimestamp>>,
    term: &CourseTerm,
    kind: AssignmentTeachingSettingsField,
) -> Result<PreviewTimeField, question_model::AssignmentTeachingSettingsLocalError> {
    Ok(PreviewTimeField {
        value: field
            .value
            .map(|v| CourseLocalDateTime::from_activity_timestamp(v, term, kind))
            .transpose()?,
        source: preview_source_layer(&field.source),
    })
}
fn limit(
    field: &crate::effective_assignment_policy::ResolvedField<Option<std::num::NonZeroU32>>,
) -> PreviewLimitField {
    PreviewLimitField {
        value: field.value.map(|v| v.get()),
        source: preview_source_layer(&field.source),
    }
}
fn late(
    field: &crate::effective_assignment_policy::ResolvedField<question_model::LateSubmissionPolicy>,
) -> PreviewLateSubmissionField {
    PreviewLateSubmissionField {
        value: field.value,
        source: preview_source_layer(&field.source),
    }
}
fn deadline(
    field: &crate::effective_assignment_policy::ResolvedField<
        question_model::AssignmentDeadlineBehavior,
    >,
) -> PreviewDeadlineBehaviorField {
    PreviewDeadlineBehaviorField {
        value: field.value,
        source: preview_source_layer(&field.source),
    }
}

/// Projects only the reusable window and limit fields.
pub fn project_preview_schedule(
    policy: &EffectiveAssignmentPolicy,
    term: &CourseTerm,
) -> Result<PreviewScheduleProjection, question_model::AssignmentTeachingSettingsLocalError> {
    Ok(PreviewScheduleProjection {
        available_at: time(
            &policy.available_at,
            term,
            AssignmentTeachingSettingsField::AvailableAt,
        )?,
        due_at: time(&policy.due_at, term, AssignmentTeachingSettingsField::DueAt)?,
        closes_at: time(
            &policy.closes_at,
            term,
            AssignmentTeachingSettingsField::ClosesAt,
        )?,
        time_limit_seconds: limit(&policy.time_limit_seconds),
        attempt_limit: limit(&policy.attempt_limit),
        late_submission: late(&policy.late_submission),
        deadline_behavior: deadline(&policy.deadline_behavior),
    })
}

/// Maps S5 result to the transport-safe entitlement outcome.
pub fn project_preview_entitlement(decision: &EntitlementDecision) -> PreviewEntitlementOutcome {
    match decision {
        EntitlementDecision::Granted(_) => PreviewEntitlementOutcome::Granted {
            reason: PreviewEntitlementGrantReason::ActiveStudentCourseMembership,
        },
        EntitlementDecision::Denied(_) => PreviewEntitlementOutcome::Denied {
            reason: PreviewEntitlementDenialReason::NotEntitled,
        },
    }
}

/// Runs S4 at the requested preview boundary. Due and Close remain unavailable when absent.
pub fn project_preview_disclosure(
    effective: &EffectivePolicyDecision,
    disclosure: StudentDisclosurePolicy,
    moment: PreviewDisclosureMoment,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
) -> PreviewDisclosureProjection {
    let boundary = match moment {
        PreviewDisclosureMoment::Now => Some(now),
        PreviewDisclosureMoment::Due => {
            allowed_policy(effective).and_then(|policy| policy.due_at.value)
        }
        PreviewDisclosureMoment::Close => {
            allowed_policy(effective).and_then(|policy| policy.closes_at.value)
        }
    };
    let Some(moment_time) = boundary else {
        return PreviewDisclosureProjection::Unavailable {
            moment,
            reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
        };
    };
    let Some(value) = evaluate_student_disclosure(disclosure, effective, moment_time, submitted_at)
    else {
        return PreviewDisclosureProjection::Unavailable {
            moment,
            reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
        };
    };
    PreviewDisclosureProjection::Available {
        moment,
        flags: PreviewDisclosureFlags {
            score_shown: value.score,
            correctness_shown: value.per_item_correctness,
            feedback_shown: value.feedback_text,
            solution_shown: value.solution,
            statistics_shown: value.class_statistics,
        },
    }
}

fn allowed_policy(value: &EffectivePolicyDecision) -> Option<&EffectiveAssignmentPolicy> {
    match value {
        EffectivePolicyDecision::Allowed { policy, .. } => Some(policy),
        EffectivePolicyDecision::Denied { .. } => None,
    }
}

/// A T3 denial is deliberately closed; callers must not attach any resolved data.
pub fn preview_denial_for(
    entitlement: &EntitlementDecision,
    current_revision_matches: bool,
) -> Option<PreviewDenialReason> {
    if !current_revision_matches {
        return Some(PreviewDenialReason::StaleRevision);
    }
    matches!(entitlement, EntitlementDecision::Denied(_))
        .then_some(PreviewDenialReason::NotEntitled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effective_assignment_policy::{
        AssignmentLifecycleGate, AuthorizationGate, BaseAssignmentPolicy, LateVerdict,
        PolicySource, ResolveEffectivePolicyInput, ResolvedField, StartVerdict,
        resolve_effective_policy,
    };
    use crate::entitlement::{
        ActiveStudentMembership, EntitlementDenial, EntitlementFacts,
        evaluate_assignment_entitlement,
    };
    use chrono::TimeZone;
    use question_model::{
        AccountId, AssignmentDeadlineBehavior, AssignmentId, CourseId, CourseMembershipId,
        CourseTerm, LateSubmissionPolicy,
        StudentDisclosureTiming, StudentRecordId,
    };
    use std::num::NonZeroU32;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
    fn allowed() -> (EntitlementDecision, EffectivePolicyDecision) {
        let facts = EntitlementFacts {
            course: CourseId::from_uuid(id(2)),
            assignment: AssignmentId::from_uuid(id(3)),
            student_account: AccountId::from_uuid(id(4)),
            membership: Some(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(id(5)),
                student_record: StudentRecordId::from_uuid(id(6)),
            }),
        };
        let entitlement = evaluate_assignment_entitlement(facts);
        let effective = resolve_effective_policy(ResolveEffectivePolicyInput {
            lifecycle: AssignmentLifecycleGate::Open,
            authorization: AuthorizationGate::Authorized,
            entitlement: entitlement.clone(),
            now: ActivityTimestamp::from_unix_millis(10),
            prior_run_count: 0,
            base: BaseAssignmentPolicy {
                available_at: None,
                due_at: Some(ActivityTimestamp::from_unix_millis(20)),
                closes_at: None,
                time_limit_seconds: None,
                attempt_limit: None,
                late_submission: LateSubmissionPolicy::Accept,
                deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
            },
            accommodation: None,
        })
        .unwrap();
        (entitlement, effective)
    }

    #[test]
    fn s5_s3_s4_projection_has_parity_and_missing_boundaries_are_explicit() {
        let (entitlement, effective) = allowed();
        assert!(matches!(
            project_preview_entitlement(&entitlement),
            PreviewEntitlementOutcome::Granted { .. }
        ));
        let disclosure = StudentDisclosurePolicy::default();
        assert!(matches!(
            project_preview_disclosure(
                &effective,
                disclosure,
                PreviewDisclosureMoment::Now,
                ActivityTimestamp::from_unix_millis(10),
                None
            ),
            PreviewDisclosureProjection::Available { .. }
        ));
        assert!(matches!(
            project_preview_disclosure(
                &effective,
                disclosure,
                PreviewDisclosureMoment::Due,
                ActivityTimestamp::from_unix_millis(10),
                None
            ),
            PreviewDisclosureProjection::Available { .. }
        ));
        assert!(matches!(
            project_preview_disclosure(
                &effective,
                disclosure,
                PreviewDisclosureMoment::Close,
                ActivityTimestamp::from_unix_millis(10),
                None
            ),
            PreviewDisclosureProjection::Unavailable {
                reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
                ..
            }
        ));
    }

    #[test]
    fn stale_revision_precedes_entitlement_and_denial_is_closed() {
        let (entitlement, _) = allowed();
        assert_eq!(
            preview_denial_for(&entitlement, false),
            Some(PreviewDenialReason::StaleRevision)
        );
        let denied = EntitlementDecision::Denied(EntitlementDenial::StudentNotActiveCourse);
        assert_eq!(
            preview_denial_for(&denied, true),
            Some(PreviewDenialReason::NotEntitled)
        );
        assert_eq!(
            serde_json::to_value(PreviewDenialReason::NotEntitled).unwrap(),
            serde_json::json!("notEntitled")
        );
    }

    #[test]
    fn actual_and_hypothetical_individual_sources_share_the_safe_layer() {
        let student = StudentRecordId::from_uuid(id(9));
        assert_eq!(
            preview_source_layer(&PolicySource::Accommodation(student)),
            PreviewPolicySourceLayer::Accommodation
        );
        assert_eq!(
            preview_source_layer(&PolicySource::HypotheticalAccommodation),
            PreviewPolicySourceLayer::Accommodation
        );
    }

    #[test]
    fn project_preview_policy_and_schedule_project_course_local_values_and_all_sources() {
        let student = StudentRecordId::from_uuid(id(9));
        let at = |hour| {
            ActivityTimestamp::from_unix_millis(
                chrono::Utc
                    .with_ymd_and_hms(2026, 8, 20, hour, 0, 0)
                    .unwrap()
                    .timestamp_millis(),
            )
        };
        let policy = EffectiveAssignmentPolicy {
            available_at: ResolvedField {
                value: Some(at(14)),
                source: PolicySource::Base,
            },
            due_at: ResolvedField {
                value: Some(at(15)),
                source: PolicySource::Accommodation(student),
            },
            closes_at: ResolvedField {
                value: Some(at(16)),
                source: PolicySource::Accommodation(student),
            },
            time_limit_seconds: ResolvedField {
                value: NonZeroU32::new(1_200),
                source: PolicySource::Accommodation(student),
            },
            attempt_limit: ResolvedField {
                value: NonZeroU32::new(3),
                source: PolicySource::Base,
            },
            late_submission: ResolvedField {
                value: LateSubmissionPolicy::Accept,
                source: PolicySource::Base,
            },
            deadline_behavior: ResolvedField {
                value: AssignmentDeadlineBehavior::AutoSubmit,
                source: PolicySource::Base,
            },
        };
        let term = CourseTerm::from_parts("2026-08-01", "2026-08-31", "America/Chicago").unwrap();
        let projected = project_preview_policy(&policy, &term).unwrap();
        let schedule = project_preview_schedule(&policy, &term).unwrap();

        assert_eq!(
            projected.available_at().value.as_ref().unwrap().as_str(),
            "2026-08-20T09:00:00.000"
        );
        assert_eq!(
            projected.due_at().value.as_ref().unwrap().as_str(),
            "2026-08-20T10:00:00.000"
        );
        assert_eq!(
            projected.closes_at().value.as_ref().unwrap().as_str(),
            "2026-08-20T11:00:00.000"
        );
        assert_eq!(
            projected.available_at().source,
            PreviewPolicySourceLayer::Base
        );
        assert_eq!(
            projected.due_at().source,
            PreviewPolicySourceLayer::Accommodation
        );
        assert_eq!(
            projected.closes_at().source,
            PreviewPolicySourceLayer::Accommodation
        );
        assert_eq!(
            projected.time_limit_seconds().source,
            PreviewPolicySourceLayer::Accommodation
        );
        assert_eq!(projected.time_limit_seconds().value, Some(1_200));
        assert_eq!(projected.attempt_limit().value, Some(3));
        assert_eq!(
            projected.late_submission().value,
            LateSubmissionPolicy::Accept
        );
        assert_eq!(
            projected.deadline_behavior().value,
            AssignmentDeadlineBehavior::AutoSubmit
        );
        assert_eq!(schedule.available_at, *projected.available_at());
        assert_eq!(schedule.due_at, *projected.due_at());
        assert_eq!(schedule.closes_at, *projected.closes_at());
        assert_eq!(schedule.time_limit_seconds, *projected.time_limit_seconds());
        assert_eq!(schedule.attempt_limit, *projected.attempt_limit());
        assert_eq!(schedule.late_submission, *projected.late_submission());
        assert_eq!(schedule.deadline_behavior, *projected.deadline_behavior());

        let wire = serde_json::to_string(&projected).unwrap();
        for forbidden in [
            "00000000-0000-0000-0000-000000000007",
            "00000000-0000-0000-0000-000000000008",
            "00000000-0000-0000-0000-000000000009",
        ] {
            assert!(
                !wire.contains(forbidden),
                "internal source ID leaked: {wire}"
            );
        }
    }

    #[test]
    fn denied_s5_s3_s4_never_produces_preview_data() {
        let entitlement = EntitlementDecision::Denied(EntitlementDenial::StudentNotActiveCourse);
        let effective = resolve_effective_policy(ResolveEffectivePolicyInput {
            lifecycle: AssignmentLifecycleGate::Open,
            authorization: AuthorizationGate::Authorized,
            entitlement: entitlement.clone(),
            now: ActivityTimestamp::from_unix_millis(10),
            prior_run_count: 0,
            base: BaseAssignmentPolicy {
                available_at: None,
                due_at: None,
                closes_at: None,
                time_limit_seconds: None,
                attempt_limit: None,
                late_submission: LateSubmissionPolicy::Accept,
                deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
            },
            accommodation: None,
        })
        .unwrap();

        assert!(matches!(&effective, EffectivePolicyDecision::Denied { .. }));

        assert_eq!(
            project_preview_entitlement(&entitlement),
            PreviewEntitlementOutcome::Denied {
                reason: PreviewEntitlementDenialReason::NotEntitled,
            }
        );
        assert_eq!(
            preview_denial_for(&entitlement, true),
            Some(PreviewDenialReason::NotEntitled)
        );
        for moment in [
            PreviewDisclosureMoment::Now,
            PreviewDisclosureMoment::Due,
            PreviewDisclosureMoment::Close,
        ] {
            assert_eq!(
                project_preview_disclosure(
                    &effective,
                    StudentDisclosurePolicy::default(),
                    moment,
                    ActivityTimestamp::from_unix_millis(10),
                    None,
                ),
                PreviewDisclosureProjection::Unavailable {
                    moment,
                    reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
                }
            );
        }
    }

    #[test]
    fn disclosure_now_due_close_matches_s4_flags() {
        let policy = EffectiveAssignmentPolicy {
            available_at: ResolvedField {
                value: None,
                source: PolicySource::Base,
            },
            due_at: ResolvedField {
                value: Some(ActivityTimestamp::from_unix_millis(20)),
                source: PolicySource::Base,
            },
            closes_at: ResolvedField {
                value: Some(ActivityTimestamp::from_unix_millis(30)),
                source: PolicySource::Base,
            },
            time_limit_seconds: ResolvedField {
                value: None,
                source: PolicySource::Base,
            },
            attempt_limit: ResolvedField {
                value: None,
                source: PolicySource::Base,
            },
            late_submission: ResolvedField {
                value: LateSubmissionPolicy::Accept,
                source: PolicySource::Base,
            },
            deadline_behavior: ResolvedField {
                value: AssignmentDeadlineBehavior::AutoSubmit,
                source: PolicySource::Base,
            },
        };
        let effective = EffectivePolicyDecision::Allowed {
            policy: Box::new(policy),
            start: StartVerdict::MayStart {
                late: LateVerdict::OnTime,
            },
        };
        let disclosure = StudentDisclosurePolicy {
            score: StudentDisclosureTiming::DuringAttempt,
            per_item_correctness: StudentDisclosureTiming::AfterSubmit,
            feedback_text: StudentDisclosureTiming::AfterDue,
            solution: StudentDisclosureTiming::AfterClose,
            class_statistics: StudentDisclosureTiming::Never,
        };
        let flags = |score_shown, feedback_shown, solution_shown| PreviewDisclosureFlags {
            score_shown,
            correctness_shown: false,
            feedback_shown,
            solution_shown,
            statistics_shown: false,
        };
        for (moment, expected) in [
            (PreviewDisclosureMoment::Now, flags(true, false, false)),
            (PreviewDisclosureMoment::Due, flags(true, true, false)),
            (PreviewDisclosureMoment::Close, flags(true, true, true)),
        ] {
            assert_eq!(
                project_preview_disclosure(
                    &effective,
                    disclosure,
                    moment,
                    ActivityTimestamp::from_unix_millis(10),
                    None,
                ),
                PreviewDisclosureProjection::Available {
                    moment,
                    flags: expected,
                }
            );
        }
    }
}
