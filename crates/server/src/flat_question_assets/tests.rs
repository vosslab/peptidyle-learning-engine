use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use learning_data_access::in_memory::{MemoryFlatQuestionGraderStore, MemoryStore};
use learning_data_access::{
    FlatQuestionAssetStore, FlatQuestionGradingStore, SessionLifetime, SessionSubject,
    TenantContext,
};
use objects::memory::MemoryObjectStore;
use objects::{
    ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject, SignedUrl, StoredObject,
};
use question_model::{TenantId, UserId, UserRole, WorkspaceId};
use tower::ServiceExt;

use crate::auth::{CookieTransport, SessionConfig, issue_session};
use crate::catalog::ReviewNotRequired;
use crate::native_backend::NativeBackend;

use super::*;

const SOURCE: &str = r#"{
  "format":"pleFlatQuestion", "version":2, "title":"Image fixture",
  "prompt":"Choose the image", "response":{"kind":"singleChoice","choices":[
    {"id":"one","text":"One","feedback":"private"},
    {"id":"two","text":"Two","feedback":"private"}], "correctChoice":"one"},
  "feedback":{"correct":"private", "incorrect":"private"}, "points":1.0,
  "attemptPolicy":{"maxAttempts":null},
  "timingPolicy":{"kind":"untimed"}, "license":{"kind":"cc0"}, "language":"en-US"
}"#;

struct Fixture {
    store: Arc<MemoryStore>,
    grader: Arc<MemoryFlatQuestionGraderStore>,
    objects: Arc<MemoryObjectStore>,
    tenant: TenantId,
    workspace: WorkspaceId,
    owner_cookie: String,
    learner_cookie: String,
}

impl Fixture {
    fn uri(&self) -> String {
        format!("/api/workspaces/{}/flat-question-assets", self.workspace)
    }

    fn app(&self) -> Router {
        let grader: Arc<dyn FlatQuestionGradingStore> = Arc::clone(&self.grader) as Arc<_>;
        let backend = Arc::new(NativeBackend::with_flat_grader(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&self.store),
            grader,
        ));
        crate::flat_question_publication::router(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            backend,
            Arc::new(ReviewNotRequired),
        )
        .merge(router(Arc::clone(&self.store), Arc::clone(&self.objects)))
    }
}

struct RecordingObjectStore {
    inner: Arc<MemoryObjectStore>,
    deleted: Arc<Mutex<Vec<ObjectKey>>>,
}

#[async_trait::async_trait]
impl ObjectStore for RecordingObjectStore {
    async fn put(&self, request: PutObject) -> Result<ObjectRecord, ObjectStoreError> {
        self.inner.put(request).await
    }

    async fn get(&self, key: &ObjectKey) -> Result<StoredObject, ObjectStoreError> {
        self.inner.get(key).await
    }

    async fn delete(&self, key: &ObjectKey) -> Result<(), ObjectStoreError> {
        self.deleted
            .lock()
            .expect("delete recorder lock")
            .push(key.clone());
        self.inner.delete(key).await
    }

    async fn signed_url(
        &self,
        key: &ObjectKey,
        now: question_model::ActivityTimestamp,
    ) -> Result<SignedUrl, ObjectStoreError> {
        self.inner.signed_url(key, now).await
    }
}

fn id(value: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(value)
}

async fn cookie(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
) -> String {
    issue_session(
        store,
        SessionSubject::new(tenant, user, "asset route fixture", roles).expect("fixture identity"),
        SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("fixture lifetime"),
            CookieTransport::FirstPartyHttps,
        ),
    )
    .await
    .expect("fixture session")
    .set_cookie
    .split(';')
    .next()
    .expect("cookie pair")
    .to_string()
}

async fn fixture() -> Fixture {
    let (store, grader) = MemoryStore::with_flat_question_grader();
    let store = Arc::new(store);
    let tenant = TenantId::from_uuid(id(1));
    let owner = UserId::from_uuid(id(2));
    let workspace = WorkspaceId::from_uuid(id(3));
    let owner_cookie = cookie(&store, tenant, owner, vec![UserRole::Instructor]).await;
    let learner_cookie = cookie(
        &store,
        tenant,
        UserId::from_uuid(id(4)),
        vec![UserRole::Student],
    )
    .await;
    let fixture = Fixture {
        store,
        grader: Arc::new(grader),
        objects: Arc::new(MemoryObjectStore::default()),
        tenant,
        workspace,
        owner_cookie,
        learner_cookie,
    };
    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/api/workspaces/{}/flat-question",
            fixture.workspace
        ))
        .header("cookie", &fixture.owner_cookie)
        .body(Body::from(SOURCE))
        .expect("source request");
    let response = fixture
        .app()
        .oneshot(request)
        .await
        .expect("source response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "fixture saves a visible flat draft"
    );
    fixture
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = RgbImage::from_pixel(width, height, Rgb([12, 34, 56]));
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(image.as_raw(), width, height, ExtendedColorType::Rgb8)
        .expect("fixture PNG encodes");
    bytes
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn apng() -> Vec<u8> {
    let source = png(2, 1);
    let data = [0, 0, 0, 2, 0, 0, 0, 0];
    let mut chunk_input = b"acTL".to_vec();
    chunk_input.extend_from_slice(&data);
    let mut animation_control = Vec::new();
    animation_control.extend_from_slice(&8_u32.to_be_bytes());
    animation_control.extend_from_slice(b"acTL");
    animation_control.extend_from_slice(&data);
    animation_control.extend_from_slice(&crc32(&chunk_input).to_be_bytes());
    let mut output = source[..33].to_vec();
    output.extend_from_slice(&animation_control);
    output.extend_from_slice(&source[33..]);
    output
}

