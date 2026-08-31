//! Pure composition of the S5 active-membership gate, S3 policy, and S4 disclosure for WP-INST-T3.
//!
//! The Store owns route locators and authorization. It resolves them, discards all
//! identity-bearing values, owns the resulting `StudentViewScenario`, and passes this
//! module only the already-resolved S5/S3 facts. Evaluation borrows those facts
//! and returns an owned, closed browser projection.

use question_model::{
    ActiveStudentCourseMembershipDenialReason, ActiveStudentCourseMembershipGrantReason,
    ActiveStudentCourseMembershipOutcome, ActivityTimestamp, AssignmentPolicySourceKind,
    AssignmentWorkingCopyDefinitionField, CourseLocalDateAndTime, CourseTerm,
    EffectiveAssignmentPolicyView, PreviewAssignmentDeadlineRuleField, PreviewDenialReason,
    PreviewDisclosureFlags, PreviewDisclosureMoment, PreviewDisclosureUnavailableReason,
    PreviewLateWorkRuleField, PreviewLimitField, PreviewResolvedPolicy, PreviewTimeField,
    StudentFeedbackReleaseRule, StudentFeedbackReleaseView,
};

use crate::{
    active_student_course_membership::ActiveStudentCourseMembershipDecision,
    effective_assignment_policy::{
        AssignmentAccessDecision, EffectiveAssignmentPolicy, PolicySource,
    },
    student_feedback_release::evaluate_student_feedback_release,
};

/// Maps an internal S3 source to a closed label, discarding every source identifier.
pub fn assignment_policy_source_kind(source: &PolicySource) -> AssignmentPolicySourceKind {
    match source {
        PolicySource::Base => AssignmentPolicySourceKind::Base,
        PolicySource::Accommodation(_) | PolicySource::HypotheticalAccommodation => {
            AssignmentPolicySourceKind::Accommodation
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
            AssignmentWorkingCopyDefinitionField::AvailableAt,
        )
        .map_err(|_| "invalid local preview time")?,
        time(
            &policy.due_at,
            term,
            AssignmentWorkingCopyDefinitionField::DueAt,
        )
        .map_err(|_| "invalid local preview time")?,
        time(
            &policy.closes_at,
            term,
            AssignmentWorkingCopyDefinitionField::ClosesAt,
        )
        .map_err(|_| "invalid local preview time")?,
        limit(&policy.assignment_attempt_time_limit_seconds),
        limit(&policy.attempt_limit),
        late(&policy.late_work_rule),
        deadline(&policy.assignment_deadline_rule),
    )
}
fn time(
    field: &crate::effective_assignment_policy::EffectiveAssignmentPolicyValue<
        Option<ActivityTimestamp>,
    >,
    term: &CourseTerm,
    kind: AssignmentWorkingCopyDefinitionField,
) -> Result<PreviewTimeField, question_model::AssignmentWorkingCopyDefinitionLocalError> {
    Ok(PreviewTimeField {
        value: field
            .value
            .map(|v| CourseLocalDateAndTime::from_activity_timestamp(v, term, kind))
            .transpose()?,
        source: assignment_policy_source_kind(&field.source),
    })
}
fn limit(
    field: &crate::effective_assignment_policy::EffectiveAssignmentPolicyValue<
        Option<std::num::NonZeroU32>,
    >,
) -> PreviewLimitField {
    PreviewLimitField {
        value: field.value.map(|v| v.get()),
        source: assignment_policy_source_kind(&field.source),
    }
}
fn late(
    field: &crate::effective_assignment_policy::EffectiveAssignmentPolicyValue<
        question_model::LateWorkRule,
    >,
) -> PreviewLateWorkRuleField {
    PreviewLateWorkRuleField {
        value: field.value,
        source: assignment_policy_source_kind(&field.source),
    }
}
fn deadline(
    field: &crate::effective_assignment_policy::EffectiveAssignmentPolicyValue<
        question_model::AssignmentDeadlineRule,
    >,
) -> PreviewAssignmentDeadlineRuleField {
    PreviewAssignmentDeadlineRuleField {
        value: field.value,
        source: assignment_policy_source_kind(&field.source),
    }
}

