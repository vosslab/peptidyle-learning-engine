use std::num::NonZeroU32;

use uuid::Uuid;

use super::*;

use crate::{
    AlphaCourseReference, AssignmentDeadlineBehavior, AssignmentReference,
    AssignmentTeachingSettingsFailureReason, BlueprintReference, CompletionRequirement,
    ContinuedPractice, CourseReference, GradePolicy, LateSubmissionPolicy, LearnerDisclosurePolicy,
    LearnerDisclosureTiming, LocalTimeOfDay, MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    MAX_ASSIGNMENT_TIME_LIMIT_SECONDS, ProblemId, RunPolicies, VariationPolicy, VersionId,
};
fn reference(value: u128) -> ProblemVersionRef {
    ProblemVersionRef {
        problem: ProblemId::from_uuid(Uuid::from_u128(value)),
        version: VersionId::from_uuid(Uuid::from_u128(value + 100)),
    }
}

fn defaults() -> ReusableAssignmentDefaults {
    ReusableAssignmentDefaults {
        time_limit_seconds: None,
        attempt_limit: None,
        late_submission: LateSubmissionPolicy::Accept,
        deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
        run_policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
        learner_disclosure: LearnerDisclosurePolicy {
            score: LearnerDisclosureTiming::AfterSubmit,
            per_item_correctness: LearnerDisclosureTiming::AfterSubmit,
            feedback_text: LearnerDisclosureTiming::AfterSubmit,
            solution: LearnerDisclosureTiming::AfterClose,
            class_statistics: LearnerDisclosureTiming::Never,
        },
    }
}

fn relative(day_offset: i32, local_time: &str) -> RelativeScheduleMoment {
    RelativeScheduleMoment {
        day_offset,
        local_time: LocalTimeOfDay::parse(local_time).expect("valid local time"),
    }
}

fn entries() -> Vec<CurriculumSemanticAssignmentEntry> {
    vec![
        CurriculumSemanticAssignmentEntry::Fixed {
            reference: reference(1),
            points_possible: PointValue::from_whole(3),
            scoring_mode: AssignmentScoringMode::Normal,
        },
        CurriculumSemanticAssignmentEntry::Pool(
            CurriculumSemanticPool::new(
                vec![reference(2), reference(3)],
                1,
                PointValue::from_whole(2),
                SelectionOrdering::Randomized,
                PoolDrawAlgorithm::V1,
            )
            .expect("valid semantic pool"),
        ),
    ]
}

fn assignment() -> CurriculumSemanticAssignment {
    CurriculumSemanticAssignment::new(
        "Protein structure practice".to_string(),
        AssignmentInstructions::try_new("Explain each choice.".to_string())
            .expect("valid instructions"),
        entries(),
        defaults(),
        RelativeAssignmentSchedule::default(),
    )
    .expect("valid semantic assignment")
}

fn assert_changed(original: &CurriculumSemanticPayload, changed: CurriculumSemanticAssignment) {
    assert!(matches!(
        original.compare(&CurriculumSemanticPayload::assignment(changed)),
        CurriculumSemanticComparison::Changed { .. }
    ));
}

