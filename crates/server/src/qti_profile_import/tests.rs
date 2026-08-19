use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use futures_core::Stream;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand, DraftRecord,
    JobClaimFilter, JobFailureKind, JobKind, JobLeaseDuration, JobStore,
    PersistedFlatImportProfile, QtiImportProfileSummary, QtiImportStore, QtiUnsupportedFeature,
    SessionLifetime, SessionSubject, Store, TenantContext,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectKey, ObjectStore, ObjectStoreError, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, DraftQuestionDefinition, DraftQuestionSource, GradingDefinition,
    QuestionMetadata, TenantId, UserId, UserRole, WorkspaceId, WorkspaceImportId,
};
use tower::ServiceExt;

use crate::auth::{CookieTransport, SessionConfig, issue_session};

use super::*;

struct PollTrackingStream {
    polled: Arc<AtomicBool>,
    yielded: bool,
}

impl Stream for PollTrackingStream {
    type Item = Result<axum::body::Bytes, Infallible>;

    fn poll_next(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.yielded {
            return Poll::Ready(None);
        }
        this.polled.store(true, Ordering::SeqCst);
        this.yielded = true;
        Poll::Ready(Some(Ok(axum::body::Bytes::from_static(
            b"body must stay unpolled",
        ))))
    }
}

fn poll_tracking_body() -> (Body, Arc<AtomicBool>) {
    let polled = Arc::new(AtomicBool::new(false));
    let body = Body::from_stream(PollTrackingStream {
        polled: Arc::clone(&polled),
        yielded: false,
    });
    (body, polled)
}

struct Fixture {
    store: Arc<MemoryStore>,
    objects: Arc<MemoryObjectStore>,
    tenant: TenantId,
    foreign_tenant: TenantId,
    workspace: WorkspaceId,
    owner: UserId,
    owner_cookie: String,
    stranger_cookie: String,
    foreign_cookie: String,
    student_cookie: String,
}

impl Fixture {
    fn app(&self) -> Router {
        router(Arc::clone(&self.store), Arc::clone(&self.objects))
    }

    fn uri(&self, import: WorkspaceImportId) -> String {
        format!("/api/workspaces/{}/qti-imports/{import}", self.workspace)
    }

    fn context(&self) -> TenantContext {
        TenantContext::from_authenticated_session(self.tenant)
    }

    fn key(&self, import: WorkspaceImportId) -> ObjectKey {
        let object = workspace_qti_archive_object_id(self.tenant, self.workspace, import);
        ObjectKey::WorkspaceSource {
            tenant: self.tenant,
            workspace: self.workspace,
            import,
            object,
        }
    }
}

fn id(value: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(value)
}

fn draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "qti-import-prerequisite".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "Which amino acids form a peptide bond?".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
            unit: None,
        },
        attempt_policy: AttemptPolicy { max_attempts: None },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "QTI import destination".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::AllRightsReserved,
            language: "en-US".to_string(),
        },
    }
}

