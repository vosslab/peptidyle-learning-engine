use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use learning_data_access::in_memory::{MemoryQtiGraderStore, MemoryStore};
use learning_data_access::{
    AssetDeliveryId, AssetDeliveryRecord, AssetStore, AssignmentRecord, AuthorizedAssetDelivery,
    CatalogAssetBinding, CatalogStore, CommitPreparedQtiImport, CommitPreparedQtiImportOutcome,
    CourseRecord, EnqueueJob, JobLeaseDuration, JobPayload, JobStore, PublishDraftCommand,
    QtiGradingStore, QtiImportGradingPayload, QtiImportStore, SessionLifetime, SessionSubject,
    Store, TenantContext,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectKey, ObjectStore, PutObject};
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssetId, AssignmentEnrollment, AssignmentId, AssignmentRun,
    AttemptTimerRecord, CourseId, CourseMembership, CourseMembershipRole, EnrollmentId,
    GradingDefinition, ObjectId, ProblemId, PublicationScope, QuestionAttempt, QuestionAttemptId,
    QuestionMetadata, RunId, StudentId, TenantId, UserId, VersionId, WorkspaceId,
    WorkspaceImportId,
};
use tower::ServiceExt;

use super::*;
use crate::auth::{CookieTransport, SessionConfig, issue_session};
use crate::qti_import::QtiImportHandler;
use crate::qti_publication::QtiPublicationPreparer;
use crate::run::router as run_router;
use crate::worker::{JobExecution, JobHandler};

const PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAgAdLsHXcJKi+S6AAAAiwEAAA4AAABpdGVtcy9pdGVtLnhtbH2QSw7CMAxErxLlAETs",
    "XUu0sOgGUDlBCEaN1CZVHH63J7QgKEXsrPEbe2zQzMTckotlpFbYQ6rs0VLIpE2CRAjEnXdMSzKNDjpa70ZYtdpt",
    "N+vdKqHGh0AmVk8Hwlk3J8I9qKEANSHUj/EIj9W5P9wQOixq75lErEk83UI7vlCYgerSztpbQ6WLFLTpw70mlr9C",
    "ilZfi97CmZynzGzbrqFBGt2lJS5Afbb/wHuJ+TesJtGS9r5MjV+Pd1BLAQIUAxQAAAAIAHS7B13yXbGdXwAAAIsA",
    "AAAPAAAAAAAAAAAAAACAAQAAAABpbXNtYW5pZmVzdC54bWxQSwECFAMUAAAACAB0uwddwkqL5LoAAACLAQAADgAA",
    "AAAAAAAAAAAAgAGMAAAAaXRlbXMvaXRlbS54bWxQSwUGAAAAAAIAAgB5AAAAcgEAAAAA",
);
const CHOICE_IMAGE_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIANghCF0ZBjDNaAAAAJMAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RCoMwEESvEvYABvsdcxUJ6USX",
    "Gk2zq9jbNyCl7d8w7zGMy2HlBFHDd6zKiVEHKiE+wgTyrkK2vUbIN/6Zcd44goy+CgbiLE/lkRV5PPNy3EpPZq5I",
    "DbVO7KV3jZH1zv6s288R/wZQSwMEFAAAAAgA2CEIXYeao83GAAAAoAEAABAAAABpdGVtcy9jaG9pY2UueG1sfZHN",
    "TsMwDIBfJcoD1OLuWmLj0uvewMtMZ6n5URwQvD2hZYIyxM1yvtifbWQzMYuS2tQkOr30SJ9V6ujDNWsQT1jFSk4m",
    "TxIWrtw0px146kzItUpopy+U8JWXF6EzwhYg3BHwR11C7RqHfHknLHS85mwyIJTeYLWZUpPKYVW4fZ92Ki7y23Fl",
    "bfQP3cw0lkW21E6bPT0i/Hz+Bz73ShpnZzWMfhjgc2/NYLMaSpq946XdluY08iwe6PC7AdzN0XPfM8P+HvQBUEsD",
    "BBQAAAAIANghCF32FIo6EgAAABAAAAARAAAAYXNzZXRzL2Nob2ljZS5wbmfrDPBz5+WS4sovykzPzEvMAQBQSwEC",
    "FAMUAAAACADYIQhdGQYwzWgAAACTAAAADwAAAAAAAAAAAAAAgAEAAAAAaW1zbWFuaWZlc3QueG1sUEsBAhQDFAAA",
    "AAgA2CEIXYeao83GAAAAoAEAABAAAAAAAAAAAAAAAIABlQAAAGl0ZW1zL2Nob2ljZS54bWxQSwECFAMUAAAACADY",
    "IQhd9hSKOhIAAAAQAAAAEQAAAAAAAAAAAAAAgAGJAQAAYXNzZXRzL2Nob2ljZS5wbmdQSwUGAAAAAAMAAwC6AAAA",
    "ygEAAAAA",
);

#[test]
fn qti_runtime_reports_the_honest_generic_profile_identity() {
    assert_eq!(
        implementation_version().id,
        "ple-qti-assessment-item-single-choice/v1"
    );
}

#[derive(Clone)]
struct FixtureSources {
    tenant: TenantId,
    artifact: PublishedSourceArtifact,
    bindings: Vec<CatalogAssetBinding>,
}

#[async_trait]
impl CatalogSourceStore for FixtureSources {
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError> {
        Ok(
            (context.tenant_id() == self.tenant && reference == self.artifact.reference)
                .then(|| self.artifact.clone()),
        )
    }
}