/// Projects only the reusable window and limit fields.
pub fn project_preview_schedule(
    policy: &EffectiveAssignmentPolicy,
    term: &CourseTerm,
) -> Result<EffectiveAssignmentPolicyView, question_model::AssignmentWorkingCopyDefinitionLocalError>
{
    Ok(EffectiveAssignmentPolicyView {
        available_at: time(
            &policy.available_at,
            term,
            AssignmentWorkingCopyDefinitionField::AvailableAt,
        )?,
        due_at: time(
            &policy.due_at,
            term,
            AssignmentWorkingCopyDefinitionField::DueAt,
        )?,
        closes_at: time(
            &policy.closes_at,
            term,
            AssignmentWorkingCopyDefinitionField::ClosesAt,
        )?,
        assignment_attempt_time_limit_seconds: limit(&policy.assignment_attempt_time_limit_seconds),
        attempt_limit: limit(&policy.attempt_limit),
        late_work_rule: late(&policy.late_work_rule),
        assignment_deadline_rule: deadline(&policy.assignment_deadline_rule),
    })
}

/// Maps the S5 active-membership result to the transport-safe Assignment Access outcome.
pub fn project_active_student_course_membership(
    decision: &ActiveStudentCourseMembershipDecision,
) -> ActiveStudentCourseMembershipOutcome {
    match decision {
        ActiveStudentCourseMembershipDecision::Granted(_) => {
            ActiveStudentCourseMembershipOutcome::Granted {
                reason: ActiveStudentCourseMembershipGrantReason::ActiveStudentCourseMembership,
            }
        }
        ActiveStudentCourseMembershipDecision::Denied(_) => {
            ActiveStudentCourseMembershipOutcome::Denied {
                reason: ActiveStudentCourseMembershipDenialReason::NoActiveStudentCourseMembership,
            }
        }
    }
}

