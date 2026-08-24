use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderValue, Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssignmentRecord, CatalogStore,
    CourseRecord, CourseRosterStore, CreateCourseCommand, DraftRecord, FlatQuestionGradingPayload,
    FlatQuestionPublicationPromotion, FlatQuestionStore, IssuedQuestionFamilyWitnessV1,
    IssuedQuestionSnapshotV1, PublishDraftCommand, PublishedSourceArtifact, SessionLifetime,
    SessionSubject, Store, UpsertCourseMember, UpsertFlatQuestionCommand,
};
use objects::{ObjectKey, ObjectRecord, Sha256Digest};
use question_model::answer::SelectionCardinality;
use question_model::capability::Capability;
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssetId, AssignmentId, AssignmentItem, AssignmentItemId,
    BackendCapabilities, CourseId, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition,
    PointValue, ProblemId, QuestionAttemptId, QuestionBackend, QuestionMetadata, QuestionSource,
    RunId, TenantId, UserId, UserRole, VersionId, WorkspaceId,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::*;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn choice(id: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: id.to_string(),
        }],
    }
}

fn draft_question(workspace: WorkspaceId, image: AssetId) -> DraftQuestionDefinition {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec!["alanine".to_string(), "glycine".to_string()],
        },
    );
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
        },
        prompt: vec![
            ContentBlock::Text {
                markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                    .to_string(),
            },
            ContentBlock::Image {
                asset: AssetRef {
                    asset: image,
                    checksum: "bridge-fixture".to_string(),
                },
                description: "A peptide bond diagram.".to_string(),
            },
        ],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![choice("ester"), choice("amide"), choice("ether")],
            selection: SelectionCardinality::ExactlyOne,
        },
        attempt_policy: AttemptPolicy { max_attempts: None },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: adapter_native::peptide_bond_geometry::GENERATOR_ID.to_string(),
                version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION.to_string(),
            },
            parameters,
        },
        grading: GradingDefinition::AllOrNothing { points: 2.0 },
        metadata: QuestionMetadata {
            title: "Peptide bond geometry".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

fn asset_record(
    reference: ProblemVersionRef,
    asset: AssetId,
    object: question_model::ObjectId,
) -> AssetDeliveryRecord {
    let key = ObjectKey::RestrictedProblemAsset {
        problem: reference.problem,
        version: reference.version,
        asset,
        object,
    };
    AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(asset),
        object: ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(b"native bridge asset"),
            size_bytes: 19,
            media_type: "image/svg+xml".to_string(),
            category: objects::ObjectCategory::Asset,
            version: Some(reference.version),
            license: "CC BY 4.0".to_string(),
            provenance: "native bridge test".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        },
        intrinsic_width: None,
        intrinsic_height: None,
        scope: AssetDeliveryScope::Catalog { asset, reference },
        publication: learning_data_access::AssetPublication::Ready,
        pending_source: None,
    }
}

const FLAT_SOURCE: &str = r#"{
        "format":"pleFlatQuestion","version":2,
        "title":"Favorite color","prompt":"What is my favorite color?",
        "response":{"kind":"singleChoice","choices":[
            {"id":"red","text":"Red","feedback":"Red feedback."},
            {"id":"blue","text":"Blue","feedback":"Blue feedback."}
        ],"correctChoice":"blue"},
        "feedback":{"correct":"Correct feedback.","incorrect":"Incorrect feedback."},
        "points":10.0,"attemptPolicy":{"maxAttempts":null},
        "timingPolicy":{"kind":"untimed"},"license":{"kind":"cc0"},"language":"en-US"
    }"#;

fn flat_source_record(
    tenant: TenantId,
    workspace: WorkspaceId,
    object: question_model::ObjectId,
    bytes: &[u8],
) -> ObjectRecord {
    let key = ObjectKey::WorkspaceQuestionSource {
        tenant,
        workspace,
        object,
    };
    ObjectRecord {
        id: object,
        bucket: key.bucket(),
        key,
        sha256: Sha256Digest::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: adapter_native::flat_question::FLAT_QUESTION_MEDIA_TYPE.to_string(),
        category: objects::ObjectCategory::Source,
        version: None,
        license: "CC0-1.0".to_string(),
        provenance: "native backend flat fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1_000),
    }
}

