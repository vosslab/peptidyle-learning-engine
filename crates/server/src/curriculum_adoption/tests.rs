use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    CurriculumAdoptionStore, SessionLifetime, SessionRecord, SessionStore, SessionSubject,
    SessionTokenHash, StoreError, TenantContext, in_memory::MemoryStore,
};
use question_model::{
    ActivityTimestamp, AlphaCourseReference, AlphaInstantiationCommand,
    AlphaInstantiationCompleted, AlphaInstantiationPreviewRequest, AlphaInstantiationPreviewView,
    AssignmentFastForwardCommand, AssignmentFastForwardCompleted,
    AssignmentFastForwardPreviewRequest, AssignmentFastForwardPreviewView,
    BlueprintInstantiationCommand, BlueprintInstantiationCompleted,
    BlueprintInstantiationPreviewRequest, BlueprintInstantiationPreviewView, CourseReference,
    CourseRolloverCommand, CourseRolloverCompleted, CourseRolloverPreviewRequest,
    CourseRolloverPreviewView, CourseTermShiftCommand, CourseTermShiftCompleted,
    CourseTermShiftPreviewOutcome, CourseTermShiftPreviewRequest,
    CreateSourceDerivedAssignmentCommand, CurriculumAdoptionReconciliationResult,
    CurriculumCourseImportView, ForkAlphaCommand, ForkAlphaCompleted, ForkAlphaPreviewRequest,
    ForkAlphaPreviewView, ReconcileCurriculumAdoptionCommand, SourceDerivedAssignmentCompleted,
    SourceDerivedAssignmentPreviewRequest, SourceDerivedAssignmentPreviewView, TenantId, UserId,
    UserRole,
};
use tower::ServiceExt;
use uuid::Uuid;

use super::*;

