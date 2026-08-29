//! Disposable production-path evidence for one Chapter 1 catalog question.
//!
//! This host-only seed adjunct drives issued and submitted assignment activity
//! so the Store owns every statistics receipt and aggregate update.

use super::*;
use grading::GradeOutcome;
use question_model::generation::Seed;
use question_model::presentation::build_presentation_v1;
use question_model::{QuestionEnvelope, ResponseDefinition, StudentResponse};

const STATISTICS_SEED_SLUG: &str = "chapter-one-statistics";
const STATISTICS_ASSIGNMENT_TITLE: &str = "Molecular Foundations: Charged Functional Groups";
const STATISTICS_LEARNERS: [(&str, &str); 5] = [
    ("statistics-learner-one", "Amina Okoye"),
    ("statistics-learner-two", "Diego Ramirez"),
    ("statistics-learner-three", "Keiko Tanaka"),
    ("statistics-learner-four", "Noah Williams"),
    ("statistics-learner-five", "Priya Shah"),
];

#[derive(Clone, Copy)]
struct StatisticsAssignment {
    course: CourseId,
    assignment: AssignmentId,
}

fn statistics_instructions(chapter_slug: &str) -> Result<&'static str> {
    match chapter_slug {
        "genetics-chapter-1" => Ok(
            "Use charged functional groups to connect molecular structure with the biochemical consequences of genetic variation.",
        ),
        "biochemistry-chapter-1" => Ok(
            "Use charged functional groups to explain how molecular structure supports protein function.",
        ),
        _ => bail!("Chapter 1 statistics evidence has an unrecognized teaching course"),
    }
}

/// Completes five distinct assigned runs for one published flat question in
/// the two ordinary Chapter 1 teaching courses. The ordinary submission
/// transition records anonymous evidence through the Store-owned path.
pub(super) async fn seed_chapter_one_statistics(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    arguments: &SeedArguments,
    fixture: ChapterOneStatisticsFixture,
    chapters: &[ChapterManifest],
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
    if chapters.len() != 2 {
        bail!("Chapter 1 statistics evidence requires exactly two teaching courses");
    }
    let mut assignments = Vec::with_capacity(chapters.len());
    for chapter in chapters {
        let assignment = AssignmentId::from_uuid(pilot_uuid(
            arguments.tenant,
            &chapter.slug,
            "statistics-assignment",
        ));
        ensure_webwork_pilot_assignment(
            store,
            context,
            arguments.instructor,
            AssignmentRecord {
                id: assignment,
                tenant: arguments.tenant,
                course_id: chapter.course_id,
                title: STATISTICS_ASSIGNMENT_TITLE.to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::try_new(
                    statistics_instructions(&chapter.slug)?.to_string(),
                )
                .expect("statistics seed instructions are valid"),
                audience: question_model::AssignmentAudience::CourseWide,
                disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(pilot_uuid(
                        arguments.tenant,
                        &chapter.slug,
                        "statistics-assignment-item",
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
        assignments.push(StatisticsAssignment {
            course: chapter.course_id,
            assignment,
        });
    }
    for (index, (learner_slug, display_name)) in STATISTICS_LEARNERS.iter().enumerate() {
        let target = assignments
            .get(index % assignments.len())
            .expect("two Chapter 1 statistics assignments exist");
        let learner = UserId::from_uuid(pilot_uuid(
            arguments.tenant,
            STATISTICS_SEED_SLUG,
            learner_slug,
        ));
        upsert_chapter_one_student(
            store,
            context,
            arguments.instructor,
            learner,
            target.course,
            target.assignment,
            display_name,
        )
        .await?;
        let run_id = RunId::from_uuid(pilot_uuid(
            arguments.tenant,
            STATISTICS_SEED_SLUG,
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
                learning_data_access::StudentWorkRoutingBinding::new(
                    target.course,
                    target.assignment,
                ),
                run_id,
            )
            .await
            .context("starting assigned Chapter 1 statistics run")?;
        let seed = u64::try_from(index + 1).expect("five statistics learners fit u64");
        let issued = NativeAdapter::new()
            .issue(&question.question, Seed::new(seed), &[])
            .context("issuing Chapter 1 statistics question")?;
        let response = indexed_choice_response(&issued.envelope, index)?;
        let evaluation = private
            .evaluate(&question.question, &response)
            .context("grading Chapter 1 statistics response with the private flat contract")?;
        let GradeOutcome::Graded(result) = evaluation.outcome else {
            bail!("Chapter 1 statistics flat question must produce a numeric grade");
        };
        let presentation = build_presentation_v1(&issued.envelope, &[])
            .context("building Chapter 1 statistics presentation")?;
        let attempt = store
            .issue_or_resume_question_attempt(
                context,
                IssueQuestionAttemptCommand {
                    actor: learner,
                    binding: learning_data_access::StudentWorkRoutingBinding::new(
                        target.course,
                        target.assignment,
                    ),
                    attempt: QuestionAttemptId::from_uuid(pilot_uuid(
                        arguments.tenant,
                        STATISTICS_SEED_SLUG,
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
                    binding: learning_data_access::StudentWorkRoutingBinding::new(
                        target.course,
                        target.assignment,
                    ),
                    attempt: attempt.id,
                    response,
                    result,
                    feedback: evaluation.feedback,
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

/// Selects one valid issued choice without consulting private answer material.
///
/// The learner index keeps the disposable evidence varied while the private
/// grading contract remains the sole source of the recorded outcome and
/// feedback.
fn indexed_choice_response(
    envelope: &QuestionEnvelope,
    response_index: usize,
) -> Result<StudentResponse> {
    let ResponseDefinition::MultipleChoice { choices, .. } = &envelope.response else {
        bail!("Chapter 1 statistics question must remain a multiple-choice question");
    };
    if choices.is_empty() {
        bail!("Chapter 1 statistics question has no choices");
    }
    let choice = choices
        .get(response_index % choices.len())
        .expect("a modulo index is within the issued choices");
    Ok(StudentResponse::MultipleChoice {
        selected: vec![choice.id.clone()],
    })
}
