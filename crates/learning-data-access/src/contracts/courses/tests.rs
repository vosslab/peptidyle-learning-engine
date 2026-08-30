use super::*;
use question_model::{
    AssignmentScoringMode, AssignmentSelectionCandidate, AttemptTimerRecord, ImplementationVersion,
    PointValue, ProblemVersionRef,
};

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn selection_run(id_value: u128, enrollment: u128, run_number: u32) -> AssignmentRun {
    AssignmentRun {
        id: RunId::from_uuid(id(id_value)),
        reference: question_model::RunReference::new(
            u64::try_from(id_value).expect("run reference"),
        )
        .expect("valid run reference"),
        enrollment: EnrollmentId::from_uuid(id(enrollment)),
        run_number,
        started_at: ActivityTimestamp::from_unix_millis(0),
        completed_at: None,
        score: None,
        mode: question_model::RunMode::Assigned,
        variation: question_model::VariationPolicy::NewSeeds,
    }
}

#[test]
fn run_selection_is_reproducible_and_freezes_expanded_order() {
    let reference = |value| ProblemVersionRef {
        problem: ProblemId::from_uuid(id(10 + value)),
        version: VersionId::from_uuid(id(20 + value)),
    };
    let assignment = AssignmentRecord {
        id: AssignmentId::from_uuid(id(1)),
        course_id: CourseId::from_uuid(id(3)),
        title: "Selection fixture".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: AssignmentItemId::from_uuid(id(30)),
            reference: reference(0),
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: vec![AssignmentSelectionGroup {
            id: question_model::AssignmentSelectionGroupId::from_uuid(id(31)),
            position: 1,
            draw_count: 2,
            points_per_item: PointValue::from_whole(2),
            ordering: SelectionOrdering::Randomized,
            algorithm: question_model::PoolDrawAlgorithm::V1,
            candidates: (1..=4)
                .map(|value| AssignmentSelectionCandidate {
                    id: AssignmentItemId::from_uuid(id(40 + value)),
                    position: u32::try_from(value - 1).expect("fixture position"),
                    reference: reference(value),
                    delivery_state: if value == 4 {
                        AssignmentDeliveryState::Retired
                    } else {
                        AssignmentDeliveryState::Active
                    },
                })
                .collect(),
        }],
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    let run = selection_run(100, 101, 1);
    let first = select_assignment_run_items(&assignment, &run).expect("valid selection");
    let replay = select_assignment_run_items(&assignment, &run).expect("repeat selection");

    assert_eq!(first, replay);
    assert_eq!(first.len(), 3);
    assert_eq!(
        first
            .iter()
            .map(|item| item.issued_position)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(first[0].selection_group.is_none());
    assert!(first[1..].iter().all(|item| item.selection_seed.is_some()));
    assert!(
        first
            .iter()
            .all(|item| item.assignment_item != AssignmentItemId::from_uuid(id(44)))
    );
    let later_run = selection_run(102, 101, 2);
    let later = select_assignment_run_items(&assignment, &later_run)
        .expect("later stable-enrollment selection");
    assert_eq!(
        first
            .iter()
            .map(|item| (item.assignment_item, item.selection_seed))
            .collect::<Vec<_>>(),
        later
            .iter()
            .map(|item| (item.assignment_item, item.selection_seed))
            .collect::<Vec<_>>()
    );
}

#[test]
fn full_regeneration_derives_a_new_draw_basis_without_promising_a_new_sample() {
    let mut assignment = immutable_assignment_fixture();
    assignment.policies.variation = question_model::VariationPolicy::FullRegeneration;
    let first_run = selection_run(110, 111, 1);
    let later_run = selection_run(112, 111, 2);

    let first =
        select_assignment_run_items(&assignment, &first_run).expect("first regenerated selection");
    let later =
        select_assignment_run_items(&assignment, &later_run).expect("later regenerated selection");

    assert_ne!(
        first
            .iter()
            .find_map(|item| item.selection_seed)
            .expect("first selected seed"),
        later
            .iter()
            .find_map(|item| item.selection_seed)
            .expect("later selected seed")
    );
}

#[test]
fn selected_problem_variants_refuse_pool_draws_until_a_variant_model_exists() {
    let mut assignment = immutable_assignment_fixture();
    assignment.policies.variation = question_model::VariationPolicy::SelectedProblemVariants;

    assert!(matches!(
        select_assignment_run_items(&assignment, &selection_run(120, 121, 1)),
        Err(StoreError::InvalidRecord(message)) if message.contains("explicit pool-variant")
    ));
}

#[test]
fn current_attempt_points_apply_every_scoring_mode_and_attempt_exclusion() {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(200)),
        version: VersionId::from_uuid(id(201)),
    };
    let modes = [
        AssignmentScoringMode::Normal,
        AssignmentScoringMode::FullCredit,
        AssignmentScoringMode::ExtraCredit,
        AssignmentScoringMode::Excluded,
    ];
    let assignment = AssignmentRecord {
        id: AssignmentId::from_uuid(id(202)),
        course_id: CourseId::from_uuid(id(204)),
        title: "Scoring modes".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: modes
            .into_iter()
            .enumerate()
            .map(|(position, scoring_mode)| AssignmentItem {
                id: AssignmentItemId::from_uuid(id(210 + position as u128)),
                reference,
                position: u32::try_from(position).expect("fixture position"),
                points_possible: PointValue::from_whole(2),
                delivery_state: AssignmentDeliveryState::Active,
                scoring_mode,
            })
            .collect(),
        selection_groups: Vec::new(),
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    let result = |credit: f64| AttemptResult {
        correct: credit == 1.0,
        points_earned: credit,
        points_possible: 1.0,
    };

    assert_eq!(
        current_attempt_points(
            &assignment,
            assignment.items[0].id,
            AttemptStatus::Submitted,
            result(-0.5),
        ),
        Ok((-1.0, 2.0)),
        "normal scoring retains negative credit"
    );
    assert_eq!(
        current_attempt_points(
            &assignment,
            assignment.items[1].id,
            AttemptStatus::Submitted,
            result(-0.5),
        ),
        Ok((2.0, 2.0)),
        "full credit ignores the normalized result"
    );
    assert_eq!(
        current_attempt_points(
            &assignment,
            assignment.items[2].id,
            AttemptStatus::Submitted,
            result(1.25),
        ),
        Ok((2.5, 0.0)),
        "extra credit changes only the numerator"
    );
    assert_eq!(
        current_attempt_points(
            &assignment,
            assignment.items[3].id,
            AttemptStatus::Submitted,
            result(1.0),
        ),
        Ok((0.0, 0.0)),
        "excluded items change neither numerator nor denominator"
    );
    assert_eq!(
        current_attempt_points(
            &assignment,
            assignment.items[0].id,
            AttemptStatus::Cleared,
            result(1.0),
        ),
        Ok((0.0, 0.0)),
        "cleared attempts are absent from current scoring"
    );
    assert_eq!(
        current_attempt_points(
            &assignment,
            assignment.items[0].id,
            AttemptStatus::Submitted,
            result(4.000_000_000_000_3),
        ),
        Ok((8.0, 2.0)),
        "computed points are rounded before persistence"
    );
}

#[test]
fn completed_run_score_is_rounded_before_persistence() {
    let questions = vec![Some(CurrentRunQuestion {
        assignment_item: AssignmentItemId::from_uuid(id(250)),
        result: AttemptResult {
            correct: false,
            points_earned: 1.0,
            points_possible: 3.0,
        },
        earned_points: 1.0,
        possible_points: 3.0,
    })];

    assert_eq!(
        completed_run_score(&questions, question_model::CompletionRequirement::AnswerAll),
        Ok(Some(0.3333))
    );
}

#[test]
fn selected_group_items_complete_from_the_immutable_delivered_order() {
    let run = selection_run(301, 305, 1);
    let reference = |value| ProblemVersionRef {
        problem: ProblemId::from_uuid(id(310 + value)),
        version: VersionId::from_uuid(id(320 + value)),
    };
    let assignment = AssignmentRecord {
        id: AssignmentId::from_uuid(id(302)),
        course_id: CourseId::from_uuid(id(303)),
        title: "Selected completion".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: Vec::new(),
        selection_groups: vec![AssignmentSelectionGroup {
            id: question_model::AssignmentSelectionGroupId::from_uuid(id(304)),
            position: 0,
            draw_count: 2,
            points_per_item: PointValue::from_whole(2),
            ordering: SelectionOrdering::CandidateOrder,
            algorithm: question_model::PoolDrawAlgorithm::V1,
            candidates: (0..2)
                .map(|position| AssignmentSelectionCandidate {
                    id: AssignmentItemId::from_uuid(id(330 + u128::from(position))),
                    position,
                    reference: reference(u128::from(position)),
                    delivery_state: AssignmentDeliveryState::Active,
                })
                .collect(),
        }],
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    };
    let run_items = select_assignment_run_items(&assignment, &run).expect("selected run items");
    let attempts = run_items
        .iter()
        .enumerate()
        .map(|(index, item)| QuestionAttempt {
            id: QuestionAttemptId::from_uuid(id(340 + index as u128)),
            run: run.id,
            problem: item.reference.problem,
            question_version: item.reference.version,
            assignment_position: item.issued_position,
            seed: u64::try_from(index).expect("fixture seed"),
            parameter_hash: format!("selected-{index}"),
            response: Some(StudentResponse::Numeric { value: 1.0 }),
            status: AttemptStatus::Submitted,
            result: Some(AttemptResult {
                correct: true,
                points_earned: 1.0,
                points_possible: 1.0,
            }),
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(index as i64),
                deadline: None,
                submitted_at: Some(ActivityTimestamp::from_unix_millis(index as i64 + 1)),
            },
            provenance: AttemptProvenance {
                adapter: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: Vec::new(),
                grading: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: format!("selected-render-{index}"),
            },
            issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
        })
        .collect::<Vec<_>>();
    let questions = current_run_questions(
        &assignment,
        &run_items,
        &attempts,
        attempts.last().expect("selected current attempt"),
    )
    .expect("selected questions resolve");

    assert_eq!(questions.len(), 2);
    assert_eq!(
        completed_run_score(&questions, question_model::CompletionRequirement::AnswerAll),
        Ok(Some(1.0))
    );
    assert!(questions.iter().all(|question| {
        question.is_some_and(|question| {
            question.earned_points == 2.0 && question.possible_points == 2.0
        })
    }));
}

