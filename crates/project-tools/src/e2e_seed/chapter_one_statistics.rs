//! Disposable production-path evidence for one Chapter 1 catalog question.
//!
//! This host-only seed adjunct drives issued and submitted assignment activity
//! so the Store owns every statistics receipt and aggregate update.

use super::*;
use question_model::generation::Seed;
use question_model::presentation::build_presentation_v1;
use question_model::{QuestionEnvelope, ResponseDefinition, StudentResponse};

const STATISTICS_COHORT_SLUG: &str = "chapter-one-statistics";
const STATISTICS_LEARNERS: [&str; 5] = [
    "statistics-learner-one",
    "statistics-learner-two",
    "statistics-learner-three",
    "statistics-learner-four",
    "statistics-learner-five",
];

/// Completes five distinct assigned runs for exactly one published flat
/// question. The ordinary submission transition records anonymous evidence;
/// every other Chapter 1 question remains suppressed in this corpus.
pub(super) async fn seed_chapter_one_statistics(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    arguments: &SeedArguments,
    fixture: ChapterOneStatisticsFixture,
) -> Result<()> {
    let question = store
        .get_catalog_problem(context, fixture.reference)
        .await
        .context("loading the published Chapter 1 statistics question")?
        .ok_or_else(|| anyhow::anyhow!("published Chapter 1 statistics question is missing"))?;
    let document = adapter_native::flat_question::FlatQuestionDocument::parse(fixture.source)
        .context("parsing the reviewed Chapter 1 statistics source")?;
    let (_compiled, private) = document
        .compile(question.question.workspace)
        .context("compiling the reviewed Chapter 1 statistics source")?
        .into_parts();
    let grading = FlatQuestionGradingPayload::from_private(&private)
        .context("encoding Chapter 1 statistics private grading")?;
    for (index, learner_slug) in STATISTICS_LEARNERS.iter().enumerate() {
        let course = CourseId::from_uuid(pilot_uuid(
            arguments.tenant,
            STATISTICS_COHORT_SLUG,
            &format!("{learner_slug}-course"),
        ));
        let assignment = AssignmentId::from_uuid(pilot_uuid(
            arguments.tenant,
            STATISTICS_COHORT_SLUG,
            &format!("{learner_slug}-assignment"),
        ));
        ensure_webwork_pilot_course(
            store,
            context,
            arguments.instructor,
            CourseRecord {
                id: course,
                tenant: arguments.tenant,
                title: format!("Chapter 1 discovery evidence cohort {}", index + 1),
                term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("explicit fixture course term"),
            },
        )
        .await?;
        ensure_webwork_pilot_assignment(
            store,
            context,
            arguments.instructor,
            AssignmentRecord {
                id: assignment,
                tenant: arguments.tenant,
                course_id: course,
                title: "Chapter 1 phenylalanine evidence activity".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::try_new(
                    "Compare the molecular evidence before you choose an answer.".to_string(),
                )
                .expect("statistics seed instructions are valid"),
                audience: question_model::AssignmentAudience::CourseWide,
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(pilot_uuid(
                        arguments.tenant,
                        STATISTICS_COHORT_SLUG,
                        &format!("{learner_slug}-assignment-item"),
                    )),
                    reference: fixture.reference,
                    position: 0,
                    points_possible: PointValue::from_whole(1),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await?;
        let learner = UserId::from_uuid(pilot_uuid(
            arguments.tenant,
            STATISTICS_COHORT_SLUG,
            learner_slug,
        ));
        upsert_chapter_one_student(
            store,
            context,
            arguments.instructor,
            learner,
            course,
            assignment,
        )
        .await?;
        let run_id = RunId::from_uuid(pilot_uuid(
            arguments.tenant,
            STATISTICS_COHORT_SLUG,
            &format!("{learner_slug}-run"),
        ));
        if store
            .get_run(context, run_id)
            .await
            .context("checking deterministic Chapter 1 statistics run")?
            .is_some_and(|run| run.completed_at.is_some())
        {
            continue;
        }
        let run = store
            .start_or_resume_run(
                context,
                learner,
                learning_data_access::LearnerWorkRoutingBinding::new(course, assignment),
                run_id,
            )
            .await
            .context("starting assigned Chapter 1 statistics run")?;
        let seed = u64::try_from(index + 1).expect("five statistics learners fit u64");
        let issued = NativeAdapter::new()
            .issue(&question.question, Seed::new(seed), &[])
            .context("issuing Chapter 1 statistics question")?;
        let presentation = build_presentation_v1(&issued.envelope, &[])
            .context("building Chapter 1 statistics presentation")?;
        let attempt = store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor: learner,
                    binding: learning_data_access::LearnerWorkRoutingBinding::new(
                        course, assignment,
                    ),
                    attempt: QuestionAttemptId::from_uuid(pilot_uuid(
                        arguments.tenant,
                        STATISTICS_COHORT_SLUG,
                        &format!("{learner_slug}-attempt"),
                    )),
                    run: run.id,
                    assignment_position: 0,
                    problem: fixture.reference.problem,
                    question_version: fixture.reference.version,
                    issued_question_snapshot: chapter_one_statistics_snapshot(
                        &question.question,
                        fixture.reference.problem,
                        fixture.reference.version,
                    )?,
                    seed,
                    presentation_capability: PresentationCapability::EnvelopeV1,
                    presentation: Some(PresentationBindingV1::new(
                        presentation.envelope.presentation_nonce,
                        presentation.digest,
                    )),
                    presentation_snapshot: Some(
                        learning_data_access::ReceiptPresentationSnapshot {
                            envelope: presentation.envelope,
                            asset_bindings: presentation.asset_bindings,
                        },
                    ),
                    grading_envelope: Some(issued.envelope.clone()),
                    native_execution_envelope_capability:
                        learning_data_access::NativeExecutionEnvelopeCapability::NotApplicable,
                    flat_grading: Some(
                        IssuedFlatGradingContract::new(question.question.clone(), grading.clone())
                            .context("binding Chapter 1 flat grading at issuance")?,
                    ),
                    flat_grading_capability: FlatGradingCapability::Required,
                    webwork_grading: None,
                    webwork_grading_capability:
                        learning_data_access::WebworkGradingCapability::NotApplicable,
                    webwork_replay: None,
                    qti_grading: None,
                    qti_grading_capability:
                        learning_data_access::QtiGradingCapability::NotApplicable,
                    parameter_hash: issued.parameter_hash,
                    provenance: issued.provenance,
                    prefetched: None,
                    predecessor_submission: None,
                },
            )
            .await
            .context("issuing assigned Chapter 1 statistics attempt")?;
        store
            .submit_question_attempt(
                context,
                SubmitQuestionAttemptCommand {
                    actor: learner,
                    binding: learning_data_access::LearnerWorkRoutingBinding::new(
                        course, assignment,
                    ),
                    attempt: attempt.id,
                    response: first_choice_response(&issued.envelope)?,
                    result: AttemptResult {
                        correct: false,
                        points_earned: 0.0,
                        points_possible: 1.0,
                    },
                    feedback: FeedbackContent::default(),
                    idempotency_key: SubmissionIdempotencyKey::parse(format!(
                        "chapter-one-statistics-{index}"
                    ))?,
                },
            )
            .await
            .context("completing assigned Chapter 1 statistics run")?;
    }
    Ok(())
}

