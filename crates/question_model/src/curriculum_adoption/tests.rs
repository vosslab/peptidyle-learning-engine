use std::num::NonZeroU32;

use uuid::Uuid;

use super::*;
use crate::AssignmentRevision;

use crate::{
    AlphaCourseReference, AssignmentDeadlineBehavior, AssignmentReference,
    AssignmentTeachingSettingsFailureReason, BlueprintReference, CompletionRequirement,
    ContinuedPractice, CourseReference, GradePolicy, LateSubmissionPolicy, LocalTimeOfDay,
    MAX_ASSIGNMENT_ATTEMPT_LIMIT, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TIME_LIMIT_SECONDS, ProblemId, QuestionId, RunPolicies, StudentDisclosurePolicy,
    StudentDisclosureTiming, VariationPolicy, VersionId,
};

mod commands;
mod inspection;
mod reconciliation;
mod recovery;

fn reference(value: u128) -> ProblemVersionRef {
    ProblemVersionRef {
        problem: ProblemId::from_uuid(Uuid::from_u128(value)),
        version: VersionId::from_uuid(Uuid::from_u128(value + 100)),
    }
}

fn alpha_assignment_source(
    source: ObservedAlphaSource,
    module_index: u16,
    assignment_index: u16,
) -> AssignmentDefinitionSourceView {
    AssignmentDefinitionSourceView::Alpha(
        ObservedAlphaAssignmentSource::new(source, module_index, assignment_index)
            .expect("bounded Alpha assignment source"),
    )
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
        student_disclosure: StudentDisclosurePolicy {
            score: StudentDisclosureTiming::AfterSubmit,
            per_item_correctness: StudentDisclosureTiming::AfterSubmit,
            feedback_text: StudentDisclosureTiming::AfterSubmit,
            solution: StudentDisclosureTiming::AfterClose,
            class_statistics: StudentDisclosureTiming::Never,
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
    let envelope = original.canonical_envelope();
    assert_eq!(envelope.version(), original.canonical_version());
    assert_eq!(envelope.digest(), original.digest());
    assert_eq!(
        envelope,
        CurriculumSemanticPayload::assignment(original_assignment.clone()).canonical_envelope()
    );
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
    changed.defaults.student_disclosure.score = StudentDisclosureTiming::AfterClose;
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

fn source_timestamp(
    term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
    value: &str,
) -> ActivityTimestamp {
    CourseLocalDateTime::parse(value)
        .expect("valid source local time")
        .resolve_for_course(term, field)
        .expect("resolvable source local time")
}

#[test]
fn base_policy_projection_preserves_source_calendar_days_and_wall_clock_times() {
    let term = CourseTerm::from_parts("2026-12-31", "2027-01-03", "America/Chicago")
        .expect("valid source term");
    let policy = BaseAssignmentPolicy {
        available_at: Some(source_timestamp(
            &term,
            AssignmentTeachingSettingsField::AvailableAt,
            "2026-12-31T08:00:00.000",
        )),
        due_at: Some(source_timestamp(
            &term,
            AssignmentTeachingSettingsField::DueAt,
            "2027-01-01T17:30:00.000",
        )),
        closes_at: Some(source_timestamp(
            &term,
            AssignmentTeachingSettingsField::ClosesAt,
            "2027-01-03T23:00:00.000",
        )),
        ..BaseAssignmentPolicy::default()
    };

    let schedule = RelativeAssignmentSchedule::from_base_policy(&policy, &term)
        .expect("stored schedule projects");
    assert_eq!(schedule.available_at, Some(relative(0, "08:00:00.000")));
    assert_eq!(schedule.due_at, Some(relative(1, "17:30:00.000")));
    assert_eq!(schedule.closes_at, Some(relative(3, "23:00:00.000")));
}

#[test]
fn base_policy_projection_preserves_a_partial_schedule() {
    let term = CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago")
        .expect("valid source term");
    let policy = BaseAssignmentPolicy {
        due_at: Some(source_timestamp(
            &term,
            AssignmentTeachingSettingsField::DueAt,
            "2026-09-01T17:30:00.000",
        )),
        ..BaseAssignmentPolicy::default()
    };

    let schedule = RelativeAssignmentSchedule::from_base_policy(&policy, &term)
        .expect("partial stored schedule projects");
    assert_eq!(schedule.available_at, None);
    assert_eq!(schedule.due_at, Some(relative(8, "17:30:00.000")));
    assert_eq!(schedule.closes_at, None);
}

#[test]
fn base_policy_projection_refuses_outside_term_and_out_of_order_schedules() {
    let term = CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago")
        .expect("valid source term");
    let outside = BaseAssignmentPolicy {
        due_at: Some(source_timestamp(
            &CourseTerm::from_parts("2026-08-24", "2027-01-02", "America/Chicago")
                .expect("wider source term"),
            AssignmentTeachingSettingsField::DueAt,
            "2026-12-13T17:30:00.000",
        )),
        ..BaseAssignmentPolicy::default()
    };
    assert_eq!(
        RelativeAssignmentSchedule::from_base_policy(&outside, &term),
        Err(AssignmentTeachingSettingsLocalError::OutsideCourseTerm(
            AssignmentTeachingSettingsField::DueAt
        ))
    );

    let out_of_order = BaseAssignmentPolicy {
        available_at: Some(source_timestamp(
            &term,
            AssignmentTeachingSettingsField::AvailableAt,
            "2026-09-02T08:00:00.000",
        )),
        due_at: Some(source_timestamp(
            &term,
            AssignmentTeachingSettingsField::DueAt,
            "2026-09-01T17:30:00.000",
        )),
        ..BaseAssignmentPolicy::default()
    };
    assert_eq!(
        RelativeAssignmentSchedule::from_base_policy(&out_of_order, &term),
        Err(AssignmentTeachingSettingsLocalError::ScheduleOutOfOrder)
    );
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
    assert_eq!(import.value(), 42);
    assert_eq!(serde_json::json!(import), serde_json::json!("42"));
    for invalid in ["", "0", "01", "+2", "-2", "9223372036854775808"] {
        assert!(invalid.parse::<CurriculumImportRevision>().is_err());
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

    let oversized = (1..=MAX_ASSIGNMENT_ORDERED_ENTRIES + 1)
        .map(|number| ObservedAssignmentRevision {
            assignment: AssignmentReference::new(
                u64::try_from(number).expect("assignment bound fits route reference"),
            )
            .expect("assignment reference"),
            revision: AssignmentRevision::INITIAL,
        })
        .collect();
    assert_eq!(
        CourseScheduleWitness::new(course, CourseScheduleRevision::INITIAL, oversized),
        Err(CourseScheduleWitnessError::TooManyAssignments)
    );
    let oversized_wire = serde_json::json!({
        "course": course,
        "scheduleRevision": "1",
        "assignmentRevisions": (1..=MAX_ASSIGNMENT_ORDERED_ENTRIES + 1)
            .map(|number| serde_json::json!({
                "assignment": format!("A-{number}"),
                "revision": "1",
            }))
            .collect::<Vec<_>>(),
    });
    assert!(serde_json::from_value::<CourseScheduleWitness>(oversized_wire).is_err());
}

#[test]
fn operation_specific_preview_requests_are_strict_and_truthful() {
    macro_rules! assert_rejects_authority_field {
        ($request:ty, $value:expr) => {{
            let mut wire = serde_json::to_value(&$value).expect("preview request serializes");
            wire.as_object_mut()
                .expect("preview request is an object")
                .insert("tenant".to_owned(), serde_json::json!("forged"));
            assert!(serde_json::from_value::<$request>(wire).is_err());
        }};
    }

    let term = CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
    let course = CourseReference::new(3).expect("course reference");
    let alpha = ObservedAlphaSource {
        reference: AlphaCourseReference::new(4).expect("Alpha reference"),
        revision: "2".parse().expect("Alpha revision"),
    };
    let blueprint = ObservedBlueprintSource {
        reference: BlueprintReference::new(5).expect("blueprint reference"),
        revision: "2".parse().expect("blueprint revision"),
    };
    let assignment = ObservedAssignmentRevision {
        assignment: AssignmentReference::new(9).expect("assignment reference"),
        revision: AssignmentRevision::new(3).expect("assignment revision"),
    };
    let witness = CourseScheduleWitness::new(
        course,
        CourseScheduleRevision::new(4).expect("schedule revision"),
        vec![assignment],
    )
    .expect("witness");

    let fork = ForkAlphaPreviewRequest {
        source: alpha,
        replacements: CurriculumPinReplacements::default(),
    };
    assert_rejects_authority_field!(ForkAlphaPreviewRequest, fork);
    let fork_wire = serde_json::to_value(&fork).expect("fork preview serializes");
    assert!(serde_json::from_value::<ForkAlphaPreviewRequest>(fork_wire.clone()).is_ok());
    for absent in ["course", "term", "title", "tenant", "authority"] {
        assert!(!fork_wire.to_string().contains(absent));
    }
    let mut forged_fork = fork_wire;
    forged_fork
        .as_object_mut()
        .expect("object")
        .insert("course".to_owned(), serde_json::json!("C-3"));
    assert!(serde_json::from_value::<ForkAlphaPreviewRequest>(forged_fork).is_err());

    let blueprint_request = BlueprintInstantiationPreviewRequest {
        source: blueprint,
        course,
        target_term: term.clone(),
        replacements: CurriculumPinReplacements::default(),
    };
    assert_rejects_authority_field!(BlueprintInstantiationPreviewRequest, blueprint_request);
    assert!(
        serde_json::from_value::<BlueprintInstantiationPreviewRequest>(
            serde_json::to_value(blueprint_request).expect("Blueprint preview serializes"),
        )
        .is_ok()
    );
    let alpha_request = AlphaInstantiationPreviewRequest {
        source: alpha,
        title: CurriculumAdoptionTitle::parse("Fall Biochemistry").expect("course title"),
        target_term: term.clone(),
        replacements: CurriculumPinReplacements::default(),
    };
    assert_rejects_authority_field!(AlphaInstantiationPreviewRequest, alpha_request);
    assert!(
        serde_json::from_value::<AlphaInstantiationPreviewRequest>(
            serde_json::to_value(alpha_request).expect("Alpha preview serializes"),
        )
        .is_ok()
    );
    assert!(
        serde_json::from_value::<CourseRolloverPreviewRequest>(
            serde_json::to_value(CourseRolloverPreviewRequest {
                witness: witness.clone(),
                title: CurriculumAdoptionTitle::parse("Biochemistry next term")
                    .expect("course title"),
                target_term: term.clone(),
                replacements: CurriculumPinReplacements::default(),
            })
            .expect("rollover preview serializes"),
        )
        .is_ok()
    );
    assert_rejects_authority_field!(
        CourseRolloverPreviewRequest,
        CourseRolloverPreviewRequest {
            witness: witness.clone(),
            title: CurriculumAdoptionTitle::parse("Biochemistry next term").expect("course title"),
            target_term: term.clone(),
            replacements: CurriculumPinReplacements::default(),
        }
    );
    assert!(
        serde_json::from_value::<CourseTermShiftPreviewRequest>(
            serde_json::to_value(CourseTermShiftPreviewRequest {
                witness: witness.clone(),
                target_term: term.clone(),
            })
            .expect("term-shift preview serializes"),
        )
        .is_ok()
    );
    assert_rejects_authority_field!(
        CourseTermShiftPreviewRequest,
        CourseTermShiftPreviewRequest {
            witness: witness.clone(),
            target_term: term.clone(),
        }
    );
    assert!(
        serde_json::from_value::<AssignmentFastForwardPreviewRequest>(
            serde_json::to_value(AssignmentFastForwardPreviewRequest {
                course,
                assignment,
                import_revision: "1".parse().expect("import revision"),
                source: alpha_assignment_source(alpha, 2, 3),
            })
            .expect("fast-forward preview serializes"),
        )
        .is_ok()
    );
    assert_rejects_authority_field!(
        AssignmentFastForwardPreviewRequest,
        AssignmentFastForwardPreviewRequest {
            course,
            assignment,
            import_revision: "1".parse().expect("import revision"),
            source: alpha_assignment_source(alpha, 2, 3),
        }
    );
    assert!(
        serde_json::from_value::<SourceDerivedAssignmentPreviewRequest>(
            serde_json::to_value(SourceDerivedAssignmentPreviewRequest {
                course,
                source: AssignmentDefinitionSourceView::Blueprint(blueprint),
                replacements: CurriculumPinReplacements::default(),
            })
            .expect("source-derived preview serializes"),
        )
        .is_ok()
    );
    assert_rejects_authority_field!(
        SourceDerivedAssignmentPreviewRequest,
        SourceDerivedAssignmentPreviewRequest {
            course,
            source: AssignmentDefinitionSourceView::Blueprint(blueprint),
            replacements: CurriculumPinReplacements::default(),
        }
    );
}

#[test]
fn operation_specific_completed_shapes_bind_their_own_receipts() {
    fn receipt() -> CurriculumAdoptionReceiptBinding {
        CurriculumAdoptionReceiptBinding {
            idempotency_key: CurriculumAdoptionIdempotencyKey::parse("adopt-2026-08-25")
                .expect("key"),
        }
    }

    let term = CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
    let course = CourseReference::new(7).expect("course");
    let assignment = AssignmentReference::new(9).expect("assignment");
    let source_course = CourseReference::new(3).expect("source course");
    let source = ObservedAlphaSource {
        reference: AlphaCourseReference::new(4).expect("source"),
        revision: "3".parse().expect("revision"),
    };

    let fork_wire = serde_json::to_value(ForkAlphaCompleted {
        source,
        alpha: AlphaCourseReference::new(8).expect("result Alpha"),
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("fork completed serializes");
    let blueprint_wire = serde_json::to_value(BlueprintInstantiationCompleted {
        course,
        assignment,
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("Blueprint completed serializes");
    let alpha_wire = serde_json::to_value(AlphaInstantiationCompleted {
        source,
        course,
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("Alpha completed serializes");
    let rollover_wire = serde_json::to_value(CourseRolloverCompleted {
        source_course,
        course,
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("rollover completed serializes");
    let term_shift_wire = serde_json::to_value(CourseTermShiftCompleted {
        course,
        term: term.clone(),
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("term-shift completed serializes");
    let fast_forward_wire = serde_json::to_value(AssignmentFastForwardCompleted {
        course,
        assignment,
        import_revision: "2".parse().expect("import revision"),
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("fast-forward completed serializes");
    let source_derived_wire = serde_json::to_value(SourceDerivedAssignmentCompleted {
        course,
        assignment,
        replay: CurriculumReplayStatus::Replayed,
        receipt: receipt(),
    })
    .expect("source-derived completed serializes");

    assert_eq!(fork_wire["alpha"], "AC-8");
    assert_eq!(blueprint_wire["assignment"], "A-9");
    assert_eq!(alpha_wire["source"]["reference"], "AC-4");
    assert_eq!(rollover_wire["sourceCourse"], "C-3");
    assert_eq!(term_shift_wire["term"], serde_json::json!(term));
    assert_eq!(fast_forward_wire["importRevision"], "2");
    assert_eq!(source_derived_wire["assignment"], "A-9");

    for wire in [
        &fork_wire,
        &blueprint_wire,
        &alpha_wire,
        &rollover_wire,
        &term_shift_wire,
        &fast_forward_wire,
        &source_derived_wire,
    ] {
        assert_eq!(wire["replay"], "replayed");
        assert_eq!(wire["receipt"]["idempotencyKey"], "adopt-2026-08-25");
        for absent in ["operation", "tenant", "authority", "answer", "grade"] {
            assert!(!wire.to_string().contains(absent));
        }
    }

    assert!(serde_json::from_value::<ForkAlphaCompleted>(fork_wire).is_ok());
    assert!(serde_json::from_value::<BlueprintInstantiationCompleted>(blueprint_wire).is_ok());
    assert!(serde_json::from_value::<AlphaInstantiationCompleted>(alpha_wire).is_ok());
    assert!(serde_json::from_value::<CourseRolloverCompleted>(rollover_wire).is_ok());
    assert!(serde_json::from_value::<CourseTermShiftCompleted>(term_shift_wire).is_ok());
    assert!(serde_json::from_value::<AssignmentFastForwardCompleted>(fast_forward_wire).is_ok());
    assert!(
        serde_json::from_value::<SourceDerivedAssignmentCompleted>(source_derived_wire).is_ok()
    );
}

#[test]
fn assignment_definition_source_requires_an_exact_bounded_alpha_location() {
    let source = ObservedAlphaSource {
        reference: AlphaCourseReference::new(4).expect("Alpha reference"),
        revision: "3".parse().expect("Alpha revision"),
    };
    let observed =
        ObservedAlphaAssignmentSource::new(source, 7, 11).expect("bounded Alpha assignment source");
    let wire = serde_json::to_value(AssignmentDefinitionSourceView::Alpha(observed))
        .expect("assignment source serializes");

    assert_eq!(
        wire,
        serde_json::json!({
            "kind": "alpha",
            "reference": "AC-4",
            "revision": "3",
            "moduleIndex": 7,
            "assignmentIndex": 11,
        })
    );
    assert_eq!(
        serde_json::from_value::<AssignmentDefinitionSourceView>(wire)
            .expect("assignment source decodes"),
        AssignmentDefinitionSourceView::Alpha(observed)
    );

    let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES).expect("position bound fits u16");
    assert!(ObservedAlphaAssignmentSource::new(source, bound, 0).is_err());
    assert!(ObservedAlphaAssignmentSource::new(source, 0, bound).is_err());
    assert!(
        serde_json::from_value::<AssignmentDefinitionSourceView>(serde_json::json!({
            "kind": "alpha",
            "reference": "AC-4",
            "revision": "3",
            "moduleIndex": bound,
            "assignmentIndex": 0,
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<AssignmentDefinitionSourceView>(serde_json::json!({
            "kind": "alpha",
            "reference": "AC-4",
            "revision": "3",
            "moduleIndex": 0,
            "assignmentIndex": bound,
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<AssignmentDefinitionSourceView>(serde_json::json!({
            "kind": "alpha",
            "reference": "AC-4",
            "revision": "3",
            "moduleIndex": 7,
            "assignmentIndex": 11,
            "tenant": "browser-supplied",
        }))
        .is_err()
    );
}