#[tokio::test]
async fn authentication_precedes_protected_body_decoding_and_refusals_are_no_store() {
    let app = router(Arc::new(MemoryStore::default()));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/alpha-courses/AC-unknown/fork/preview")
                .header("content-type", "application/json")
                .body(Body::from("{malformed"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()["cache-control"], "no-store");
}

#[test]
fn apply_request_rejects_unknown_fields() {
    assert!(
        serde_json::from_str::<ApplyBody<ForkAlphaPreviewView>>(
            r#"{"preview":{},"idempotencyKey":"retry-1","surprise":true}"#,
        )
        .is_err()
    );
}

#[tokio::test]
async fn bounded_body_refusal_is_non_cacheable() {
    let response = strict_json_body::<serde_json::Value>(
        Request::builder()
            .body(Body::from(vec![
                b'x';
                MAX_CURRICULUM_ADOPTION_BODY_BYTES + 1
            ]))
            .expect("request"),
    )
    .await
    .expect_err("oversized body is refused");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(response.headers()["cache-control"], "no-store");
}

#[test]
fn stale_preview_maps_to_a_recoverable_non_cacheable_response() {
    let response = store_error(StoreError::Conflict);
    assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
    assert_eq!(response.headers()["cache-control"], "no-store");
}

struct RecordingStore {
    session: SessionRecord,
    preflight: Result<(), StoreError>,
    mutations: AtomicUsize,
}

impl RecordingStore {
    fn approved() -> (Self, String) {
        let token_bytes = [0x64_u8; 32];
        let token = URL_SAFE_NO_PAD.encode(token_bytes);
        let token_hash = SessionTokenHash::compute(&token_bytes);
        let tenant = TenantId::from_uuid(Uuid::from_u128(6_401));
        let subject = SessionSubject::new(
            tenant,
            UserId::from_uuid(Uuid::from_u128(6_402)),
            "Instructor",
            vec![UserRole::Instructor],
        )
        .expect("subject");
        (
            Self {
                session: SessionRecord {
                    token_hash,
                    subject,
                    created_at: ActivityTimestamp::from_unix_millis(0),
                    expires_at: ActivityTimestamp::from_unix_millis(3_600_000),
                },
                preflight: Ok(()),
                mutations: AtomicUsize::new(0),
            },
            token,
        )
    }
}

#[async_trait]
impl SessionStore for RecordingStore {
    async fn create_session(
        &self,
        _: SessionTokenHash,
        _: SessionSubject,
        _: SessionLifetime,
    ) -> Result<SessionRecord, StoreError> {
        Err(StoreError::Forbidden)
    }
    async fn resolve_session(
        &self,
        token: SessionTokenHash,
    ) -> Result<Option<SessionRecord>, StoreError> {
        Ok((token == self.session.token_hash).then(|| self.session.clone()))
    }
    async fn revoke_session(&self, _: SessionTokenHash) -> Result<(), StoreError> {
        Err(StoreError::Forbidden)
    }
}

#[async_trait]
impl CurriculumAdoptionStore for RecordingStore {
    async fn preflight_curriculum_adoption(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
    ) -> Result<(), StoreError> {
        self.preflight.clone()
    }
    async fn preview_fork_alpha(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: ForkAlphaPreviewRequest,
    ) -> Result<ForkAlphaPreviewView, StoreError> {
        self.mutations.fetch_add(1, Ordering::Relaxed);
        Err(StoreError::NotFound)
    }
    async fn apply_fork_alpha(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: ForkAlphaCommand,
    ) -> Result<ForkAlphaCompleted, StoreError> {
        self.mutations.fetch_add(1, Ordering::Relaxed);
        Err(StoreError::NotFound)
    }
    async fn preview_blueprint_instantiation(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: BlueprintInstantiationPreviewRequest,
    ) -> Result<BlueprintInstantiationPreviewView, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn apply_blueprint_instantiation(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: BlueprintInstantiationCommand,
    ) -> Result<BlueprintInstantiationCompleted, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn preview_alpha_instantiation(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: AlphaInstantiationPreviewRequest,
    ) -> Result<AlphaInstantiationPreviewView, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn apply_alpha_instantiation(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: AlphaInstantiationCommand,
    ) -> Result<AlphaInstantiationCompleted, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn preview_course_rollover(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: CourseRolloverPreviewRequest,
    ) -> Result<CourseRolloverPreviewView, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn apply_course_rollover(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: CourseRolloverCommand,
    ) -> Result<CourseRolloverCompleted, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn preview_course_term_shift(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: CourseTermShiftPreviewRequest,
    ) -> Result<CourseTermShiftPreviewOutcome, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn apply_course_term_shift(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: CourseTermShiftCommand,
    ) -> Result<CourseTermShiftCompleted, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn preview_assignment_fast_forward(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: AssignmentFastForwardPreviewRequest,
    ) -> Result<AssignmentFastForwardPreviewView, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn apply_assignment_fast_forward(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: AssignmentFastForwardCommand,
    ) -> Result<AssignmentFastForwardCompleted, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn preview_source_derived_assignment(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: SourceDerivedAssignmentPreviewRequest,
    ) -> Result<SourceDerivedAssignmentPreviewView, StoreError> {
        Err(StoreError::NotFound)
    }
    async fn create_source_derived_assignment(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: CreateSourceDerivedAssignmentCommand,
    ) -> Result<SourceDerivedAssignmentCompleted, StoreError> {
        self.mutations.fetch_add(1, Ordering::Relaxed);
        Err(StoreError::NotFound)
    }
    async fn inspect_curriculum_imports(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: CourseReference,
    ) -> Result<Option<CurriculumCourseImportView>, StoreError> {
        Ok(None)
    }
    async fn reconcile_curriculum_adoption(
        &self,
        _: TenantContext,
        _: SessionTokenHash,
        _: ReconcileCurriculumAdoptionCommand,
    ) -> Result<CurriculumAdoptionReconciliationResult, StoreError> {
        self.mutations.fetch_add(1, Ordering::Relaxed);
        Err(StoreError::NotFound)
    }
}

#[tokio::test]
async fn authenticated_preflight_refusal_precedes_malformed_protected_body() {
    let (mut store, token) = RecordingStore::approved();
    store.preflight = Err(StoreError::Forbidden);
    let app = router(Arc::new(store));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/alpha-courses/AC-1/fork/preview")
                .header("cookie", format!("ple_session={token}"))
                .body(Body::from("{malformed"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn authenticated_route_source_mismatch_is_concealed_without_store_mutation() {
    let (store, token) = RecordingStore::approved();
    let store = Arc::new(store);
    let app = router(Arc::clone(&store));
    let request = ForkAlphaPreviewRequest {
        source: question_model::ObservedAlphaSource {
            reference: AlphaCourseReference::new(2).expect("source reference"),
            revision: "1".parse().expect("revision"),
        },
        replacements: question_model::CurriculumPinReplacements::default(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/alpha-courses/AC-1/fork/preview")
                .header("cookie", format!("ple_session={token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("request JSON"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(store.mutations.load(Ordering::Relaxed), 0);
}