#[test]
fn semantic_digest_binds_every_assignment_meaning_category() {
    let original_assignment = assignment();
    let original = CurriculumSemanticPayload::assignment(original_assignment.clone());
    assert_eq!(
        original.compare(&CurriculumSemanticPayload::assignment(
            original_assignment.clone()
        )),
        CurriculumSemanticComparison::Equivalent {
            digest: original.digest()
        }
    );
    assert_eq!(
        original.canonical_version(),
        CURRICULUM_SEMANTIC_CANONICAL_VERSION
    );

    let mut changed = original_assignment.clone();
    changed.title.push_str(" revised");
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    changed.instructions = AssignmentInstructions::try_new("Show your reasoning.".to_string())
        .expect("valid instructions");
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    if let CurriculumSemanticAssignmentEntry::Fixed { reference: pin, .. } = &mut changed.entries[0]
    {
        *pin = reference(8);
    }
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    if let CurriculumSemanticAssignmentEntry::Fixed {
        points_possible, ..
    } = &mut changed.entries[0]
    {
        *points_possible = PointValue::from_whole(4);
    }
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    if let CurriculumSemanticAssignmentEntry::Fixed { scoring_mode, .. } = &mut changed.entries[0] {
        *scoring_mode = AssignmentScoringMode::ExtraCredit;
    }
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    if let CurriculumSemanticAssignmentEntry::Pool(pool) = &mut changed.entries[1] {
        pool.candidates.reverse();
    }
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    if let CurriculumSemanticAssignmentEntry::Pool(pool) = &mut changed.entries[1] {
        pool.draw_count = 2;
        pool.points_per_item = PointValue::from_whole(5);
        pool.ordering = SelectionOrdering::CandidateOrder;
    }
    assert_changed(&original, changed);

    let mut changed = original_assignment.clone();
    changed.defaults.late_submission = LateSubmissionPolicy::MarkLate;
    changed.defaults.run_policies.variation = VariationPolicy::FullRegeneration;
    changed.defaults.learner_disclosure.score = LearnerDisclosureTiming::AfterClose;
    assert_changed(&original, changed);

    let mut changed = original_assignment;
    changed.schedule.due_at = Some(relative(7, "17:00:00.000"));
    assert_changed(&original, changed);
}

#[test]
fn course_semantics_bind_labels_nested_assignments_and_authored_order() {
    let first = CurriculumSemanticModule::new("Week 1".to_string(), vec![assignment()])
        .expect("valid module");
    let mut second_assignment = assignment();
    second_assignment.title = "Protein folding practice".to_string();
    let second = CurriculumSemanticModule::new("Week 2".to_string(), vec![second_assignment])
        .expect("valid module");
    let course = CurriculumSemanticCourse::new(
        "Biochemistry".to_string(),
        vec![first.clone(), second.clone()],
    )
    .expect("valid course meaning");
    let original = CurriculumSemanticPayload::course(course.clone());

    let mut changed = course.clone();
    changed.title.push_str(" II");
    assert!(matches!(
        original.compare(&CurriculumSemanticPayload::course(changed)),
        CurriculumSemanticComparison::Changed { .. }
    ));

    let mut changed = course.clone();
    changed.modules[0].label = "Foundations".to_string();
    assert!(matches!(
        original.compare(&CurriculumSemanticPayload::course(changed)),
        CurriculumSemanticComparison::Changed { .. }
    ));

    let mut changed = course.clone();
    changed.modules[0].assignments[0].title = "A different assignment".to_string();
    assert!(matches!(
        original.compare(&CurriculumSemanticPayload::course(changed)),
        CurriculumSemanticComparison::Changed { .. }
    ));

    let mut changed = course;
    changed.modules.reverse();
    assert!(matches!(
        original.compare(&CurriculumSemanticPayload::course(changed)),
        CurriculumSemanticComparison::Changed { .. }
    ));
}