async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId, role: UserRole) -> String {
    let issued = issue_session(
        store,
        SessionSubject::new(tenant, user, "QTI route fixture", vec![role])
            .expect("fixture identity"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
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
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let foreign_tenant = TenantId::from_uuid(id(2));
    let workspace = WorkspaceId::from_uuid(id(3));
    let owner = UserId::from_uuid(id(4));
    let stranger = UserId::from_uuid(id(5));
    let foreign = UserId::from_uuid(id(6));
    let student = UserId::from_uuid(id(7));
    store
        .upsert_draft(
            TenantContext::from_authenticated_session(tenant),
            owner,
            None,
            DraftRecord {
                tenant,
                question: draft(workspace),
                derived_from: None,
            },
        )
        .await
        .expect("owner-visible draft");
    Fixture {
        owner_cookie: cookie(&store, tenant, owner, UserRole::Instructor).await,
        stranger_cookie: cookie(&store, tenant, stranger, UserRole::Instructor).await,
        foreign_cookie: cookie(&store, foreign_tenant, foreign, UserRole::Instructor).await,
        student_cookie: cookie(&store, tenant, student, UserRole::Student).await,
        store,
        objects: Arc::new(MemoryObjectStore::default()),
        tenant,
        foreign_tenant,
        workspace,
        owner,
    }
}

async fn response_parts(response: Response) -> (StatusCode, HeaderMap, Vec<u8>) {
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded response body")
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

async fn put(
    fixture: &Fixture,
    cookie: Option<&str>,
    import: WorkspaceImportId,
    media_type: &str,
    bytes: &'static [u8],
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method("PUT")
        .uri(fixture.uri(import))
        .header("content-type", media_type);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = fixture
        .app()
        .oneshot(request.body(Body::from(bytes)).expect("QTI upload request"))
        .await
        .expect("QTI upload response");
    response_parts(response).await
}

async fn get(
    fixture: &Fixture,
    cookie: &str,
    import: WorkspaceImportId,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = fixture
        .app()
        .oneshot(
            Request::builder()
                .uri(fixture.uri(import))
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("QTI status request"),
        )
        .await
        .expect("QTI status response");
    response_parts(response).await
}

async fn assert_no_archive_or_job(fixture: &Fixture, import: WorkspaceImportId) {
    assert_eq!(
        fixture.objects.get(&fixture.key(import)).await,
        Err(ObjectStoreError::NotFound)
    );
    assert!(
        fixture
            .store
            .qti_import_view(fixture.context(), fixture.owner, fixture.workspace, import)
            .await
            .expect("QTI view lookup")
            .is_none()
    );
}

fn qti_filter() -> JobClaimFilter {
    JobClaimFilter::new([JobKind::QtiImport]).expect("QTI job filter")
}

async fn claim_import(fixture: &Fixture) -> learning_data_access::ClaimedJob {
    fixture
        .store
        .claim_next_job(
            &qti_filter(),
            JobLeaseDuration::from_seconds(60).expect("bounded lease"),
        )
        .await
        .expect("claim query")
        .expect("queued QTI import")
}

fn fixed_defaults() -> Vec<QtiUnsupportedFeature> {
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

async fn commit_all_rejected_blackboard_profile_import(
    fixture: &Fixture,
    import: WorkspaceImportId,
    claim: learning_data_access::ClaimedJob,
) {
    let source = fixture
        .objects
        .get(&fixture.key(import))
        .await
        .expect("uploaded archive")
        .record;
    let reference = QtiImportRef {
        tenant: fixture.tenant,
        workspace: fixture.workspace,
        import,
    };
    let diagnostic = QtiUnsupportedFeature {
        code: "markup".to_string(),
        location: "prompt".to_string(),
        detail: "Unsupported markup prevents import.".to_string(),
    };
    let profile_summary = QtiImportProfileSummary::new(
        PersistedFlatImportProfile::BlackboardQti21V1,
        Sha256Digest::compute(b"safe visible all-rejected report"),
        fixed_defaults(),
    )
    .expect("recognized profile summary");
    fixture
        .store
        .prepare_qti_import(
            fixture.context(),
            CreateQtiImportCommand {
                registry: QtiImportRegistry {
                    reference,
                    source,
                    source_format: "qti".to_string(),
                    source_identifier: None,
                    importer: "adapter_qti".to_string(),
                    parse_schema: profile_summary.profile_id().to_string(),
                    adapter_version: "test-v1".to_string(),
                    profile_summary: Some(profile_summary),
                    items: Vec::new(),
                    item_results: vec![learning_data_access::QtiImportItemResult {
                        source_identifier: "canvas-item-rejected".to_string(),
                        title: Some("Unsupported source item".to_string()),
                        item_id: None,
                        normalized_sha256: None,
                        status: QtiImportItemStatus::Rejected,
                        diagnostics: vec![diagnostic],
                        defaults: vec![QtiUnsupportedFeature {
                            code: "points".to_string(),
                            location: "points".to_string(),
                            detail: "Blackboard item points were absent; PLE default 1.0 applied."
                                .to_string(),
                        }],
                        warnings: Vec::new(),
                    }],
                    assets: Vec::new(),
                    unsupported_features: Vec::new(),
                },
                item_bindings: Vec::new(),
            },
        )
        .await
        .expect("recognized import preparation");
    assert_eq!(
        fixture
            .store
            .commit_prepared_qti_import(
                fixture.context(),
                CommitPreparedQtiImport {
                    job: claim.id,
                    lease: claim.lease_token,
                    reference,
                    source_object: reference_source_object(reference),
                },
            )
            .await
            .expect("recognized import commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
}

fn reference_source_object(reference: QtiImportRef) -> question_model::ObjectId {
    workspace_qti_archive_object_id(reference.tenant, reference.workspace, reference.import)
}

#[tokio::test]
async fn upload_refusals_authorize_before_mutating_and_always_disable_caching() {
    let fixture = fixture().await;
    let unauthenticated = WorkspaceImportId::from_uuid(id(101));
    let inaccessible = WorkspaceImportId::from_uuid(id(102));
    let wrong_media = WorkspaceImportId::from_uuid(id(103));
    let student = WorkspaceImportId::from_uuid(id(104));

    let (status, headers, _) = put(
        &fixture,
        None,
        unauthenticated,
        "application/zip",
        b"private body",
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_no_store(&headers);
    assert_no_archive_or_job(&fixture, unauthenticated).await;

    let (status, headers, _) = put(
        &fixture,
        Some(&fixture.stranger_cookie),
        inaccessible,
        "application/zip",
        b"private body",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_no_store(&headers);
    assert_no_archive_or_job(&fixture, inaccessible).await;

    let (status, headers, _) = put(
        &fixture,
        Some(&fixture.owner_cookie),
        wrong_media,
        "application/zip; charset=binary",
        b"private body",
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_no_store(&headers);
    assert_no_archive_or_job(&fixture, wrong_media).await;

    let (status, headers, _) = put(
        &fixture,
        Some(&fixture.student_cookie),
        student,
        "application/zip",
        b"private body",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_no_store(&headers);
    assert_no_archive_or_job(&fixture, student).await;
}

#[tokio::test]
async fn authentication_and_workspace_refusals_do_not_poll_upload_bodies() {
    let fixture = fixture().await;
    let cases = [
        (
            None,
            WorkspaceImportId::from_uuid(id(111)),
            StatusCode::UNAUTHORIZED,
        ),
        (
            Some(fixture.stranger_cookie.as_str()),
            WorkspaceImportId::from_uuid(id(112)),
            StatusCode::NOT_FOUND,
        ),
    ];

    for (cookie, import, expected_status) in cases {
        let (body, polled) = poll_tracking_body();
        let mut request = Request::builder()
            .method("PUT")
            .uri(fixture.uri(import))
            .header("content-type", "application/zip");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = fixture
            .app()
            .oneshot(request.body(body).expect("tracked upload request"))
            .await
            .expect("tracked upload response");

        assert_eq!(response.status(), expected_status);
        assert_no_store(response.headers());
        assert!(!polled.load(Ordering::SeqCst));
    }
}

#[tokio::test]
async fn upload_exact_replay_keeps_one_job_and_divergent_bytes_conflict() {
    let fixture = fixture().await;
    let import = WorkspaceImportId::from_uuid(id(201));
    const ARCHIVE: &[u8] = b"PK\x03\x04deterministic QTI archive";

    for _ in 0..2 {
        let (status, headers, body) = put(
            &fixture,
            Some(&fixture.owner_cookie),
            import,
            "application/zip",
            ARCHIVE,
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_no_store(&headers);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).expect("processing receipt")["state"],
            "queued"
        );
        fixture
            .store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
            .expect("advance clock before immutable replay");
    }
    assert_eq!(
        fixture
            .store
            .ready_queue_depth(&qti_filter())
            .await
            .expect("queue depth")
            .ready,
        1
    );
    let stored = fixture
        .objects
        .get(&fixture.key(import))
        .await
        .expect("uploaded archive");
    assert_eq!(stored.bytes, ARCHIVE);
    assert_eq!(stored.record.license, WORKSPACE_ARCHIVE_LICENSE);
    assert_eq!(stored.record.provenance, WORKSPACE_ARCHIVE_PROVENANCE);

    let (status, headers, _) = put(
        &fixture,
        Some(&fixture.owner_cookie),
        import,
        "application/zip",
        b"PK\x03\x04different QTI archive",
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);
    assert_eq!(
        fixture
            .objects
            .get(&fixture.key(import))
            .await
            .expect("original archive remains")
            .bytes,
        ARCHIVE
    );
}

#[tokio::test]
async fn status_reports_queued_failed_and_recognized_ready_without_private_material() {
    let queued_fixture = fixture().await;
    let queued_import = WorkspaceImportId::from_uuid(id(301));
    assert_eq!(
        put(
            &queued_fixture,
            Some(&queued_fixture.owner_cookie),
            queued_import,
            "application/zip",
            b"PK\x03\x04queued status fixture",
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let (status, headers, queued_body) =
        get(&queued_fixture, &queued_fixture.owner_cookie, queued_import).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_no_store(&headers);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&queued_body).expect("queued JSON")["state"],
        "queued"
    );

    let failed_fixture = fixture().await;
    let failed_import = WorkspaceImportId::from_uuid(id(302));
    assert_eq!(
        put(
            &failed_fixture,
            Some(&failed_fixture.owner_cookie),
            failed_import,
            "application/zip",
            b"PK\x03\x04failed status fixture",
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let failed_claim = claim_import(&failed_fixture).await;
    assert_eq!(
        failed_claim.payload,
        learning_data_access::JobPayload::QtiImport {
            workspace: failed_fixture.workspace,
            import: failed_import,
            source_object: reference_source_object(QtiImportRef {
                tenant: failed_fixture.tenant,
                workspace: failed_fixture.workspace,
                import: failed_import,
            }),
        }
    );
    let (status, headers, processing_body) =
        get(&failed_fixture, &failed_fixture.owner_cookie, failed_import).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_no_store(&headers);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&processing_body).expect("processing JSON")["state"],
        "processing"
    );
    failed_fixture
        .store
        .fail_job(
            failed_claim.id,
            failed_claim.lease_token,
            JobFailureKind::Permanent,
        )
        .await
        .expect("permanent import failure");
    let (status, headers, failed_body) =
        get(&failed_fixture, &failed_fixture.owner_cookie, failed_import).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_no_store(&headers);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&failed_body).expect("failed JSON")["state"],
        "failed"
    );

    let ready_fixture = fixture().await;
    let ready_import = WorkspaceImportId::from_uuid(id(303));
    assert_eq!(
        put(
            &ready_fixture,
            Some(&ready_fixture.owner_cookie),
            ready_import,
            "application/zip",
            b"PK\x03\x04ready status fixture PRIVATE_VENDOR_FEEDBACK",
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let ready_claim = claim_import(&ready_fixture).await;
    assert_eq!(
        ready_claim.payload,
        learning_data_access::JobPayload::QtiImport {
            workspace: ready_fixture.workspace,
            import: ready_import,
            source_object: reference_source_object(QtiImportRef {
                tenant: ready_fixture.tenant,
                workspace: ready_fixture.workspace,
                import: ready_import,
            }),
        }
    );
    commit_all_rejected_blackboard_profile_import(&ready_fixture, ready_import, ready_claim).await;
    let (status, headers, ready_body) =
        get(&ready_fixture, &ready_fixture.owner_cookie, ready_import).await;
    assert_eq!(status, StatusCode::OK);
    assert_no_store(&headers);
    let report: serde_json::Value = serde_json::from_slice(&ready_body).expect("ready report JSON");
    assert_eq!(report["state"], "ready");
    assert_eq!(
        report["profileId"],
        "blackboard-qti-2.1-static-single-choice-pool/v1"
    );
    assert_eq!(report["items"][0]["status"], "rejected");
    assert!(
        report["items"][0]["defaults"]
            .as_array()
            .is_some_and(|defaults| defaults.iter().any(|value| {
                value["detail"] == "Blackboard item points were absent; PLE default 1.0 applied."
            })),
        "the persisted item-specific Blackboard points default remains visible"
    );
    assert!(
        report["pleDefaults"]
            .as_array()
            .is_some_and(|defaults| !defaults.is_empty()),
        "recognized all-rejected packages retain visible conversion defaults"
    );
    assert!(report["reportRevision"].is_string());
    assert!(report["reviewToken"].is_string());
    let view = ready_fixture
        .store
        .qti_import_view(
            ready_fixture.context(),
            ready_fixture.owner,
            ready_fixture.workspace,
            ready_import,
        )
        .await
        .expect("ready report view")
        .expect("ready import remains visible");
    let registry = view.registry.expect("ready registry");
    let original = qti_profile_report_acknowledgement(ready_import, &registry)
        .expect("recognized report acknowledgement");
    let mut changed = registry;
    changed.item_results[0].defaults[0].detail =
        "Blackboard item points were absent; a different visible default applies.".to_string();
    let changed = qti_profile_report_acknowledgement(ready_import, &changed)
        .expect("changed visible report acknowledgement");
    assert_ne!(original.report_revision(), changed.report_revision());
    assert_ne!(original.review_token(), changed.review_token());
    let serialized = String::from_utf8(ready_body).expect("report is UTF-8 JSON");
    for forbidden in [
        "sourceObject",
        "packageObject",
        "modelSha256",
        "normalizedSha256",
        "profileReportSha256",
        "correctChoice",
        "choiceMap",
        "grading",
        "PRIVATE_VENDOR_FEEDBACK",
        "PK\\u0003\\u0004",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "ready report leaked forbidden material {forbidden}: {serialized}"
        );
    }
}

#[tokio::test]
async fn generic_ready_is_honestly_unsupported_and_import_lookups_do_not_enumerate() {
    let fixture = fixture().await;
    let import = WorkspaceImportId::from_uuid(id(401));
    assert_eq!(
        put(
            &fixture,
            Some(&fixture.owner_cookie),
            import,
            "application/zip",
            b"PK\x03\x04generic fixture",
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let claim = claim_import(&fixture).await;
    let source = fixture
        .objects
        .get(&fixture.key(import))
        .await
        .expect("generic archive")
        .record;
    let reference = QtiImportRef {
        tenant: fixture.tenant,
        workspace: fixture.workspace,
        import,
    };
    fixture
        .store
        .prepare_qti_import(
            fixture.context(),
            CreateQtiImportCommand {
                registry: QtiImportRegistry {
                    reference,
                    source,
                    source_format: "qti".to_string(),
                    source_identifier: None,
                    importer: "adapter_qti".to_string(),
                    parse_schema: "ple-qti-assessment-item-single-choice/v1".to_string(),
                    adapter_version: "test-v1".to_string(),
                    profile_summary: None,
                    items: Vec::new(),
                    item_results: vec![learning_data_access::QtiImportItemResult {
                        source_identifier: "generic-item".to_string(),
                        title: None,
                        item_id: None,
                        normalized_sha256: None,
                        status: QtiImportItemStatus::Rejected,
                        diagnostics: vec![QtiUnsupportedFeature {
                            code: "unsupported-interaction".to_string(),
                            location: "item".to_string(),
                            detail: "This item is outside the generic subset.".to_string(),
                        }],
                        defaults: Vec::new(),
                        warnings: Vec::new(),
                    }],
                    assets: Vec::new(),
                    unsupported_features: Vec::new(),
                },
                item_bindings: Vec::new(),
            },
        )
        .await
        .expect("generic import preparation");
    assert_eq!(
        fixture
            .store
            .commit_prepared_qti_import(
                fixture.context(),
                CommitPreparedQtiImport {
                    job: claim.id,
                    lease: claim.lease_token,
                    reference,
                    source_object: reference_source_object(reference),
                },
            )
            .await
            .expect("generic import commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let (status, _, body) = get(&fixture, &fixture.owner_cookie, import).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("unsupported profile JSON")["state"],
        "unsupportedProfile"
    );

    let missing = WorkspaceImportId::from_uuid(id(402));
    let owner_missing = get(&fixture, &fixture.owner_cookie, missing).await;
    let inaccessible = get(&fixture, &fixture.stranger_cookie, import).await;
    let foreign = get(&fixture, &fixture.foreign_cookie, import).await;
    assert_eq!(owner_missing.0, StatusCode::NOT_FOUND);
    assert_eq!(inaccessible.0, owner_missing.0);
    assert_eq!(foreign.0, owner_missing.0);
    assert_eq!(inaccessible.2, owner_missing.2);
    assert_eq!(foreign.2, owner_missing.2);
    assert_no_store(&owner_missing.1);
    assert_no_store(&inaccessible.1);
    assert_no_store(&foreign.1);
    assert_ne!(fixture.foreign_tenant, fixture.tenant);
}

#[tokio::test]
async fn malformed_path_extractor_response_is_no_store() {
    let fixture = fixture().await;
    let response = fixture
        .app()
        .oneshot(
            Request::builder()
                .uri("/api/workspaces/not-a-uuid/qti-imports/not-a-uuid")
                .header("cookie", &fixture.owner_cookie)
                .body(Body::empty())
                .expect("malformed path request"),
        )
        .await
        .expect("malformed path response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_no_store(response.headers());
}
