#[path = "tests/hotspot_publication.rs"]
mod hotspot_publication;
#[path = "tests/imported_publication.rs"]
mod imported_publication;
#[path = "tests/v2_publication.rs"]
mod v2_publication;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use learning_data_access::in_memory::{MemoryFlatQuestionGraderStore, MemoryStore};
use learning_data_access::{
    AssetStore, CatalogSourceStore, CatalogStore, CommitPreparedQtiImport,
    CommitPreparedQtiImportOutcome, CreateQtiImportCommand, DraftRecord, EnqueueJob,
    FlatImportChoiceMapPayload, FlatImportConversionVersion, FlatImportIntegrityDigests,
    FlatImportProvenanceStore, FlatQuestionGradingPayload, FlatQuestionGradingStore,
    FlatQuestionStore, JobClaimFilter, JobLeaseDuration, JobPayload, JobStore,
    PersistedFlatImportProfile, QtiImportGradingPayload, QtiImportItem, QtiImportItemRegistration,
    QtiImportItemResult, QtiImportItemStatus, QtiImportProfileSummary, QtiImportRef,
    QtiImportRegistry, QtiImportStore, QtiProfileFlatConversionCommand, QtiProfileImportEvidence,
    QtiUnsupportedFeature, SessionLifetime, SessionSubject, Store, TenantContext,
    UpsertFlatQuestionCommand, WorkspaceFlatImportOrigin, WorkspaceFlatQuestionAsset,
};
use objects::memory::MemoryObjectStore;
use objects::{
    ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject, Sha256Digest, SignedUrl,
    StoredObject, published_import_archive_object_id,
};
use question_model::{
    ActivityTimestamp, AssetId, ObjectId, ProblemId, ProblemVersionRef, TenantId, UserId, UserRole,
    VersionId, WorkspaceId, WorkspaceImportId,
};
use tower::ServiceExt;

use crate::auth::{CookieTransport, SessionConfig, issue_session};
use crate::catalog::ReviewNotRequired;
use crate::native_backend::NativeBackend;

use super::*;

const OWNER: u128 = 1;
const FOREIGN: u128 = 2;
const WORKSPACE: u128 = 3;
const OWNER_TENANT: u128 = 4;

const FLAT_SOURCE: &str = r#"{
  "format":"pleFlatQuestion",
  "version":2,
  "title":"Favorite color",
  "prompt":"What is my favorite color?",
  "response":{"kind":"singleChoice","choices":[
    {"id":"blue","text":"Blue","feedback":"PRIVATE_CHOICE_FEEDBACK"},
    {"id":"red","text":"Red","feedback":"PRIVATE_RED_FEEDBACK"}
  ],"correctChoice":"blue"},
  "feedback":{"correct":"PRIVATE_CORRECT_FEEDBACK","incorrect":"PRIVATE_INCORRECT_FEEDBACK"},
  "points":10.0,
  "attemptPolicy":{"maxAttempts":null},
  "timingPolicy":{"kind":"untimed"},
  "license":{"kind":"cc0"},
  "language":"en-US"
}"#;

struct Fixture {
    store: Arc<MemoryStore>,
    grader: Arc<MemoryFlatQuestionGraderStore>,
    objects: Arc<MemoryObjectStore>,
    tenant: TenantId,
    workspace: WorkspaceId,
    owner: UserId,
    foreign: UserId,
    owner_cookie: String,
    foreign_cookie: String,
}

impl Fixture {
    fn app(&self) -> Router {
        self.app_with_objects(Arc::clone(&self.objects))
    }

    fn app_with_objects<O>(&self, objects: Arc<O>) -> Router
    where
        O: ObjectStore + 'static,
    {
        let grader: Arc<dyn FlatQuestionGradingStore> = Arc::clone(&self.grader) as Arc<_>;
        let backend = Arc::new(NativeBackend::with_flat_grader(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&self.store),
            grader,
        ));
        router(
            Arc::clone(&self.store),
            objects,
            backend,
            Arc::new(ReviewNotRequired),
        )
    }

    fn save_uri(&self) -> String {
        format!("/api/workspaces/{}/flat-question", self.workspace)
    }

    fn publish_uri(&self) -> String {
        format!("/api/problems/{}/flat-question-publish", self.workspace)
    }

    fn context(&self) -> TenantContext {
        TenantContext::from_authenticated_session(self.tenant)
    }
}

struct InconsistentReadObjectStore {
    inner: Arc<MemoryObjectStore>,
}

