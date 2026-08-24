//! Server-only issue bridge and contract-only grading for published QTI.
//!
//! Trusted issue preparation resolves the published archive and copies the
//! private grading payload into attempt-local evidence.  First grading and
//! replay never reopen a catalog, source object, or private grading lookup.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_qti::qti_question_asset_checksums;
use async_trait::async_trait;
use learning_data_access::{
    AssetStore, CatalogSourceStore, PublishedSourceArtifact, QtiGradingStore, StoreError,
    TenantContext,
};
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError, Sha256Digest};
use question_model::generation::Seed;
use question_model::{
    AttemptProvenance, ImplementationVersion, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition, QuestionEnvelope, QuestionSource, SourceArtifact, StudentResponse,
};

use crate::run::{
    GradeReceipt, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition,
};

const QTI_ADAPTER_ID: &str = adapter_qti::QtiProfileId::GENERIC.as_str();

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
            // The durable catalog binding names the exact immutable key.  Do
            // not probe alternate public/private keys: scope is selected at
            // publication and must not be re-inferred from storage presence.
            let key = binding.key.clone();
            let stored = self.objects.get(&key).await.map_err(map_object_error)?;
            if stored.record.id != binding.object
                || stored.record.key != key
                || stored.record.bucket != key.bucket()
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
        let payload = self
            .grader
            .qti_publication_grading(context, reference, &resolved.item_id)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Unavailable(
                    "published QTI grading binding is unavailable during issue preparation"
                        .to_string(),
                )
            })?;
        let qti_grading = learning_data_access::IssuedQtiGradingContractV1::new(
            question,
            resolved.item_id.clone(),
            payload,
        )
        .map_err(map_store_error)?;
        Ok(IssuedAttemptMetadata {
            envelope: resolved.envelope,
            parameter_hash: resolved.parameter_hash,
            provenance: resolved.provenance,
            webwork_replay: None,
            flat_grading: None,
            flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
            webwork_grading: None,
            webwork_grading_capability:
                learning_data_access::WebworkGradingCapability::NotApplicable,
            qti_grading: Some(qti_grading),
            qti_grading_capability: learning_data_access::QtiGradingCapability::Required,
        })
    }

    async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
    ) -> Result<QuestionEnvelope, RunBackendError> {
        let _ = (context, reference, question, attempt);
        Err(RunBackendError::Unsupported(
            "QTI replay uses the issued receipt before backend reconstruction".to_string(),
        ))
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        let _ = (context, reference, question, attempt, response);
        Err(RunBackendError::Unsupported(
            "QTI first grading requires the prepared issued contract".to_string(),
        ))
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        validate_attempt_reference(
            submission.reference,
            submission.question(),
            submission.attempt,
        )?;
        let contract = submission.issued_qti_grading.ok_or_else(|| {
            RunBackendError::Unavailable("issued QTI grading contract is unavailable".to_string())
        })?;
        if contract.item_id()
            != match &submission.question().source {
                QuestionSource::Qti { item_id, .. } => item_id,
                _ => {
                    return Err(RunBackendError::Unsupported(
                        "issued question is not QTI".to_string(),
                    ));
                }
            }
        {
            return Err(RunBackendError::Unavailable(
                "issued QTI grading contract item disagrees with issued snapshot".to_string(),
            ));
        }
        let correct = contract
            .payload()
            .map_err(map_store_error)?
            .server_correct_choice()
            .map_err(map_store_error)?;
        let outcome = grading::grade(
            submission.question(),
            submission.response,
            Some(&grading::AnswerKey::MultipleChoice {
                correct: BTreeSet::from([correct]),
            }),
        )
        .map_err(|error| RunBackendError::Invalid(error.to_string()))?;
        match outcome {
            grading::GradeOutcome::Graded(result) => {
                Ok(SubmissionDisposition::Grade(GradeReceipt::empty(result)))
            }
            grading::GradeOutcome::NeedsManualGrading | grading::GradeOutcome::Ungraded => {
                Err(RunBackendError::Invalid(
                    "issued QTI contract did not produce a deterministic grade".to_string(),
                ))
            }
        }
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
        || artifact.object.bucket != Bucket::PrivateContent
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
#[path = "qti_backend/tests/mod.rs"]
mod tests;
