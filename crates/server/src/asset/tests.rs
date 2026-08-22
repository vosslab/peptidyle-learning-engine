use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::http::Request;
use axum::middleware;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AssetDeliveryRecord, AssetDeliveryScope, AssetPublication, CatalogStore, CourseRecord,
    CourseRosterStore, CreateCourseCommand, DraftRecord, JobLeaseDuration, JobPayload, JobStore,
    PublishDraftCommand, RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime,
    SessionSubject, Store, TenantContext, UpsertCourseMember,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectCategory, ObjectKey, ObjectStoreError, PutObject, Sha256Digest};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssetId, BackendCapabilities, Capability, CourseId, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ObjectId, ProblemId, ProblemVersionRef,
    PublicationScope, QuestionMetadata, QuestionSource, TenantId, UserId, UserRole, VersionId,
    WorkspaceId, WorkspaceImportId,
};
use tower::ServiceExt;
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

#[derive(Debug, Default)]
struct SigningCounterObjectStore {
    inner: MemoryObjectStore,
    signed_url_calls: AtomicUsize,
}

impl SigningCounterObjectStore {
    fn signed_url_calls(&self) -> usize {
        self.signed_url_calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ObjectStore for SigningCounterObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        self.inner.put(request).await
    }

    async fn get(&self, key: &ObjectKey) -> Result<objects::StoredObject, ObjectStoreError> {
        self.inner.get(key).await
    }

    async fn delete(&self, key: &ObjectKey) -> Result<(), ObjectStoreError> {
        self.inner.delete(key).await
    }

    async fn signed_url(
        &self,
        key: &ObjectKey,
        now: ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.signed_url_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.signed_url(key, now).await
    }
}

fn question(_version: VersionId, workspace: WorkspaceId) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "asset-fixture".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "Identify the peptide bond.".to_string(),
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
            title: "Peptide bond".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBySa,
            language: "en-US".to_string(),
        },
    }
}

async fn publish(
    store: &MemoryStore,
    context: TenantContext,
    publisher: UserId,
    problem: ProblemId,
    version: VersionId,
    workspace: WorkspaceId,
    scope: PublicationScope,
) {
    let draft = DraftRecord {
        tenant: context.tenant_id(),
        question: question(version, workspace),
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: "asset-fixture".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_string())
                        .expect("valid test byline"),
                ])
                .expect("valid test byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish");
}

async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId) -> String {
    let subject = SessionSubject::new(tenant, user, "Asset Fixture", vec![UserRole::Student])
        .expect("session subject");
    crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("session")
    .set_cookie
    .split(';')
    .next()
    .expect("cookie pair")
    .to_string()
}

