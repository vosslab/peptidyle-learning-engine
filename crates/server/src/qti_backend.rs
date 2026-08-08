//! Server-only execution bridge for immutable published QTI questions.
//!
//! QTI archives are reparsed from the exact published source object on every
//! issue, replay, and grade operation.  The public package never carries its
//! correct choice: that binding is resolved only through the injected,
//! least-privilege [`store::QtiGradingStore`] capability.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_qti::qti_question_asset_checksums;
use async_trait::async_trait;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, Sha256Digest};
use question_model::generation::Seed;
use question_model::{
    AttemptProvenance, ImplementationVersion, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition, QuestionEnvelope, QuestionSource, SourceArtifact, StudentResponse,
};
use store::{
    AssetStore, CatalogSourceStore, PublishedSourceArtifact, QtiGradingStore, StoreError,
    TenantContext,
};

use crate::run::{IssuedAttemptMetadata, RunBackend, RunBackendError};

const QTI_ADAPTER_ID: &str = "qti-1.2-subset";

/// Immutable-source resolver and server-only grader for published QTI items.
///
/// `G` is deliberately the only dependency that can recover answer-bearing
/// material.  Catalog, object, and asset dependencies resolve public
/// immutable evidence only.
pub struct QtiBackend<S, G, O> {
    sources: Arc<S>,
    grader: Arc<G>,
    objects: Arc<O>,
}

impl<S, G, O> QtiBackend<S, G, O> {
    pub fn new(sources: Arc<S>, grader: Arc<G>, objects: Arc<O>) -> Self {
        Self {
            sources,
            grader,
            objects,
        }
    }
}

impl<S, G, O> QtiBackend<S, G, O>
where
    S: CatalogSourceStore + AssetStore + Send + Sync + 'static,
    G: QtiGradingStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
{
    async fn resolve(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<ResolvedQtiQuestion, RunBackendError> {
        validate_reference(reference, question)?;
        let QuestionSource::Qti {
            item_id,
            package_object,
            package_sha256,
        } = &question.source
        else {
            return Err(RunBackendError::Unsupported(
                "published question is not backed by QTI".to_string(),
            ));
        };
        let artifact = self
            .sources
            .catalog_source_artifact(context, reference)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Invalid("published QTI source is unavailable".to_string())
            })?;
        validate_source_artifact(reference, package_object, package_sha256, &artifact)?;
        let source = self
            .objects
            .get(&artifact.object.key)
            .await
            .map_err(map_object_error)?;
        if source.record != artifact.object
            || Sha256Digest::compute(&source.bytes).to_string() != *package_sha256
        {
            return Err(RunBackendError::Invalid(
                "published QTI archive integrity does not match its immutable binding".to_string(),
            ));
        }
        let package = adapter_qti::QtiImporter::default()
            .import(&source.bytes)
            .map_err(|_| {
                RunBackendError::Invalid("published QTI archive cannot be reparsed".to_string())
            })?;
        if package.worker_original_sha256() != package_sha256
            || package.worker_original_size_bytes() != artifact.object.size_bytes
        {
            return Err(RunBackendError::Invalid(
                "published QTI archive reparse does not reproduce its immutable metadata"
                    .to_string(),
            ));
        }
        let imported = package
            .questions
            .into_iter()
            .find(|candidate| candidate.item_id == *item_id)
            .ok_or_else(|| {
                RunBackendError::Invalid(
                    "published QTI item is absent from its archive".to_string(),
                )
            })?;
        if imported.prompt != question.prompt || imported.response != question.response {
            return Err(RunBackendError::Invalid(
                "published QTI item does not match its immutable question definition".to_string(),
            ));
        }
        let asset_objects = self
            .resolve_assets(
                context,
                reference,
                &qti_question_asset_checksums(&imported).map_err(|_| {
                    RunBackendError::Invalid(
                        "published QTI item has conflicting asset checksums".to_string(),
                    )
                })?,
            )
            .await?;
        let envelope = QuestionEnvelope {
            version: reference.version,
            seed: Seed::new(seed),
            title: question.metadata.title.clone(),
            prompt: question.prompt.clone(),
            response: question.response.clone(),
        };
        let rendered_question_sha256 =
            Sha256Digest::compute(&serde_json::to_vec(&envelope).map_err(|_| {
                RunBackendError::Invalid("QTI envelope cannot be encoded".to_string())
            })?)
            .to_string();
        let provenance = AttemptProvenance {
            adapter: implementation_version(),
            renderer: None,
            generator: None,
            source_artifact: Some(SourceArtifact {
                object: artifact.object.id,
                sha256: package_sha256.clone(),
            }),
            asset_objects,
            grading: ImplementationVersion {
                id: "qti-private-choice".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            rendered_question_sha256: rendered_question_sha256.clone(),
        };
        Ok(ResolvedQtiQuestion {
            envelope,
            parameter_hash: Sha256Digest::compute(b"qti-static-v1").to_string(),
            provenance,
            item_id: item_id.clone(),
        })
    }

    async fn resolve_assets(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        expected: &std::collections::BTreeMap<question_model::AssetId, String>,
    ) -> Result<Vec<question_model::ObjectId>, RunBackendError> {
        let bindings = self
            .sources
            .catalog_asset_bindings(context, reference)
            .await
            .map_err(map_store_error)?;
        let actual = bindings
            .iter()
            .map(|binding| binding.asset)
            .collect::<BTreeSet<_>>();
        if actual != expected.keys().copied().collect() || bindings.len() != actual.len() {
            return Err(RunBackendError::Invalid(
                "published QTI asset bindings do not match the reparsed item".to_string(),
            ));
        }
        let mut objects = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let key = ObjectKey::ProblemAsset {
                problem: reference.problem,
                version: reference.version,
                asset: binding.asset,
                object: binding.object,
            };
            let stored = self.objects.get(&key).await.map_err(map_object_error)?;
            if stored.record.id != binding.object
                || stored.record.key != key
                || stored.record.bucket != Bucket::Content
                || stored.record.category != ObjectCategory::Asset
                || stored.record.version != Some(reference.version)
                || Sha256Digest::compute(&stored.bytes) != stored.record.sha256
                || stored.record.sha256.to_string()
                    != *expected.get(&binding.asset).ok_or_else(|| {
                        RunBackendError::Invalid(
                            "published QTI asset is not referenced by the reparsed item"
                                .to_string(),
                        )
                    })?
            {
                return Err(RunBackendError::Invalid(
                    "published QTI asset binding is invalid".to_string(),
                ));
            }
            objects.push(binding.object);
        }
        Ok(objects)
    }
}