#[async_trait]
impl AssetStore for FixtureSources {
    async fn register_asset_delivery(
        &self,
        _context: TenantContext,
        _record: AssetDeliveryRecord,
    ) -> Result<(), StoreError> {
        Err(StoreError::InvalidRecord(
            "fixture does not register assets".to_string(),
        ))
    }

    async fn get_public_asset_delivery(
        &self,
        _delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError> {
        Ok(None)
    }

    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError> {
        if context.tenant_id() == self.tenant && reference == self.artifact.reference {
            Ok(self.bindings.clone())
        } else {
            Ok(Vec::new())
        }
    }

    async fn authorize_asset_delivery(
        &self,
        _context: TenantContext,
        _actor: question_model::UserId,
        _delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        Err(StoreError::InvalidRecord(
            "fixture does not authorize assets".to_string(),
        ))
    }
}

#[derive(Clone)]
struct RecordedGrader {
    tenant: TenantId,
    reference: ProblemVersionRef,
    item: String,
    payload: QtiImportGradingPayload,
    calls: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct CountingQtiGrader {
    inner: Arc<MemoryQtiGraderStore>,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl QtiGradingStore for CountingQtiGrader {
    async fn qti_import_grading(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
        import: question_model::WorkspaceImportId,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        self.inner
            .qti_import_grading(context, workspace, import, item_id)
            .await
    }

    async fn qti_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner
            .qti_published_grading(context, reference, item_id)
            .await
    }
}

#[async_trait]
impl QtiGradingStore for RecordedGrader {
    async fn qti_import_grading(
        &self,
        _context: TenantContext,
        _workspace: WorkspaceId,
        _import: question_model::WorkspaceImportId,
        _item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        Ok(None)
    }

    async fn qti_published_grading(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        item_id: &str,
    ) -> Result<Option<QtiImportGradingPayload>, StoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok((context.tenant_id() == self.tenant
            && reference == self.reference
            && item_id == self.item)
            .then(|| self.payload.clone()))
    }
}

struct Fixture {
    backend: QtiBackend<FixtureSources, RecordedGrader, MemoryObjectStore>,
    context: TenantContext,
    reference: ProblemVersionRef,
    question: QuestionDefinition,
    correct: ChoiceId,
    incorrect: ChoiceId,
    grader_calls: Arc<AtomicUsize>,
}

async fn fixture() -> Fixture {
    let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(7_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid::Uuid::from_u128(7_002)),
        version: VersionId::from_uuid(uuid::Uuid::from_u128(7_003)),
    };
    let workspace = WorkspaceId::from_uuid(uuid::Uuid::from_u128(7_004));
    let object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_005));
    let bytes = STANDARD
        .decode(PACKAGE.trim())
        .expect("fixture ZIP decodes");
    let parsed = adapter_qti::QtiImporter::default()
        .import(&bytes)
        .expect("fixture ZIP parses");
    let imported = parsed
        .questions
        .into_iter()
        .next()
        .expect("fixture item exists");
    let ResponseDefinition::MultipleChoice { choices, .. } = &imported.response else {
        panic!("fixture QTI is single choice")
    };
    let correct = choices
        .first()
        .expect("fixture has a first choice")
        .id
        .clone();
    let incorrect = choices
        .last()
        .expect("fixture has a last choice")
        .id
        .clone();
    assert_ne!(correct, incorrect, "fixture must exercise a wrong answer");
    let objects = Arc::new(MemoryObjectStore::default());
    let record = objects
        .put(PutObject {
            key: ObjectKey::ProblemSource {
                problem: reference.problem,
                version: reference.version,
                object,
            },
            bytes,
            media_type: "application/zip".to_string(),
            license: "CC-BY-4.0".to_string(),
            provenance: "QTI fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("published fixture source stores");
    let question = QuestionDefinition {
        problem: reference.problem,
        version: reference.version,
        workspace,
        source: QuestionSource::Qti {
            item_id: imported.item_id.clone(),
            package_object: object,
            package_sha256: record.sha256.to_string(),
        },
        prompt: imported.prompt,
        response: imported.response,
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateCorrectness,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: question_model::generation::RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Published QTI fixture".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    };
    let sources = Arc::new(FixtureSources {
        tenant,
        artifact: PublishedSourceArtifact {
            reference,
            backend: question_model::QuestionBackend::Qti,
            object: record,
        },
        bindings: Vec::new(),
    });
    let grader_calls = Arc::new(AtomicUsize::new(0));
    let grader = Arc::new(RecordedGrader {
        tenant,
        reference,
        item: imported.item_id,
        payload: QtiImportGradingPayload::new(
            serde_json::to_vec(&correct).expect("choice serializes"),
        )
        .expect("private payload is bounded"),
        calls: Arc::clone(&grader_calls),
    });
    Fixture {
        backend: QtiBackend::new(sources, grader, objects),
        context,
        reference,
        question,
        correct,
        incorrect,
        grader_calls,
    }
}

fn attempt(fixture: &Fixture, issued: IssuedAttemptMetadata) -> QuestionAttempt {
    QuestionAttempt {
        id: QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(7_006)),
        tenant: fixture.context.tenant_id(),
        run: RunId::from_uuid(uuid::Uuid::from_u128(7_007)),
        problem: fixture.reference.problem,
        question_version: fixture.reference.version,
        assignment_position: 0,
        seed: 41,
        parameter_hash: issued.parameter_hash,
        response: None,
        status: question_model::AttemptStatus::InProgress,
        result: None,
        timer: AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(1),
            deadline: None,
            submitted_at: None,
        },
        provenance: issued.provenance,
        issued_capability: question_model::IssuedAttemptCapabilityV1::PresentationEnvelope,
    }
}

mod private_grading;
mod run_lifecycle;