async fn fixture() -> (
    Arc<MemoryStore>,
    Router,
    String,
    String,
    AssetDeliveryId,
    AssetDeliveryId,
    AssetDeliveryId,
) {
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(id(2));
    let student = UserId::from_uuid(id(3));
    let outsider = UserId::from_uuid(id(4));
    let course = CourseId::from_uuid(id(5));
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
        .expect("clock");
    let objects = Arc::new(MemoryObjectStore::default());
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Asset route course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                initial_instructor: publisher,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Asset learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");

    let public_problem = ProblemId::from_uuid(id(10));
    let public_version = VersionId::from_uuid(id(11));
    publish(
        store.as_ref(),
        context,
        publisher,
        public_problem,
        public_version,
        WorkspaceId::from_uuid(id(12)),
        PublicationScope::Public,
    )
    .await;
    let institution_problem = ProblemId::from_uuid(id(20));
    let institution_version = VersionId::from_uuid(id(21));
    publish(
        store.as_ref(),
        context,
        publisher,
        institution_problem,
        institution_version,
        WorkspaceId::from_uuid(id(22)),
        PublicationScope::Institution,
    )
    .await;

    let public_asset = AssetId::from_uuid(id(30));
    let public_key = ObjectKey::ProblemAsset {
        problem: public_problem,
        version: public_version,
        asset: public_asset,
        object: ObjectId::from_uuid(id(31)),
    };
    let public_object = objects
        .put(PutObject {
            key: public_key,
            bytes: b"public asset".to_vec(),
            media_type: "image/svg+xml".to_string(),
            license: "CC BY-SA 4.0".to_string(),
            provenance: "test".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
        })
        .await
        .expect("public bytes");
    let public = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(public_asset),
        object: public_object,
        intrinsic_width: None,
        intrinsic_height: None,
        scope: AssetDeliveryScope::Catalog {
            asset: public_asset,
            reference: ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        },
        publication: learning_data_access::AssetPublication::Ready,
        pending_source: None,
    };

    let institution_asset = AssetId::from_uuid(id(40));
    let institution_key = ObjectKey::published_problem_asset(
        PublicationScope::Institution,
        institution_problem,
        institution_version,
        institution_asset,
        ObjectId::from_uuid(id(41)),
    );
    let institution_object = objects
        .put(PutObject {
            key: institution_key,
            bytes: b"institution asset".to_vec(),
            media_type: "image/png".to_string(),
            license: "institution".to_string(),
            provenance: "test".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
        })
        .await
        .expect("institution bytes");
    let institution = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(institution_asset),
        object: institution_object,
        intrinsic_width: None,
        intrinsic_height: None,
        scope: AssetDeliveryScope::Catalog {
            asset: institution_asset,
            reference: ProblemVersionRef {
                problem: institution_problem,
                version: institution_version,
            },
        },
        publication: learning_data_access::AssetPublication::Ready,
        pending_source: None,
    };

    let student_object_id = ObjectId::from_uuid(id(50));
    let student_object = objects
        .put(PutObject {
            key: ObjectKey::StudentRecord {
                tenant,
                object: student_object_id,
            },
            bytes: b"student export".to_vec(),
            media_type: "application/pdf".to_string(),
            license: "educational record".to_string(),
            provenance: "test export".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
        })
        .await
        .expect("student bytes");
    let student_record = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(student_object_id),
        object: student_object,
        intrinsic_width: None,
        intrinsic_height: None,
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            course,
            authorized_users: vec![student],
        },
        publication: learning_data_access::AssetPublication::Ready,
        pending_source: None,
    };

    for record in [&public, &institution, &student_record] {
        store
            .register_asset_delivery(context, record.clone())
            .await
            .expect("register delivery");
    }
    let student_cookie = cookie(store.as_ref(), tenant, student).await;
    let outsider_cookie = cookie(store.as_ref(), tenant, outsider).await;
    let app = router(
        Arc::clone(&store),
        objects,
        Arc::new(PublicAssetBaseUrl::new("https://cdn.example.test/content").expect("CDN base")),
    );
    (
        store,
        app,
        student_cookie,
        outsider_cookie,
        public.id,
        institution.id,
        student_record.id,
    )
}

async fn prepare_archive_fence(store: &MemoryStore, tenant: TenantId, course: CourseId) {
    store
        .seed_retention_cleanup_for_test(
            tenant,
            course,
            (0..4)
                .map(|offset| ObjectId::from_uuid(id(90 + offset)))
                .collect(),
        )
        .expect("archive cleanup fixture");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease duration"),
        )
        .await
        .expect("archive claim")
        .expect("archive job");
    let (claimed_course, stage, generation) = match claim.payload {
        JobPayload::Retention {
            course,
            stage,
            generation,
        } => (course, stage, generation),
        _ => panic!("fixture must claim retention work"),
    };
    assert_eq!(claimed_course, course);
    store
        .prepare_retention_work(RetentionWorkerCommand {
            tenant,
            course,
            stage,
            generation,
            job: claim.id,
            lease: claim.lease_token,
        })
        .await
        .expect("archive prepare fence");
}

