use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use learning_data_access::{
    AssignmentRecord, CourseRecord, CourseRosterStore, CreateCourseCommand,
    IssueQuestionAttemptCommand, SessionLifetime, SessionSubject, Store, TenantContext,
    UpsertCourseMember,
};
use question_model::response::{ChoiceId, StudentResponse};
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    CompletionRequirement, ContinuedPractice, CourseId, GradePolicy, PointValue, QuestionAttempt,
    QuestionAttemptId, RunId, RunPolicies, UserId, UserRole, VariationPolicy,
};
use tower::ServiceExt;

use super::*;
use crate::composite_backend::CompositeBackend;
use crate::native_backend::NativeBackend;

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
    question: &question_model::QuestionDefinition,
    issued: &adapter_webwork::WebworkIssuedAttempt,
) -> (UserId, QuestionAttempt) {
    let tenant = context.tenant_id();
    let instructor = UserId::from_uuid(id(15));
    let actor = UserId::from_uuid(id(20));
    let course = CourseId::from_uuid(id(21));
    let assignment = AssignmentId::from_uuid(id(22));
    backend
        .sources
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Recorded WeBWorK course".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    backend
        .sources
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: actor,
                display_name: "Recorded learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    backend
        .sources
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                audience: question_model::AssignmentAudience::CourseWide,
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
                presentation_capability: learning_data_access::PresentationCapability::EnvelopeV1,
                presentation: Some(question_model::PresentationBindingV1::new(
                    presentation.envelope.presentation_nonce,
                    presentation.digest,
                )),
                presentation_snapshot: Some(learning_data_access::ReceiptPresentationSnapshot {
                    envelope: presentation.envelope.clone(),
                    asset_bindings: presentation.asset_bindings.clone(),
                }),
                grading_envelope: Some(issued.envelope.clone()),
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: Some(
                    learning_data_access::IssuedWebworkGradingContract::new(question.clone())
                        .expect("fixture WebWork definition is valid"),
                ),
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::Required,
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
    let (backend, context, question, renders, grades, unavailable) = fixture().await;
    let issued = backend
        .issue(context, reference(), &question, 99)
        .await
        .expect("issues with private replay");
    let (actor, attempt) = persist_attempt(&backend, context, &question, &issued).await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new("water")],
    };
    let grading_contract =
        learning_data_access::IssuedWebworkGradingContract::new(question.clone())
            .expect("fixture WebWork definition is valid");

    assert!(matches!(
        backend
            .grade(
                context,
                UserId::from_uuid(id(28)),
                reference(),
                &attempt,
                &grading_contract,
                &response,
            )
            .await,
        Err(RunBackendError::Invalid(_))
    ));
    assert_eq!(grades.load(Ordering::SeqCst), 0);

    unavailable.store(true, Ordering::SeqCst);

    let outcome = backend
        .grade(
            context,
            actor,
            reference(),
            &attempt,
            &grading_contract,
            &response,
        )
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
    assert_eq!(
        renders.load(Ordering::SeqCst),
        1,
        "only issuance renders; an unavailable renderer cannot block receipt-bound grading"
    );
    assert_eq!(grades.load(Ordering::SeqCst), 1, "one private grade RPC");
}

#[tokio::test]
async fn persisted_attempt_refuses_renderer_identity_drift_before_grade_rpc() {
    let (backend, context, question, _renders, _grades, _unavailable) = fixture().await;
    let issued = backend
        .issue(context, reference(), &question, 99)
        .await
        .expect("renderer A issues the attempt");
    let (actor, attempt) = persist_attempt(&backend, context, &question, &issued).await;
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
    let grading_contract =
        learning_data_access::IssuedWebworkGradingContract::new(question.clone())
            .expect("fixture WebWork definition is valid");

    assert!(matches!(
        drift_backend
            .grade(
                context,
                actor,
                reference(),
                &attempt,
                &grading_contract,
                &response,
            )
            .await,
        Err(RunBackendError::Unavailable(_))
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

#[tokio::test]
async fn http_submit_translates_rendered_webwork_choice_without_rerendering() {
    let (webwork, context, question, renders, grades, unavailable) = fixture().await;
    let issued = webwork
        .issue(context, reference(), &question, 99)
        .await
        .expect("issue a stored WebWork attempt");
    let (actor, attempt) = persist_attempt(&webwork, context, &question, &issued).await;
    let store = Arc::clone(&webwork.sources);
    let session = crate::auth::issue_session(
        store.as_ref(),
        SessionSubject::new(
            context.tenant_id(),
            actor,
            "Student",
            vec![UserRole::Student],
        )
        .expect("student session subject"),
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("student session");
    let cookie = session
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string();
    let backend = Arc::new(CompositeBackend::new(
        NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&store),
        ),
        webwork,
    ));
    let app = crate::run::router(Arc::clone(&store), backend);

    let issued_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/attempts/{}/question", attempt.id))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("issued question request"),
        )
        .await
        .expect("issued question response");
    assert_eq!(issued_response.status(), StatusCode::OK);
    let issued_body = to_bytes(issued_response.into_body(), 256 * 1024)
        .await
        .expect("issued question body");
    let issued_json: serde_json::Value = serde_json::from_slice(&issued_body).expect("issued JSON");
    let rendered_choice = issued_json["response"]["choices"][0]["id"]
        .as_str()
        .expect("rendered choice ID")
        .to_string();
    assert_ne!(
        rendered_choice, "water",
        "the browser never receives the durable ID"
    );

    unavailable.store(true, Ordering::SeqCst);
    let body = serde_json::json!({
        "response": { "kind": "multipleChoice", "selected": [rendered_choice] },
    });
    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{}", attempt.id))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "webwork-public-choice")
                .body(Body::from(body.to_string()))
                .expect("first submission request"),
        )
        .await
        .expect("first submission response");
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_body = to_bytes(first_response.into_body(), 256 * 1024)
        .await
        .expect("first receipt body");
    let first_json: serde_json::Value = serde_json::from_slice(&first_body).expect("first receipt");
    assert_eq!(first_json["feedback"]["correctness"], true);
    assert!(
        first_json["attempt"]["result"].is_null(),
        "immediate-correctness receipts do not expose points through the legacy attempt field"
    );
    assert_eq!(renders.load(Ordering::SeqCst), 1, "grade does not rerender");
    assert_eq!(grades.load(Ordering::SeqCst), 1, "one private grade RPC");

    let replay_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/submissions/{}", attempt.id))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "webwork-public-choice")
                .body(Body::from(body.to_string()))
                .expect("replay submission request"),
        )
        .await
        .expect("replay submission response");
    assert_eq!(replay_response.status(), StatusCode::OK);
    let replay_body = to_bytes(replay_response.into_body(), 256 * 1024)
        .await
        .expect("replay receipt body");
    let replay_json: serde_json::Value =
        serde_json::from_slice(&replay_body).expect("replay receipt");
    assert_eq!(
        replay_json, first_json,
        "replay returns the durable receipt"
    );
    assert_eq!(
        renders.load(Ordering::SeqCst),
        1,
        "replay does not rerender"
    );
    assert_eq!(
        grades.load(Ordering::SeqCst),
        1,
        "replay does not grade again"
    );
}
