use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use learning_data_access::{
    AssignmentRecord, CourseRecord, IssueQuestionAttemptCommand, Store, TenantContext,
};
use question_model::response::{ChoiceId, StudentResponse};
use question_model::{
    AssignmentDeliveryState, AssignmentEnrollment, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentScoringMode, CompletionRequirement, ContinuedPractice, CourseId, CourseMembership,
    CourseMembershipRole, EnrollmentId, GradePolicy, PointValue, QuestionAttempt,
    QuestionAttemptId, RunId, RunPolicies, StudentId, UserId, VariationPolicy,
};

use super::*;

#[test]
fn replay_persistence_rekeys_durable_choices_to_rendered_ids() {
    let presentation =
        question_model::presentation::build_presentation_v1(&question_envelope(99), &[])
            .expect("presentation");
    let adapter = adapter_webwork::renderer_contract::WebworkReplayMappingV1::SingleChoice {
        controls: BTreeMap::from([
            (
                ChoiceId::new("salt"),
                adapter_webwork::renderer_contract::UpstreamControlV1 {
                    field: "AnSwEr0001".into(),
                    value: "0".into(),
                },
            ),
            (
                ChoiceId::new("water"),
                adapter_webwork::renderer_contract::UpstreamControlV1 {
                    field: "AnSwEr0001".into(),
                    value: "1".into(),
                },
            ),
        ]),
    };
    let stored = persist_replay_mapping(adapter.clone(), &presentation).expect("persistable");
    let json = serde_json::to_string(&stored).expect("stored replay JSON");
    assert!(!json.contains("water"));
    assert!(!json.contains("salt"));
    assert!(
        presentation
            .item_bindings
            .iter()
            .all(|binding| json.contains(binding.rendered.as_str()))
    );
    assert!(
        restore_replay_mapping(stored, &presentation).expect("restored adapter mapping") == adapter
    );
}

async fn persist_attempt(
    backend: &WebworkBackend<
        learning_data_access::in_memory::MemoryStore,
        MemoryObjectStore,
        RecordedRenderer,
    >,
    context: TenantContext,
    issued: &adapter_webwork::WebworkIssuedAttempt,
) -> (UserId, QuestionAttempt) {
    let tenant = context.tenant_id();
    let instructor = UserId::from_uuid(id(15));
    let actor = UserId::from_uuid(id(20));
    let course = CourseId::from_uuid(id(21));
    let assignment = AssignmentId::from_uuid(id(22));
    backend
        .sources
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Recorded WeBWorK course".into(),
                members: vec![
                    CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: actor,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .expect("course");
    backend
        .sources
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Recorded WeBWorK assignment".into(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(id(23)),
                    reference: reference(),
                    position: 0,
                    points_possible: PointValue::from_whole(1),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                policies: RunPolicies {
                    completion: CompletionRequirement::AllCorrect,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .expect("assignment");
    backend
        .sources
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(id(24)),
                tenant,
                assignment,
                user: actor,
                student: StudentId::from_uuid(id(25)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("enrollment");
    let run = backend
        .sources
        .start_or_resume_run(context, actor, assignment, RunId::from_uuid(id(26)))
        .await
        .expect("run");
    let presentation = question_model::presentation::build_presentation_v1(&issued.envelope, &[])
        .expect("presentation");
    let replay = persist_replay_mapping(
        issued.replay.clone().expect("private replay"),
        &presentation,
    )
    .expect("persisted replay");
    let attempt = backend
        .sources
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor,
                attempt: QuestionAttemptId::from_uuid(id(27)),
                run: run.id,
                assignment_position: 0,
                problem: reference().problem,
                question_version: reference().version,
                seed: issued.envelope.seed.value(),
                presentation: Some(question_model::PresentationBindingV1::new(
                    presentation.envelope.presentation_nonce,
                    presentation.digest,
                )),
                parameter_hash: issued.parameter_hash.clone(),
                provenance: issued.provenance.clone(),
                webwork_replay: Some(replay),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("attempt");
    (actor, attempt)
}

#[tokio::test]
async fn persisted_replay_grades_with_one_private_rpc_and_no_rerender() {
    let (backend, context, question, renders, grades, _unavailable) = fixture().await;
    let issued = backend
        .issue(context, reference(), &question, 99)
        .await
        .expect("issues with private replay");
    let (actor, attempt) = persist_attempt(&backend, context, &issued).await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new("water")],
    };

    assert!(matches!(
        backend
            .grade(
                context,
                UserId::from_uuid(id(28)),
                reference(),
                &question,
                &attempt,
                &response,
            )
            .await,
        Err(RunBackendError::Invalid(_))
    ));
    assert_eq!(grades.load(Ordering::SeqCst), 0);

    let outcome = backend
        .grade(context, actor, reference(), &question, &attempt, &response)
        .await
        .expect("persisted replay grades");
    assert!(matches!(
        outcome,
        grading::GradeOutcome::Graded(question_model::AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        })
    ));
    assert_eq!(renders.load(Ordering::SeqCst), 1, "grade uses safe cache");
    assert_eq!(grades.load(Ordering::SeqCst), 1, "one private grade RPC");
}

#[tokio::test]
async fn persisted_attempt_refuses_renderer_identity_drift_before_grade_rpc() {
    let (backend, context, question, _renders, _grades, _unavailable) = fixture().await;
    let issued = backend
        .issue(context, reference(), &question, 99)
        .await
        .expect("renderer A issues the attempt");
    let (actor, attempt) = persist_attempt(&backend, context, &issued).await;
    let before = backend
        .sources
        .get_question_attempt(context, attempt.id)
        .await
        .expect("attempt read")
        .expect("attempt exists");

    let drift_renders = Arc::new(AtomicUsize::new(0));
    let drift_grades = Arc::new(AtomicUsize::new(0));
    let drift_backend = WebworkBackend::new(
        Arc::clone(&backend.sources),
        Arc::clone(&backend.objects),
        Arc::new(WebworkAdapter::new(
            backend.objects.as_ref().clone(),
            RecordedRenderer {
                renders: Arc::clone(&drift_renders),
                grades: Arc::clone(&drift_grades),
                unavailable: Arc::new(AtomicBool::new(false)),
                identity: adapter_webwork::renderer_contract::RendererIdentity {
                    id: "recorded-opl".to_string(),
                    version: "2".to_string(),
                },
            },
        )),
    );
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new("water")],
    };

    assert!(matches!(
        drift_backend
            .grade(context, actor, reference(), &question, &attempt, &response)
            .await,
        Err(RunBackendError::Invalid(_))
    ));
    assert_eq!(drift_renders.load(Ordering::SeqCst), 0);
    assert_eq!(drift_grades.load(Ordering::SeqCst), 0);
    let after = backend
        .sources
        .get_question_attempt(context, attempt.id)
        .await
        .expect("attempt reread")
        .expect("attempt remains");
    assert_eq!(after, before, "identity drift leaves the attempt unchanged");
}
