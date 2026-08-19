//! Assignment-export worker preparation and atomic bundle finalization.
//!
//! The durable export request freezes ordered published version references and
//! four object identities before it enters the queue. This module resolves
//! only those inputs under the claimed tenant, writes immutable private bytes,
//! and hands the closed metadata effect to one Store transaction. It never
//! receives browser-selected questions, paths, URLs, answer keys, or bytes.

use std::{collections::BTreeMap, future::Future, sync::Arc};

use async_trait::async_trait;
use export_crate::{
    ExportArtifact, ExportCandidate, PrintExam, PrintableAsset, TrustedAssetResolver,
};
use learning_data_access::{
    AuthoritativeTimeStore, CatalogAssetBinding, CatalogStore, ExportArtifactKind,
    ExportArtifactRecord, ExportCommitDisposition, ExportJobCommit, ExportJobStore, JobFailureKind,
    JobPayload, StoreError, TenantContext,
};
use objects::{
    Bucket, ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, ObjectStoreError, PutObject,
    Sha256Digest,
};
use question_model::{
    AssetId, ObjectId, ProblemVersionRef, QuestionDefinition, ResponseDefinition,
    envelope::{AssetRef, ContentBlock},
};

use crate::worker::{
    self, EffectCommitOutcome, EffectCommitter, JobCommitClaim, JobExecution, JobHandler,
    PreparedExportArtifacts, PreparedJobEffect,
};

/// Cancellable producer for one immutable four-artifact assignment export.
pub(crate) struct ExportJobHandler<S, O> {
    store: Arc<S>,
    objects: Arc<O>,
}

impl<S, O> ExportJobHandler<S, O> {
    pub(crate) fn new(store: Arc<S>, objects: Arc<O>) -> Self {
        Self { store, objects }
    }
}

/// The server-only finalization sink for assignment-export effects.
pub(crate) struct ExportJobCommitter<S> {
    store: Arc<S>,
}