/// Canonical in-memory Flat publication fixture shared by server tests.
/// It stages source/private grading material and publishes atomically.
pub(crate) async fn published_flat_fixture() -> (
    NativeBackend<MemoryStore>,
    Arc<MemoryStore>,
    TenantContext,
    ProblemVersionRef,
    QuestionDefinition,
    QuestionAttempt,
    ChoiceId,
    ChoiceId,
) {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let store = Arc::new(store);
    let tenant = TenantId::from_uuid(uuid(101));
    let context = TenantContext::from_authenticated_session(tenant);
    let owner = UserId::from_uuid(uuid(102));
    let workspace = WorkspaceId::from_uuid(uuid(103));
    let source = adapter_native::flat_question::FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes())
        .expect("fixture source parses");
    let source_bytes = source
        .canonical_bytes()
        .expect("fixture source canonicalizes");
    let compiled = source.compile(workspace).expect("fixture source compiles");
    let (draft_question, private) = compiled.into_parts();
    let draft = DraftRecord {
        tenant,
        question: draft_question.clone(),
        derived_from: None,
    };
    let staged_source = flat_source_record(
        tenant,
        workspace,
        question_model::ObjectId::from_uuid(uuid(104)),
        &source_bytes,
    );
    let staged = store
        .upsert_flat_question(
            context,
            owner,
            UpsertFlatQuestionCommand {
                expected_revision: None,
                draft: draft.clone(),
                source: staged_source.clone(),
                canonical_source_sha256: staged_source.sha256.to_string(),
                public_binding_sha256: private.public_binding_sha256().to_string(),
                grading: FlatQuestionGradingPayload::from_private(&private)
                    .expect("private material validates"),
            },
        )
        .await
        .expect("flat source stages");
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(105)),
        version: VersionId::from_uuid(uuid(106)),
    };
    let published_object = question_model::ObjectId::from_uuid(uuid(107));
    let key = ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object: published_object,
    };
    let artifact = PublishedSourceArtifact {
        reference,
        backend: QuestionBackend::Native,
        object: ObjectRecord {
            id: published_object,
            bucket: key.bucket(),
            key,
            sha256: staged_source.sha256,
            size_bytes: staged_source.size_bytes,
            media_type: staged_source.media_type.clone(),
            category: objects::ObjectCategory::Source,
            version: Some(reference.version),
            license: staged_source.license.clone(),
            provenance: "published native backend flat fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_001),
        },
    };
    let published_question = draft.question.clone();
    store
        .publish_draft(
            context,
            owner,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: staged.workspace_revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: adapter_native::flat_question::FLAT_SINGLE_CHOICE_V2_FAMILY.to_string(),
                },
                source_artifact: Some(artifact),
                qti_promotion: None,
                flat_question_promotion: Some(FlatQuestionPublicationPromotion {
                    source: staged,
                    import_origin: None,
                    published_question,
                    assets: Vec::new(),
                }),
                publisher: owner,
                scope: question_model::PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("flat source publishes");
    let question = QuestionDefinition::from_draft(
        draft_question,
        reference.problem,
        reference.version,
        QuestionSource::Native {
            family: adapter_native::flat_question::FLAT_SINGLE_CHOICE_V2_FAMILY.to_string(),
        },
    );
    let backend = NativeBackend::with_flat_grader(
        Arc::new(adapter_native::NativeAdapter::new()),
        Arc::clone(&store),
        Arc::new(grader),
    );
    let issued = backend
        .issue(context, reference, &question, 108)
        .await
        .expect("flat question issues without private material");
    let attempt = QuestionAttempt {
        id: QuestionAttemptId::from_uuid(uuid(109)),
        tenant,
        run: RunId::from_uuid(uuid(110)),
        problem: reference.problem,
        question_version: reference.version,
        assignment_position: 0,
        seed: 108,
        parameter_hash: issued.parameter_hash,
        response: None,
        status: question_model::AttemptStatus::InProgress,
        result: None,
        timer: question_model::AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(1_000),
            deadline: None,
            submitted_at: None,
        },
        provenance: issued.provenance,
        issued_capability: question_model::IssuedAttemptCapabilityV1::FlatPresentation,
    };
    (
        backend,
        store,
        context,
        reference,
        question,
        attempt,
        ChoiceId::new("blue"),
        ChoiceId::new("red"),
    )
}