#[tokio::test]
async fn public_assets_bypass_auth_and_signing_for_immutable_cdn_urls() {
    let (store, app, _, _, public, _, _) = fixture().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/assets/{public}"))
                .body(Body::empty())
                .expect("public request"),
        )
        .await
        .expect("public response");
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert!(
        response.headers()[LOCATION]
            .to_str()
            .expect("location")
            .starts_with("https://cdn.example.test/content/problems/")
    );
    assert_eq!(
        response.headers()[CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert!(response.headers().contains_key(ETAG));
    assert!(
        store
            .asset_access_events()
            .expect("audit events")
            .is_empty()
    );
}

#[tokio::test]
async fn pending_public_assets_are_concealed_before_audit_or_signing() {
    let (store, _, student_cookie, _, _, _, _) = fixture().await;
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let problem = ProblemId::from_uuid(id(10));
    let version = VersionId::from_uuid(id(11));
    let asset = AssetId::from_uuid(id(60));
    let final_object = ObjectId::from_uuid(id(61));
    let source_object = ObjectId::from_uuid(id(62));
    let pending = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(asset),
        object: ObjectRecord {
            id: final_object,
            bucket: Bucket::PublicAssets,
            sha256: Sha256Digest::compute(b"pending public target"),
            size_bytes: 21,
            media_type: "image/png".to_string(),
            category: ObjectCategory::Asset,
            version: Some(version),
            license: "CC BY-SA 4.0".to_string(),
            provenance: "pending route fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
            key: ObjectKey::ProblemAsset {
                problem,
                version,
                asset,
                object: final_object,
            },
        },
        intrinsic_width: None,
        intrinsic_height: None,
        scope: AssetDeliveryScope::Catalog {
            asset,
            reference: ProblemVersionRef { problem, version },
        },
        publication: AssetPublication::Pending,
        pending_source: Some(ObjectRecord {
            id: source_object,
            bucket: Bucket::PrivateContent,
            sha256: Sha256Digest::compute(b"private pending source"),
            size_bytes: 22,
            media_type: "image/png".to_string(),
            category: ObjectCategory::Asset,
            version: None,
            license: "CC BY-SA 4.0".to_string(),
            provenance: "pending route fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(10_000),
            key: ObjectKey::WorkspaceQuestionAsset {
                tenant,
                workspace: WorkspaceId::from_uuid(id(63)),
                asset,
                object: source_object,
            },
        }),
    };
    store
        .register_asset_delivery(context, pending.clone())
        .await
        .expect("pending public asset should register");

    let objects = Arc::new(SigningCounterObjectStore::default());
    let app = router(
        Arc::clone(&store),
        Arc::clone(&objects),
        Arc::new(PublicAssetBaseUrl::new("https://cdn.example.test/content").expect("CDN base")),
    );
    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assets/{}", pending.id))
                .body(Body::empty())
                .expect("pending GET request"),
        )
        .await
        .expect("pending GET response");
    assert_eq!(get.status(), StatusCode::NOT_FOUND);
    assert_eq!(get.headers()[CACHE_CONTROL], "no-store");

    let post = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{}/delivery", pending.id))
                .header("cookie", student_cookie)
                .body(Body::empty())
                .expect("pending delivery request"),
        )
        .await
        .expect("pending delivery response");
    assert_eq!(post.status(), StatusCode::NOT_FOUND);
    assert_eq!(post.headers()[CACHE_CONTROL], "no-store");
    let body = axum::body::to_bytes(post.into_body(), 1_024)
        .await
        .expect("pending error body");
    assert!(!String::from_utf8_lossy(&body).contains("url"));
    assert!(
        store
            .asset_access_events()
            .expect("audit events")
            .is_empty(),
        "pending delivery must be refused before an access audit"
    );
    assert_eq!(
        objects.signed_url_calls(),
        0,
        "pending delivery must be refused before object signing"
    );
}