fn immutable_assignment_fixture() -> AssignmentRecord {
    let reference = |value: u128| ProblemVersionRef {
        problem: ProblemId::from_uuid(id(10 + value)),
        version: VersionId::from_uuid(id(20 + value)),
    };
    let item = |id_value, reference, position| AssignmentItem {
        id: AssignmentItemId::from_uuid(id(id_value)),
        reference,
        position,
        points_possible: PointValue::from_whole(1),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    };
    AssignmentRecord {
        id: AssignmentId::from_uuid(id(1)),
        course_id: CourseId::from_uuid(id(3)),
        title: "Immutable item fixture".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Draft,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: vec![item(30, reference(1), 0), item(31, reference(2), 1)],
        selection_groups: vec![AssignmentSelectionGroup {
            id: question_model::AssignmentSelectionGroupId::from_uuid(id(40)),
            position: 2,
            draw_count: 1,
            points_per_item: PointValue::from_whole(1),
            ordering: SelectionOrdering::CandidateOrder,
            algorithm: question_model::PoolDrawAlgorithm::V1,
            candidates: vec![AssignmentSelectionCandidate {
                id: AssignmentItemId::from_uuid(id(41)),
                position: 0,
                reference: reference(3),
                delivery_state: AssignmentDeliveryState::Active,
            }],
        }],
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: RunPolicies {
            completion: question_model::CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: question_model::ContinuedPractice::Unlimited,
            variation: question_model::VariationPolicy::NewSeeds,
        },
    }
}