impl<S> ExportJobCommitter<S> {
    pub(crate) fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

fn store_failure(error: StoreError) -> JobFailureKind {
    match error {
        StoreError::RetryableTransaction | StoreError::Unavailable(_) => JobFailureKind::Transient,
        StoreError::NotFound
        | StoreError::AlreadyExists
        | StoreError::TenantMismatch
        | StoreError::Conflict
        | StoreError::Forbidden
        | StoreError::InvalidRecord(_)
        | StoreError::RunModel(_)
        | StoreError::TimedOut => JobFailureKind::Permanent,
    }
}

fn object_failure(error: ObjectStoreError) -> JobFailureKind {
    match error {
        ObjectStoreError::Unavailable(_) => JobFailureKind::Transient,
        ObjectStoreError::NotFound
        | ObjectStoreError::ChecksumMismatch
        | ObjectStoreError::AlreadyExists
        | ObjectStoreError::NotSignable
        | ObjectStoreError::NumericOverflow => JobFailureKind::Permanent,
    }
}

enum CancellableOperation<E> {
    Cancelled,
    Operation(E),
}

/// Selects every remote/store operation against the worker cancellation token.
/// Object and Store calls remain ordinary async operations; no detached task or
/// blocking worker thread survives a timed-out lease.
async fn cancellable<F, T, E>(
    execution: &JobExecution,
    operation: F,
) -> Result<T, CancellableOperation<E>>
where
    F: Future<Output = Result<T, E>>,
{
    tokio::select! {
        result = operation => result.map_err(CancellableOperation::Operation),
        () = execution.cancelled() => Err(CancellableOperation::Cancelled),
    }
}

fn map_store<T>(result: Result<T, CancellableOperation<StoreError>>) -> Result<T, JobFailureKind> {
    result.map_err(|error| match error {
        CancellableOperation::Cancelled => JobFailureKind::TimedOut,
        CancellableOperation::Operation(error) => store_failure(error),
    })
}

fn map_object<T>(
    result: Result<T, CancellableOperation<ObjectStoreError>>,
) -> Result<T, JobFailureKind> {
    result.map_err(|error| match error {
        CancellableOperation::Cancelled => JobFailureKind::TimedOut,
        CancellableOperation::Operation(error) => object_failure(error),
    })
}

/// Synchronous resolver passed only verified immutable bytes already fetched
/// under the worker's tenant context.
struct ExportAssetResolver {
    assets: BTreeMap<AssetId, PrintableAsset>,
}

impl TrustedAssetResolver for ExportAssetResolver {
    fn resolve(&self, asset: &AssetRef) -> Result<PrintableAsset, String> {
        self.assets
            .get(&asset.asset)
            .cloned()
            .ok_or_else(|| "published asset binding is absent".to_string())
    }
}

fn referenced_assets(question: &QuestionDefinition) -> Vec<AssetRef> {
    let mut assets = assets_in_blocks(&question.prompt);
    match &question.response {
        ResponseDefinition::MultipleChoice { choices, .. } => {
            for choice in choices {
                assets.extend(assets_in_blocks(&choice.body));
            }
        }
        ResponseDefinition::Ordering { items } => {
            for item in items {
                assets.extend(assets_in_blocks(&item.body));
            }
        }
        ResponseDefinition::MultiBlank { blanks } => {
            for blank in blanks {
                assets.extend(assets_in_blocks(&blank.label));
            }
        }
        ResponseDefinition::Matching { prompts, choices } => {
            for item in prompts.iter().chain(choices) {
                assets.extend(assets_in_blocks(&item.body));
            }
        }
        ResponseDefinition::Hotspot {
            surface, regions, ..
        } => {
            assets.push(surface.clone());
            for region in regions {
                assets.extend(assets_in_blocks(&region.label));
            }
        }
        ResponseDefinition::Numeric { .. }
        | ResponseDefinition::ShortText { .. }
        | ResponseDefinition::FileUpload { .. }
        | ResponseDefinition::ExternalTool {} => {}
    }
    assets
}

fn assets_in_blocks(blocks: &[ContentBlock]) -> Vec<AssetRef> {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Image { asset, .. } => Some(asset.clone()),
            ContentBlock::Text { .. }
            | ContentBlock::Math { .. }
            | ContentBlock::Code { .. }
            | ContentBlock::Table { .. } => None,
        })
        .collect()
}

fn expected_artifacts(
    expected: &[(ExportArtifactKind, ObjectId)],
) -> Result<BTreeMap<ExportArtifactKind, ObjectId>, JobFailureKind> {
    if expected.len() != ExportArtifactKind::ALL.len() {
        return Err(JobFailureKind::Permanent);
    }
    let mapped = expected.iter().copied().collect::<BTreeMap<_, _>>();
    if mapped.len() != ExportArtifactKind::ALL.len()
        || ExportArtifactKind::ALL
            .iter()
            .any(|kind| !mapped.contains_key(kind))
    {
        return Err(JobFailureKind::Permanent);
    }
    Ok(mapped)
}

fn validate_published(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
) -> Result<(), JobFailureKind> {
    if question.problem != reference.problem || question.version != reference.version {
        return Err(JobFailureKind::Permanent);
    }
    Ok(())
}

fn binding_index(
    bindings: Vec<CatalogAssetBinding>,
) -> Result<BTreeMap<AssetId, ObjectId>, JobFailureKind> {
    let mut indexed = BTreeMap::new();
    for binding in bindings {
        if indexed.insert(binding.asset, binding.object).is_some() {
            return Err(JobFailureKind::Permanent);
        }
    }
    Ok(indexed)
}

fn exact_output_record(
    record: ObjectRecord,
    key: &ObjectKey,
    bytes: &[u8],
    media_type: &str,
) -> Result<ObjectRecord, JobFailureKind> {
    let size = u64::try_from(bytes.len()).map_err(|_| JobFailureKind::Permanent)?;
    if record.key != *key
        || record.id != key.object_id()
        || record.bucket != Bucket::StudentRecords
        || record.category != ObjectCategory::Export
        || record.media_type != media_type
        || record.size_bytes != size
        || record.sha256 != Sha256Digest::compute(bytes)
    {
        return Err(JobFailureKind::Permanent);
    }
    Ok(record)
}

