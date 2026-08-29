use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use learning_data_access::{
    AssignmentRecord, CourseRecord, CourseRosterStore, CreateCourseCommand,
    IssueQuestionAttemptCommand, IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1,
    SealedPrivateExecutionStore, SessionLifetime, SessionSubject, Store, StudentWorkRoutingBinding,
    SubmissionIdempotencyKey, SubmissionPreparation, TenantContext, UpsertCourseMember,
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
) -> (UserId, StudentWorkRoutingBinding, QuestionAttempt) {
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
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    backend.sources.as_ref(),
                    tenant,
                    course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("course");
    backend
        .sources
        .upsert_course_member(
            context,
            instructor,
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
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Recorded WeBWorK assignment".into(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(id(23)),
                        reference: reference(),
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: AssignmentDeliveryState::Active,
                        scoring_mode: AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::StudentDisclosurePolicy::default(),
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("assignment");
    crate::course::tests::fixtures::publish_assignment(
        backend.sources.as_ref(),
        context,
        instructor,
        course,
        assignment,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    let run = backend
        .sources
        .start_or_resume_run(
            context,
            actor,
            learning_data_access::StudentWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(id(26)),
        )
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
                binding: learning_data_access::StudentWorkRoutingBinding::new(course, assignment),
                assignment_position: 0,
                problem: reference().problem,
                question_version: reference().version,
                issued_question_snapshot: IssuedQuestionSnapshotV1::new(
                    question.clone(),
                    IssuedQuestionFamilyWitnessV1::Webwork {},
                )
                .expect("fixture WebWork snapshot is valid"),
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
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: Some(
                    learning_data_access::IssuedWebworkGradingContract::new(question.clone())
                        .expect("fixture WebWork definition is valid"),
                ),
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::Required,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: issued.parameter_hash.clone(),
                provenance: issued.provenance.clone(),
                webwork_replay: Some(replay),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("attempt");
    (
        actor,
        StudentWorkRoutingBinding::new(course, assignment),
        attempt,
    )
}

async fn prepare_grade(
    backend: &WebworkBackend<
        learning_data_access::in_memory::MemoryStore,
        MemoryObjectStore,
        RecordedRenderer,
    >,
    context: TenantContext,
    actor: UserId,
    binding: StudentWorkRoutingBinding,
    attempt: QuestionAttemptId,
    response: &StudentResponse,
    key: &str,
) -> learning_data_access::PreparedQuestionSubmission {
    let key = SubmissionIdempotencyKey::parse(key).expect("fixture key");
    let intent = match backend
        .sources
        .prepare_question_submission(context, actor, binding, attempt, response, &key)
        .await
        .expect("bound submission preparation")
    {
        SubmissionPreparation::FirstEffect(intent) => *intent,
        SubmissionPreparation::Replay(_) => panic!("fresh fixture cannot replay"),
        SubmissionPreparation::AcceptedPending(_) => {
            panic!("fresh fixture cannot already be accepted")
        }
    };
    match backend
        .sources
        .sealed_private_execution_store()
        .prepare_sealed_private_execution(context, actor, binding, intent, response, &key)
        .await
        .expect("sealed private submission preparation")
    {
        learning_data_access::SealedPrivateExecutionPreparation::Grade(prepared) => *prepared,
        learning_data_access::SealedPrivateExecutionPreparation::Replay(_) => {
            panic!("fresh fixture cannot replay")
        }
    }
}

#[tokio::test]
async fn persisted_replay_grades_with_one_private_rpc_and_no_rerender() {
    let (backend, context, question, renders, grades, unavailable) = fixture().await;
    let issued = backend
        .issue(context, reference(), &question, 99)
        .await
        .expect("issues with private replay");
    let (actor, binding, attempt) = persist_attempt(&backend, context, &question, &issued).await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new("water")],
    };
    assert!(matches!(
        backend
            .sources
            .prepare_question_submission(
                context,
                UserId::from_uuid(id(28)),
                binding,
                attempt.id,
                &response,
                &SubmissionIdempotencyKey::parse("foreign-actor").expect("fixture key"),
            )
            .await,
        Err(learning_data_access::StoreError::NotFound)
    ));
    assert_eq!(grades.load(Ordering::SeqCst), 0);

    let prepared = prepare_grade(
        &backend,
        context,
        actor,
        binding,
        attempt.id,
        &response,
        "persisted-grade",
    )
    .await;

    unavailable.store(true, Ordering::SeqCst);

    let outcome = backend
        .grade(
            context,
            actor,
            reference(),
            &prepared.attempt,
            prepared
                .webwork_grading
                .as_ref()
                .expect("prepared WebWork grading contract"),
            prepared
                .presentation_binding
                .expect("prepared presentation binding"),
            prepared
                .webwork_replay
                .as_ref()
                .expect("prepared WebWork replay"),
            prepared
                .presentation
                .as_ref()
                .expect("prepared presentation snapshot"),
            prepared
                .grading_envelope
                .as_ref()
                .expect("prepared grading envelope"),
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
    let (actor, binding, attempt) = persist_attempt(&backend, context, &question, &issued).await;
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
    let prepared = prepare_grade(
        &backend,
        context,
        actor,
        binding,
        attempt.id,
        &response,
        "renderer-drift",
    )
    .await;

    assert!(matches!(
        drift_backend
            .grade(
                context,
                actor,
                reference(),
                &prepared.attempt,
                prepared
                    .webwork_grading
                    .as_ref()
                    .expect("prepared WebWork grading contract"),
                prepared
                    .presentation_binding
                    .expect("prepared presentation binding"),
                prepared
                    .webwork_replay
                    .as_ref()
                    .expect("prepared WebWork replay"),
                prepared
                    .presentation
                    .as_ref()
                    .expect("prepared presentation snapshot"),
                prepared
                    .grading_envelope
                    .as_ref()
                    .expect("prepared grading envelope"),
                &response,
            )
            .await,
        Err(RunBackendError::Deterministic(
            DeterministicGraderFailure::IssuedEvidenceIntegrity
        ))
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
async fn persisted_attempt_refuses_duplicate_replay_mapping_before_grade_rpc() {
    let (backend, context, question, renders, grades, _unavailable) = fixture().await;
    let issued = backend
        .issue(context, reference(), &question, 99)
        .await
        .expect("renderer issues the attempt");
    let (actor, binding, attempt) = persist_attempt(&backend, context, &question, &issued).await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new("water")],
    };
    let mut prepared = prepare_grade(
        &backend,
        context,
        actor,
        binding,
        attempt.id,
        &response,
        "duplicate-replay-map",
    )
    .await;
    let Some(learning_data_access::WebworkGradeReplayStateV1 {
        mapping: learning_data_access::WebworkReplayMappingV1::SingleChoice { items },
        ..
    }) = prepared.webwork_replay.as_mut()
    else {
        panic!("fixture has a single-choice replay mapping")
    };
    items.push(items.first().expect("fixture has one control").clone());

    assert!(matches!(
        backend
            .grade(
                context,
                actor,
                reference(),
                &prepared.attempt,
                prepared
                    .webwork_grading
                    .as_ref()
                    .expect("prepared WebWork grading contract"),
                prepared
                    .presentation_binding
                    .expect("prepared presentation binding"),
                prepared
                    .webwork_replay
                    .as_ref()
                    .expect("prepared WebWork replay"),
                prepared
                    .presentation
                    .as_ref()
                    .expect("prepared presentation snapshot"),
                prepared
                    .grading_envelope
                    .as_ref()
                    .expect("prepared grading envelope"),
                &response,
            )
            .await,
        Err(RunBackendError::Deterministic(
            DeterministicGraderFailure::IssuedEvidenceIntegrity
        ))
    ));
    assert_eq!(renders.load(Ordering::SeqCst), 1, "grade does not rerender");
    assert_eq!(
        grades.load(Ordering::SeqCst),
        0,
        "corrupt replay never grades"
    );
}

#[tokio::test]
async fn http_submit_translates_rendered_webwork_choice_without_rerendering() {
    let (webwork, context, question, renders, grades, unavailable) = fixture().await;
    let issued = webwork
        .issue(context, reference(), &question, 99)
        .await
        .expect("issue a stored WebWork attempt");
    let (actor, binding, attempt) = persist_attempt(&webwork, context, &question, &issued).await;
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
            crate::auth::CookieTransport::FirstPartyHttps,
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
    let app = crate::run::router(
        Arc::clone(&store),
        Arc::clone(&backend),
        Arc::new(
            learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(Arc::clone(
                &store,
            )),
        ),
        Arc::clone(&store) as Arc<dyn learning_data_access::StudentSubmissionStatusStore>,
        Arc::clone(&store) as Arc<dyn learning_data_access::AutomatedGradingStore>,
    );

    let issued_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{}/assignments/{}/attempts/{}/question",
                    binding.course, binding.assignment, attempt.id
                ))
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
                .uri(format!(
                    "/api/courses/{}/assignments/{}/attempts/{}/submissions",
                    CourseId::from_uuid(id(21)),
                    AssignmentId::from_uuid(id(22)),
                    attempt.id
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .header("idempotency-key", "webwork-public-choice")
                .body(Body::from(body.to_string()))
                .expect("first submission request"),
        )
        .await
        .expect("first submission response");
    assert_eq!(first_response.status(), StatusCode::ACCEPTED);
    let pending_body = to_bytes(first_response.into_body(), 256 * 1024)
        .await
        .expect("accepted response body");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&pending_body).expect("accepted response"),
        serde_json::json!({
            "kind": "accepted_pending",
            "accepted": true,
            "attemptId": attempt.id,
            "automatedGradingStatus": "pending",
            "nextAction": "check_status",
        })
    );
    assert_eq!(grades.load(Ordering::SeqCst), 0);
    crate::test_fixtures::drain_one_accepted_submission(&store, Arc::clone(&backend)).await;
    let completed_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{}/assignments/{}/attempts/{}/submission-status",
                    CourseId::from_uuid(id(21)),
                    AssignmentId::from_uuid(id(22)),
                    attempt.id
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("completed status request"),
        )
        .await
        .expect("completed status response");
    assert_eq!(completed_response.status(), StatusCode::OK);
    let first_body = to_bytes(completed_response.into_body(), 256 * 1024)
        .await
        .expect("completed receipt body");
    let first_json: serde_json::Value = serde_json::from_slice(&first_body).expect("first receipt");
    assert_eq!(
        first_json["feedback"]["correctness"],
        serde_json::Value::Null,
        "correctness remains withheld until the score is current"
    );
    assert_eq!(
        first_json["attempt"]["result"],
        serde_json::Value::Null,
        "the score-bearing aggregate remains hidden until recalculation is current"
    );
    assert_eq!(first_json["scoringStatus"], "recalculating");
    assert_eq!(renders.load(Ordering::SeqCst), 1, "grade does not rerender");
    assert_eq!(grades.load(Ordering::SeqCst), 1, "one private grade RPC");

    let replay_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{}/assignments/{}/attempts/{}/submissions",
                    CourseId::from_uuid(id(21)),
                    AssignmentId::from_uuid(id(22)),
                    attempt.id
                ))
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