async fn parts(response: Response) -> (StatusCode, HeaderMap, String) {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body");
    (
        status,
        headers,
        String::from_utf8(bytes.to_vec()).expect("json response"),
    )
}

async fn upload(
    fixture: &Fixture,
    cookie: Option<&str>,
    bytes: Vec<u8>,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder().method("POST").uri(fixture.uri());
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    request = request
        .header(ASSET_LABEL_HEADER, "Cell membrane")
        .header(ASSET_PROVENANCE_HEADER, "Instructor-created diagram");
    for (name, value) in extra_headers {
        request = request.header(*name, *value);
    }
    parts(
        fixture
            .app()
            .oneshot(request.body(Body::from(bytes)).expect("asset request"))
            .await
            .expect("asset response"),
    )
    .await
}

#[tokio::test]
async fn upload_sniffs_private_png_and_lists_a_safe_descriptor() {
    let fixture = fixture().await;
    let image = png(3, 2);
    let checksum = objects::Sha256Digest::compute(&image).to_string();
    let (status, headers, body) = upload(&fixture, Some(&fixture.owner_cookie), image, &[]).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        headers
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    assert!(body.contains("\"mediaType\":\"image/png\""));
    assert!(body.contains("\"intrinsicWidth\":3"));
    assert!(body.contains(&format!("\"contentChecksum\":\"{checksum}\"")));
    for secret in [
        "sha256",
        "provenance",
        "license",
        "key",
        "object",
        "createdAt",
    ] {
        assert!(
            !body.contains(secret),
            "safe descriptor leaked {secret}: {body}"
        );
    }
    let (status, _, body) = parts(
        fixture
            .app()
            .oneshot(
                Request::builder()
                    .uri(fixture.uri())
                    .header("cookie", &fixture.owner_cookie)
                    .body(Body::empty())
                    .expect("list request"),
            )
            .await
            .expect("list response"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Cell membrane"));
    assert_eq!(
        fixture
            .store
            .list_workspace_flat_question_assets(
                TenantContext::from_authenticated_session(fixture.tenant),
                fixture.workspace
            )
            .await
            .expect("stored descriptors")
            .len(),
        1
    );
}

#[tokio::test]
async fn asset_upload_refuses_invalid_content_metadata_and_non_authors_without_object_mutation() {
    let fixture = fixture().await;
    let (status, _, _) = upload(
        &fixture,
        Some(&fixture.owner_cookie),
        b"not an image".to_vec(),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = upload(
        &fixture,
        Some(&fixture.owner_cookie),
        png(1, 1),
        &[("x-ple-browser-checksum", "forged")],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = upload(
        &fixture,
        Some(&fixture.owner_cookie),
        png(1, 1),
        &[("x-ple-content-checksum", "forged")],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, body) = upload(&fixture, Some(&fixture.owner_cookie), apng(), &[]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("still image"));
    let (status, _, _) = upload(
        &fixture,
        Some(&fixture.owner_cookie),
        vec![0; MAX_FLAT_QUESTION_ASSET_BODY_BYTES + 1],
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    let (status, _, body) = upload(
        &fixture,
        Some(&fixture.learner_cookie),
        vec![0; MAX_FLAT_QUESTION_ASSET_BODY_BYTES + 1],
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "role is refused before image inspection"
    );
    assert!(
        !body.contains("image"),
        "absence does not reveal upload rules"
    );
    assert!(
        fixture
            .store
            .list_workspace_flat_question_assets(
                TenantContext::from_authenticated_session(fixture.tenant),
                fixture.workspace
            )
            .await
            .expect("stored descriptors")
            .is_empty()
    );
}

#[tokio::test]
async fn foreign_workspace_and_missing_session_look_absent_or_unauthenticated() {
    let fixture = fixture().await;
    let foreign_workspace = WorkspaceId::from_uuid(id(99));
    let uri = format!("/api/workspaces/{foreign_workspace}/flat-question-assets");
    let (status, _, body) = parts(
        fixture
            .app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("cookie", &fixture.owner_cookie)
                    .body(Body::empty())
                    .expect("foreign request"),
            )
            .await
            .expect("foreign response"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, "{\"error\":\"workspace not found\"}");
    let (status, _, _) = upload(&fixture, None, png(1, 1), &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejected_descriptor_compensates_only_the_newly_written_object() {
    let fixture = fixture().await;
    let deleted = Arc::new(Mutex::new(Vec::new()));
    let objects = Arc::new(RecordingObjectStore {
        inner: Arc::new(MemoryObjectStore::default()),
        deleted: Arc::clone(&deleted),
    });
    let app = router(Arc::clone(&fixture.store), Arc::clone(&objects));
    let label = "x".repeat(learning_data_access::MAX_WORKSPACE_FLAT_QUESTION_ASSET_LABEL_CHARS + 1);
    let (status, _, _) = parts(
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(fixture.uri())
                .header("cookie", &fixture.owner_cookie)
                .header(ASSET_LABEL_HEADER, label)
                .header(ASSET_PROVENANCE_HEADER, "Instructor-created diagram")
                .body(Body::from(png(1, 1)))
                .expect("asset request"),
        )
        .await
        .expect("asset response"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let deleted = deleted.lock().expect("delete recorder lock");
    assert_eq!(
        deleted.len(),
        1,
        "only the just-written object is compensated"
    );
    assert!(matches!(
        deleted.as_slice(),
        [ObjectKey::WorkspaceQuestionAsset { .. }]
    ));
}
