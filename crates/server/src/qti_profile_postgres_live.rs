//! Disposable full-path PostgreSQL oracle for recognized QTI authoring.
//!
//! This stays ignored in ordinary Cargo runs. The maintained database baseline
//! supplies both short-lived credentials and owns the only invocation.

use std::io::{Cursor, Write};
use std::sync::Arc;

use adapter_native::flat_question::FlatQuestionDocument;
use axum::body::Body;
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, Request, StatusCode};
use grading::GradeOutcome;
use learning_data_access::postgres::{
    PostgresGraderStore, PostgresStore, lazy_pool, verify_application_schema,
};
use learning_data_access::{
    AuthoritativeTimeStore, CatalogSourceStore, CatalogStore, DraftRecord,
    FlatImportProvenanceStore, FlatQuestionGradingStore, FlatQuestionStore, JobClaimFilter,
    JobKind, JobLeaseDuration, JobPayload, JobStore, QTI_PROFILE_ARCHIVE_MEDIA_TYPE,
    QtiImportApiStore, QtiImportRef, SessionLifetime, SessionSubject, Store, TenantContext,
};
use objects::memory::MemoryObjectStore;
use objects::{
    ObjectKey, ObjectStore, ObjectStoreError, Sha256Digest, published_import_archive_object_id,
    workspace_qti_archive_object_id,
};
use question_model::response::{ChoiceId, StudentResponse};
use question_model::{
    ActivityTimestamp, AttemptStatus, AttemptTimerRecord, CatalogProblemSummary, ProblemDisplayRef,
    ProblemVersionRef, QuestionAttempt, QuestionAttemptId, RunId, TenantId, UserId, UserRole,
    WorkspaceId, WorkspaceImportId,
};
use serde_json::Value;
use tower::ServiceExt;

use crate::auth::{CookieTransport, SessionConfig, issue_session};
use crate::catalog::ReviewNotRequired;
use crate::native_backend::NativeBackend;
use crate::qti_import::{QtiImportCommitter, QtiImportHandler};
use crate::run::RunBackend;
use crate::worker::{
    EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
};

const CANVAS_MANIFEST: &str =
    include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_positive_manifest.xml");
const CANVAS_META: &str =
    include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_assessment_meta.xml");
const CANVAS_ITEM: &str =
    include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_positive_item.xml");
const EXISTING_SOURCE: &str = r#"{
  "format":"pleFlatQuestion",
  "version":2,
  "title":"Existing author draft",
  "prompt":"Which draft existed before the import?",
  "response":{"kind":"singleChoice","choices":[{"id":"first","text":"First"},{"id":"second","text":"Second"}],"correctChoice":"first"},
  "feedback":{},
  "points":1.0,
  "attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},
  "timingPolicy":{"kind":"untimed"},
  "license":{"kind":"allRightsReserved"},
  "language":"en-US"
}"#;

fn random_uuid() -> uuid::Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    uuid::Uuid::from_bytes(bytes)
}

fn mixed_canvas_archive() -> Vec<u8> {
    let start = CANVAS_ITEM.find("      <item ").expect("item start");
    let end = CANVAS_ITEM[start..]
        .find("      </item>")
        .map(|offset| start + offset + "      </item>".len())
        .expect("item end");
    let rejected = CANVAS_ITEM[start..end]
        .replace("canvas-1", "canvas-2")
        .replacen("rcardinality=\"Single\"", "rcardinality=\"Multiple\"", 1);
    let item = CANVAS_ITEM.replacen("    </section>", &format!("{rejected}\n    </section>"), 1);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (path, contents) in [
        ("imsmanifest.xml", CANVAS_MANIFEST),
        ("canvas_qti12_questions/assessment_meta.xml", CANVAS_META),
        ("canvas_qti12_questions/canvas-1.xml", item.as_str()),
    ] {
        zip.start_file(path, options).expect("fixture ZIP entry");
        zip.write_all(contents.as_bytes())
            .expect("fixture ZIP bytes");
    }
    zip.finish().expect("fixture ZIP closes").into_inner()
}

