//! Receipt persistence guards for the in-memory effective-policy backend.

use super::*;
use domain::effective_assignment_policy::{EffectiveAssignmentPolicy, PolicySource, ResolvedField};
use question_model::{
    AssignmentAudience, AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentItem,
    AssignmentItemId, AssignmentLifecycle, AssignmentScoringMode, CourseTerm, LateSubmissionPolicy,
    RunPolicies, StudentDisclosurePolicy,
};

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
    let attempt = QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(73_002));
    let group = CourseGroupId::from_uuid(uuid::Uuid::from_u128(73_003));
    let student = StudentId::from_uuid(uuid::Uuid::from_u128(73_004));
    let mut state = State::default();

    store_issued_effective_policy_receipt(
        &mut state,
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

#[test]
fn assignment_reference_validation_requires_published_new_pins_and_retains_exact_history() {
    let course = CourseId::from_uuid(uuid::Uuid::from_u128(73_102));
    let retained_assignment = AssignmentId::from_uuid(uuid::Uuid::from_u128(73_103));
    let new_assignment = AssignmentId::from_uuid(uuid::Uuid::from_u128(73_104));
    let mut published = super::super::catalog_search_tests::record(73_105);
    let reference = question_model::ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    published.lifecycle = question_model::CatalogLifecycle::Archived {
        reason: "Historical immutable evidence".to_string(),
    };
    let assignment = |id| AssignmentRecord {
        id,
        course_id: course,
        title: "Reference validation".to_string(),
        lifecycle: AssignmentLifecycle::Draft,
        instructions: AssignmentInstructions::default(),
        audience: AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: AssignmentItemId::from_uuid(uuid::Uuid::from_u128(73_106)),
            reference,
            position: 0,
            points_possible: question_model::PointValue::from_whole(1),
            delivery_state: question_model::AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        disclosure_policy: StudentDisclosurePolicy::default(),
        policies: RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: question_model::GradePolicy::First,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    let mut state = State::default();
    state.courses.insert(
        course,
        CourseRecord {
            id: course,
            title: "Reference validation course".to_string(),
            term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                .expect("valid course term"),
        },
    );
    state
        .published
        .insert((published.problem, published.version), published);
    let retained = assignment(retained_assignment);
    state
        .assignments
        .insert(retained_assignment, retained.clone());

    validate_memory_assignment_references(&state, &retained)
        .expect("unchanged exact archived pin remains valid evidence");
    assert!(matches!(
        validate_memory_assignment_references(&state, &assignment(new_assignment)),
        Err(StoreError::InvalidRecord(_))
    ));
    state
        .published
        .remove(&(reference.problem, reference.version));
    assert!(matches!(
        validate_memory_assignment_references(&state, &retained),
        Err(StoreError::InvalidRecord(_))
    ));
}