#[async_trait]
impl<S, G, O> RunBackend for QtiBackend<S, G, O>
where
    S: CatalogSourceStore + AssetStore + Send + Sync + 'static,
    G: QtiGradingStore + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        let resolved = self.resolve(context, reference, question, seed).await?;
        Ok(IssuedAttemptMetadata {
            envelope: resolved.envelope,
            parameter_hash: resolved.parameter_hash,
            provenance: resolved.provenance,
        })
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let resolved = self
            .resolve(context, reference, question, attempt.seed)
            .await?;
        if attempt.parameter_hash != resolved.parameter_hash
            || attempt.provenance != resolved.provenance
        {
            return Err(RunBackendError::Invalid(
                "persisted QTI attempt provenance does not reproduce".to_string(),
            ));
        }
        Ok(resolved.envelope)
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let resolved = self
            .resolve(context, reference, question, attempt.seed)
            .await?;
        if attempt.parameter_hash != resolved.parameter_hash
            || attempt.provenance != resolved.provenance
        {
            return Err(RunBackendError::Invalid(
                "persisted QTI attempt provenance does not reproduce".to_string(),
            ));
        }
        let payload = self
            .grader
            .qti_published_grading(context, reference, &resolved.item_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Invalid("published QTI grading binding is unavailable".to_string())
            })?;
        let correct = payload.server_correct_choice().map_err(map_store_error)?;
        grading::grade(
            question,
            response,
            Some(&grading::AnswerKey::MultipleChoice {
                correct: BTreeSet::from([correct]),
            }),
        )
        .map_err(|error| RunBackendError::Invalid(error.to_string()))
    }
}

struct ResolvedQtiQuestion {
    envelope: QuestionEnvelope,
    parameter_hash: String,
    provenance: AttemptProvenance,
    item_id: String,
}

