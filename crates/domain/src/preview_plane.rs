//! Pure composition of active Student Course Membership, Assessment policy, and Student Feedback disclosure.
//!
//! The Store owns route IDs and authorization. It resolves them, discards all
//! identity-bearing values, owns the resulting `StudentViewScenario`, and passes this
//! module only the already-resolved Active Student Course Membership and Effective
//! Assessment Policy facts. Evaluation borrows those facts
//! and returns an owned, closed Assessment Release Validation.

use question_model::{
    AccountTimeZone, ActiveStudentCourseMembershipDenialReason,
    ActiveStudentCourseMembershipGrantReason, ActiveStudentCourseMembershipOutcome,
    AssessmentAuthoredContentField, AssessmentPolicySourceKind, CourseTerm,
    EffectiveAssessmentPolicyView, LocalDateAndTime, PreviewDenialReason, PreviewDisclosureFlags,
    PreviewDisclosureMoment, PreviewDisclosureUnavailableReason, PreviewLateWorkRuleField,
    PreviewLimitField, PreviewResolvedPolicy, PreviewTimeField, StudentFeedbackReleaseRule,
    StudentFeedbackReleaseView, Timestamp,
};

use crate::{
    active_student_course_membership::ActiveStudentCourseMembershipDecision,
    effective_assessment_properties::{
        AssessmentAccessDecision, AssessmentPolicySource, EffectiveAssessmentPolicy,
    },
    student_feedback_release::evaluate_student_feedback_release,
};

/// Maps an internal Assessment Policy Source to a closed label, discarding its identifier.
pub fn assessment_policy_source_kind(
    source: &AssessmentPolicySource,
) -> AssessmentPolicySourceKind {
    match source {
        AssessmentPolicySource::Base => AssessmentPolicySourceKind::Base,
        AssessmentPolicySource::Accommodation(_) => AssessmentPolicySourceKind::Accommodation,
        AssessmentPolicySource::HypotheticalStudentViewScenario => {
            AssessmentPolicySourceKind::HypotheticalStudentViewScenario
        }
    }
}

/// Copies an Effective Assessment Policy into its identity-free preview representation.
pub fn project_preview_policy(
    policy: &EffectiveAssessmentPolicy,
    term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
) -> Result<PreviewResolvedPolicy, &'static str> {
    PreviewResolvedPolicy::new(
        time(
            &policy.available_at,
            term,
            account_time_zone,
            AssessmentAuthoredContentField::AvailableAt,
        )
        .map_err(|_| "invalid local preview time")?,
        time(
            &policy.due_at,
            term,
            account_time_zone,
            AssessmentAuthoredContentField::DueAt,
        )
        .map_err(|_| "invalid local preview time")?,
        time(
            &policy.closes_at,
            term,
            account_time_zone,
            AssessmentAuthoredContentField::ClosesAt,
        )
        .map_err(|_| "invalid local preview time")?,
        limit(&policy.assessment_attempt_time_limit_seconds),
        limit(&policy.attempt_limit),
        late(&policy.late_work_rule),
    )
}
fn time(
    field: &crate::effective_assessment_properties::EffectiveAssessmentPolicyValue<
        Option<Timestamp>,
    >,
    term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    kind: AssessmentAuthoredContentField,
) -> Result<PreviewTimeField, question_model::AssessmentAuthoredContentLocalError> {
    Ok(PreviewTimeField {
        value: field
            .value
            .map(|v| {
                LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                    v,
                    term,
                    account_time_zone,
                    kind,
                )
            })
            .transpose()?,
        source: assessment_policy_source_kind(&field.source),
    })
}
fn limit(
    field: &crate::effective_assessment_properties::EffectiveAssessmentPolicyValue<
        Option<std::num::NonZeroU32>,
    >,
) -> PreviewLimitField {
    PreviewLimitField {
        value: field.value.map(|v| v.get()),
        source: assessment_policy_source_kind(&field.source),
    }
}
fn late(
    field: &crate::effective_assessment_properties::EffectiveAssessmentPolicyValue<
        question_model::LateWorkRule,
    >,
) -> PreviewLateWorkRuleField {
    PreviewLateWorkRuleField {
        value: field.value,
        source: assessment_policy_source_kind(&field.source),
    }
}
/// Projects only the reusable window and limit fields.
pub fn project_preview_schedule(
    policy: &EffectiveAssessmentPolicy,
    term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
) -> Result<EffectiveAssessmentPolicyView, question_model::AssessmentAuthoredContentLocalError> {
    Ok(EffectiveAssessmentPolicyView {
        available_at: time(
            &policy.available_at,
            term,
            account_time_zone,
            AssessmentAuthoredContentField::AvailableAt,
        )?,
        due_at: time(
            &policy.due_at,
            term,
            account_time_zone,
            AssessmentAuthoredContentField::DueAt,
        )?,
        closes_at: time(
            &policy.closes_at,
            term,
            account_time_zone,
            AssessmentAuthoredContentField::ClosesAt,
        )?,
        assessment_attempt_time_limit_seconds: limit(&policy.assessment_attempt_time_limit_seconds),
        attempt_limit: limit(&policy.attempt_limit),
        late_work_rule: late(&policy.late_work_rule),
    })
}

/// Maps the Active Student Course Membership result to the transport-safe Assessment Access outcome.
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

/// Runs Student Feedback Release at the requested preview boundary. Due and Close remain unavailable when absent.
pub fn project_preview_student_feedback_release(
    effective: &AssessmentAccessDecision,
    rule: StudentFeedbackReleaseRule,
    moment: PreviewDisclosureMoment,
    now: Timestamp,
    submitted_at: Option<Timestamp>,
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
            question_answer_shown: value.question_answer,
            question_answer_explanation_shown: value.question_answer_explanation,
            statistics_shown: value.class_statistics,
        },
    }
}

fn allowed_policy(value: &AssessmentAccessDecision) -> Option<&EffectiveAssessmentPolicy> {
    match value {
        AssessmentAccessDecision::Allowed { policy, .. } => Some(policy),
        AssessmentAccessDecision::Denied { .. } => None,
    }
}

/// A preview denial is deliberately closed; callers must not attach any resolved data.
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
