use std::io::{Cursor, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use adapter_native::flat_question::FlatQuestionDocument;
use axum::body::Body;
use axum::http::{HeaderMap, Request};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, FlatQuestionStore, JobClaimFilter,
    JobLeaseDuration, JobPayload, JobStore, QtiImportApiStore, QtiImportRef, QtiImportStore,
    QueueQtiImportCommand, SessionLifetime, SessionSubject,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectRecord, PutObject, SignedUrl, StoredObject, workspace_qti_archive_object_id};
use question_model::{
    ActivityTimestamp, ProblemId, ProblemVersionRef, TenantId, UserId, VersionId,
};
use tower::ServiceExt;

use crate::auth::{CookieTransport, SessionConfig, issue_session};
use crate::qti_import::QtiImportHandler;
use crate::worker::{JobExecution, JobHandler};

use super::*;

const CANVAS_MANIFEST: &str =
    include_str!("../../../adapters/qti/tests/fixtures/profiles/canvas_positive_manifest.xml");
const CANVAS_META: &str =
    include_str!("../../../adapters/qti/tests/fixtures/profiles/canvas_assessment_meta.xml");
const CANVAS_ITEM: &str =
    include_str!("../../../adapters/qti/tests/fixtures/profiles/canvas_positive_item.xml");
const EXISTING_SOURCE: &str = r#"{
  "format":"pleFlatQuestion",
  "version":1,
  "kind":"singleChoice",
  "title":"Existing draft",
  "prompt":"Which draft existed before import?",
  "choices":[{"id":"first","text":"First"},{"id":"second","text":"Second"}],
  "correctChoice":"first",
  "feedback":{},
  "points":1.0,
  "attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},
  "timingPolicy":{"kind":"untimed"},
  "license":{"kind":"allRightsReserved"},
  "language":"en-US"
}"#;

struct CountingObjectStore {
    inner: MemoryObjectStore,
    puts: AtomicUsize,
    corrupt_archive_read: AtomicBool,
}

impl Default for CountingObjectStore {
    fn default() -> Self {
        Self {
            inner: MemoryObjectStore::default(),
            puts: AtomicUsize::new(0),
            corrupt_archive_read: AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl ObjectStore for CountingObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        self.puts.fetch_add(1, Ordering::SeqCst);
        self.inner.put(request).await
    }

    async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
        let mut stored = self.inner.get(key).await?;
        if self.corrupt_archive_read.load(Ordering::SeqCst)
            && matches!(key, ObjectKey::WorkspaceSource { .. })
        {
            stored.bytes.push(0);
        }
        Ok(stored)
    }

    async fn delete(&self, key: &ObjectKey) -> Result<(), ObjectStoreError> {
        self.inner.delete(key).await
    }

    async fn signed_url(
        &self,
        key: &ObjectKey,
        now: ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.inner.signed_url(key, now).await
    }
}

struct Fixture {
    store: Arc<MemoryStore>,
    objects: Arc<CountingObjectStore>,
    tenant: TenantId,
    owner: UserId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
    owner_cookie: String,
    other_cookie: String,
    report_revision: String,
    review_token: String,
    lineage: ProblemVersionRef,
    initial_revision: WorkspaceDraftRevision,
    initial_etag: String,
}

impl Fixture {
    fn app(&self) -> Router {
        router(Arc::clone(&self.store), Arc::clone(&self.objects))
    }

    fn uri(&self, item: &str) -> String {
        format!(
            "/api/workspaces/{}/qti-imports/{}/items/{item}/convert-flat",
            self.workspace, self.import
        )
    }

    fn context(&self) -> learning_data_access::TenantContext {
        learning_data_access::TenantContext::from_authenticated_session(self.tenant)
    }
}

fn id(value: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(value)
}