#[async_trait::async_trait]
impl ObjectStore for InconsistentReadObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        self.inner.put(request).await
    }

    async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
        let mut stored = self.inner.get(key).await?;
        stored.record.provenance.push_str(" (mismatched)");
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

fn id(value: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(value)
}

async fn issued_cookie(store: &MemoryStore, tenant: TenantId, user: UserId) -> String {
    issued_cookie_with_roles(store, tenant, user, vec![UserRole::Instructor]).await
}

async fn issued_cookie_with_roles(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
) -> String {
    let issued = issue_session(
        store,
        SessionSubject::new(tenant, user, "flat-question publication fixture", roles)
            .expect("fixture identity"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("valid session lifetime"),
            CookieTransport::FirstPartyHttps,
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
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let store = Arc::new(store);
    let grader = Arc::new(grader);
    let tenant = TenantId::from_uuid(id(OWNER_TENANT));
    let owner = UserId::from_uuid(id(OWNER));
    let foreign = UserId::from_uuid(id(FOREIGN));
    Fixture {
        owner_cookie: issued_cookie(&store, tenant, owner).await,
        foreign_cookie: issued_cookie(&store, tenant, foreign).await,
        store,
        grader,
        objects: Arc::new(MemoryObjectStore::default()),
        tenant,
        workspace: WorkspaceId::from_uuid(id(WORKSPACE)),
        owner,
        foreign,
    }
}

async fn response_parts(response: axum::response::Response) -> (StatusCode, HeaderMap, Vec<u8>) {
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded response body");
    (status, headers, body.to_vec())
}

fn assert_no_store(headers: &HeaderMap) {
    assert_eq!(
        headers
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
}

fn assert_no_private_tokens(bytes: &[u8]) {
    let serialized = String::from_utf8_lossy(bytes);
    for token in [
        "correctChoice",
        "PRIVATE_CHOICE_FEEDBACK",
        "PRIVATE_RED_FEEDBACK",
        "PRIVATE_CORRECT_FEEDBACK",
        "PRIVATE_INCORRECT_FEEDBACK",
        "publicSha256",
        "base64",
        "key",
    ] {
        assert!(
            !serialized.contains(token),
            "browser response leaked private token {token}: {serialized}"
        );
    }
}

async fn save(
    fixture: &Fixture,
    cookie: &str,
    body: impl Into<Body>,
    revision: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method("PUT")
        .uri(fixture.save_uri())
        .header("cookie", cookie);
    if let Some(revision) = revision {
        request = request.header("if-match", revision);
    }
    response_parts(
        fixture
            .app()
            .oneshot(request.body(body.into()).expect("save request"))
            .await
            .expect("save response"),
    )
    .await
}

async fn read_source(fixture: &Fixture, cookie: Option<&str>) -> (StatusCode, HeaderMap, Vec<u8>) {
    read_source_from_app(fixture.app(), fixture.save_uri(), cookie).await
}

async fn read_source_from_app(
    app: Router,
    uri: String,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    response_parts(
        app.oneshot(request.body(Body::empty()).expect("source read request"))
            .await
            .expect("source read response"),
    )
    .await
}

async fn publish(fixture: &Fixture, revision: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    publish_with_scope(fixture, revision, "institution").await
}

async fn published_record(
    fixture: &Fixture,
    body: &[u8],
) -> learning_data_access::PublishedProblemRecord {
    let summary: question_model::CatalogProblemSummary =
        serde_json::from_slice(body).expect("safe catalog publication summary");
    fixture
        .store
        .resolve_catalog_problem(
            fixture.context(),
            question_model::ProblemDisplayRef {
                question_id: summary.question_id,
            },
        )
        .await
        .expect("publication lookup")
        .expect("published question remains visible to the author")
}

async fn publish_with_scope(
    fixture: &Fixture,
    revision: &str,
    scope: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    response_parts(
        fixture
            .app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(fixture.publish_uri())
                    .header("cookie", &fixture.owner_cookie)
                    .header("if-match", revision)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"scope":"{scope}","byline":{{"names":["PLE fixture"]}}}}"#
                    )))
                    .expect("publish request"),
            )
            .await
            .expect("publish response"),
    )
    .await
}

struct ImportedFlatFixture {
    etag: String,
    archive_key: ObjectKey,
    archive_bytes: Vec<u8>,
    origin: WorkspaceFlatImportOrigin,
}