async fn put_artifact<O: ObjectStore>(
    objects: &O,
    execution: &JobExecution,
    tenant: question_model::TenantId,
    object: ObjectId,
    kind: ExportArtifactKind,
    artifact: ExportArtifact,
    created_at: question_model::ActivityTimestamp,
) -> Result<ExportArtifactRecord, JobFailureKind> {
    if artifact.media_type != kind.media_type() {
        return Err(JobFailureKind::Permanent);
    }
    let key = ObjectKey::StudentRecord { tenant, object };
    let bytes = artifact.bytes;
    let media_type = artifact.media_type.to_string();
    let record = match cancellable(
        execution,
        objects.put(PutObject {
            key: key.clone(),
            bytes: bytes.clone(),
            media_type: media_type.clone(),
            license: "educational-record".to_string(),
            provenance: "assignment export".to_string(),
            created_at,
        }),
    )
    .await
    {
        Ok(record) => record,
        Err(CancellableOperation::Operation(ObjectStoreError::AlreadyExists)) => {
            map_object(cancellable(execution, objects.get(&key)).await)?.record
        }
        Err(CancellableOperation::Cancelled) => return Err(JobFailureKind::TimedOut),
        Err(CancellableOperation::Operation(error)) => return Err(object_failure(error)),
    };
    let object = exact_output_record(record, &key, &bytes, &media_type)?;
    Ok(ExportArtifactRecord {
        kind,
        filename: artifact.filename,
        object,
    })
}