#[test]
fn semantic_constructors_refuse_invalid_reusable_meaning() {
    let empty = CurriculumSemanticAssignment::new(
        "Assignment".to_string(),
        AssignmentInstructions::try_new(String::new()).expect("empty instructions are valid"),
        Vec::new(),
        defaults(),
        RelativeAssignmentSchedule::default(),
    );
    assert_eq!(
        empty,
        Err(ReusableCurriculumValidationError::InvalidEntryCount)
    );

    assert_eq!(
        CurriculumSemanticPool::new(
            vec![reference(1), reference(1)],
            1,
            PointValue::from_whole(1),
            SelectionOrdering::CandidateOrder,
            PoolDrawAlgorithm::V1,
        ),
        Err(ReusableCurriculumValidationError::DuplicatePoolCandidate)
    );
    assert_eq!(
        CurriculumSemanticPool::new(
            vec![reference(1)],
            0,
            PointValue::from_whole(1),
            SelectionOrdering::CandidateOrder,
            PoolDrawAlgorithm::V1,
        ),
        Err(ReusableCurriculumValidationError::InvalidPoolDrawCount)
    );

    let mut invalid_defaults = defaults();
    invalid_defaults.time_limit_seconds = NonZeroU32::new(MAX_ASSIGNMENT_TIME_LIMIT_SECONDS + 1);
    invalid_defaults.attempt_limit = NonZeroU32::new(MAX_ASSIGNMENT_ATTEMPT_LIMIT + 1);
    assert_eq!(
        CurriculumSemanticAssignment::new(
            "Assignment".to_string(),
            AssignmentInstructions::try_new(String::new()).expect("empty instructions are valid"),
            entries(),
            invalid_defaults,
            RelativeAssignmentSchedule::default(),
        ),
        Err(ReusableCurriculumValidationError::TimeLimitOutOfRange)
    );

    let mut invalid_defaults = defaults();
    invalid_defaults.attempt_limit = NonZeroU32::new(MAX_ASSIGNMENT_ATTEMPT_LIMIT + 1);
    assert_eq!(
        CurriculumSemanticAssignment::new(
            "Assignment".to_string(),
            AssignmentInstructions::try_new(String::new()).expect("empty instructions are valid"),
            entries(),
            invalid_defaults,
            RelativeAssignmentSchedule::default(),
        ),
        Err(ReusableCurriculumValidationError::AttemptLimitOutOfRange)
    );

    let reversed_schedule = RelativeAssignmentSchedule {
        available_at: Some(relative(1, "08:00:00.000")),
        due_at: Some(relative(0, "17:00:00.000")),
        closes_at: None,
    };
    assert_eq!(
        CurriculumSemanticAssignment::new(
            "Assignment".to_string(),
            AssignmentInstructions::try_new(String::new()).expect("empty instructions are valid"),
            entries(),
            defaults(),
            reversed_schedule,
        ),
        Err(ReusableCurriculumValidationError::InvalidScheduleOrder)
    );

    assert_eq!(
        CurriculumSemanticModule::new("Week 1".to_string(), Vec::new()),
        Err(ReusableCurriculumValidationError::InvalidModuleDefinitionCount)
    );
    assert_eq!(
        CurriculumSemanticCourse::new("Biochemistry".to_string(), Vec::new()),
        Err(ReusableCurriculumValidationError::InvalidModuleCount)
    );
}

#[test]
fn schedule_revision_is_a_canonical_positive_postgres_bigint() {
    let revision: CourseScheduleRevision = "41".parse().expect("canonical revision");
    assert_eq!(revision.value(), 41);
    assert_eq!(
        revision.checked_next().expect("next revision").to_string(),
        "42"
    );
    assert!(
        CourseScheduleRevision::new(i64::MAX as u64)
            .expect("maximum revision")
            .checked_next()
            .is_none()
    );
    for invalid in ["0", "01", "+1", "-1", "", "9223372036854775808"] {
        assert!(invalid.parse::<CourseScheduleRevision>().is_err());
    }
}

#[test]
fn relative_schedule_resolves_calendar_days_and_safe_preview_fields() {
    let term = CourseTerm::from_parts("2026-12-31", "2027-01-03", "America/Chicago")
        .expect("valid target term");
    let schedule = RelativeAssignmentSchedule {
        available_at: Some(relative(0, "08:00:00.000")),
        due_at: Some(relative(1, "17:30:00.000")),
        closes_at: None,
    };
    let resolved = schedule
        .resolve_for_target_term(&term)
        .expect("schedule resolves");
    assert_eq!(resolved.time_zone.as_str(), "America/Chicago");
    assert_eq!(
        resolved
            .available_at
            .as_ref()
            .expect("available value")
            .local
            .as_str(),
        "2026-12-31T08:00:00.000"
    );
    assert_eq!(
        resolved.due_at.as_ref().expect("due value").local.as_str(),
        "2027-01-01T17:30:00.000"
    );
    let wire = serde_json::to_string(&resolved).expect("safe preview serializes");
    assert!(wire.contains("America/Chicago"));
    assert!(wire.contains("2027-01-01T17:30:00.000"));
    assert!(!wire.contains("00000000-0000-0000-0000-000000000001"));
}