async fn flat_run_fixture() -> (Router, String, CourseId, AssignmentId) {
    let (backend, store, context, reference, _question, _attempt, _correct, _incorrect) =
        published_flat_fixture().await;
    let tenant = context.tenant_id();
    let instructor = UserId::from_uuid(uuid(102));
    let student = UserId::from_uuid(uuid(120));
    let course = CourseId::from_uuid(uuid(121));
    let assignment = AssignmentId::from_uuid(uuid(122));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Retry semantics".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: crate::test_fixtures::sysadmin_course_creation_authority(
                    store.as_ref(),
                    tenant,
                    course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("retry fixture course saves");
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Retry learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("retry fixture student membership");
    store
        .create_assignment(
            context,
            learning_data_access::CreateAssignmentCommand {
                actor: instructor,
                assignment: AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Retry semantics".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    instructions: question_model::AssignmentInstructions::default(),
                    items: vec![AssignmentItem {
                        id: AssignmentItemId::from_uuid(uuid(123)),
                        reference,
                        position: 0,
                        points_possible: PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
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
        .expect("retry fixture assignment saves");
    crate::course::tests::fixtures::publish_assignment(
        store.as_ref(),
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
    let subject = SessionSubject::new(tenant, student, "Retry student", vec![UserRole::Student])
        .expect("retry fixture session subject");
    let issued = crate::auth::issue_session(
        store.as_ref(),
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("retry fixture session lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("retry fixture session issues");
    let cookie = issued
        .set_cookie
        .split(';')
        .next()
        .expect("retry fixture cookie pair")
        .to_string();
    (
        crate::run::router(
            Arc::clone(&store),
            Arc::new(backend),
            Arc::new(
                learning_data_access::in_memory::MemorySealedPrivateExecutionStore::new(
                    Arc::clone(&store),
                ),
            ),
        ),
        cookie,
        course,
        assignment,
    )
}

fn post_json(path: &str, cookie: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("cookie", cookie)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("retry fixture request")
}

fn submission_json(
    path: &str,
    cookie: &str,
    idempotency_key: &'static str,
    body: serde_json::Value,
) -> Request<Body> {
    let mut request = post_json(path, cookie, body);
    request
        .headers_mut()
        .insert("idempotency-key", HeaderValue::from_static(idempotency_key));
    request
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), 256 * 1_024)
        .await
        .expect("retry fixture response body");
    serde_json::from_slice(&bytes).expect("retry fixture response JSON")
}

async fn active_attempt_id(app: &Router, run: &str, cookie: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/runs/{run}/attempts"))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("retry fixture attempts request"),
        )
        .await
        .expect("retry fixture attempts response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "attempt list remains available"
    );
    response_json(response)
        .await
        .get("items")
        .and_then(serde_json::Value::as_array)
        .and_then(|attempts| {
            attempts.iter().find_map(|attempt| {
                attempt
                    .get("response")
                    .filter(|response| response.is_null())
                    .and_then(|_| attempt.get("id"))
                    .and_then(serde_json::Value::as_str)
            })
        })
        .map(str::to_string)
        .expect("an active retry attempt is issued")
}

async fn rendered_choice_id(
    app: &Router,
    course: CourseId,
    assignment: AssignmentId,
    attempt: &str,
    cookie: &str,
    label: &str,
) -> ChoiceId {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/question"
                ))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("rendered choice request"),
        )
        .await
        .expect("rendered choice response");
    assert_eq!(response.status(), StatusCode::OK);
    let envelope = response_json(response).await;
    let expected_body = serde_json::json!([{
        "kind": "text",
        "markdown": label,
    }]);
    let identifier = envelope
        .pointer("/response/choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| {
            choices.iter().find_map(|choice| {
                (choice.get("body") == Some(&expected_body))
                    .then(|| choice.get("id").and_then(serde_json::Value::as_str))
                    .flatten()
            })
        })
        .expect("visible choice has a rendered ID");
    ChoiceId::new(identifier)
}