#[tokio::test]
async fn protected_assets_are_concealed_on_get_and_post_issues_audited_capabilities() {
    let (store, app, student_cookie, outsider_cookie, public, institution, student_record) =
        fixture().await;
    let protected_get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assets/{institution}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("protected GET request"),
        )
        .await
        .expect("protected GET response");
    assert_eq!(protected_get.status(), StatusCode::NOT_FOUND);
    assert_eq!(protected_get.headers()[CACHE_CONTROL], "no-store");
    assert!(
        store
            .asset_access_events()
            .expect("audit events")
            .is_empty(),
        "GET must not authorize or append an asset-access event"
    );

    let institution_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{institution}/delivery"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("institution request"),
        )
        .await
        .expect("institution response");
    assert_eq!(institution_response.status(), StatusCode::OK);
    let institution_body = axum::body::to_bytes(institution_response.into_body(), 1024)
        .await
        .expect("institution body");
    let institution_delivery: serde_json::Value =
        serde_json::from_slice(&institution_body).expect("delivery JSON");
    assert!(
        institution_delivery["url"]
            .as_str()
            .is_some_and(|url| !url.is_empty())
    );

    let student_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{student_record}/delivery"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student record request"),
        )
        .await
        .expect("student record response");
    assert_eq!(student_response.status(), StatusCode::OK);
    let student_body = axum::body::to_bytes(student_response.into_body(), 1024)
        .await
        .expect("student body");
    let student_delivery: serde_json::Value =
        serde_json::from_slice(&student_body).expect("delivery JSON");
    assert!(
        student_delivery["url"]
            .as_str()
            .is_some_and(|url| !url.is_empty())
    );

    let hidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{student_record}/delivery"))
                .header("cookie", outsider_cookie)
                .body(Body::empty())
                .expect("hidden request"),
        )
        .await
        .expect("hidden response");
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
    let events = store.asset_access_events().expect("audit events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].delivery, institution);
    assert_eq!(events[1].delivery, student_record);
    assert!(
        events
            .iter()
            .all(|event| event.occurred_at == ActivityTimestamp::from_unix_millis(10_000))
    );

    prepare_archive_fence(
        store.as_ref(),
        TenantId::from_uuid(id(1)),
        CourseId::from_uuid(id(5)),
    )
    .await;
    let archived = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{student_record}/delivery"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived student-record request"),
        )
        .await
        .expect("archived student-record response");
    assert_eq!(archived.status(), StatusCode::NOT_FOUND);
    assert_eq!(archived.headers()[CACHE_CONTROL], "no-store");
    assert_eq!(
        store.asset_access_events().expect("audit events").len(),
        events.len(),
        "archive refusal must happen before signing authorization is audited"
    );

    let public_after_archive = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/assets/{public}"))
                .body(Body::empty())
                .expect("public asset after archive"),
        )
        .await
        .expect("public asset response");
    assert_eq!(
        public_after_archive.status(),
        StatusCode::TEMPORARY_REDIRECT
    );
    assert_eq!(
        public_after_archive.headers()[CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn production_boundary_refuses_cross_origin_delivery_before_authorization() {
    let (store, app, student_cookie, _, _, institution, _) = fixture().await;
    let production_cookie = student_cookie;
    let app = app.layer(middleware::from_fn_with_state(
        crate::auth::ProductionBrowserBoundary::new(Arc::from("https://learn.example.test"))
            .expect("production browser boundary"),
        crate::auth::production_cookie_boundary,
    ));

    let cross_origin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{institution}/delivery"))
                .header("host", "learn.example.test")
                .header("origin", "https://attacker.example.test")
                .header("cookie", &production_cookie)
                .body(Body::empty())
                .expect("cross-origin request"),
        )
        .await
        .expect("cross-origin response");
    assert_eq!(cross_origin.status(), StatusCode::FORBIDDEN);
    assert!(
        store
            .asset_access_events()
            .expect("audit events")
            .is_empty(),
        "CSRF rejection must precede protected-asset authorization"
    );

    let same_origin = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/assets/{institution}/delivery"))
                .header("host", "learn.example.test")
                .header("origin", "https://learn.example.test")
                .header("cookie", production_cookie)
                .body(Body::empty())
                .expect("same-origin request"),
        )
        .await
        .expect("same-origin response");
    assert_eq!(same_origin.status(), StatusCode::OK);
    assert_eq!(
        store.asset_access_events().expect("audit events").len(),
        1,
        "only the same-origin POST may authorize and audit delivery"
    );
}