fn implementation_version() -> ImplementationVersion {
    ImplementationVersion {
        id: QTI_ADAPTER_ID.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn validate_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
) -> Result<(), RunBackendError> {
    if question.problem != reference.problem || question.version != reference.version {
        return Err(RunBackendError::Invalid(
            "published question does not match immutable problem version reference".to_string(),
        ));
    }
    if !matches!(question.source, QuestionSource::Qti { .. }) {
        return Err(RunBackendError::Unsupported(
            "published question is not backed by QTI".to_string(),
        ));
    }
    Ok(())
}

fn validate_attempt_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &QuestionAttempt,
) -> Result<(), RunBackendError> {
    validate_reference(reference, question)?;
    if attempt.problem != reference.problem || attempt.question_version != reference.version {
        return Err(RunBackendError::Invalid(
            "attempt does not match immutable problem version reference".to_string(),
        ));
    }
    Ok(())
}

fn validate_source_artifact(
    reference: ProblemVersionRef,
    package_object: &question_model::ObjectId,
    package_sha256: &str,
    artifact: &PublishedSourceArtifact,
) -> Result<(), RunBackendError> {
    let expected_key = ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object: *package_object,
    };
    if artifact.reference != reference
        || artifact.backend != question_model::QuestionBackend::Qti
        || artifact.object.id != *package_object
        || artifact.object.key != expected_key
        || artifact.object.bucket != Bucket::Content
        || artifact.object.category != ObjectCategory::Source
        || artifact.object.version != Some(reference.version)
        || artifact.object.sha256.to_string() != package_sha256
    {
        return Err(RunBackendError::Invalid(
            "published QTI source binding is invalid".to_string(),
        ));
    }
    Ok(())
}

fn map_store_error(error: StoreError) -> RunBackendError {
    match error {
        StoreError::Unavailable(_) => {
            RunBackendError::Unavailable("question backend is temporarily unavailable".to_string())
        }
        other => RunBackendError::Invalid(other.to_string()),
    }
}