#[tokio::test]
async fn native_bridge_reproduces_only_with_exact_memory_catalog_assets() {
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(uuid(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let problem = ProblemId::from_uuid(uuid(2));
    let version = VersionId::from_uuid(uuid(3));
    let workspace = WorkspaceId::from_uuid(uuid(4));
    let asset = AssetId::from_uuid(uuid(5));
    let object = question_model::ObjectId::from_uuid(uuid(6));
    let publisher = UserId::from_uuid(uuid(7));
    let draft = DraftRecord {
        tenant,
        question: draft_question(workspace, asset),
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft saves before publication");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: question_model::PublicationScope::Institution,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([
                    Capability::AlgorithmicGeneration,
                    Capability::ClientRendering,
                    Capability::ServerGrading,
                ]),
            },
        )
        .await
        .expect("native question publishes");
    let reference = ProblemVersionRef { problem, version };
    store
        .register_asset_delivery(context, asset_record(reference, asset, object))
        .await
        .expect("exact version asset registers");

    let backend = NativeBackend::new(Arc::new(adapter_native::NativeAdapter::new()), store);
    let published = QuestionDefinition::from_draft(
        draft_question(workspace, asset),
        problem,
        version,
        QuestionSource::Native {
            family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
        },
    );
    let issued = backend
        .issue(context, reference, &published, 37)
        .await
        .expect("native issue resolves only exact catalog assets");
    assert_eq!(issued.provenance.asset_objects, vec![object]);
    let attempt = QuestionAttempt {
        id: QuestionAttemptId::from_uuid(uuid(8)),
        tenant,
        run: RunId::from_uuid(uuid(9)),
        problem,
        question_version: version,
        assignment_position: 0,
        seed: 37,
        parameter_hash: issued.parameter_hash,
        response: None,
        status: question_model::AttemptStatus::InProgress,
        result: None,
        timer: question_model::AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(1_000),
            deadline: None,
            submitted_at: None,
        },
        provenance: issued.provenance,
        issued_capability: question_model::IssuedAttemptCapabilityV1::PresentationEnvelope,
    };
    let envelope = backend
        .reproduce(context, reference, &published, &attempt)
        .await
        .expect("stored native attempt reproduces through memory asset resolver");
    let body = serde_json::to_string(&envelope).expect("envelope serializes");
    assert!(!body.contains("{{residue}}"));
    assert!(!body.contains("correct"));
    assert!(body.contains("alanine") || body.contains("glycine"));
    let outcome = backend
        .grade(
            context,
            reference,
            &published,
            &attempt,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("amide")],
            },
        )
        .await
        .expect("native server grading remains behind the bridge");
    assert!(matches!(outcome, GradeOutcome::Graded(result) if result.correct));

    let wrong_reference = ProblemVersionRef {
        problem,
        version: VersionId::from_uuid(uuid(10)),
    };
    assert!(matches!(
        backend
            .reproduce(context, wrong_reference, &published, &attempt)
            .await,
        Err(RunBackendError::Invalid(_))
    ));

    let foreign_context = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(11)));
    assert!(matches!(
        backend
            .reproduce(foreign_context, reference, &published, &attempt)
            .await,
        Err(RunBackendError::Invalid(_))
    ));

    let mut tampered = attempt.clone();
    tampered.provenance.asset_objects = vec![question_model::ObjectId::from_uuid(uuid(12))];
    assert!(matches!(
        backend
            .reproduce(context, reference, &published, &tampered)
            .await,
        Err(RunBackendError::Invalid(_))
    ));
}

