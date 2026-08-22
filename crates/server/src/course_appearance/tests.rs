use std::sync::atomic::{AtomicUsize, Ordering};

use ::image::codecs::png::{CompressionType, FilterType, PngEncoder};
use ::image::{ExtendedColorType, GenericImageView, ImageEncoder, ImageFormat, Rgb, RgbImage};
use async_trait::async_trait;
use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, COOKIE};
use axum::http::{Method, Request};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    AuthoritativeTimeStore, CourseAppearanceStore, CourseBannerCleanupBatch, CourseRecord,
    CourseRosterStore, CreateCourseCommand, RegisterCourseBannerCandidate, SaveCourseAppearance,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, TenantContext,
    UpsertCourseMember,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectStore, ObjectStoreError, PutObject, SignedUrl, StoredObject};
use question_model::{
    CourseAppearance, CourseAppearanceUpdate, CourseBannerAlternativeText,
    CourseBannerCandidateReceipt, CourseBannerMutation, CourseId, CourseThemeId, TenantId, UserId,
    UserRole,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::*;

struct Fixture<O> {
    store: Arc<MemoryStore>,
    objects: Arc<O>,
    app: Router,
    tenant: TenantId,
    course: CourseId,
    instructor_cookie: String,
    student_cookie: String,
    outsider_cookie: String,
}

async fn fixture_with_objects<O>(objects: Arc<O>) -> Fixture<O>
where
    O: ObjectStore + 'static,
{
    let store = Arc::new(MemoryStore::default());
    let tenant = TenantId::from_uuid(Uuid::from_u128(81_001));
    let course = CourseId::from_uuid(Uuid::from_u128(81_002));
    let instructor = UserId::from_uuid(Uuid::from_u128(81_003));
    let student = UserId::from_uuid(Uuid::from_u128(81_004));
    let outsider = UserId::from_uuid(Uuid::from_u128(81_005));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Keyboard-accessible biochemistry".to_string(),
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
        .expect("course fixture should persist");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Appearance learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("student roster membership");
    let instructor_cookie = session_cookie(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        "Instructor",
    )
    .await;
    let student_cookie =
        session_cookie(&store, tenant, student, vec![UserRole::Student], "Student").await;
    let outsider_cookie = session_cookie(
        &store,
        tenant,
        outsider,
        vec![UserRole::Instructor],
        "Outsider",
    )
    .await;
    let app = super::router(Arc::clone(&store), Arc::clone(&objects));
    Fixture {
        store,
        objects,
        app,
        tenant,
        course,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
    }
}

async fn fixture() -> Fixture<MemoryObjectStore> {
    fixture_with_objects(Arc::new(MemoryObjectStore::default())).await
}

async fn session_cookie(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
    label: &str,
) -> String {
    let issued = crate::auth::issue_session(
        store,
        SessionSubject::new(tenant, user, label, roles).expect("session subject should validate"),
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3 * 3_600).expect("session lifetime should validate"),
            crate::auth::CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("session should issue");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair should exist")
        .to_string()
}

fn appearance_uri(course: CourseId) -> String {
    format!("/api/courses/{course}/appearance")
}

fn candidate_uri(course: CourseId) -> String {
    format!("/api/courses/{course}/appearance/banner-candidates")
}

fn request(method: Method, uri: String, cookie: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(COOKIE, cookie)
        .body(body.into())
        .expect("request should build")
}

fn banner_png(color: Rgb<u8>) -> Vec<u8> {
    let image = RgbImage::from_pixel(1_200, 328, color);
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::Sub)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .expect("PNG fixture should encode");
    bytes
}

async fn upload_candidate<O>(
    fixture: &Fixture<O>,
    cookie: &str,
    bytes: Vec<u8>,
) -> (StatusCode, Option<CourseBannerCandidateReceipt>)
where
    O: ObjectStore + 'static,
{
    let mut request = request(Method::POST, candidate_uri(fixture.course), cookie, bytes);
    request
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    let response = fixture
        .app
        .clone()
        .oneshot(request)
        .await
        .expect("candidate request should run");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1_024)
        .await
        .expect("candidate response should be bounded");
    let receipt = (status == StatusCode::CREATED)
        .then(|| serde_json::from_slice(&body).expect("receipt should decode"));
    (status, receipt)
}