fn chapter_one_statistics_snapshot(
    question: &question_model::QuestionDefinition,
    problem: question_model::ProblemId,
    version: question_model::VersionId,
) -> Result<learning_data_access::IssuedQuestionSnapshotV1> {
    let snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        question.clone(),
        learning_data_access::IssuedQuestionFamilyWitnessV1::Flat {},
    )
    .context("building Chapter 1 statistics issued question snapshot")?;
    snapshot
        .validate_for_attempt(problem, version)
        .context("validating Chapter 1 statistics issued question identity")?;
    snapshot
        .validate_for_issuance_context(
            learning_data_access::FlatGradingCapability::Required,
            learning_data_access::WebworkGradingCapability::NotApplicable,
            learning_data_access::QtiGradingCapability::NotApplicable,
            None,
        )
        .context("validating Chapter 1 statistics issued question authority")?;
    Ok(snapshot)
}

fn first_choice_response(envelope: &QuestionEnvelope) -> Result<StudentResponse> {
    let ResponseDefinition::MultipleChoice { choices, .. } = &envelope.response else {
        bail!("Chapter 1 statistics question must remain a multiple-choice question");
    };
    let choice = choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("Chapter 1 statistics question has no choices"))?;
    Ok(StudentResponse::MultipleChoice {
        selected: vec![choice.id.clone()],
    })
}