#[tokio::test]
async fn flat_question_grades_from_isolated_memory_grader_and_keeps_issue_answer_free() {
    let (backend, _store, context, reference, question, attempt, correct, incorrect) =
        published_flat_fixture().await;
    let envelope = backend
        .reproduce(context, reference, &question, &attempt)
        .await
        .expect("flat question reproduces through its public adapter path");
    let serialized = serde_json::to_string(&envelope).expect("public envelope serializes");
    assert!(!serialized.contains("correctChoice"));
    assert!(!serialized.contains("Correct feedback."));

    let right = backend
        .grade(
            context,
            reference,
            &question,
            &attempt,
            &StudentResponse::MultipleChoice {
                selected: vec![correct],
            },
        )
        .await
        .expect("private grader grades the correct choice");
    assert!(matches!(right, GradeOutcome::Graded(result) if result.correct));
    let wrong_response = StudentResponse::MultipleChoice {
        selected: vec![incorrect],
    };
    let wrong = backend
        .grade(context, reference, &question, &attempt, &wrong_response)
        .await
        .expect("private grader grades the incorrect choice");
    assert!(matches!(wrong, GradeOutcome::Graded(result) if !result.correct));

    let issued_flat_grading = backend
        .issue(context, reference, &question, attempt.seed)
        .await
        .expect("issue retains flat grading authority")
        .flat_grading
        .expect("flat family requires an issued private grading contract");

    let receipt_backend = NativeBackend::new(
        Arc::new(adapter_native::NativeAdapter::new()),
        Arc::clone(&_store),
    );
    let issued_snapshot =
        IssuedQuestionSnapshotV1::new(question.clone(), IssuedQuestionFamilyWitnessV1::Flat {})
            .expect("flat snapshot");
    let disposition = receipt_backend
        .submit(RunSubmission {
            context,
            actor: UserId::from_uuid(uuid(111)),
            idempotency_key: learning_data_access::SubmissionIdempotencyKey::parse("flat-test")
                .expect("fixture key is valid"),
            reference,
            issued_question_snapshot: &issued_snapshot,
            attempt: &attempt,
            issued_grading_envelope: Some(&envelope),
            issued_flat_grading: Some(&issued_flat_grading),
            issued_webwork_grading: None,
            issued_qti_grading: None,
            issued_webwork_replay: None,
            issued_presentation_binding: None,
            issued_presentation: None,
            response: &wrong_response,
        })
        .await
        .expect("issued contract grades without a current flat-question grader");
    let SubmissionDisposition::Grade(receipt) = disposition else {
        panic!("flat question should return a numerical receipt");
    };
    assert!(!receipt.result.correct);
    assert!(
        receipt.feedback.hint.is_some(),
        "trusted receipt keeps teaching feedback for the run policy projection"
    );
    assert!(receipt.feedback.correct_response.is_some());
}