fn ordinary_update(record: &AssignmentRecord) -> AssignmentUpdate {
    AssignmentUpdate {
        title: record.title.clone(),
        audience: record.audience.clone(),
        items: record.items.clone(),
        selection_groups: record.selection_groups.clone(),
        disclosure_policy: record.disclosure_policy,
        policies: record.policies,
    }
}

#[test]
fn ordinary_assignment_save_rejects_content_identity_changes() {
    let record = immutable_assignment_fixture();
    let mut changed_reference = ordinary_update(&record);
    changed_reference.items[0].reference.version = VersionId::from_uuid(id(99));
    let mut removed = ordinary_update(&record);
    removed.items.pop();
    let mut added = ordinary_update(&record);
    let mut fresh = added.items[0].clone();
    fresh.id = AssignmentItemId::from_uuid(id(98));
    added.items.push(fresh);
    let mut candidate_substitution = ordinary_update(&record);
    candidate_substitution.selection_groups[0].candidates[0]
        .reference
        .problem = ProblemId::from_uuid(id(97));

    for update in [changed_reference, removed, added, candidate_substitution] {
        assert!(matches!(
            ensure_assignment_update_preserves_references(&record, &update),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}

#[test]
fn ordinary_assignment_save_allows_reordering_and_authored_settings() {
    let record = immutable_assignment_fixture();
    let mut update = ordinary_update(&record);
    update.items.swap(0, 1);
    update.items[0].position = 0;
    update.items[1].position = 1;
    update.items[0].points_possible = PointValue::from_whole(4);
    update.items[1].scoring_mode = AssignmentScoringMode::ExtraCredit;
    update.selection_groups[0].position = 4;
    update.selection_groups[0].candidates[0].position = 3;

    assert_eq!(
        ensure_assignment_update_preserves_references(&record, &update),
        Ok(())
    );
}