async fn save<O>(
    fixture: &Fixture<O>,
    cookie: &str,
    revision: &str,
    update: &CourseAppearanceUpdate,
) -> axum::response::Response
where
    O: ObjectStore + 'static,
{
    let mut request = request(
        Method::PUT,
        appearance_uri(fixture.course),
        cookie,
        serde_json::to_vec(update).expect("update should encode"),
    );
    request
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    request.headers_mut().insert(
        IF_MATCH,
        HeaderValue::from_str(revision).expect("valid fixture ETag"),
    );
    fixture
        .app
        .clone()
        .oneshot(request)
        .await
        .expect("save should run")
}

async fn response_json<T: serde::de::DeserializeOwned>(response: Response) -> T {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("JSON response should be bounded");
    serde_json::from_slice(&body).expect("response should decode")
}

async fn assert_author_atomic_flow_student_read_and_current_only_delivery<O>(fixture: Fixture<O>)
where
    O: ObjectStore + 'static,
{
    let initial_request = request(
        Method::GET,
        appearance_uri(fixture.course),
        &fixture.instructor_cookie,
        Body::empty(),
    );
    let initial_response = fixture
        .app
        .clone()
        .oneshot(initial_request)
        .await
        .expect("initial read should run");
    assert_eq!(initial_response.status(), StatusCode::OK);
    assert_eq!(initial_response.headers()[ETAG], "\"1\"");
    assert_eq!(initial_response.headers()[CACHE_CONTROL], "no-store");
    let initial: CourseAppearance = response_json(initial_response).await;
    assert_eq!(initial.theme, CourseThemeId::Grass);

    let (_, first_receipt) = upload_candidate(
        &fixture,
        &fixture.instructor_cookie,
        banner_png(Rgb([12, 34, 56])),
    )
    .await;
    let first_receipt = first_receipt.expect("valid author upload should return a receipt");
    let first_update = CourseAppearanceUpdate {
        theme: CourseThemeId::Forest,
        banner: CourseBannerMutation::Replace {
            candidate: first_receipt.candidate,
            alternative_text: CourseBannerAlternativeText::Decorative,
        },
    };
    let first_response = save(&fixture, &fixture.instructor_cookie, "\"1\"", &first_update).await;
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(first_response.headers()[ETAG], "\"2\"");
    let first: CourseAppearance = response_json(first_response).await;
    let first_banner = first.banner.expect("replacement should select a banner").id;

    let stored = fixture
        .objects
        .get(&ObjectKey::CourseBanner {
            tenant: fixture.tenant,
            course: fixture.course,
            banner: first_banner,
        })
        .await
        .expect("normalized current object should exist");
    let decoded = ::image::load_from_memory_with_format(&stored.bytes, ImageFormat::WebP)
        .expect("current object should be WebP");
    assert_eq!(decoded.dimensions(), (1_200, 328));

    let student_read = fixture
        .app
        .clone()
        .oneshot(request(
            Method::GET,
            appearance_uri(fixture.course),
            &fixture.student_cookie,
            Body::empty(),
        ))
        .await
        .expect("student read should run");
    assert_eq!(student_read.status(), StatusCode::OK);
    assert_eq!(student_read.headers()[ETAG], "\"2\"");

    let public_assets = Arc::new(
        crate::asset::PublicAssetBaseUrl::new("https://cdn.example.test/content")
            .expect("public asset base should validate"),
    );
    let asset_app = crate::asset::router(
        Arc::clone(&fixture.store),
        Arc::clone(&fixture.objects),
        public_assets,
    );
    let concealed_get = asset_app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/assets/{first_banner}"),
            &fixture.student_cookie,
            Body::empty(),
        ))
        .await
        .expect("protected GET should run");
    assert_eq!(concealed_get.status(), StatusCode::NOT_FOUND);
    assert_eq!(concealed_get.headers()[CACHE_CONTROL], "no-store");
    assert!(
        fixture
            .store
            .asset_access_events()
            .expect("asset access audit should be readable")
            .is_empty(),
        "a protected GET must not authorize or append an audit event"
    );

    let current_delivery = fixture
        .app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/course-banners/{first_banner}/delivery"),
            &fixture.student_cookie,
            Body::empty(),
        ))
        .await
        .expect("current protected delivery should run");
    assert_eq!(current_delivery.status(), StatusCode::OK);
    assert_eq!(current_delivery.headers()[CACHE_CONTROL], "no-store");
    assert_eq!(current_delivery.headers()[CONTENT_TYPE], BANNER_MEDIA_TYPE);
    assert_eq!(
        current_delivery.headers()["x-content-type-options"],
        "nosniff"
    );
    assert_eq!(
        current_delivery.headers()["cross-origin-resource-policy"],
        "same-origin"
    );
    assert_eq!(current_delivery.headers()[REFERRER_POLICY], "no-referrer");
    let delivered = to_bytes(current_delivery.into_body(), MAX_BANNER_UPLOAD_BYTES)
        .await
        .expect("protected banner bytes should be bounded");
    assert_eq!(delivered.as_ref(), stored.bytes.as_slice());
    assert_eq!(
        fixture
            .store
            .asset_access_events()
            .expect("asset access audit should be readable")
            .len(),
        1,
        "only the explicit protected delivery POST should audit access"
    );

    let stale = save(
        &fixture,
        &fixture.instructor_cookie,
        "\"1\"",
        &CourseAppearanceUpdate {
            theme: CourseThemeId::Desert,
            banner: CourseBannerMutation::Remove,
        },
    )
    .await;
    assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED);
    let after_stale = fixture
        .app
        .clone()
        .oneshot(request(
            Method::GET,
            appearance_uri(fixture.course),
            &fixture.instructor_cookie,
            Body::empty(),
        ))
        .await
        .expect("read after stale save should run");
    let after_stale: CourseAppearance = response_json(after_stale).await;
    assert_eq!(after_stale.theme, CourseThemeId::Forest);
    assert_eq!(
        after_stale.banner.expect("old banner remains").id,
        first_banner
    );

    let (_, second_receipt) = upload_candidate(
        &fixture,
        &fixture.instructor_cookie,
        banner_png(Rgb([90, 80, 70])),
    )
    .await;
    let replacement = save(
        &fixture,
        &fixture.instructor_cookie,
        "\"2\"",
        &CourseAppearanceUpdate {
            theme: CourseThemeId::Forest,
            banner: CourseBannerMutation::Replace {
                candidate: second_receipt.expect("second receipt").candidate,
                alternative_text: CourseBannerAlternativeText::Decorative,
            },
        },
    )
    .await;
    assert_eq!(replacement.status(), StatusCode::OK);
    let replacement: CourseAppearance = response_json(replacement).await;
    let second_banner = replacement
        .banner
        .expect("replacement should select the second banner")
        .id;

    let superseded = asset_app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/assets/{first_banner}"),
            &fixture.student_cookie,
            Body::empty(),
        ))
        .await
        .expect("superseded protected GET should run");
    assert_eq!(superseded.status(), StatusCode::NOT_FOUND);

    let superseded_delivery = fixture
        .app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/course-banners/{first_banner}/delivery"),
            &fixture.student_cookie,
            Body::empty(),
        ))
        .await
        .expect("superseded protected delivery should run");
    assert_eq!(superseded_delivery.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        fixture
            .store
            .asset_access_events()
            .expect("asset access audit should be readable")
            .len(),
        1,
        "a superseded protected delivery must fail before appending an audit event"
    );

    fixture
        .store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(
            CANDIDATE_LIFETIME_MILLIS + 1,
        ))
        .expect("fixture clock should pass candidate expiry");
    for _ in 0..2 {
        let cleanup_trigger = fixture
            .app
            .clone()
            .oneshot(request(
                Method::GET,
                appearance_uri(fixture.course),
                &fixture.student_cookie,
                Body::empty(),
            ))
            .await
            .expect("authorized read should trigger bounded cleanup");
        assert_eq!(cleanup_trigger.status(), StatusCode::OK);
    }
    assert_eq!(
        fixture
            .objects
            .get(&ObjectKey::CourseBannerCandidate {
                tenant: fixture.tenant,
                course: fixture.course,
                candidate: first_receipt.candidate,
            })
            .await,
        Err(ObjectStoreError::NotFound)
    );
    assert_eq!(
        fixture
            .objects
            .get(&ObjectKey::CourseBanner {
                tenant: fixture.tenant,
                course: fixture.course,
                banner: first_banner,
            })
            .await,
        Err(ObjectStoreError::NotFound)
    );
    fixture
        .objects
        .get(&ObjectKey::CourseBanner {
            tenant: fixture.tenant,
            course: fixture.course,
            banner: second_banner,
        })
        .await
        .expect("cleanup must preserve the exact current promoted object");
}

