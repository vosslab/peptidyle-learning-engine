//! Receipt persistence guards for the in-memory effective-policy backend.

use super::*;
use domain::effective_assignment_policy::{EffectiveAssignmentPolicy, PolicySource, ResolvedField};
use question_model::{AssignmentDeadlineBehavior, LateSubmissionPolicy};

fn policy_with_sources(
    available_at: PolicySource,
    due_at: PolicySource,
    closes_at: PolicySource,
    time_limit_seconds: PolicySource,
    attempt_limit: PolicySource,
    late_submission: PolicySource,
    deadline_behavior: PolicySource,
) -> EffectiveAssignmentPolicy {
    EffectiveAssignmentPolicy {
        available_at: ResolvedField {
            value: None,
            source: available_at,
        },
        due_at: ResolvedField {
            value: None,
            source: due_at,
        },
        closes_at: ResolvedField {
            value: None,
            source: closes_at,
        },
        time_limit_seconds: ResolvedField {
            value: None,
            source: time_limit_seconds,
        },
        attempt_limit: ResolvedField {
            value: None,
            source: attempt_limit,
        },
        late_submission: ResolvedField {
            value: LateSubmissionPolicy::Accept,
            source: late_submission,
        },
        deadline_behavior: ResolvedField {
            value: AssignmentDeadlineBehavior::AutoSubmit,
            source: deadline_behavior,
        },
    }
}

#[test]
fn effective_policy_receipts_reject_hypothetical_sources_without_mutation() {
    let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(73_001));
    let attempt = QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(73_002));
    let group = CourseGroupId::from_uuid(uuid::Uuid::from_u128(73_003));
    let student = StudentId::from_uuid(uuid::Uuid::from_u128(73_004));
    let mut state = State::default();

    store_issued_effective_policy_receipt(
        &mut state,
        tenant,
        attempt,
        policy_with_sources(
            PolicySource::Base,
            PolicySource::GroupScheduleOffsets(vec![group]),
            PolicySource::GroupAccommodations(vec![group]),
            PolicySource::IndividualException(student),
            PolicySource::Base,
            PolicySource::Base,
            PolicySource::Base,
        ),
    )
    .expect("real Base, group, and individual sources persist");
    let receipts_before = state.issued_effective_policy_receipts.clone();
    let sources_before = state.issued_effective_policy_field_sources.clone();
    let current_before = state.attempt_effective_policy_current.clone();

    let error = store_issued_effective_policy_receipt(
        &mut state,
        tenant,
        attempt,
        policy_with_sources(
            PolicySource::Base,
            PolicySource::Base,
            PolicySource::Base,
            PolicySource::Base,
            PolicySource::HypotheticalIndividualException,
            PolicySource::Base,
            PolicySource::Base,
        ),
    )
    .expect_err("hypothetical provenance cannot become issued authority");

    assert_eq!(
        error,
        StoreError::InvalidRecord(
            "hypothetical individual exceptions cannot be persisted in effective-policy receipts"
                .to_string(),
        )
    );
    assert_eq!(state.issued_effective_policy_receipts, receipts_before);
    assert_eq!(state.issued_effective_policy_field_sources, sources_before);
    assert_eq!(state.attempt_effective_policy_current, current_before);
}