fn canvas_archive_with_item_identifier(identifier: &str) -> Vec<u8> {
    let item = CANVAS_ITEM.replace("ident=\"canvas-1\"", &format!("ident=\"{identifier}\""));
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("imsmanifest.xml", options)
        .expect("fixture entry");
    zip.write_all(CANVAS_MANIFEST.as_bytes())
        .expect("fixture body");
    zip.start_file("canvas_qti12_questions/assessment_meta.xml", options)
        .expect("fixture entry");
    zip.write_all(CANVAS_META.as_bytes()).expect("fixture body");
    zip.start_file("canvas_qti12_questions/canvas-1.xml", options)
        .expect("fixture entry");
    zip.write_all(item.as_bytes()).expect("fixture body");
    zip.finish().expect("fixture archive").into_inner()
}

async fn issued_cookie(store: &MemoryStore, tenant: TenantId, user: UserId) -> String {
    let issued = issue_session(
        store,
        SessionSubject::new(
            tenant,
            user,
            "QTI conversion fixture",
            vec![UserRole::Instructor],
        )
        .expect("fixture subject"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
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

async fn fixture() -> Fixture {
    fixture_for_item_identifier("canvas-1").await
}

async fn fixture_for_item_identifier(item_identifier: &str) -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(CountingObjectStore::default());
    let tenant = TenantId::from_uuid(id(1));
    let owner = UserId::from_uuid(id(2));
    let other = UserId::from_uuid(id(3));
    let workspace = WorkspaceId::from_uuid(id(4));
    let import = WorkspaceImportId::from_uuid(id(5));
    let context = learning_data_access::TenantContext::from_authenticated_session(tenant);
    let lineage = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(6)),
        version: VersionId::from_uuid(id(7)),
    };
    let question = FlatQuestionDocument::parse(EXISTING_SOURCE.as_bytes())
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
                question,
                revises: Some(lineage),
                derived_from: None,
            },
        )
        .await
        .expect("existing visible draft");

    let bytes = canvas_archive_with_item_identifier(item_identifier);
    let object = workspace_qti_archive_object_id(tenant, workspace, import);
    let source = objects
        .put(PutObject {
            key: ObjectKey::WorkspaceSource {
                tenant,
                workspace,
                import,
                object,
            },
            bytes,
            media_type: QTI_PROFILE_ARCHIVE_MEDIA_TYPE.to_string(),
            license: "allRightsReserved".to_string(),
            provenance: "author-uploaded QTI workspace import archive".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(100),
        })
        .await
        .expect("archive persists");
    store
        .queue_qti_import(
            context,
            owner,
            QueueQtiImportCommand {
                reference: QtiImportRef {
                    tenant,
                    workspace,
                    import,
                },
                source,
                max_attempts: 3,
            },
        )
        .await
        .expect("import queues");
    let handler = QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects));
    let payload = JobPayload::QtiImport {
        workspace,
        import,
        source_object: object,
    };
    handler
        .prepare(context, payload, JobExecution::new())
        .await
        .expect("profile worker prepares");
    let claim = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("lease"),
        )
        .await
        .expect("claim succeeds")
        .expect("QTI job is ready");
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job: claim.id,
                    lease: claim.lease_token,
                    reference: QtiImportRef {
                        tenant,
                        workspace,
                        import,
                    },
                    source_object: object,
                },
            )
            .await
            .expect("profile commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let view = store
        .qti_import_view(context, owner, workspace, import)
        .await
        .expect("ready view")
        .expect("visible import");
    let registry = view.registry.expect("ready registry");
    let acknowledgement =
        qti_profile_report_acknowledgement(import, &registry).expect("visible acknowledgement");

    Fixture {
        owner_cookie: issued_cookie(&store, tenant, owner).await,
        other_cookie: issued_cookie(&store, tenant, other).await,
        store,
        objects,
        tenant,
        owner,
        workspace,
        import,
        report_revision: acknowledgement.report_revision().to_string(),
        review_token: acknowledgement.review_token().to_string(),
        lineage,
        initial_revision: initial.revision,
        initial_etag: format!("\"{}\"", initial.revision.value()),
    }
}