#[tokio::test]
async fn author_atomic_flow_student_read_and_current_only_delivery_conform() {
    assert_author_atomic_flow_student_read_and_current_only_delivery(fixture().await).await;
}

#[tokio::test]
#[ignore = "requires the disposable MinIO course-appearance acceptance stack"]
async fn minio_author_atomic_flow_student_read_and_current_only_delivery_conform() {
    let objects = minio_objects();

    assert_author_atomic_flow_student_read_and_current_only_delivery(
        fixture_with_objects(objects).await,
    )
    .await;
}

fn minio_objects() -> Arc<objects::s3::S3ObjectStore> {
    use objects::minio::{EndpointConfig, client};
    use objects::s3::{BucketNames, S3ObjectStore};

    let settings = EndpointConfig {
        endpoint_url: std::env::var("PLE_S3_ENDPOINT").expect("PLE_S3_ENDPOINT must be set"),
        region: std::env::var("PLE_S3_REGION").expect("PLE_S3_REGION must be set"),
        access_key_id: std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set"),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
            .expect("AWS_SECRET_ACCESS_KEY must be set"),
    };
    Arc::new(S3ObjectStore::new(
        client(&settings),
        BucketNames::default(),
    ))
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL and MinIO course-appearance acceptance stack"]
async fn postgres_minio_cleanup_deletes_superseded_objects_and_preserves_current() {
    use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};

    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = Arc::new(PostgresStore::new(pool));
    let objects = minio_objects();
    let tenant = TenantId::from_uuid(Uuid::from_u128(82_001));
    let course = CourseId::from_uuid(Uuid::from_u128(82_002));
    let instructor = UserId::from_uuid(Uuid::from_u128(82_003));
    let student = UserId::from_uuid(Uuid::from_u128(82_004));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor_session = SessionTokenHash::compute(b"combined-live-instructor");
    let student_session = SessionTokenHash::compute(b"combined-live-student");
    for (session, user, roles) in [
        (instructor_session, instructor, vec![UserRole::Instructor]),
        (student_session, student, vec![UserRole::Student]),
    ] {
        store
            .create_session(
                session,
                SessionSubject::new(tenant, user, "Combined live cleanup", roles)
                    .expect("valid live session subject"),
                SessionLifetime::from_seconds(3_600).expect("valid live session lifetime"),
            )
            .await
            .expect("live session should persist");
    }
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Combined PostgreSQL and MinIO cleanup".to_string(),
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
        .expect("live course should persist");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "Combined cleanup learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("live student roster membership");
    let now = store
        .authoritative_time(context)
        .await
        .expect("live database time should resolve");
    let expires_at = ActivityTimestamp::from_unix_millis(
        now.as_unix_millis()
            .checked_add(3_000)
            .expect("live fixture expiry should fit"),
    );

    let first_candidate = CourseBannerCandidateId::generate();
    let first_banner = CourseBannerId::generate();
    let first_candidate_key = ObjectKey::CourseBannerCandidate {
        tenant,
        course,
        candidate: first_candidate,
    };
    let first_banner_key = ObjectKey::CourseBanner {
        tenant,
        course,
        banner: first_banner,
    };
    let first_bytes = b"first normalized live banner".to_vec();
    let first_candidate_object = objects
        .put(PutObject {
            key: first_candidate_key.clone(),
            bytes: first_bytes.clone(),
            media_type: BANNER_MEDIA_TYPE.to_string(),
            license: BANNER_LICENSE.to_string(),
            provenance: CANDIDATE_PROVENANCE.to_string(),
            created_at: now,
        })
        .await
        .expect("first candidate bytes should persist in MinIO");
    store
        .register_course_banner_candidate(
            context,
            instructor_session,
            course,
            RegisterCourseBannerCandidate {
                candidate: first_candidate,
                object: first_candidate_object,
                banner: first_banner,
                width: learning_data_access::COURSE_BANNER_WIDTH,
                height: learning_data_access::COURSE_BANNER_HEIGHT,
                expires_at,
            },
        )
        .await
        .expect("first PostgreSQL candidate should persist");
    let first_promoted = objects
        .put(PutObject {
            key: first_banner_key.clone(),
            bytes: first_bytes,
            media_type: BANNER_MEDIA_TYPE.to_string(),
            license: BANNER_LICENSE.to_string(),
            provenance: PROMOTED_PROVENANCE.to_string(),
            created_at: now,
        })
        .await
        .expect("first promoted bytes should persist in MinIO");
    let first_saved = store
        .save_course_appearance(
            context,
            instructor_session,
            course,
            SaveCourseAppearance {
                expected_revision: question_model::CourseAppearanceRevision::INITIAL,
                update: CourseAppearanceUpdate {
                    theme: CourseThemeId::Forest,
                    banner: CourseBannerMutation::Replace {
                        candidate: first_candidate,
                        alternative_text: CourseBannerAlternativeText::Decorative,
                    },
                },
                promoted_object: Some(first_promoted),
            },
        )
        .await
        .expect("first current banner should persist");

    let second_candidate = CourseBannerCandidateId::generate();
    let second_banner = CourseBannerId::generate();
    let second_candidate_key = ObjectKey::CourseBannerCandidate {
        tenant,
        course,
        candidate: second_candidate,
    };
    let second_banner_key = ObjectKey::CourseBanner {
        tenant,
        course,
        banner: second_banner,
    };
    let second_bytes = b"second normalized live banner".to_vec();
    let second_candidate_object = objects
        .put(PutObject {
            key: second_candidate_key.clone(),
            bytes: second_bytes.clone(),
            media_type: BANNER_MEDIA_TYPE.to_string(),
            license: BANNER_LICENSE.to_string(),
            provenance: CANDIDATE_PROVENANCE.to_string(),
            created_at: now,
        })
        .await
        .expect("second candidate bytes should persist in MinIO");
    store
        .register_course_banner_candidate(
            context,
            instructor_session,
            course,
            RegisterCourseBannerCandidate {
                candidate: second_candidate,
                object: second_candidate_object,
                banner: second_banner,
                width: learning_data_access::COURSE_BANNER_WIDTH,
                height: learning_data_access::COURSE_BANNER_HEIGHT,
                expires_at,
            },
        )
        .await
        .expect("second PostgreSQL candidate should persist");
    let second_promoted = objects
        .put(PutObject {
            key: second_banner_key.clone(),
            bytes: second_bytes,
            media_type: BANNER_MEDIA_TYPE.to_string(),
            license: BANNER_LICENSE.to_string(),
            provenance: PROMOTED_PROVENANCE.to_string(),
            created_at: now,
        })
        .await
        .expect("second promoted bytes should persist in MinIO");
    store
        .save_course_appearance(
            context,
            instructor_session,
            course,
            SaveCourseAppearance {
                expected_revision: first_saved.revision,
                update: CourseAppearanceUpdate {
                    theme: CourseThemeId::Forest,
                    banner: CourseBannerMutation::Replace {
                        candidate: second_candidate,
                        alternative_text: CourseBannerAlternativeText::Decorative,
                    },
                },
                promoted_object: Some(second_promoted),
            },
        )
        .await
        .expect("second current banner should supersede the first");

    tokio::time::sleep(std::time::Duration::from_millis(3_200)).await;
    cleanup_expired_course_banners(store.as_ref(), objects.as_ref(), context).await;
    cleanup_expired_course_banners(store.as_ref(), objects.as_ref(), context).await;

    for deleted in [
        &first_candidate_key,
        &first_banner_key,
        &second_candidate_key,
    ] {
        assert_eq!(
            objects.get(deleted).await,
            Err(ObjectStoreError::NotFound),
            "expired candidate or superseded promoted bytes must be deleted"
        );
    }
    objects
        .get(&second_banner_key)
        .await
        .expect("exact current promoted object must survive cleanup");
    store
        .authorize_course_banner_delivery(context, student_session, second_banner)
        .await
        .expect("current delivery must remain student-readable after cleanup");
    assert!(
        store
            .claim_course_banner_cleanup(
                context,
                CourseBannerCleanupBatch::new(10).expect("valid live cleanup batch"),
            )
            .await
            .expect("post-cleanup claim should run")
            .is_empty(),
        "idempotent cleanup must leave no expired object work"
    );
}