#[test]
fn target_term_and_dst_refusals_keep_the_exact_schedule_field() {
    let one_day = CourseTerm::from_parts("2026-08-24", "2026-08-24", "America/Chicago")
        .expect("one-day target term");
    let outside = RelativeAssignmentSchedule {
        available_at: None,
        due_at: Some(relative(1, "17:00:00.000")),
        closes_at: None,
    };
    assert_eq!(
        outside.resolve_for_target_term(&one_day),
        Err(AssignmentTeachingSettingsLocalError::OutsideCourseTerm(
            AssignmentTeachingSettingsField::DueAt
        ))
    );

    let dst_term = CourseTerm::from_parts("2026-03-08", "2026-11-01", "America/Chicago")
        .expect("DST-spanning target term");
    let gap = RelativeAssignmentSchedule {
        available_at: Some(relative(0, "02:30:00.000")),
        due_at: None,
        closes_at: None,
    };
    assert_eq!(
        gap.resolve_for_target_term(&dst_term),
        Err(AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
            AssignmentTeachingSettingsField::AvailableAt
        ))
    );
    let ambiguity = RelativeAssignmentSchedule {
        available_at: None,
        due_at: Some(relative(238, "01:30:00.000")),
        closes_at: None,
    };
    assert_eq!(
        ambiguity.resolve_for_target_term(&dst_term),
        Err(AssignmentTeachingSettingsLocalError::AmbiguousLocalTime(
            AssignmentTeachingSettingsField::DueAt
        ))
    );
}

#[test]
fn adoption_revisions_and_idempotency_keys_use_canonical_bounded_wire_values() {
    let import: CurriculumImportRevision = "42".parse().expect("canonical import revision");
    let assignment_revision: AssignmentRevision = "43".parse().expect("canonical assignment");
    assert_eq!(import.value(), 42);
    assert_eq!(assignment_revision.value(), 43);
    assert_eq!(serde_json::json!(import), serde_json::json!("42"));
    assert_eq!(
        serde_json::json!(assignment_revision),
        serde_json::json!("43")
    );
    for invalid in ["", "0", "01", "+2", "-2", "9223372036854775808"] {
        assert!(invalid.parse::<CurriculumImportRevision>().is_err());
        assert!(invalid.parse::<AssignmentRevision>().is_err());
    }

    let key =
        CurriculumAdoptionIdempotencyKey::parse("instantiation-2026_08.25").expect("opaque key");
    assert_eq!(key.as_str(), "instantiation-2026_08.25");
    assert!(format!("{key:?}").contains("opaque"));
    for invalid in ["", "contains space", "slashes/not-allowed"] {
        assert!(CurriculumAdoptionIdempotencyKey::parse(invalid).is_err());
    }
}

#[test]
fn schedule_witness_normalizes_assignment_order_and_refuses_duplicates() {
    let course = CourseReference::new(8).expect("course route reference");
    let first = ObservedAssignmentRevision {
        assignment: AssignmentReference::new(4).expect("assignment route reference"),
        revision: AssignmentRevision::new(2).expect("assignment revision"),
    };
    let second = ObservedAssignmentRevision {
        assignment: AssignmentReference::new(2).expect("assignment route reference"),
        revision: AssignmentRevision::new(3).expect("assignment revision"),
    };
    let witness = CourseScheduleWitness::new(
        course,
        CourseScheduleRevision::new(7).expect("schedule revision"),
        vec![first, second],
    )
    .expect("distinct assignments");
    assert_eq!(witness.assignment_revisions(), &[second, first]);
    let noncanonical_order = serde_json::json!({
        "course": course,
        "scheduleRevision": "7",
        "assignmentRevisions": [first, second],
    });
    let decoded: CourseScheduleWitness =
        serde_json::from_value(noncanonical_order).expect("wire normalization is safe");
    assert_eq!(decoded.assignment_revisions(), &[second, first]);
    assert_eq!(
        CourseScheduleWitness::new(
            course,
            CourseScheduleRevision::INITIAL,
            vec![
                first,
                ObservedAssignmentRevision {
                    revision: AssignmentRevision::new(4).expect("revision"),
                    ..first
                }
            ],
        ),
        Err(CourseScheduleWitnessError::DuplicateAssignment)
    );
    let duplicate_wire = serde_json::json!({
        "course": course,
        "scheduleRevision": "1",
        "assignmentRevisions": [first, first],
    });
    assert!(serde_json::from_value::<CourseScheduleWitness>(duplicate_wire).is_err());
}