async fn issued_cookie(store: &PostgresStore, tenant: TenantId, user: UserId) -> String {
    let issued = issue_session(
        store,
        SessionSubject::new(
            tenant,
            user,
            "QTI profile live fixture",
            vec![UserRole::Instructor],
        )
        .expect("fixture session subject"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
            CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("fixture session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn response_parts(response: axum::response::Response) -> (StatusCode, HeaderMap, Vec<u8>) {
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .expect("bounded live response")
        .to_vec();
    (status, headers, body)
}

fn assert_no_store(headers: &HeaderMap) {
    assert_eq!(
        headers
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
}

fn assert_safe_report(body: &[u8]) {
    let text = String::from_utf8_lossy(body);
    for forbidden in [
        "correctChoice",
        "<questestinterop",
        "imsmanifest.xml",
        "privateMapping",
        "choiceMap",
        "profileReportSha256",
        "sourceArchive",
        "WorkspaceSource",
        "payloadBase64",
        "blue",
        "red",
    ] {
        assert!(
            !text.contains(forbidden),
            "safe profile report leaked forbidden material {forbidden}: {text}"
        );
    }
}

fn assert_answer_free_conversion(body: &[u8]) {
    let text = String::from_utf8_lossy(body);
    for forbidden in [
        "correctChoice",
        "privateMapping",
        "choiceMap",
        "profileReportSha256",
        "sourceArchive",
        "WorkspaceSource",
        "payloadBase64",
    ] {
        assert!(
            !text.contains(forbidden),
            "converted draft leaked forbidden material {forbidden}: {text}"
        );
    }
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_profile_upload_worker_conversion_publication_and_grading_are_complete() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let grader_url = std::env::var("PLE_TEST_GRADER_DATABASE_URL")
        .expect("PLE_TEST_GRADER_DATABASE_URL must name the disposable grader connection");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = Arc::new(PostgresStore::with_question_id_secret(pool, [0x42; 32]));
    let grader = Arc::new(
        PostgresGraderStore::connect_local_development(&grader_url)
            .await
            .expect("dedicated grader credentials are accepted"),
    );
    let objects = Arc::new(MemoryObjectStore::default());

    let tenant = TenantId::from_uuid(random_uuid());
    let foreign_tenant = TenantId::from_uuid(random_uuid());
    let owner = UserId::from_uuid(random_uuid());
    let workspace = WorkspaceId::from_uuid(random_uuid());
    let import = WorkspaceImportId::from_uuid(random_uuid());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let owner_cookie = issued_cookie(&store, tenant, owner).await;
    let foreign_cookie = issued_cookie(&store, foreign_tenant, owner).await;

    let existing_question = FlatQuestionDocument::parse(EXISTING_SOURCE.as_bytes())
        .expect("existing source parses")
        .compile(workspace)
        .expect("existing source compiles")
        .into_parts()
        .0;
    let initial = store
        .upsert_draft(
            context,
            owner,
            None,
            DraftRecord {
                tenant,
                question: existing_question,
                derived_from: None,
            },
        )
        .await
        .expect("existing workspace is created before upload");
    let initial_etag = format!("\"{}\"", initial.revision.value());
    let archive = mixed_canvas_archive();
    let import_uri = format!("/api/workspaces/{workspace}/qti-imports/{import}");
    let import_app = crate::qti_profile_import::router(Arc::clone(&store), Arc::clone(&objects));

    for attempt in 0..2 {
        let response = import_app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(&import_uri)
                    .header("cookie", &owner_cookie)
                    .header("content-type", QTI_PROFILE_ARCHIVE_MEDIA_TYPE)
                    .body(Body::from(archive.clone()))
                    .expect("upload request"),
            )
            .await
            .expect("upload response");
        let (status, headers, body) = response_parts(response).await;
        assert_eq!(
            status,
            StatusCode::ACCEPTED,
            "upload attempt {attempt}: {}",
            String::from_utf8_lossy(&body)
        );
        assert_no_store(&headers);
        let receipt: Value = serde_json::from_slice(&body).expect("upload receipt JSON");
        assert_eq!(receipt["importId"], import.to_string());
        assert_eq!(receipt["state"], "queued");
    }

    let source_object = workspace_qti_archive_object_id(tenant, workspace, import);
    let payload = JobPayload::QtiImport {
        workspace,
        import,
        source_object,
    };
    let qti_filter = JobClaimFilter::new([JobKind::QtiImport]).expect("QTI-only worker filter");
    let claim = store
        .claim_next_job(
            &qti_filter,
            JobLeaseDuration::from_seconds(120).expect("worker lease"),
        )
        .await
        .expect("QTI claim succeeds")
        .expect("exact upload queued one QTI job");
    assert_eq!(claim.tenant, tenant);
    assert_eq!(claim.payload, payload);
    let effect = QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects))
        .prepare(context, payload, JobExecution::new())
        .await
        .expect("real profile worker prepares the archive");
    assert_eq!(
        QtiImportCommitter::new(Arc::clone(&store))
            .commit(JobCommitClaim::new(claim.id, claim.lease_token), effect)
            .await
            .expect("real profile worker commits the archive"),
        EffectCommitOutcome::Committed
    );
    assert!(
        store
            .claim_next_job(
                &qti_filter,
                JobLeaseDuration::from_seconds(120).expect("worker lease"),
            )
            .await
            .expect("post-commit QTI claim succeeds")
            .is_none(),
        "exact upload replay must not enqueue another QTI job"
    );

    let ready_response = import_app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&import_uri)
                .header("cookie", &owner_cookie)
                .body(Body::empty())
                .expect("ready report request"),
        )
        .await
        .expect("ready report response");
    let (status, headers, ready_body) = response_parts(ready_response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&ready_body)
    );
    assert_no_store(&headers);
    assert_safe_report(&ready_body);
    let report: Value = serde_json::from_slice(&ready_body).expect("ready report JSON");
    assert_eq!(report["state"], "ready");
    assert_eq!(
        report["profileId"],
        "canvas-qti-1.2-static-single-choice/v1"
    );
    let items = report["items"].as_array().expect("ordered item report");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["sourceIdentifier"], "canvas-1");
    assert_eq!(items[0]["status"], "accepted");
    assert_eq!(items[1]["sourceIdentifier"], "canvas-2");
    assert_eq!(items[1]["status"], "rejected");
    assert!(
        items[1]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty())
    );
    let report_revision = report["reportRevision"]
        .as_str()
        .expect("opaque report revision");
    let review_token = report["reviewToken"].as_str().expect("opaque review token");

    let foreign_response = import_app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&import_uri)
                .header("cookie", &foreign_cookie)
                .body(Body::empty())
                .expect("foreign report request"),
        )
        .await
        .expect("foreign report response");
    assert_eq!(foreign_response.status(), StatusCode::NOT_FOUND);
    assert_no_store(foreign_response.headers());
    assert!(
        store
            .qti_import_view(foreign_context, owner, workspace, import)
            .await
            .expect("foreign Store lookup is non-enumerating")
            .is_none()
    );

    let conversion_uri = format!("{import_uri}/items/canvas-1/convert-flat");
    let conversion_response =
        crate::qti_profile_conversion::router(Arc::clone(&store), Arc::clone(&objects))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(conversion_uri)
                    .header("cookie", &owner_cookie)
                    .header(IF_MATCH, &initial_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "reportRevision": report_revision,
                            "reviewToken": review_token,
                        }))
                        .expect("conversion JSON"),
                    ))
                    .expect("conversion request"),
            )
            .await
            .expect("conversion response");
    let (status, conversion_headers, conversion_body) = response_parts(conversion_response).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&conversion_body)
    );
    assert_no_store(&conversion_headers);
    assert_answer_free_conversion(&conversion_body);
    let converted_etag = conversion_headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("conversion returns a strong revision")
        .to_string();
    assert_ne!(converted_etag, initial_etag);

    let current_origin = store
        .workspace_flat_import_origin(context, owner, workspace)
        .await
        .expect("current origin lookup")
        .expect("conversion records current origin");
    let source_key = ObjectKey::WorkspaceSource {
        tenant,
        workspace,
        import,
        object: source_object,
    };
    let stored_archive = objects
        .get(&source_key)
        .await
        .expect("workspace archive exists");
    assert_eq!(stored_archive.bytes, archive);
    assert_eq!(stored_archive.record, *current_origin.source_archive());
    assert_eq!(
        stored_archive.record.sha256,
        Sha256Digest::compute(&archive)
    );
    assert_eq!(
        objects
            .signed_url(&source_key, ActivityTimestamp::from_unix_millis(1))
            .await,
        Err(ObjectStoreError::NotSignable)
    );
    let converted_source = store
        .flat_question_source(context, owner, workspace)
        .await
        .expect("converted source lookup")
        .expect("conversion stages canonical source");
    let stored_source = objects
        .get(&converted_source.source_record.key)
        .await
        .expect("canonical source object exists");
    assert_eq!(
        Sha256Digest::compute(&stored_source.bytes).to_string(),
        converted_source.canonical_source_sha256
    );
    assert_eq!(
        converted_source.canonical_source_sha256,
        current_origin.mapped_canonical_source_sha256().to_string()
    );
    FlatQuestionDocument::parse(&stored_source.bytes)
        .expect("converted source remains canonical flat authoring JSON");

    let grader_capability: Arc<dyn FlatQuestionGradingStore> = grader;
    let backend = Arc::new(NativeBackend::with_flat_grader(
        Arc::new(adapter_native::NativeAdapter::new()),
        Arc::clone(&store),
        grader_capability,
    ));
    let publication_response = crate::flat_question_publication::router(
        Arc::clone(&store),
        Arc::clone(&objects),
        Arc::clone(&backend),
        Arc::new(ReviewNotRequired),
    )
    .oneshot(
        Request::builder()
            .method("POST")
            .uri(format!("/api/problems/{workspace}/flat-question-publish"))
            .header("cookie", &owner_cookie)
            .header(IF_MATCH, &converted_etag)
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
            ))
            .expect("publication request"),
    )
    .await
    .expect("publication response");
    let (status, publication_headers, publication_body) =
        response_parts(publication_response).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&publication_body)
    );
    assert_no_store(&publication_headers);
    let summary: CatalogProblemSummary =
        serde_json::from_slice(&publication_body).expect("safe catalog publication summary");
    let catalog = store
        .resolve_catalog_problem(
            context,
            ProblemDisplayRef {
                question_id: summary.question_id.clone(),
            },
        )
        .await
        .expect("published catalog lookup")
        .expect("published native flat question exists");
    assert_eq!(catalog.summary(), summary);
    let question = catalog.question;
    let reference = ProblemVersionRef {
        problem: question.problem,
        version: question.version,
    };
    assert!(
        store
            .get_catalog_problem(foreign_context, reference)
            .await
            .expect("foreign institution catalog lookup is non-enumerating")
            .is_none()
    );
    let published_source = store
        .catalog_source_artifact(context, reference)
        .await
        .expect("published source lookup")
        .expect("published canonical source exists");
    let published_source_bytes = objects
        .get(&published_source.object.key)
        .await
        .expect("published source bytes");
    assert_eq!(published_source_bytes.bytes, stored_source.bytes);
    assert_eq!(
        Sha256Digest::compute(&published_source_bytes.bytes),
        published_source.object.sha256
    );

    let published_archive_object = published_import_archive_object_id(
        tenant,
        reference.problem,
        reference.version,
        import,
        stored_archive.record.sha256,
    );
    let published_archive_key = ObjectKey::PublishedImportArchive {
        tenant,
        problem: reference.problem,
        version: reference.version,
        import,
        object: published_archive_object,
    };
    let published_archive = objects
        .get(&published_archive_key)
        .await
        .expect("immutable published import archive exists");
    assert_eq!(published_archive.bytes, archive);
    assert_eq!(
        published_archive.record.sha256,
        stored_archive.record.sha256
    );
    assert_eq!(
        objects
            .signed_url(
                &published_archive_key,
                ActivityTimestamp::from_unix_millis(1),
            )
            .await,
        Err(ObjectStoreError::NotSignable)
    );
    assert!(
        store
            .workspace_flat_import_origin(context, owner, workspace)
            .await
            .expect("workspace origin lookup after publication")
            .is_none(),
        "publication cleanup removes current-only origin"
    );
    assert!(
        store
            .get_draft(context, owner, workspace)
            .await
            .expect("workspace lookup after publication")
            .is_none(),
        "publication cleanup removes workspace staging"
    );

    let issued = backend
        .issue(context, reference, &question, 42)
        .await
        .expect("published flat question issues through the native backend");
    let attempt = QuestionAttempt {
        id: QuestionAttemptId::from_uuid(random_uuid()),
        tenant,
        run: RunId::from_uuid(random_uuid()),
        problem: reference.problem,
        question_version: reference.version,
        assignment_position: 0,
        seed: 42,
        parameter_hash: issued.parameter_hash,
        response: None,
        status: AttemptStatus::InProgress,
        result: None,
        timer: AttemptTimerRecord {
            issued_at: store
                .authoritative_time(context)
                .await
                .expect("authoritative attempt time"),
            deadline: None,
            submitted_at: None,
        },
        provenance: issued.provenance,
        issued_capability: question_model::IssuedAttemptCapabilityV1::FlatPresentation,
    };
    let correct = backend
        .grade(
            context,
            reference,
            &question,
            &attempt,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("blue")],
            },
        )
        .await
        .expect("isolated grader accepts the correct response");
    assert!(matches!(correct, GradeOutcome::Graded(result) if result.correct));
    let incorrect = backend
        .grade(
            context,
            reference,
            &question,
            &attempt,
            &StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("red")],
            },
        )
        .await
        .expect("isolated grader accepts the incorrect response");
    assert!(matches!(incorrect, GradeOutcome::Graded(result) if !result.correct));

    let import_reference = QtiImportRef {
        tenant,
        workspace,
        import,
    };
    assert_eq!(current_origin.import(), import_reference);
}