async fn convert(
    fixture: &Fixture,
    cookie: &str,
    item: &str,
    etag: Option<&str>,
    report_revision: &str,
    review_token: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method("POST")
        .uri(fixture.uri(item))
        .header("cookie", cookie)
        .header("content-type", "application/json");
    if let Some(etag) = etag {
        request = request.header(IF_MATCH, etag);
    }
    let response = fixture
        .app()
        .oneshot(
            request
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "reportRevision": report_revision,
                        "reviewToken": review_token,
                    }))
                    .expect("request JSON"),
                ))
                .expect("conversion request"),
        )
        .await
        .expect("conversion response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded response")
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

fn assert_answer_free(body: &[u8]) {
    let text = String::from_utf8_lossy(body);
    for forbidden in [
        "correctChoice",
        "response1",
        "root_section",
        "imsmanifest",
        "profileReportSha256",
        "privateMapping",
        "choiceMap",
        "canonicalSource",
        "WorkspaceSource",
        "grader",
    ] {
        assert!(
            !text.contains(forbidden),
            "conversion response leaked forbidden evidence {forbidden}: {text}"
        );
    }
}

#[test]
fn conversion_revision_requires_one_strong_current_etag_shape() {
    let mut strong = HeaderMap::new();
    strong.insert(IF_MATCH, HeaderValue::from_static("\"7\""));
    let Ok(revision) = required_revision(&strong) else {
        panic!("strong decimal ETag must parse");
    };
    assert_eq!(revision.value(), 7);

    for value in ["W/\"7\"", "7", "\"0\"", "\"7\", \"8\""] {
        let mut malformed = HeaderMap::new();
        malformed.insert(
            IF_MATCH,
            HeaderValue::from_str(value).expect("valid test header"),
        );
        assert!(matches!(
            required_revision(&malformed),
            Err(RevisionError::Malformed)
        ));
    }
    assert!(matches!(
        required_revision(&HeaderMap::new()),
        Err(RevisionError::Missing)
    ));
}