#[async_trait]
impl<S, O> JobHandler for ExportJobHandler<S, O>
where
    S: ExportJobStore
        + CatalogStore
        + learning_data_access::AssetStore
        + AuthoritativeTimeStore
        + Send
        + Sync
        + 'static,
    O: ObjectStore + Send + Sync + 'static,
{
    async fn prepare(
        &self,
        context: TenantContext,
        payload: JobPayload,
        execution: JobExecution,
    ) -> Result<PreparedJobEffect, JobFailureKind> {
        let JobPayload::Export {
            delivery_object: manifest,
        } = payload
        else {
            return Err(JobFailureKind::Permanent);
        };
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let export = map_store(
            cancellable(&execution, self.store.load_export_job(context, manifest)).await,
        )?
        .ok_or(JobFailureKind::Permanent)?;
        if export.tenant != context.tenant_id() || export.manifest != manifest {
            return Err(JobFailureKind::Permanent);
        }
        let output_ids = expected_artifacts(&export.expected_artifacts)?;

        let mut questions = Vec::with_capacity(export.problems.len());
        let mut capabilities = Vec::with_capacity(export.problems.len());
        let mut assets = BTreeMap::new();
        for reference in &export.problems {
            if execution.cancellation_requested() {
                return Err(JobFailureKind::TimedOut);
            }
            let published = map_store(
                cancellable(
                    &execution,
                    self.store.get_catalog_problem(context, *reference),
                )
                .await,
            )?
            .ok_or(JobFailureKind::Permanent)?;
            validate_published(*reference, &published.question)?;
            let bindings = map_store(
                cancellable(
                    &execution,
                    self.store.catalog_asset_bindings(context, *reference),
                )
                .await,
            )?;
            let bindings = binding_index(bindings)?;
            for asset in referenced_assets(&published.question) {
                if execution.cancellation_requested() {
                    return Err(JobFailureKind::TimedOut);
                }
                let object = bindings
                    .get(&asset.asset)
                    .copied()
                    .ok_or(JobFailureKind::Permanent)?;
                let key = ObjectKey::published_problem_asset(
                    published.scope,
                    reference.problem,
                    reference.version,
                    asset.asset,
                    object,
                );
                let stored = map_object(cancellable(&execution, self.objects.get(&key)).await)?;
                if stored.record.id != object
                    || stored.record.key != key
                    || stored.record.bucket != key.bucket()
                    || stored.record.category != ObjectCategory::Asset
                    || stored.record.sha256.to_string() != asset.checksum
                {
                    return Err(JobFailureKind::Permanent);
                }
                let printable = PrintableAsset {
                    media_type: stored.record.media_type,
                    bytes: stored.bytes,
                };
                match assets.get(&asset.asset) {
                    Some(existing) if existing != &printable => {
                        return Err(JobFailureKind::Permanent);
                    }
                    Some(_) => {}
                    None => {
                        assets.insert(asset.asset, printable);
                    }
                }
            }
            questions.push(published.question);
            capabilities.push(published.capabilities);
        }
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let candidates = questions
            .iter()
            .zip(&capabilities)
            .map(|(question, capabilities)| ExportCandidate {
                question,
                capabilities,
            });
        let exam =
            PrintExam::build_with_assets(export.title, candidates, &ExportAssetResolver { assets })
                .map_err(|_| JobFailureKind::Permanent)?;
        let bundle = exam.render_all();
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let created_at =
            map_store(cancellable(&execution, self.store.authoritative_time(context)).await)?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let docx = put_artifact(
            self.objects.as_ref(),
            &execution,
            export.tenant,
            output_ids[&ExportArtifactKind::Docx],
            ExportArtifactKind::Docx,
            bundle.docx,
            created_at,
        )
        .await?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let pdf = put_artifact(
            self.objects.as_ref(),
            &execution,
            export.tenant,
            output_ids[&ExportArtifactKind::Pdf],
            ExportArtifactKind::Pdf,
            bundle.pdf,
            created_at,
        )
        .await?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let accessible_docx = put_artifact(
            self.objects.as_ref(),
            &execution,
            export.tenant,
            output_ids[&ExportArtifactKind::AccessibleDocx],
            ExportArtifactKind::AccessibleDocx,
            bundle.accessible_docx,
            created_at,
        )
        .await?;
        if execution.cancellation_requested() {
            return Err(JobFailureKind::TimedOut);
        }
        let accessible_pdf = put_artifact(
            self.objects.as_ref(),
            &execution,
            export.tenant,
            output_ids[&ExportArtifactKind::AccessiblePdf],
            ExportArtifactKind::AccessiblePdf,
            bundle.accessible_pdf,
            created_at,
        )
        .await?;
        Ok(PreparedJobEffect::Export {
            tenant: export.tenant,
            manifest,
            artifacts: Box::new(PreparedExportArtifacts {
                docx,
                pdf,
                accessible_docx,
                accessible_pdf,
            }),
        })
    }
}

impl<S> worker::sealed::EffectCommitter for ExportJobCommitter<S> where S: Send + Sync + 'static {}