#[tokio::test]
async fn mutation_authorization_media_bounds_and_strict_json_are_safe() {
    let fixture = fixture().await;

    let student_upload = upload_candidate(
        &fixture,
        &fixture.student_cookie,
        banner_png(Rgb([1, 2, 3])),
    )
    .await;
    assert_eq!(student_upload.0, StatusCode::FORBIDDEN);

    let outsider_read = fixture
        .app
        .clone()
        .oneshot(request(
            Method::GET,
            appearance_uri(fixture.course),
            &fixture.outsider_cookie,
            Body::empty(),
        ))
        .await
        .expect("outsider read should run");
    assert_eq!(outsider_read.status(), StatusCode::NOT_FOUND);

    let unsupported = request(
        Method::POST,
        candidate_uri(fixture.course),
        &fixture.instructor_cookie,
        "<svg/>",
    );
    let unsupported = fixture
        .app
        .clone()
        .oneshot(unsupported)
        .await
        .expect("unsupported upload should run");
    assert_eq!(unsupported.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let malformed =
        upload_candidate(&fixture, &fixture.instructor_cookie, b"not a PNG".to_vec()).await;
    assert_eq!(malformed.0, StatusCode::UNPROCESSABLE_ENTITY);

    let mut oversized = request(
        Method::POST,
        candidate_uri(fixture.course),
        &fixture.instructor_cookie,
        vec![0_u8; MAX_BANNER_UPLOAD_BYTES + 2],
    );
    oversized
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    let oversized = fixture
        .app
        .clone()
        .oneshot(oversized)
        .await
        .expect("oversized upload should run");
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let missing_precondition = request(
        Method::PUT,
        appearance_uri(fixture.course),
        &fixture.instructor_cookie,
        r#"{"theme":"forest","banner":{"kind":"remove"}}"#,
    );
    let missing_precondition = fixture
        .app
        .clone()
        .oneshot(missing_precondition)
        .await
        .expect("missing precondition save should run");
    assert_eq!(
        missing_precondition.status(),
        StatusCode::PRECONDITION_REQUIRED
    );

    let mut unknown_field = request(
        Method::PUT,
        appearance_uri(fixture.course),
        &fixture.instructor_cookie,
        r#"{"theme":"forest","banner":{"kind":"remove"},"course":"forged"}"#,
    );
    unknown_field
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    unknown_field
        .headers_mut()
        .insert(IF_MATCH, HeaderValue::from_static("\"1\""));
    let unknown_field = fixture
        .app
        .clone()
        .oneshot(unknown_field)
        .await
        .expect("strict JSON save should run");
    assert_eq!(unknown_field.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let student_save = save(
        &fixture,
        &fixture.student_cookie,
        "\"1\"",
        &CourseAppearanceUpdate {
            theme: CourseThemeId::Forest,
            banner: CourseBannerMutation::Remove,
        },
    )
    .await;
    assert_eq!(student_save.status(), StatusCode::FORBIDDEN);
}

#[derive(Default)]
struct FailSecondPutObjectStore {
    inner: MemoryObjectStore,
    puts: AtomicUsize,
}

#[async_trait]
impl ObjectStore for FailSecondPutObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        if self.puts.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(ObjectStoreError::Unavailable(
                "injected promotion failure".to_string(),
            ));
        }
        self.inner.put(request).await
    }

    async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
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
        self.inner.signed_url(key, now).await
    }
}

#[tokio::test]
async fn promoted_object_failure_preserves_the_old_appearance() {
    let fixture = fixture_with_objects(Arc::new(FailSecondPutObjectStore::default())).await;
    let (_, receipt) = upload_candidate(
        &fixture,
        &fixture.instructor_cookie,
        banner_png(Rgb([1, 2, 3])),
    )
    .await;
    let failed = save(
        &fixture,
        &fixture.instructor_cookie,
        "\"1\"",
        &CourseAppearanceUpdate {
            theme: CourseThemeId::Magma,
            banner: CourseBannerMutation::Replace {
                candidate: receipt.expect("candidate receipt").candidate,
                alternative_text: CourseBannerAlternativeText::Decorative,
            },
        },
    )
    .await;
    assert_eq!(failed.status(), StatusCode::SERVICE_UNAVAILABLE);

    let current = fixture
        .app
        .oneshot(request(
            Method::GET,
            appearance_uri(fixture.course),
            &fixture.instructor_cookie,
            Body::empty(),
        ))
        .await
        .expect("appearance read should run");
    let current: CourseAppearance = response_json(current).await;
    assert_eq!(current.theme, CourseThemeId::Grass);
    assert!(current.banner.is_none());
    assert_eq!(current.revision.value(), 1);
}