#[tokio::test]
async fn recognized_item_conversion_creates_answer_free_flat_draft_source_and_origin() {
    let fixture = fixture().await;
    let baseline_puts = fixture.objects.puts.load(Ordering::SeqCst);
    let (status, headers, body) = convert(
        &fixture,
        &fixture.owner_cookie,
        "canvas-1",
        Some(&fixture.initial_etag),
        &fixture.report_revision,
        &fixture.review_token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_no_store(&headers);
    assert_answer_free(&body);
    assert_eq!(
        fixture.objects.puts.load(Ordering::SeqCst),
        baseline_puts + 1
    );

    let draft = fixture
        .store
        .get_draft(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("draft lookup")
        .expect("converted draft");
    let returned_etag = headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("conversion returns an ETag");
    assert!(returned_etag.starts_with('"') && returned_etag.ends_with('"'));
    assert!(!returned_etag.starts_with("W/"));
    assert_ne!(returned_etag, fixture.initial_etag);
    assert_eq!(returned_etag, format!("\"{}\"", draft.revision.value()));
    assert_eq!(draft.record.revises, Some(fixture.lineage));
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("source lookup")
            .is_some()
    );
    assert!(
        fixture
            .store
            .workspace_flat_import_origin(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("origin lookup")
            .is_some()
    );
}

#[tokio::test]
async fn encoded_source_identifier_with_slash_and_unicode_is_one_path_segment() {
    let fixture = fixture_for_item_identifier("canvas/beta-\u{03b2}").await;
    let (status, headers, body) = convert(
        &fixture,
        &fixture.owner_cookie,
        "canvas%2Fbeta-%CE%B2",
        Some(&fixture.initial_etag),
        &fixture.report_revision,
        &fixture.review_token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_no_store(&headers);
    assert_answer_free(&body);
}

#[tokio::test]
async fn conversion_refusals_precede_candidate_creation_and_preserve_the_draft() {
    let fixture = fixture().await;
    let baseline_puts = fixture.objects.puts.load(Ordering::SeqCst);
    let weak_etag = format!("W/{}", fixture.initial_etag);
    let stale_etag = format!("\"{}\"", fixture.initial_revision.value() + 1);
    let cases = [
        (
            None,
            "canvas-1",
            fixture.report_revision.as_str(),
            fixture.review_token.as_str(),
            StatusCode::PRECONDITION_REQUIRED,
        ),
        (
            Some(weak_etag.as_str()),
            "canvas-1",
            fixture.report_revision.as_str(),
            fixture.review_token.as_str(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            Some(stale_etag.as_str()),
            "canvas-1",
            fixture.report_revision.as_str(),
            fixture.review_token.as_str(),
            StatusCode::CONFLICT,
        ),
        (
            Some(fixture.initial_etag.as_str()),
            "canvas-1",
            "stale-report",
            fixture.review_token.as_str(),
            StatusCode::CONFLICT,
        ),
        (
            Some(fixture.initial_etag.as_str()),
            "canvas-1",
            fixture.report_revision.as_str(),
            "stale-review",
            StatusCode::CONFLICT,
        ),
        (
            Some(fixture.initial_etag.as_str()),
            "missing-item",
            fixture.report_revision.as_str(),
            fixture.review_token.as_str(),
            StatusCode::NOT_FOUND,
        ),
    ];
    for (etag, item, report, review, expected) in cases {
        let (status, headers, _) =
            convert(&fixture, &fixture.owner_cookie, item, etag, report, review).await;
        assert_eq!(status, expected);
        assert_no_store(&headers);
    }
    let unknown_field_response = fixture
        .app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(fixture.uri("canvas-1"))
                .header("cookie", &fixture.owner_cookie)
                .header(IF_MATCH, &fixture.initial_etag)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"reportRevision":"{}","reviewToken":"{}","extra":true}}"#,
                    fixture.report_revision, fixture.review_token
                )))
                .expect("strict request"),
        )
        .await
        .expect("strict response");
    assert_eq!(
        unknown_field_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_no_store(unknown_field_response.headers());
    let malformed_json_response = fixture
        .app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(fixture.uri("canvas-1"))
                .header("cookie", &fixture.owner_cookie)
                .header(IF_MATCH, &fixture.initial_etag)
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .expect("malformed JSON request"),
        )
        .await
        .expect("malformed JSON response");
    assert!(!malformed_json_response.status().is_success());
    assert_no_store(malformed_json_response.headers());

    let (status, headers, inaccessible_body) = convert(
        &fixture,
        &fixture.other_cookie,
        "canvas-1",
        Some(&fixture.initial_etag),
        &fixture.report_revision,
        &fixture.review_token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_no_store(&headers);
    let (status, _, missing_body) = convert(
        &fixture,
        &fixture.owner_cookie,
        "missing-item",
        Some(&fixture.initial_etag),
        &fixture.report_revision,
        &fixture.review_token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(inaccessible_body, missing_body);

    fixture
        .objects
        .corrupt_archive_read
        .store(true, Ordering::SeqCst);
    let (status, headers, _) = convert(
        &fixture,
        &fixture.owner_cookie,
        "canvas-1",
        Some(&fixture.initial_etag),
        &fixture.report_revision,
        &fixture.review_token,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);

    assert_eq!(fixture.objects.puts.load(Ordering::SeqCst), baseline_puts);
    let draft = fixture
        .store
        .get_draft(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("draft lookup")
        .expect("existing draft remains");
    assert_eq!(draft.revision, fixture.initial_revision);
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("source lookup")
            .is_none()
    );
    assert!(
        fixture
            .store
            .workspace_flat_import_origin(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("origin lookup")
            .is_none()
    );
}