#[async_trait]
impl<S> EffectCommitter for ExportJobCommitter<S>
where
    S: ExportJobStore + Send + Sync + 'static,
{
    async fn commit(
        &self,
        claim: JobCommitClaim,
        effect: PreparedJobEffect,
    ) -> Result<EffectCommitOutcome, StoreError> {
        let PreparedJobEffect::Export {
            tenant,
            manifest,
            artifacts,
        } = effect
        else {
            return Err(StoreError::InvalidRecord(
                "export committer received another effect family".to_string(),
            ));
        };
        match self
            .store
            .commit_export_effect(
                TenantContext::from_authenticated_session(tenant),
                ExportJobCommit {
                    job: claim.job_id(),
                    lease: claim.lease_token(),
                    manifest,
                    artifacts: (*artifacts).into_records(),
                },
            )
            .await?
        {
            ExportCommitDisposition::Committed | ExportCommitDisposition::AlreadyCommitted => {
                Ok(EffectCommitOutcome::Committed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use learning_data_access::in_memory::MemoryStore;
    use learning_data_access::{
        AssetStore, AssignmentRecord, CatalogStore, CourseRecord, CreateAssignmentExport,
        CreateCourseCommand, DraftRecord, ExportJobStore, JobLeaseDuration, JobPayload, JobStore,
        PublishDraftCommand, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash,
        Store,
    };
    use objects::{
        Bucket, ObjectCategory, ObjectKey, ObjectRecord, ObjectStore, PutObject, Sha256Digest,
        memory::MemoryObjectStore,
    };
    use question_model::answer::NumericTolerance;
    use question_model::generation::RandomizationDefinition;
    use question_model::run_policy::{
        AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
        TimingPolicy, VariationPolicy,
    };
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, AssignmentId, BackendCapabilities, Capability, CourseId,
        DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, ObjectId, ProblemId,
        ProblemVersionRef, PublicationScope, QuestionMetadata, QuestionSource, ResponseDefinition,
        RunPolicies, TenantId, UserId, UserRole, VersionId, WorkspaceId,
    };
    use uuid::Uuid;

    use super::*;

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn object(value: u128) -> ObjectId {
        ObjectId::from_uuid(id(value))
    }

    fn tenant(value: u128) -> TenantId {
        TenantId::from_uuid(id(value))
    }

    fn policies() -> RunPolicies {
        RunPolicies {
            completion: CompletionRequirement::AllCorrect,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        }
    }

    async fn published_fixture(
        store: &MemoryStore,
        context: TenantContext,
        tenant: TenantId,
        author: UserId,
    ) -> ProblemVersionRef {
        let reference = ProblemVersionRef {
            problem: ProblemId::from_uuid(id(30)),
            version: VersionId::from_uuid(id(31)),
        };
        let draft = DraftRecord {
            tenant,
            question: DraftQuestionDefinition {
                workspace: WorkspaceId::from_uuid(id(32)),
                source: DraftQuestionSource::Native {
                    family: "export-fixture".to_string(),
                },
                prompt: vec![ContentBlock::Text {
                    markdown: "Identify the peptide bond.".to_string(),
                }],
                response: ResponseDefinition::Numeric {
                    tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                    unit: None,
                },
                attempt_policy: AttemptPolicy {
                    max_attempts: None,
                    feedback: FeedbackDisclosure::ImmediateFull,
                },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::AllOrNothing { points: 1.0 },
                metadata: QuestionMetadata {
                    title: "Export fixture".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBySa,
                    language: "en-US".to_string(),
                },
            },
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, author, None, draft.clone())
            .await
            .expect("fixture draft saves");
        store
            .publish_draft(
                context,
                author,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: QuestionSource::Native {
                        family: "export-fixture".to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: author,
                    scope: PublicationScope::Public,
                    byline: question_model::PublicByline::new(vec![
                        question_model::PublicAuthorName::new("PLE fixture".to_string())
                            .expect("valid test byline"),
                    ])
                    .expect("valid test byline"),
                    capabilities: BackendCapabilities::from_iter([
                        Capability::ServerGrading,
                        Capability::PrintExport,
                    ]),
                },
            )
            .await
            .expect("fixture publishes");
        reference
    }

    async fn export_fixture() -> (
        Arc<MemoryStore>,
        Arc<MemoryObjectStore>,
        TenantContext,
        UserId,
        learning_data_access::StudentExportView,
    ) {
        let store = Arc::new(MemoryStore::default());
        let objects = Arc::new(MemoryObjectStore::default());
        let tenant = tenant(20);
        let context = TenantContext::from_authenticated_session(tenant);
        let author = UserId::from_uuid(id(21));
        let course = CourseId::from_uuid(id(22));
        let assignment = AssignmentId::from_uuid(id(23));
        store
            .create_course(
                context,
                CreateCourseCommand {
                    course: CourseRecord {
                        id: course,
                        tenant,
                        title: "BIOC 301".to_string(),
                        term: question_model::CourseTerm::from_parts(
                            "2026-08-24",
                            "2026-12-18",
                            "America/Chicago",
                        )
                        .expect("explicit fixture course term"),
                    },
                    initial_instructor: author,
                },
            )
            .await
            .expect("fixture course saves");
        let reference = published_fixture(store.as_ref(), context, tenant, author).await;
        store
            .create_untimed_assignment(
                context,
                AssignmentRecord {
                    id: assignment,
                    tenant,
                    course_id: course,
                    audience: question_model::AssignmentAudience::CourseWide,
                    title: "Peptide bond exam".to_string(),
                    items: vec![question_model::AssignmentItem {
                        id: question_model::AssignmentItemId::from_uuid(id(24)),
                        reference,
                        position: 0,
                        points_possible: question_model::PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    }],
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await
            .expect("fixture assignment saves");
        let session = SessionTokenHash::compute(b"export-worker-fixture-instructor");
        store
            .create_session(
                session,
                SessionSubject::new(
                    tenant,
                    author,
                    "Export worker fixture",
                    vec![UserRole::Instructor],
                )
                .expect("fixture session subject"),
                SessionLifetime::from_seconds(3_600).expect("fixture session lifetime"),
            )
            .await
            .expect("fixture session saves");
        let view = store
            .create_assignment_export(
                context,
                session,
                CreateAssignmentExport {
                    assignment,
                    max_attempts: 2,
                },
            )
            .await
            .expect("fixture export queues");
        (store, objects, context, author, view)
    }

    async fn queued_export() -> (
        Arc<MemoryStore>,
        Arc<MemoryObjectStore>,
        TenantContext,
        learning_data_access::ClaimedJob,
    ) {
        let (store, objects, context, _author, _view) = export_fixture().await;
        let claimed = store
            .claim_next_job(
                &learning_data_access::JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(60).expect("lease"),
            )
            .await
            .expect("claim reads")
            .expect("export job available");
        (store, objects, context, claimed)
    }

    #[test]
    fn export_effect_requires_the_closed_four_kind_set() {
        let expected = [
            (ExportArtifactKind::Docx, object(1)),
            (ExportArtifactKind::Pdf, object(2)),
            (ExportArtifactKind::AccessibleDocx, object(3)),
            (ExportArtifactKind::AccessiblePdf, object(4)),
        ];
        assert!(expected_artifacts(&expected).is_ok());
        assert_eq!(
            expected_artifacts(&expected[..3]),
            Err(JobFailureKind::Permanent)
        );
        let duplicate = [
            (ExportArtifactKind::Docx, object(1)),
            (ExportArtifactKind::Docx, object(2)),
            (ExportArtifactKind::AccessibleDocx, object(3)),
            (ExportArtifactKind::AccessiblePdf, object(4)),
        ];
        assert_eq!(
            expected_artifacts(&duplicate),
            Err(JobFailureKind::Permanent)
        );
    }

    #[test]
    fn private_export_object_must_match_the_tenant_bound_target_and_bytes() {
        let owner_tenant = tenant(10);
        let object = object(11);
        let key = ObjectKey::StudentRecord {
            tenant: owner_tenant,
            object,
        };
        let bytes = b"verified export";
        let record = ObjectRecord {
            id: object,
            bucket: Bucket::StudentRecords,
            key: key.clone(),
            sha256: Sha256Digest::compute(bytes),
            size_bytes: u64::try_from(bytes.len()).expect("fixture length"),
            media_type: ExportArtifactKind::Pdf.media_type().to_string(),
            category: ObjectCategory::Export,
            version: None,
            license: "educational-record".to_string(),
            provenance: "fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        };
        assert!(
            exact_output_record(
                record.clone(),
                &key,
                bytes,
                ExportArtifactKind::Pdf.media_type(),
            )
            .is_ok()
        );
        let foreign_key = ObjectKey::StudentRecord {
            tenant: tenant(12),
            object,
        };
        assert_eq!(
            exact_output_record(
                record,
                &foreign_key,
                bytes,
                ExportArtifactKind::Pdf.media_type(),
            ),
            Err(JobFailureKind::Permanent)
        );
    }

    #[tokio::test]
    async fn prepare_reuses_exact_private_objects_after_a_precommit_crash() {
        let (store, objects, context, claimed) = queued_export().await;
        let handler = ExportJobHandler::new(Arc::clone(&store), Arc::clone(&objects));
        let first = handler
            .prepare(context, claimed.payload.clone(), JobExecution::new())
            .await
            .expect("first preparation writes private objects");
        let retry = handler
            .prepare(context, claimed.payload, JobExecution::new())
            .await
            .expect("retry accepts exact immutable objects");
        assert_eq!(first, retry);
    }

    #[tokio::test]
    async fn prepare_refuses_a_different_existing_private_output() {
        let (store, objects, context, claimed) = queued_export().await;
        let JobPayload::Export {
            delivery_object: manifest,
        } = claimed.payload.clone()
        else {
            panic!("fixture claimed an export job");
        };
        let export = store
            .load_export_job(context, manifest)
            .await
            .expect("private job resolves")
            .expect("private job exists");
        let object = export
            .expected_artifacts
            .iter()
            .find_map(|(kind, object)| (*kind == ExportArtifactKind::Docx).then_some(*object))
            .expect("closed docx target");
        objects
            .put(PutObject {
                key: ObjectKey::StudentRecord {
                    tenant: context.tenant_id(),
                    object,
                },
                bytes: b"wrong immutable output".to_vec(),
                media_type: ExportArtifactKind::Docx.media_type().to_string(),
                license: "educational-record".to_string(),
                provenance: "test conflict".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("conflicting private object persists");
        let handler = ExportJobHandler::new(store, objects);
        assert_eq!(
            handler
                .prepare(context, claimed.payload, JobExecution::new())
                .await,
            Err(JobFailureKind::Permanent)
        );
    }

    #[tokio::test]
    async fn worker_commits_one_closed_private_bundle_and_keeps_delivery_tenant_owned() {
        let (store, objects, context, requester, queued) = export_fixture().await;
        let handler: Arc<dyn JobHandler> = Arc::new(ExportJobHandler::new(
            Arc::clone(&store),
            Arc::clone(&objects),
        ));
        let committer: Arc<dyn EffectCommitter> =
            Arc::new(ExportJobCommitter::new(Arc::clone(&store)));
        let registry = worker::JobRegistry::new([worker::JobRegistryEntry::new(
            learning_data_access::JobKind::Export,
            handler,
            committer,
        )])
        .expect("registry");
        let worker = worker::Worker::new(
            Arc::clone(&store),
            registry,
            worker::WorkerSettings::new(60, std::time::Duration::from_secs(10), 1)
                .expect("bounded worker settings"),
        );
        let report = worker.drain_once().await.expect("worker drains export");
        assert_eq!(report.completed, 1);
        let ready = store
            .get_assignment_export(context, queued.id)
            .await
            .expect("ready view reads")
            .expect("ready export exists");
        assert_eq!(ready.state, learning_data_access::StudentExportState::Ready);
        let artifacts = ready
            .artifacts
            .as_ref()
            .expect("all four deliveries become visible together");
        assert_eq!(artifacts.len(), ExportArtifactKind::ALL.len());
        for artifact in artifacts {
            let authorized = store
                .authorize_asset_delivery(context, requester, artifact.delivery)
                .await
                .expect("requester is authorized and access is audited");
            assert!(matches!(
                authorized.record.object.key,
                ObjectKey::StudentRecord { tenant, .. } if tenant == context.tenant_id()
            ));
            assert_eq!(
                store
                    .authorize_asset_delivery(
                        context,
                        UserId::from_uuid(id(99)),
                        artifact.delivery,
                    )
                    .await,
                Err(learning_data_access::StoreError::NotFound)
            );
            assert_eq!(
                store
                    .authorize_asset_delivery(
                        TenantContext::from_authenticated_session(tenant(98)),
                        requester,
                        artifact.delivery,
                    )
                    .await,
                Err(learning_data_access::StoreError::NotFound)
            );
        }
        let json = serde_json::to_string(&ready).expect("safe status serializes");
        assert!(!json.contains("provenance") && !json.contains("source") && !json.contains("key"));
    }
}