fn map_object_error(error: ObjectStoreError) -> RunBackendError {
    match error {
        ObjectStoreError::Unavailable(_) => {
            RunBackendError::Unavailable("question backend is temporarily unavailable".to_string())
        }
        other => RunBackendError::Invalid(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use objects::memory::MemoryObjectStore;
    use objects::{ObjectKey, ObjectStore, PutObject};
    use question_model::envelope::{AssetRef, ContentBlock};
    use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, AssetId, AssignmentEnrollment, AssignmentId, AssignmentRun,
        AttemptTimerRecord, CourseId, CourseMembership, CourseMembershipRole, EnrollmentId,
        GradingDefinition, ObjectId, ProblemId, PublicationScope, QuestionAttempt,
        QuestionAttemptId, QuestionMetadata, RunId, StudentId, TenantId, UserId, VersionId,
        WorkspaceId, WorkspaceImportId,
    };
    use store::memory::{MemoryQtiGraderStore, MemoryStore};
    use store::{
        AssetDeliveryId, AssetDeliveryRecord, AssetStore, AssignmentRecord,
        AuthorizedAssetDelivery, CatalogAssetBinding, CatalogStore, CommitPreparedQtiImport,
        CommitPreparedQtiImportOutcome, CourseRecord, EnqueueJob, JobLeaseDuration, JobPayload,
        JobStore, PublishDraftCommand, QtiGradingStore, QtiImportGradingPayload, QtiImportStore,
        SessionLifetime, SessionSubject, Store, TenantContext,
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
            result: None,
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(1),
                deadline: None,
                submitted_at: None,
            },
            provenance: issued.provenance,
        }
    }

    #[tokio::test]
    async fn published_qti_reparses_and_grades_only_through_the_private_grader() {
        let fixture = fixture().await;
        let issued = fixture
            .backend
            .issue(fixture.context, fixture.reference, &fixture.question, 41)
            .await
            .expect("immutable QTI issues");
        let stored = attempt(&fixture, issued.clone());
        let envelope = fixture
            .backend
            .reproduce(
                fixture.context,
                fixture.reference,
                &fixture.question,
                &stored,
            )
            .await
            .expect("immutable QTI replays");
        assert_eq!(envelope.prompt, fixture.question.prompt);
        assert!(
            !serde_json::to_string(&envelope)
                .expect("envelope serializes")
                .contains("\"correct\":")
        );
        let right = fixture
            .backend
            .grade(
                fixture.context,
                fixture.reference,
                &fixture.question,
                &stored,
                &StudentResponse::MultipleChoice {
                    selected: vec![fixture.correct],
                },
            )
            .await
            .expect("private grader grades correct response");
        assert!(matches!(right, grading::GradeOutcome::Graded(result) if result.correct));
        let wrong = fixture
            .backend
            .grade(
                fixture.context,
                fixture.reference,
                &fixture.question,
                &stored,
                &StudentResponse::MultipleChoice {
                    selected: vec![fixture.incorrect],
                },
            )
            .await
            .expect("private grader grades wrong response");
        assert!(matches!(wrong, grading::GradeOutcome::Graded(result) if !result.correct));
        assert_eq!(fixture.grader_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn foreign_or_tampered_qti_attempt_refuses_before_grading() {
        let fixture = fixture().await;
        let issued = fixture
            .backend
            .issue(fixture.context, fixture.reference, &fixture.question, 41)
            .await
            .expect("immutable QTI issues");
        let mut stored = attempt(&fixture, issued);
        stored.provenance.rendered_question_sha256 = "tampered".to_string();
        assert!(matches!(
            fixture
                .backend
                .grade(
                    fixture.context,
                    fixture.reference,
                    &fixture.question,
                    &stored,
                    &StudentResponse::MultipleChoice {
                        selected: vec![fixture.correct.clone()],
                    },
                )
                .await,
            Err(RunBackendError::Invalid(_))
        ));
        assert_eq!(
            fixture.grader_calls.load(Ordering::SeqCst),
            0,
            "tampered provenance never reaches the private grader"
        );
        let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(
            uuid::Uuid::from_u128(7_099),
        ));
        assert!(matches!(
            fixture
                .backend
                .issue(foreign, fixture.reference, &fixture.question, 41)
                .await,
            Err(RunBackendError::Invalid(_))
        ));
        assert_eq!(
            fixture.grader_calls.load(Ordering::SeqCst),
            0,
            "foreign source resolution never reaches the private grader"
        );
    }

    #[tokio::test]
    async fn choice_image_checksum_misbinding_refuses_before_private_grading() {
        let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(7_120));
        let context = TenantContext::from_authenticated_session(tenant);
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(7_121)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(7_122)),
        };
        let workspace = WorkspaceId::from_uuid(uuid::Uuid::from_u128(7_123));
        let source_object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_124));
        let replacement_object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_125));
        let bytes = STANDARD
            .decode(CHOICE_IMAGE_PACKAGE.trim())
            .expect("choice-image QTI fixture decodes");
        let parsed = adapter_qti::QtiImporter::default()
            .import(&bytes)
            .expect("choice-image QTI fixture parses");
        let imported = parsed.questions.first().expect("choice-image item").clone();
        let expected = qti_question_asset_checksums(&imported)
            .expect("choice-image asset reference is internally consistent");
        let (asset, original_checksum) = expected
            .into_iter()
            .next()
            .expect("choice body references one asset");
        let correct = parsed
            .worker_correct_choice(&imported.item_id)
            .expect("private correct choice exists");
        let objects = Arc::new(MemoryObjectStore::default());
        let source = objects
            .put(PutObject {
                key: ObjectKey::ProblemSource {
                    problem: reference.problem,
                    version: reference.version,
                    object: source_object,
                },
                bytes,
                media_type: "application/zip".to_string(),
                license: "CC-BY-4.0".to_string(),
                provenance: "choice-image QTI fixture".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("published QTI source stores");
        let replacement = objects
            .put(PutObject {
                key: ObjectKey::ProblemAsset {
                    problem: reference.problem,
                    version: reference.version,
                    asset,
                    object: replacement_object,
                },
                bytes: b"different valid image bytes".to_vec(),
                media_type: "image/png".to_string(),
                license: "CC-BY-4.0".to_string(),
                provenance: "adversarial replacement fixture".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("replacement asset stores");
        assert_ne!(replacement.sha256.to_string(), original_checksum);
        let question = QuestionDefinition {
            problem: reference.problem,
            version: reference.version,
            workspace,
            source: QuestionSource::Qti {
                item_id: imported.item_id.clone(),
                package_object: source_object,
                package_sha256: source.sha256.to_string(),
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
                title: "Choice image checksum guard".to_string(),
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
                object: source,
            },
            bindings: vec![CatalogAssetBinding {
                asset,
                object: replacement_object,
            }],
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
        let backend = QtiBackend::new(sources, grader, objects);
        assert!(matches!(
            backend.issue(context, reference, &question, 41).await,
            Err(RunBackendError::Invalid(_))
        ));
        assert_eq!(
            grader_calls.load(Ordering::SeqCst),
            0,
            "asset misbinding must refuse before private grading"
        );
    }

    #[tokio::test]
    async fn published_qti_runs_grade_server_side_and_replay_without_a_second_private_lookup() {
        use question_model::generation::RandomizationDefinition;
        use question_model::run_policy::{
            AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure,
            GradePolicy, RunPolicies, TimingPolicy, VariationPolicy,
        };
        use question_model::{DraftQuestionDefinition, DraftQuestionSource};
        use store::{DraftRecord, ProblemVersionRef};

        let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(7_200));
        let context = TenantContext::from_authenticated_session(tenant);
        let publisher = UserId::from_uuid(uuid::Uuid::from_u128(7_201));
        let student = UserId::from_uuid(uuid::Uuid::from_u128(7_202));
        let workspace = WorkspaceId::from_uuid(uuid::Uuid::from_u128(7_203));
        let import = WorkspaceImportId::from_uuid(uuid::Uuid::from_u128(7_204));
        let source_object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_205));
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(7_206)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(7_207)),
        };
        let (store, grader) = MemoryStore::with_qti_grader();
        let store = Arc::new(store);
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(10_000))
            .expect("fixture clock");
        let grader_calls = Arc::new(AtomicUsize::new(0));
        let grader = Arc::new(CountingQtiGrader {
            inner: Arc::new(grader),
            calls: Arc::clone(&grader_calls),
        });
        let objects = Arc::new(MemoryObjectStore::default());
        let bytes = STANDARD.decode(PACKAGE.trim()).expect("QTI ZIP fixture");
        let parsed = adapter_qti::QtiImporter::default()
            .import(&bytes)
            .expect("QTI fixture parses");
        let imported = parsed.questions.first().expect("QTI item").clone();
        let ResponseDefinition::MultipleChoice { choices, .. } = &imported.response else {
            panic!("fixture is a choice item");
        };
        let correct = parsed
            .worker_correct_choice(&imported.item_id)
            .expect("private correct fixture choice");
        let wrong = choices
            .iter()
            .map(|choice| choice.id.clone())
            .find(|choice| choice != &correct)
            .expect("fixture has a wrong choice");
        objects
            .put(PutObject {
                key: ObjectKey::WorkspaceSource {
                    tenant,
                    workspace,
                    import,
                    object: source_object,
                },
                bytes,
                media_type: "application/zip".to_string(),
                license: "private-workspace-import".to_string(),
                provenance: "QTI run integration fixture".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("QTI source persists");
        QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects))
            .prepare(
                context,
                JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object,
                },
                JobExecution::new(),
            )
            .await
            .expect("QTI import prepares");
        let job = store
            .enqueue_job(
                context,
                EnqueueJob {
                    tenant,
                    payload: JobPayload::QtiImport {
                        workspace,
                        import,
                        source_object,
                    },
                    max_attempts: 1,
                },
            )
            .await
            .expect("QTI commit job");
        let claim = store
            .claim_next_job(JobLeaseDuration::from_seconds(60).expect("lease"))
            .await
            .expect("claim")
            .expect("claimed QTI job");
        assert_eq!(
            store
                .commit_prepared_qti_import(
                    context,
                    CommitPreparedQtiImport {
                        job,
                        lease: claim.lease_token,
                        reference: store::QtiImportRef {
                            tenant,
                            workspace,
                            import
                        },
                        source_object,
                    },
                )
                .await
                .expect("QTI import commit"),
            CommitPreparedQtiImportOutcome::Committed
        );
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace,
                source: DraftQuestionSource::Qti {
                    item_id: imported.item_id.clone(),
                    import_id: import,
                },
                prompt: imported.prompt,
                response: imported.response,
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateCorrectness,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "QTI run integration".to_string(),
                    tags: vec![],
                    taxonomy: vec![],
                    license: License::CcBy,
                    language: "en-US".to_string(),
                },
            },
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, publisher, None, draft.clone())
            .await
            .expect("QTI draft");
        let preparer = QtiPublicationPreparer::new(Arc::clone(&store), Arc::clone(&objects));
        let validated = preparer
            .validate(context, &draft.question, import, &imported.item_id)
            .await
            .expect("exact QTI validation");
        let prepared = preparer
            .copy_candidates(&draft, reference, validated)
            .await
            .expect("candidate copy");
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: prepared.published_source,
                    source_artifact: Some(prepared.source_artifact),
                    qti_promotion: Some(prepared.promotion),
                    publisher,
                    scope: PublicationScope::Institution,
                    capabilities: question_model::BackendCapabilities::from_iter([
                        question_model::Capability::ServerGrading,
                    ]),
                },
            )
            .await
            .expect("QTI publish");
        let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(
            uuid::Uuid::from_u128(7_299),
        ));
        assert!(
            store
                .get_catalog_problem(foreign, reference)
                .await
                .expect("foreign catalog lookup")
                .is_none()
        );
        assert!(
            store
                .get_published_problem(reference.problem, reference.version)
                .await
                .expect("public catalog lookup")
                .is_none()
        );

        let course = CourseId::from_uuid(uuid::Uuid::from_u128(7_208));
        let assignment = AssignmentId::from_uuid(uuid::Uuid::from_u128(7_209));
        store
            .upsert_course(
                context,
                CourseRecord {
                    id: course,
                    tenant,
                    title: "QTI course".to_string(),
                    members: vec![
                        CourseMembership {
                            user: publisher,
                            role: CourseMembershipRole::Instructor,
                        },
                        CourseMembership {
                            user: student,
                            role: CourseMembershipRole::Student,
                        },
                    ],
                },
            )
            .await
            .expect("course");
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    title: "QTI assignment".to_string(),
                    problems: vec![reference],
                    policies: RunPolicies {
                        completion: CompletionRequirement::AllCorrect,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                },
            )
            .await
            .expect("assignment");
        store
            .create_enrollment(
                context,
                AssignmentEnrollment {
                    id: EnrollmentId::from_uuid(uuid::Uuid::from_u128(7_210)),
                    tenant,
                    assignment,
                    user: student,
                    student: StudentId::from_uuid(uuid::Uuid::from_u128(7_211)),
                    first_completed_at: None,
                    current_grade_run: None,
                    best_grade_run: None,
                },
            )
            .await
            .expect("enrollment");
        let cookie = issue_session(
            store.as_ref(),
            SessionSubject::new(
                tenant,
                student,
                "QTI learner",
                vec![question_model::UserRole::Student],
            )
            .expect("session subject"),
            SessionConfig::new(
                SessionLifetime::from_seconds(3600).expect("lifetime"),
                CookieTransport::LocalHttp,
            ),
        )
        .await
        .expect("session")
        .set_cookie
        .split(';')
        .next()
        .expect("cookie")
        .to_string();
        let app = run_router(
            Arc::clone(&store),
            Arc::new(QtiBackend::new(
                Arc::clone(&store),
                Arc::clone(&grader),
                Arc::clone(&objects),
            )),
        );
        let run = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/runs")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "assignmentId": assignment }).to_string(),
                    ))
                    .expect("start request"),
            )
            .await
            .expect("start response");
        assert_eq!(run.status(), axum::http::StatusCode::CREATED);
        let run: AssignmentRun = serde_json::from_slice(
            &axum::body::to_bytes(run.into_body(), 64 * 1024)
                .await
                .expect("run body"),
        )
        .expect("run JSON");
        let attempt = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/runs/{}/attempts", run.id))
                    .header("cookie", &cookie)
                    .body(axum::body::Body::empty())
                    .expect("attempt list"),
            )
            .await
            .expect("attempt list response");
        let attempt: QuestionAttempt = serde_json::from_value(
            serde_json::from_slice::<serde_json::Value>(
                &axum::body::to_bytes(attempt.into_body(), 64 * 1024)
                    .await
                    .expect("attempt body"),
            )
            .expect("attempt JSON")["items"][0]
                .clone(),
        )
        .expect("attempt projection");
        let envelope = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/attempts/{}/question", attempt.id))
                    .header("cookie", &cookie)
                    .body(axum::body::Body::empty())
                    .expect("question request"),
            )
            .await
            .expect("question response");
        assert_eq!(envelope.status(), axum::http::StatusCode::OK);
        let envelope_json: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(envelope.into_body(), 64 * 1024)
                .await
                .expect("envelope body"),
        )
        .expect("envelope JSON");
        for forbidden in [
            "answerKey",
            "correctResponse",
            "gradingPayload",
            "privateGrading",
        ] {
            assert!(
                !envelope_json.to_string().contains(forbidden),
                "envelope leaked {forbidden}"
            );
        }
        let submit = |choice: &ChoiceId, key: &str| {
            axum::http::Request::builder().method("POST").uri(format!("/api/submissions/{}", attempt.id)).header("cookie", &cookie).header("content-type", "application/json").header("idempotency-key", key).body(axum::body::Body::from(serde_json::json!({ "response": { "kind": "multipleChoice", "selected": [choice] } }).to_string())).expect("submit request")
        };
        let wrong_response = app
            .clone()
            .oneshot(submit(&wrong, "qti-wrong"))
            .await
            .expect("wrong response");
        assert_eq!(wrong_response.status(), axum::http::StatusCode::OK);
        let wrong_json = axum::body::to_bytes(wrong_response.into_body(), 64 * 1024)
            .await
            .expect("wrong JSON");
        let wrong_text = String::from_utf8_lossy(&wrong_json);
        assert!(
            !wrong_text.contains(&format!("\"correct\":\"{}\"", correct.as_str()))
                && !wrong_text.contains("correct-choice")
                && !wrong_text.contains("answerKey")
                && !wrong_text.contains("gradingPayload"),
            "receipt leaked QTI private grading material"
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&wrong_json).expect("receipt")["feedback"]
                ["correctness"],
            false
        );
        assert_eq!(grader_calls.load(Ordering::SeqCst), 1);
        let replay = app
            .clone()
            .oneshot(submit(&wrong, "qti-wrong"))
            .await
            .expect("replay response");
        assert_eq!(replay.status(), axum::http::StatusCode::OK);
        assert_eq!(
            wrong_json,
            axum::body::to_bytes(replay.into_body(), 64 * 1024)
                .await
                .expect("replay body")
        );
        assert_eq!(
            grader_calls.load(Ordering::SeqCst),
            1,
            "replay must not regrade"
        );
        let next = store
            .list_question_attempts(
                context,
                run.id,
                store::PageRequest::first(store::PageSize::new(10).expect("page")),
            )
            .await
            .expect("attempts");
        let retry = next
            .items
            .into_iter()
            .find(|value| value.response.is_none())
            .expect("retry after wrong response");
        let correct_response = app.oneshot(axum::http::Request::builder().method("POST").uri(format!("/api/submissions/{}", retry.id)).header("cookie", &cookie).header("content-type", "application/json").header("idempotency-key", "qti-correct").body(axum::body::Body::from(serde_json::json!({ "response": { "kind": "multipleChoice", "selected": [correct] } }).to_string())).expect("correct request")).await.expect("correct response");
        assert_eq!(correct_response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &axum::body::to_bytes(correct_response.into_body(), 64 * 1024)
                    .await
                    .expect("correct body")
            )
            .expect("correct receipt")["feedback"]["correctness"],
            true
        );
        assert_eq!(grader_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn choice_body_images_are_part_of_the_published_qti_asset_contract() {
        let prompt_asset = AssetId::from_uuid(uuid::Uuid::from_u128(7_100));
        let choice_asset = AssetId::from_uuid(uuid::Uuid::from_u128(7_101));
        let prompt = vec![ContentBlock::Image {
            asset: AssetRef {
                asset: prompt_asset,
                checksum: "a".repeat(64),
            },
            description: "prompt image".to_string(),
        }];
        let response = ResponseDefinition::MultipleChoice {
            choices: vec![ChoiceOption {
                id: ChoiceId::new("choice-a"),
                body: vec![ContentBlock::Image {
                    asset: AssetRef {
                        asset: choice_asset,
                        checksum: "b".repeat(64),
                    },
                    description: "choice image".to_string(),
                }],
            }],
            selection: question_model::answer::SelectionCardinality::Exactly { count: 1 },
        };
        assert_eq!(
            qti_question_asset_checksums(&adapter_qti::ImportedQtiQuestion {
                item_id: "choice-image".to_string(),
                prompt,
                response,
            })
            .expect("distinct image references"),
            std::collections::BTreeMap::from([
                (prompt_asset, "a".repeat(64)),
                (choice_asset, "b".repeat(64)),
            ])
        );
    }
}