/// Runs S4 at the requested preview boundary. Due and Close remain unavailable when absent.
pub fn project_preview_student_feedback_release(
    effective: &AssignmentAccessDecision,
    rule: StudentFeedbackReleaseRule,
    moment: PreviewDisclosureMoment,
    now: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
) -> StudentFeedbackReleaseView {
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
        return StudentFeedbackReleaseView::Unavailable {
            moment,
            reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
        };
    };
    let Some(value) = evaluate_student_feedback_release(rule, effective, moment_time, submitted_at)
    else {
        return StudentFeedbackReleaseView::Unavailable {
            moment,
            reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
        };
    };
    StudentFeedbackReleaseView::Available {
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

fn allowed_policy(value: &AssignmentAccessDecision) -> Option<&EffectiveAssignmentPolicy> {
    match value {
        AssignmentAccessDecision::Allowed { policy, .. } => Some(policy),
        AssignmentAccessDecision::Denied { .. } => None,
    }
}

/// A T3 denial is deliberately closed; callers must not attach any resolved data.
pub fn preview_denial_for(
    active_student_course_membership: &ActiveStudentCourseMembershipDecision,
    current_revision_matches: bool,
) -> Option<PreviewDenialReason> {
    if !current_revision_matches {
        return Some(PreviewDenialReason::StaleRevision);
    }
    matches!(
        active_student_course_membership,
        ActiveStudentCourseMembershipDecision::Denied(_)
    )
    .then_some(PreviewDenialReason::ActiveStudentCourseMembershipRequired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_student_course_membership::{
        ActiveStudentCourseMembershipDenial, ActiveStudentCourseMembershipFacts,
        ActiveStudentMembership, evaluate_active_student_course_membership,
    };
    use crate::effective_assignment_policy::{
        AssignmentStartDecision, AssignmentStatusGate, AuthorizationGate, BaseAssignmentPolicy,
        EffectiveAssignmentPolicyValue, PolicySource, ResolveEffectivePolicyInput,
        StudentLateWorkStatus, resolve_effective_policy,
    };
    use chrono::TimeZone;
    use question_model::{
        AccountId, AssignmentDeadlineRule, AssignmentId, CourseId, CourseMembershipId, CourseTerm,
        LateWorkRule, StudentFeedbackReleaseRule, StudentFeedbackReleaseTiming, StudentRecordId,
    };
    use std::num::NonZeroU32;
    use uuid::Uuid;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
    fn allowed() -> (
        ActiveStudentCourseMembershipDecision,
        AssignmentAccessDecision,
    ) {
        let facts = ActiveStudentCourseMembershipFacts {
            course: CourseId::from_uuid(id(2)),
            assignment: AssignmentId::from_uuid(id(3)),
            student_account: AccountId::from_uuid(id(4)),
            membership: Some(ActiveStudentMembership {
                id: CourseMembershipId::from_uuid(id(5)),
                student_record: StudentRecordId::from_uuid(id(6)),
            }),
        };
        let active_student_course_membership = evaluate_active_student_course_membership(facts);
        let effective = resolve_effective_policy(ResolveEffectivePolicyInput {
            assignment_status: AssignmentStatusGate::Open,
            authorization: AuthorizationGate::Authorized,
            active_student_course_membership: active_student_course_membership.clone(),
            now: ActivityTimestamp::from_unix_millis(10),
            prior_run_count: 0,
            base: BaseAssignmentPolicy {
                available_at: None,
                due_at: Some(ActivityTimestamp::from_unix_millis(20)),
                closes_at: None,
                assignment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
            },
            accommodation: None,
        })
        .unwrap();
        (active_student_course_membership, effective)
    }

    #[test]
    fn s5_s3_s4_projection_has_parity_and_missing_boundaries_are_explicit() {
        let (active_student_course_membership, effective) = allowed();
        assert!(matches!(
            project_active_student_course_membership(&active_student_course_membership),
            ActiveStudentCourseMembershipOutcome::Granted { .. }
        ));
        let disclosure = StudentFeedbackReleaseRule::default();
        assert!(matches!(
            project_preview_student_feedback_release(
                &effective,
                disclosure,
                PreviewDisclosureMoment::Now,
                ActivityTimestamp::from_unix_millis(10),
                None
            ),
            StudentFeedbackReleaseView::Available { .. }
        ));
        assert!(matches!(
            project_preview_student_feedback_release(
                &effective,
                disclosure,
                PreviewDisclosureMoment::Due,
                ActivityTimestamp::from_unix_millis(10),
                None
            ),
            StudentFeedbackReleaseView::Available { .. }
        ));
        assert!(matches!(
            project_preview_student_feedback_release(
                &effective,
                disclosure,
                PreviewDisclosureMoment::Close,
                ActivityTimestamp::from_unix_millis(10),
                None
            ),
            StudentFeedbackReleaseView::Unavailable {
                reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
                ..
            }
        ));
    }

    #[test]
    fn stale_revision_precedes_assignment_access_denial_and_is_closed() {
        let (active_student_course_membership, _) = allowed();
        assert_eq!(
            preview_denial_for(&active_student_course_membership, false),
            Some(PreviewDenialReason::StaleRevision)
        );
        let denied = ActiveStudentCourseMembershipDecision::Denied(
            ActiveStudentCourseMembershipDenial::StudentNotActiveCourse,
        );
        assert_eq!(
            preview_denial_for(&denied, true),
            Some(PreviewDenialReason::ActiveStudentCourseMembershipRequired)
        );
        assert_eq!(
            serde_json::to_value(PreviewDenialReason::ActiveStudentCourseMembershipRequired)
                .unwrap(),
            serde_json::json!("activeStudentCourseMembershipRequired")
        );
    }

    #[test]
    fn actual_and_hypothetical_individual_sources_share_the_safe_layer() {
        let student = StudentRecordId::from_uuid(id(9));
        assert_eq!(
            assignment_policy_source_kind(&PolicySource::Accommodation(student)),
            AssignmentPolicySourceKind::Accommodation
        );
        assert_eq!(
            assignment_policy_source_kind(&PolicySource::HypotheticalAccommodation),
            AssignmentPolicySourceKind::Accommodation
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
            available_at: EffectiveAssignmentPolicyValue {
                value: Some(at(14)),
                source: PolicySource::Base,
            },
            due_at: EffectiveAssignmentPolicyValue {
                value: Some(at(15)),
                source: PolicySource::Accommodation(student),
            },
            closes_at: EffectiveAssignmentPolicyValue {
                value: Some(at(16)),
                source: PolicySource::Accommodation(student),
            },
            assignment_attempt_time_limit_seconds: EffectiveAssignmentPolicyValue {
                value: NonZeroU32::new(1_200),
                source: PolicySource::Accommodation(student),
            },
            attempt_limit: EffectiveAssignmentPolicyValue {
                value: NonZeroU32::new(3),
                source: PolicySource::Base,
            },
            late_work_rule: EffectiveAssignmentPolicyValue {
                value: LateWorkRule::Accept,
                source: PolicySource::Base,
            },
            assignment_deadline_rule: EffectiveAssignmentPolicyValue {
                value: AssignmentDeadlineRule::AutoSubmit,
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
            AssignmentPolicySourceKind::Base
        );
        assert_eq!(
            projected.due_at().source,
            AssignmentPolicySourceKind::Accommodation
        );
        assert_eq!(
            projected.closes_at().source,
            AssignmentPolicySourceKind::Accommodation
        );
        assert_eq!(
            projected.assignment_attempt_time_limit_seconds().source,
            AssignmentPolicySourceKind::Accommodation
        );
        assert_eq!(
            projected.assignment_attempt_time_limit_seconds().value,
            Some(1_200)
        );
        assert_eq!(projected.attempt_limit().value, Some(3));
        assert_eq!(projected.late_work_rule().value, LateWorkRule::Accept);
        assert_eq!(
            projected.assignment_deadline_rule().value,
            AssignmentDeadlineRule::AutoSubmit
        );
        assert_eq!(schedule.available_at, *projected.available_at());
        assert_eq!(schedule.due_at, *projected.due_at());
        assert_eq!(schedule.closes_at, *projected.closes_at());
        assert_eq!(
            schedule.assignment_attempt_time_limit_seconds,
            *projected.assignment_attempt_time_limit_seconds()
        );
        assert_eq!(schedule.attempt_limit, *projected.attempt_limit());
        assert_eq!(schedule.late_work_rule, *projected.late_work_rule());
        assert_eq!(
            schedule.assignment_deadline_rule,
            *projected.assignment_deadline_rule()
        );

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
        let active_student_course_membership = ActiveStudentCourseMembershipDecision::Denied(
            ActiveStudentCourseMembershipDenial::StudentNotActiveCourse,
        );
        let effective = resolve_effective_policy(ResolveEffectivePolicyInput {
            assignment_status: AssignmentStatusGate::Open,
            authorization: AuthorizationGate::Authorized,
            active_student_course_membership: active_student_course_membership.clone(),
            now: ActivityTimestamp::from_unix_millis(10),
            prior_run_count: 0,
            base: BaseAssignmentPolicy {
                available_at: None,
                due_at: None,
                closes_at: None,
                assignment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Accept,
                assignment_deadline_rule: AssignmentDeadlineRule::AutoSubmit,
            },
            accommodation: None,
        })
        .unwrap();

        assert!(matches!(
            &effective,
            AssignmentAccessDecision::Denied { .. }
        ));

        assert_eq!(
            project_active_student_course_membership(&active_student_course_membership),
            ActiveStudentCourseMembershipOutcome::Denied {
                reason: ActiveStudentCourseMembershipDenialReason::NoActiveStudentCourseMembership,
            }
        );
        assert_eq!(
            preview_denial_for(&active_student_course_membership, true),
            Some(PreviewDenialReason::ActiveStudentCourseMembershipRequired)
        );
        for moment in [
            PreviewDisclosureMoment::Now,
            PreviewDisclosureMoment::Due,
            PreviewDisclosureMoment::Close,
        ] {
            assert_eq!(
                project_preview_student_feedback_release(
                    &effective,
                    StudentFeedbackReleaseRule::default(),
                    moment,
                    ActivityTimestamp::from_unix_millis(10),
                    None,
                ),
                StudentFeedbackReleaseView::Unavailable {
                    moment,
                    reason: PreviewDisclosureUnavailableReason::BoundaryMissing,
                }
            );
        }
    }

    #[test]
    fn disclosure_now_due_close_matches_s4_flags() {
        let policy = EffectiveAssignmentPolicy {
            available_at: EffectiveAssignmentPolicyValue {
                value: None,
                source: PolicySource::Base,
            },
            due_at: EffectiveAssignmentPolicyValue {
                value: Some(ActivityTimestamp::from_unix_millis(20)),
                source: PolicySource::Base,
            },
            closes_at: EffectiveAssignmentPolicyValue {
                value: Some(ActivityTimestamp::from_unix_millis(30)),
                source: PolicySource::Base,
            },
            assignment_attempt_time_limit_seconds: EffectiveAssignmentPolicyValue {
                value: None,
                source: PolicySource::Base,
            },
            attempt_limit: EffectiveAssignmentPolicyValue {
                value: None,
                source: PolicySource::Base,
            },
            late_work_rule: EffectiveAssignmentPolicyValue {
                value: LateWorkRule::Accept,
                source: PolicySource::Base,
            },
            assignment_deadline_rule: EffectiveAssignmentPolicyValue {
                value: AssignmentDeadlineRule::AutoSubmit,
                source: PolicySource::Base,
            },
        };
        let effective = AssignmentAccessDecision::Allowed {
            policy: Box::new(policy),
            start_decision: AssignmentStartDecision::MayStart {
                late_work_status: StudentLateWorkStatus::OnTime,
            },
        };
        let disclosure = StudentFeedbackReleaseRule {
            score: StudentFeedbackReleaseTiming::DuringAttempt,
            per_item_correctness: StudentFeedbackReleaseTiming::AfterSubmit,
            feedback_text: StudentFeedbackReleaseTiming::AfterDue,
            solution: StudentFeedbackReleaseTiming::AfterClose,
            class_statistics: StudentFeedbackReleaseTiming::Never,
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
                project_preview_student_feedback_release(
                    &effective,
                    disclosure,
                    moment,
                    ActivityTimestamp::from_unix_millis(10),
                    None,
                ),
                StudentFeedbackReleaseView::Available {
                    moment,
                    flags: expected,
                }
            );
        }
    }
}