#[test]
fn public_base_url_rejects_ambiguous_or_stateful_authorities() {
    for value in [
        "",
        "cdn.example.test/content",
        "//cdn.example.test/content",
        "javascript:alert(1)",
        "http://cdn.example.test/content",
        "https://cdn.example.test@evil.example/content",
        "https://user:password@cdn.example.test/content",
        "https://cdn.example.test/content?token=secret",
        "https://cdn.example.test/content#fragment",
        "https://cdn.example.test%2Fevil/content",
        "https://cdn.example.test/content%2Fprivate",
        "https://cdn.example.test/content/%2e%2e/private",
        "https://cdn.example.test/content//private",
        "https://cdn.example.test/content/../private",
        "https://cdn.example.test:443/content",
        "https://cdn.example.test/content ",
    ] {
        assert_eq!(PublicAssetBaseUrl::new(value), Err(PublicAssetUrlError));
    }
}

#[test]
fn public_base_url_normalizes_one_fixed_safe_path_prefix() {
    assert_eq!(
        PublicAssetBaseUrl::new("https://cdn.example.test/content/v1/")
            .expect("safe HTTPS CDN base"),
        PublicAssetBaseUrl("https://cdn.example.test/content/v1".to_string())
    );
}

fn object_record(key: ObjectKey) -> ObjectRecord {
    ObjectRecord {
        id: key.object_id(),
        bucket: key.bucket(),
        category: key.category(),
        version: key.version_id(),
        key,
        sha256: Sha256Digest::compute(b"public resolver fixture"),
        size_bytes: 23,
        media_type: "image/png".to_string(),
        license: "fixture".to_string(),
        provenance: "fixture".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    }
}

#[test]
fn public_base_url_requires_an_exact_published_problem_asset_record() {
    let resolver = PublicAssetBaseUrl::new("https://cdn.example.test/content").expect("CDN base");
    let problem = ProblemId::from_uuid(id(700));
    let version = VersionId::from_uuid(id(701));
    let public = object_record(ObjectKey::ProblemAsset {
        problem,
        version,
        asset: AssetId::from_uuid(id(702)),
        object: ObjectId::from_uuid(id(703)),
    });
    assert!(resolver.public_url(&public).is_ok());

    let tenant = TenantId::from_uuid(id(704));
    let workspace = WorkspaceId::from_uuid(id(705));
    let import = WorkspaceImportId::from_uuid(id(706));
    let rejected = [
        object_record(ObjectKey::WorkspaceSource {
            tenant,
            workspace,
            import,
            object: ObjectId::from_uuid(id(707)),
        }),
        object_record(ObjectKey::WorkspaceAsset {
            tenant,
            workspace,
            import,
            asset: AssetId::from_uuid(id(708)),
            object: ObjectId::from_uuid(id(709)),
        }),
        object_record(ObjectKey::ProblemSource {
            problem,
            version,
            object: ObjectId::from_uuid(id(710)),
        }),
        object_record(ObjectKey::RestrictedProblemAsset {
            problem,
            version,
            asset: AssetId::from_uuid(id(713)),
            object: ObjectId::from_uuid(id(714)),
        }),
        object_record(ObjectKey::ProblemRender {
            problem,
            version,
            seed: question_model::generation::Seed::new(1),
            object: ObjectId::from_uuid(id(715)),
        }),
        object_record(ObjectKey::StudentRecord {
            tenant,
            object: ObjectId::from_uuid(id(716)),
        }),
        object_record(ObjectKey::Temporary {
            object: ObjectId::from_uuid(id(717)),
        }),
    ];
    assert!(
        rejected
            .iter()
            .all(|record| resolver.public_url(record) == Err(PublicAssetUrlError))
    );

    let mut forged = public;
    forged.category = ObjectCategory::Source;
    assert_eq!(resolver.public_url(&forged), Err(PublicAssetUrlError));
}