fn fixed_profile_defaults() -> Vec<QtiUnsupportedFeature> {
    [
        "PLE default applied: unlimited attempts.",
        "PLE default applied: untimed.",
        "PLE default applied: en-US.",
        "PLE default applied: allRightsReserved.",
        "PLE default applied: empty tags.",
        "PLE default applied: empty taxonomy.",
        "PLE default applied: no feedback.",
    ]
    .into_iter()
    .map(|detail| QtiUnsupportedFeature {
        code: "policy".to_string(),
        location: "item".to_string(),
        detail: detail.to_string(),
    })
    .collect()
}

async fn install_import_origin(fixture: &Fixture) -> ImportedFlatFixture {
    let (status, _, body) = save(fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let current = fixture
        .store
        .get_draft(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("converted fixture draft lookup")
        .expect("manual save creates the conversion baseline");
    let staged = fixture
        .store
        .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("converted fixture source lookup")
        .expect("manual save stages the canonical source");
    let document =
        FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes()).expect("converted fixture parses");
    let (_, private) = document
        .compile(fixture.workspace)
        .expect("converted fixture compiles")
        .into_parts();
    let grading = FlatQuestionGradingPayload::from_private(&private)
        .expect("converted fixture grading is valid");

    let import = WorkspaceImportId::from_uuid(id(10_001));
    let archive_object = ObjectId::from_uuid(id(10_002));
    let archive_bytes = b"PK\x03\x04verified profile archive fixture".to_vec();
    let archive = fixture
        .objects
        .put(PutObject {
            key: ObjectKey::WorkspaceSource {
                tenant: fixture.tenant,
                workspace: fixture.workspace,
                import,
                object: archive_object,
            },
            bytes: archive_bytes.clone(),
            media_type: QTI_PROFILE_ARCHIVE_MEDIA_TYPE.to_string(),
            license: "private QTI import provenance".to_string(),
            provenance: "verified Canvas workspace import archive".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(10_003),
        })
        .await
        .expect("converted fixture archive persists");
    let choice_map = FlatImportChoiceMapPayload::from_canonical_bytes(
        br#"{"schema":"ple-qti-private-choice-map/v1","choices":[["vendor-blue","blue"],["vendor-red","red"]]}"#.to_vec(),
    )
    .expect("converted fixture choice map is bounded");
    let digests = FlatImportIntegrityDigests {
        normalized_item_sha256: Sha256Digest::compute(b"normalized Canvas item"),
        profile_report_sha256: Sha256Digest::compute(b"safe profile report"),
        public_mapping_sha256: Sha256Digest::compute(b"public mapping"),
        private_mapping_sha256: Sha256Digest::compute(b"private mapping"),
        mapping_sha256: Sha256Digest::compute(b"combined mapping"),
        warning_sha256: Sha256Digest::compute(b"author warnings"),
        choice_map_sha256: choice_map.sha256(),
    };
    let import_reference = QtiImportRef {
        tenant: fixture.tenant,
        workspace: fixture.workspace,
        import,
    };
    let item_id = "canvas-item-1".to_string();
    let origin = WorkspaceFlatImportOrigin::new(
        import_reference,
        item_id.clone(),
        PersistedFlatImportProfile::CanvasQti12V1,
        FlatImportConversionVersion::new("ple-qti-profile-flat-conversion/v1")
            .expect("converted fixture version"),
        archive.clone(),
        digests,
        staged.source_record.sha256,
        fixture.owner,
        ActivityTimestamp::from_unix_millis(10_004),
        choice_map,
    )
    .expect("converted fixture origin is valid");
    let item = QtiImportItem {
        item_id: item_id.clone(),
        model_sha256: Sha256Digest::compute(b"answer-free imported item"),
        assets: Vec::new(),
    };
    fixture
        .store
        .prepare_qti_import(
            fixture.context(),
            CreateQtiImportCommand {
                registry: QtiImportRegistry {
                    reference: import_reference,
                    source: archive.clone(),
                    source_format: "qti".to_string(),
                    source_identifier: Some("canvas-profile.zip".to_string()),
                    importer: "adapter_qti".to_string(),
                    parse_schema: PersistedFlatImportProfile::CanvasQti12V1
                        .profile_id()
                        .to_string(),
                    adapter_version: "v1".to_string(),
                    profile_summary: Some(
                        QtiImportProfileSummary::new(
                            PersistedFlatImportProfile::CanvasQti12V1,
                            digests.profile_report_sha256,
                            fixed_profile_defaults(),
                        )
                        .expect("converted fixture profile summary is valid"),
                    ),
                    items: vec![item.clone()],
                    item_results: vec![QtiImportItemResult {
                        source_identifier: item_id.clone(),
                        title: Some("Imported favorite color".to_string()),
                        item_id: Some(item_id.clone()),
                        normalized_sha256: Some(digests.normalized_item_sha256),
                        status: QtiImportItemStatus::Accepted,
                        diagnostics: Vec::new(),
                        defaults: Vec::new(),
                        warnings: Vec::new(),
                    }],
                    assets: Vec::new(),
                    unsupported_features: Vec::new(),
                },
                item_bindings: vec![QtiImportItemRegistration {
                    item,
                    grading: QtiImportGradingPayload::new(br#""blue""#.to_vec())
                        .expect("converted fixture QTI grading is bounded"),
                }],
            },
        )
        .await
        .expect("converted fixture import prepares");
    fixture
        .store
        .stage_qti_profile_import_evidence(
            fixture.context(),
            QtiProfileImportEvidence::new(import_reference, item_id, origin.profile(), digests)
                .expect("converted fixture profile evidence is valid"),
        )
        .await
        .expect("converted fixture profile evidence stages");
    let job = fixture
        .store
        .enqueue_job(
            fixture.context(),
            EnqueueJob {
                tenant: fixture.tenant,
                payload: JobPayload::QtiImport {
                    workspace: fixture.workspace,
                    import,
                    source_object: archive.id,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("converted fixture job enqueues");
    let claim = fixture
        .store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("converted fixture lease is bounded"),
        )
        .await
        .expect("converted fixture claim query")
        .expect("converted fixture job is claimable");
    assert_eq!(claim.id, job);
    assert_eq!(
        fixture
            .store
            .commit_prepared_qti_import(
                fixture.context(),
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: import_reference,
                    source_object: archive.id,
                },
            )
            .await
            .expect("converted fixture import commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let converted = fixture
        .store
        .convert_qti_profile_item_to_flat(
            fixture.context(),
            fixture.owner,
            QtiProfileFlatConversionCommand::new(
                Some(current.revision),
                current.record,
                staged.source_record.clone(),
                staged.canonical_source_sha256,
                staged.public_binding_sha256,
                grading,
                origin.clone(),
            )
            .expect("converted fixture command is valid"),
        )
        .await
        .expect("committed profile item converts to flat atomically");

    ImportedFlatFixture {
        etag: format!("\"{}\"", converted.workspace_revision.value()),
        archive_key: archive.key,
        archive_bytes,
        origin,
    }
}

#[tokio::test]
async fn owner_save_stages_canonical_non_signable_source_and_returns_only_public_draft() {
    let fixture = fixture().await;

    let (status, headers, body) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_no_store(&headers);
    let etag = headers.get("etag").expect("save has strong ETag");
    assert_eq!(etag.to_str().expect("ETag text"), "\"1\"");
    assert_no_private_tokens(&body);
    let saved: serde_json::Value = serde_json::from_slice(&body).expect("public draft JSON");
    assert_eq!(saved["source"]["backend"], "native");
    assert_eq!(saved["source"]["family"], FLAT_SINGLE_CHOICE_V2_FAMILY);

    let draft = fixture
        .store
        .get_draft(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("owner draft lookup")
        .expect("draft is staged");
    let serialized_draft = serde_json::to_vec(&draft.record.question).expect("draft serializes");
    assert_no_private_tokens(&serialized_draft);
    let staged = fixture
        .store
        .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("owner source lookup")
        .expect("source is staged");
    assert_eq!(staged.workspace_revision, draft.revision);
    assert!(matches!(
        staged.source_record.key,
        ObjectKey::WorkspaceQuestionSource { .. }
    ));
    assert!(matches!(
        fixture
            .objects
            .signed_url(&staged.source_record.key, staged.source_record.created_at)
            .await,
        Err(objects::ObjectStoreError::NotSignable)
    ));
    let stored = fixture
        .objects
        .get(&staged.source_record.key)
        .await
        .expect("staged source object");
    let canonical = FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes())
        .expect("fixture source parses")
        .canonical_bytes()
        .expect("fixture source canonicalizes");
    assert_eq!(stored.bytes, canonical);
}

#[tokio::test]
async fn author_source_read_returns_only_canonical_source_and_a_reusable_strong_etag() {
    let fixture = fixture().await;
    let (_, saved_headers, _) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    let saved_etag = saved_headers
        .get("etag")
        .expect("save ETag")
        .to_str()
        .expect("ETag text")
        .to_string();

    let (status, headers, body) = read_source(&fixture, Some(&fixture.owner_cookie)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_no_store(&headers);
    assert_eq!(
        headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some(FLAT_QUESTION_MEDIA_TYPE)
    );
    let etag = headers
        .get("etag")
        .expect("source read ETag")
        .to_str()
        .expect("ETag text");
    assert_eq!(etag, saved_etag);
    let canonical = FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes())
        .expect("fixture source parses")
        .canonical_bytes()
        .expect("fixture source canonicalizes");
    assert_eq!(body, canonical);
    let source_text = String::from_utf8_lossy(&body);
    for absent in [
        "publicSha256",
        "payloadSha256",
        "payloadBase64",
        "signedUrl",
        "sourceRecord",
        "sha256",
    ] {
        assert!(
            !source_text.contains(absent),
            "source response leaked implementation metadata {absent}: {source_text}"
        );
    }

    let replacement = FLAT_SOURCE.replace("Favorite color", "A revised favorite color");
    let (save_status, save_headers, _) =
        save(&fixture, &fixture.owner_cookie, replacement, Some(etag)).await;
    assert_eq!(save_status, StatusCode::OK);
    let next_etag = save_headers
        .get("etag")
        .expect("replacement ETag")
        .to_str()
        .expect("ETag text")
        .to_string();
    let (publish_status, publish_headers, _) = publish(&fixture, &next_etag).await;
    assert_eq!(publish_status, StatusCode::CREATED);
    assert_no_store(&publish_headers);
}

#[tokio::test]
async fn source_read_hides_workspace_existence_from_non_authors_and_foreign_sessions() {
    let fixture = fixture().await;
    let (_, _, _) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    let student_cookie = issued_cookie_with_roles(
        fixture.store.as_ref(),
        fixture.tenant,
        UserId::from_uuid(id(5)),
        vec![UserRole::Student],
    )
    .await;

    for cookie in [
        Some(fixture.foreign_cookie.as_str()),
        Some(student_cookie.as_str()),
    ] {
        let (status, headers, body) = read_source(&fixture, cookie).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_no_store(&headers);
        assert_eq!(body, br#"{"error":"workspace not found"}"#);
    }
    let (status, headers, _) = read_source(&fixture, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_store(&headers);
}

#[tokio::test]
async fn source_read_missing_staging_is_not_enumerable_and_corrupt_object_fails_closed() {
    let fixture = fixture().await;
    let (missing_status, missing_headers, missing_body) =
        read_source(&fixture, Some(&fixture.owner_cookie)).await;
    assert_eq!(missing_status, StatusCode::NOT_FOUND);
    assert_no_store(&missing_headers);
    assert_eq!(missing_body, br#"{"error":"workspace not found"}"#);

    let (_, _, _) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    let staged = fixture
        .store
        .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("source lookup")
        .expect("source staging");
    fixture
        .objects
        .delete(&staged.source_record.key)
        .await
        .expect("remove staged object to inject source loss");

    let (status, headers, body) = read_source(&fixture, Some(&fixture.owner_cookie)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);
    assert_eq!(
        body,
        br#"{"error":"flat-question source changed; reload it"}"#
    );
}

#[tokio::test]
async fn source_read_refuses_object_metadata_that_no_longer_matches_staging() {
    let fixture = fixture().await;
    let (_, _, _) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    let inconsistent = Arc::new(InconsistentReadObjectStore {
        inner: Arc::clone(&fixture.objects),
    });

    let (status, headers, body) = read_source_from_app(
        fixture.app_with_objects(inconsistent),
        fixture.save_uri(),
        Some(&fixture.owner_cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);
    assert_eq!(
        body,
        br#"{"error":"flat-question source changed; reload it"}"#
    );
}

#[tokio::test]
async fn invalid_or_stale_save_never_replaces_the_owner_binding_or_enumerates_it() {
    let fixture = fixture().await;
    for source in [
        "not JSON".to_string(),
        FLAT_SOURCE.replacen("\n}", ",\"unknown\":true\n}", 1),
    ] {
        let (status, headers, body) = save(&fixture, &fixture.owner_cookie, source, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_no_store(&headers);
        assert_no_private_tokens(&body);
    }
    let (status, headers, _) = save(
        &fixture,
        &fixture.owner_cookie,
        "x".repeat(MAX_FLAT_QUESTION_BODY_BYTES + 1),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_no_store(&headers);
    assert!(
        fixture
            .store
            .get_draft(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("owner draft lookup")
            .is_none()
    );
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("owner source lookup")
            .is_none()
    );

    let (_, headers, _) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    let current_etag = headers
        .get("etag")
        .expect("initial ETag")
        .to_str()
        .expect("ETag text")
        .to_string();
    let before_draft = fixture
        .store
        .get_draft(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("owner draft lookup");
    let before_source = fixture
        .store
        .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("owner source lookup");

    let replacement = FLAT_SOURCE.replace("Favorite color", "Changed title");
    let (status, headers, body) =
        save(&fixture, &fixture.owner_cookie, replacement, Some("\"99\"")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);
    assert_no_private_tokens(&body);
    assert_eq!(
        fixture
            .store
            .get_draft(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("owner draft lookup"),
        before_draft
    );
    assert_eq!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("owner source lookup"),
        before_source
    );

    let (status, headers, body) = save(
        &fixture,
        &fixture.foreign_cookie,
        FLAT_SOURCE,
        Some(&current_etag),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_no_store(&headers);
    assert_no_private_tokens(&body);
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.foreign, fixture.workspace)
            .await
            .expect("foreign lookup")
            .is_none()
    );
}

#[tokio::test]
async fn current_publish_is_private_atomic_and_stale_publish_preserves_staging() {
    let fixture = fixture().await;
    let (_, headers, _) = save(&fixture, &fixture.owner_cookie, FLAT_SOURCE, None).await;
    let etag = headers
        .get("etag")
        .expect("save ETag")
        .to_str()
        .expect("ETag text")
        .to_string();

    let (stale_status, stale_headers, stale_body) = publish(&fixture, "\"99\"").await;
    assert_eq!(stale_status, StatusCode::CONFLICT);
    assert_no_store(&stale_headers);
    assert_no_private_tokens(&stale_body);
    let staged = fixture
        .store
        .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
        .await
        .expect("source lookup after stale publish");
    assert!(staged.is_some(), "stale publish must not consume staging");

    let (status, headers, body) = publish(&fixture, &etag).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_no_store(&headers);
    assert_no_private_tokens(&body);
    let published = published_record(&fixture, &body).await;
    assert!(
        matches!(published.question.source, QuestionSource::Native { ref family } if family == FLAT_SINGLE_CHOICE_V2_FAMILY)
    );
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    let serialized = serde_json::to_vec(&published).expect("public version serializes");
    assert_no_private_tokens(&serialized);
    assert!(
        fixture
            .store
            .get_draft(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("owner draft lookup after publish")
            .is_none()
    );
    assert!(
        fixture
            .store
            .workspace_flat_import_origin(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("manual flat origin lookup after publish")
            .is_none(),
        "a manually authored flat question remains provenance-free"
    );
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("owner source lookup after publish")
            .is_none()
    );
    let artifact = fixture
        .store
        .catalog_source_artifact(fixture.context(), reference)
        .await
        .expect("published source lookup")
        .expect("published source artifact");
    assert!(matches!(
        artifact.object.key,
        ObjectKey::ProblemSource { .. }
    ));
    let source = fixture
        .objects
        .get(&artifact.object.key)
        .await
        .expect("published source object");
    assert_eq!(source.record, artifact.object);
    assert_eq!(
        source.bytes,
        FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes())
            .expect("fixture source parses")
            .canonical_bytes()
            .expect("fixture source canonicalizes")
    );
    let grading = fixture
        .grader
        .flat_question_published_grading(fixture.context(), reference)
        .await
        .expect("grader lookup")
        .expect("grader-only material is published");
    let grading_private = grading
        .decode_private()
        .expect("grader-only material decodes");
    let grading_bytes = grading_private
        .canonical_bytes()
        .expect("grader-only material canonicalizes");
    let grading_text = String::from_utf8_lossy(&grading_bytes);
    assert!(grading_text.contains("answerKey"));
    assert!(grading_text.contains("choiceFeedback"));
    assert!(grading_text.contains("PRIVATE_CORRECT_FEEDBACK"));
    assert!(
        fixture
            .store
            .get_catalog_problem(fixture.context(), reference)
            .await
            .expect("published catalog lookup")
            .is_some()
    );
}