#[tokio::test]
async fn flat_run_route_retries_wrong_first_source_choice_then_completes_correct_second_choice() {
    let (app, cookie, course, assignment) = flat_run_fixture().await;
    let start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/courses/{course}/assignments/{assignment}/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("start run request"),
        )
        .await
        .expect("run starts");
    assert_eq!(
        start.status(),
        StatusCode::CREATED,
        "run route starts assigned work"
    );
    let run = response_json(start).await;
    let run_id = run
        .get("id")
        .and_then(serde_json::Value::as_str)
        .expect("run has a public id")
        .to_string();
    let first_attempt = active_attempt_id(&app, &run_id, &cookie).await;
    let first_wrong =
        rendered_choice_id(&app, course, assignment, &first_attempt, &cookie, "Red").await;
    assert_ne!(first_wrong, ChoiceId::new("red"));

    let wrong = app
        .clone()
        .oneshot(submission_json(
            &format!(
                "/api/courses/{course}/assignments/{assignment}/attempts/{first_attempt}/submissions"
            ),
            &cookie,
            "flat-route-wrong-first",
            serde_json::json!({
                "response": StudentResponse::MultipleChoice {
                    selected: vec![first_wrong],
                }
            }),
        ))
        .await
        .expect("wrong first source choice submits");
    assert_eq!(
        wrong.status(),
        StatusCode::OK,
        "wrong source choice is accepted"
    );
    let wrong_receipt = response_json(wrong).await;
    assert_eq!(
        wrong_receipt
            .pointer("/attempt/result/correct")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "first source position remains incorrect"
    );
    let second_attempt = active_attempt_id(&app, &run_id, &cookie).await;
    assert_ne!(
        second_attempt, first_attempt,
        "retry receives a distinct attempt"
    );
    assert_eq!(
        wrong_receipt.pointer("/nextIssued/id"),
        Some(&serde_json::json!(second_attempt)),
        "wrong attempt receives a successor under unlimited AllCorrect policy"
    );
    let second_correct =
        rendered_choice_id(&app, course, assignment, &second_attempt, &cookie, "Blue").await;
    assert_ne!(second_correct, ChoiceId::new("blue"));

    let correct = app
        .clone()
        .oneshot(submission_json(
            &format!(
                "/api/courses/{course}/assignments/{assignment}/attempts/{second_attempt}/submissions"
            ),
            &cookie,
            "flat-route-correct-second",
            serde_json::json!({
                "response": StudentResponse::MultipleChoice {
                    selected: vec![second_correct],
                }
            }),
        ))
        .await
        .expect("correct second source choice submits");
    assert_eq!(
        correct.status(),
        StatusCode::OK,
        "correct source choice is accepted"
    );
    let correct_receipt = response_json(correct).await;
    assert_eq!(
        correct_receipt
            .pointer("/attempt/result/correct")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "second source position remains correct"
    );
    assert!(
        correct_receipt
            .get("nextIssued")
            .is_some_and(serde_json::Value::is_null),
        "completion does not issue a third assigned attempt"
    );
}

#[tokio::test]
async fn flat_question_without_injected_grader_or_with_foreign_tenant_fails_closed() {
    let (backend, _store, context, reference, question, attempt, _correct, incorrect) =
        published_flat_fixture().await;
    let response = StudentResponse::MultipleChoice {
        selected: vec![incorrect],
    };
    let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(112)));
    assert!(matches!(
        backend
            .grade(foreign, reference, &question, &attempt, &response)
            .await,
        Err(RunBackendError::Unavailable(_))
    ));

    let no_grader = NativeBackend::new(
        Arc::new(adapter_native::NativeAdapter::new()),
        Arc::new(MemoryStore::default()),
    );
    assert!(matches!(
        no_grader
            .grade(context, reference, &question, &attempt, &response)
            .await,
        Err(RunBackendError::Invalid(_) | RunBackendError::Unavailable(_))
    ));
}

#[derive(Clone)]
struct FixedFlatGrader(FlatQuestionGradingPayload);

#[async_trait]
impl learning_data_access::FlatQuestionGradingStore for FixedFlatGrader {
    async fn flat_question_published_grading(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
    ) -> Result<Option<FlatQuestionGradingPayload>, StoreError> {
        Ok(Some(self.0.clone()))
    }
}

#[tokio::test]
async fn flat_question_rejects_private_material_bound_to_another_public_model() {
    let (_backend, store, context, reference, question, attempt, _correct, incorrect) =
        published_flat_fixture().await;
    let mismatched_source = FLAT_SOURCE.replace("Favorite color", "Different favorite color");
    let document =
        adapter_native::flat_question::FlatQuestionDocument::parse(mismatched_source.as_bytes())
            .expect("fixture source parses");
    let compiled = document
        .compile(WorkspaceId::from_uuid(uuid(113)))
        .expect("fixture source compiles");
    let (_, private) = compiled.into_parts();
    let mismatched = FlatQuestionGradingPayload::from_private(&private)
        .expect("different private material validates");
    let backend = NativeBackend::with_flat_grader(
        Arc::new(adapter_native::NativeAdapter::new()),
        store,
        Arc::new(FixedFlatGrader(mismatched)),
    );
    assert!(matches!(
        backend
            .grade(
                context,
                reference,
                &question,
                &attempt,
                &StudentResponse::MultipleChoice {
                    selected: vec![incorrect],
                },
            )
            .await,
        Err(RunBackendError::Invalid(_))
    ));
}