#[test]
fn browser_adoption_contracts_are_strict_and_answer_free() {
    let request = CurriculumAdoptionPreviewRequest::InstantiateBlueprint {
        source: ObservedBlueprintSource {
            reference: BlueprintReference::new(5).expect("blueprint reference"),
            revision: "2".parse().expect("blueprint revision"),
        },
        course: CourseReference::new(3).expect("course reference"),
        target_term: CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago")
            .expect("term"),
    };
    let wire = serde_json::to_value(&request).expect("preview request serializes");
    assert_eq!(wire["kind"], "instantiateBlueprint");
    let rendered = wire.to_string();
    for absent in ["tenant", "userId", "authority", "uuid", "answer", "grader"] {
        assert!(!rendered.contains(absent));
    }
    let mut unknown = wire;
    unknown
        .as_object_mut()
        .expect("request object")
        .insert("tenant".to_owned(), serde_json::json!("forged"));
    assert!(serde_json::from_value::<CurriculumAdoptionPreviewRequest>(unknown).is_err());

    let correction = CurriculumScheduleCorrection::from(
        AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
            AssignmentTeachingSettingsField::DueAt,
        ),
    );
    assert_eq!(
        correction.correction.field,
        AssignmentTeachingSettingsField::DueAt
    );
    assert_eq!(
        correction.correction.reason,
        AssignmentTeachingSettingsFailureReason::NonexistentLocalTime
    );
}

#[test]
fn completed_adoption_result_binds_replay_to_a_safe_receipt_shape() {
    let completed = CurriculumAdoptionCompleted {
        result: CurriculumAdoptionResultView {
            operation: CurriculumAdoptionOperation::InstantiateAlpha,
            course: CourseReference::new(7).expect("course reference"),
            assignment: None,
            term: CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago")
                .expect("term"),
        },
        replay: CurriculumReplayStatus::Replayed,
        receipt: CurriculumAdoptionReceiptBinding {
            operation: CurriculumAdoptionOperation::InstantiateAlpha,
            idempotency_key: CurriculumAdoptionIdempotencyKey::parse("alpha-2026-08-25")
                .expect("key"),
        },
    };
    let wire = serde_json::to_value(&completed).expect("completed result serializes");
    assert_eq!(wire["replay"], "replayed");
    assert_eq!(wire["receipt"]["idempotencyKey"], "alpha-2026-08-25");
    assert!(serde_json::from_value::<CurriculumAdoptionCompleted>(wire).is_ok());
}

#[test]
fn course_and_assignment_import_views_expose_only_route_safe_provenance() {
    let term = CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
    let course = CourseReference::new(7).expect("course reference");
    let assignment = AssignmentReference::new(9).expect("assignment reference");
    let source = ObservedAlphaSource {
        reference: AlphaCourseReference::new(4).expect("Alpha reference"),
        revision: "3".parse().expect("Alpha revision"),
    };
    let schedule = RelativeAssignmentSchedule::default()
        .resolve_for_target_term(&term)
        .expect("default schedule");
    let assignment_import = CurriculumImportView {
        course,
        assignment,
        source: CurriculumImportSourceView::Alpha(source),
        revision: "5".parse().expect("import revision"),
        reusable_meaning_matches_baseline: true,
    };
    let view = CurriculumCourseImportView {
        course,
        source,
        term,
        schedule_revision: CourseScheduleRevision::new(4).expect("schedule revision"),
        assignments: vec![assignment_import],
    };
    let wire = serde_json::to_value(&view).expect("course import serializes");
    assert_eq!(wire["course"], "C-7");
    assert_eq!(wire["assignments"][0]["assignment"], "A-9");
    assert!(serde_json::from_value::<CurriculumCourseImportView>(wire).is_ok());
    assert_eq!(schedule.time_zone.as_str(), "America/Chicago");
}
