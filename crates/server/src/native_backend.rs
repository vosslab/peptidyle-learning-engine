//! Trusted server composition for the first-party native adapter.
//!
//! This bridge resolves immutable catalog asset bindings under the authenticated
//! tenant context before handing an attempt to `adapter_native`. It never
//! accepts asset mappings, implementation versions, seeds, or answer material
//! from a browser.

use std::sync::Arc;

use async_trait::async_trait;
use grading::GradeOutcome;
use question_model::generation::Seed;
use question_model::{
    BackendCapabilities, DraftQuestionSource, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition, QuestionEnvelope, QuestionSource, StudentResponse,
};
use store::{AssetStore, StoreError, TenantContext};

use crate::catalog::{BackendRegistry, BackendRegistryError};
use crate::run::{
    GradeReceipt, IssuedAttemptMetadata, RunBackend, RunBackendError, RunSubmission,
    SubmissionDisposition,
};

/// Composition bridge from server-owned persistence to native question logic.
pub struct NativeBackend<S> {
    adapter: Arc<adapter_native::NativeAdapter>,
    assets: Arc<S>,
}

impl<S> NativeBackend<S> {
    /// Creates a bridge with one immutable native adapter registry and asset store.
    pub fn new(adapter: Arc<adapter_native::NativeAdapter>, assets: Arc<S>) -> Self {
        Self { adapter, assets }
    }

    /// Returns the shared adapter registry used by this server composition.
    pub fn adapter(&self) -> &Arc<adapter_native::NativeAdapter> {
        &self.adapter
    }
}

impl<S> BackendRegistry for NativeBackend<S>
where
    S: Send + Sync,
{
    fn capabilities(
        &self,
        source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        let source = match source {
            DraftQuestionSource::Native { family } => QuestionSource::Native {
                family: family.clone(),
            },
            _ => return Err(BackendRegistryError::Unsupported),
        };
        self.adapter
            .capabilities(&source)
            .map_err(map_capability_error)
    }
}

#[async_trait]
impl<S> RunBackend for NativeBackend<S>
where
    S: AssetStore + Send + Sync + 'static,
{
    async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<IssuedAttemptMetadata, RunBackendError> {
        validate_definition_reference(reference, question)?;
        let bindings = self.asset_bindings(context, reference).await?;
        let issued = self
            .adapter
            .issue(question, Seed::new(seed), &bindings)
            .map_err(map_native_error)?;
        Ok(IssuedAttemptMetadata {
            envelope: issued.envelope,
            parameter_hash: issued.parameter_hash,
            provenance: issued.provenance,
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
        let bindings = self.asset_bindings(context, reference).await?;
        self.adapter
            .reproduce(
                question,
                Seed::new(attempt.seed),
                &attempt.parameter_hash,
                &attempt.provenance,
                &bindings,
            )
            .map_err(map_native_error)
    }

    async fn grade(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &QuestionAttempt,
        response: &StudentResponse,
    ) -> Result<GradeOutcome, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let bindings = self.asset_bindings(context, reference).await?;
        self.adapter
            .grade(
                question,
                Seed::new(attempt.seed),
                &attempt.parameter_hash,
                &attempt.provenance,
                &bindings,
                response,
            )
            .map_err(map_native_error)
    }

    async fn submit(
        &self,
        submission: RunSubmission<'_>,
    ) -> Result<SubmissionDisposition, RunBackendError> {
        validate_attempt_reference(
            submission.reference,
            submission.question,
            submission.attempt,
        )?;
        let bindings = self
            .asset_bindings(submission.context, submission.reference)
            .await?;
        let (outcome, feedback) = self
            .adapter
            .grade_with_feedback(
                submission.question,
                Seed::new(submission.attempt.seed),
                &submission.attempt.parameter_hash,
                &submission.attempt.provenance,
                &bindings,
                submission.response,
            )
            .map_err(map_native_error)?;
        let grading::GradeOutcome::Graded(result) = outcome else {
            return Err(RunBackendError::Unsupported(
                "native question did not produce a server grade".to_string(),
            ));
        };
        Ok(SubmissionDisposition::Grade(GradeReceipt {
            result,
            feedback,
        }))
    }
}

impl<S> NativeBackend<S>
where
    S: AssetStore + Send + Sync + 'static,
{
    async fn asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<adapter_native::AssetObjectBinding>, RunBackendError> {
        self.assets
            .catalog_asset_bindings(context, reference)
            .await
            .map_err(map_store_error)
            .map(|bindings| {
                bindings
                    .into_iter()
                    .map(|binding| adapter_native::AssetObjectBinding {
                        asset: binding.asset,
                        object: binding.object,
                    })
                    .collect()
            })
    }
}

fn validate_definition_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
) -> Result<(), RunBackendError> {
    if question.version != reference.version || question.problem != reference.problem {
        return Err(RunBackendError::Invalid(
            "published question does not match immutable problem version reference".to_string(),
        ));
    }
    Ok(())
}

fn validate_attempt_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &QuestionAttempt,
) -> Result<(), RunBackendError> {
    validate_definition_reference(reference, question)?;
    if attempt.problem != reference.problem || attempt.question_version != reference.version {
        return Err(RunBackendError::Invalid(
            "attempt does not match immutable problem version reference".to_string(),
        ));
    }
    Ok(())
}

fn map_capability_error(error: adapter_native::NativeAdapterError) -> BackendRegistryError {
    match error {
        adapter_native::NativeAdapterError::UnsupportedSource
        | adapter_native::NativeAdapterError::UnknownFamily(_) => BackendRegistryError::Unsupported,
        other => BackendRegistryError::Unavailable(other.to_string()),
    }
}

fn map_native_error(error: adapter_native::NativeAdapterError) -> RunBackendError {
    match error {
        adapter_native::NativeAdapterError::UnsupportedSource
        | adapter_native::NativeAdapterError::UnknownFamily(_)
        | adapter_native::NativeAdapterError::UnknownGenerator { .. } => {
            RunBackendError::Unsupported(error.to_string())
        }
        other => RunBackendError::Invalid(other.to_string()),
    }
}

fn map_store_error(error: StoreError) -> RunBackendError {
    match error {
        StoreError::Unavailable(message) => RunBackendError::Unavailable(message),
        other => RunBackendError::Invalid(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use objects::{ObjectKey, ObjectRecord, Sha256Digest};
    use question_model::answer::SelectionCardinality;
    use question_model::capability::Capability;
    use question_model::envelope::{AssetRef, ContentBlock};
    use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
    use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        ActivityTimestamp, AssetId, BackendCapabilities, DraftQuestionDefinition,
        DraftQuestionSource, GradingDefinition, ProblemId, QuestionAttemptId, QuestionMetadata,
        QuestionSource, RunId, TenantId, UserId, VersionId, WorkspaceId,
    };
    use store::memory::MemoryStore;
    use store::{
        AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, CatalogStore, DraftRecord,
        PublishDraftCommand, Store,
    };
    use uuid::Uuid;

    use super::*;

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn choice(id: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: vec![ContentBlock::Text {
                markdown: id.to_string(),
            }],
        }
    }

    fn draft_question(workspace: WorkspaceId, image: AssetId) -> DraftQuestionDefinition {
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "residue".to_string(),
            ParameterSpec::Choice {
                options: vec!["alanine".to_string(), "glycine".to_string()],
            },
        );
        DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
            },
            prompt: vec![
                ContentBlock::Text {
                    markdown: "In a peptide containing {{residue}}, which linkage is planar?"
                        .to_string(),
                },
                ContentBlock::Image {
                    asset: AssetRef {
                        asset: image,
                        checksum: "bridge-fixture".to_string(),
                    },
                    description: "A peptide bond diagram.".to_string(),
                },
            ],
            response: ResponseDefinition::MultipleChoice {
                choices: vec![choice("ester"), choice("amide"), choice("ether")],
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Seeded {
                generator: GeneratorReference {
                    id: adapter_native::peptide_bond_geometry::GENERATOR_ID.to_string(),
                    version: adapter_native::peptide_bond_geometry::GENERATOR_VERSION.to_string(),
                },
                parameters,
            },
            grading: GradingDefinition::AllOrNothing { points: 2.0 },
            metadata: QuestionMetadata {
                title: "Peptide bond geometry".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        }
    }

    fn asset_record(
        reference: ProblemVersionRef,
        asset: AssetId,
        object: question_model::ObjectId,
    ) -> AssetDeliveryRecord {
        let key = ObjectKey::ProblemAsset {
            problem: reference.problem,
            version: reference.version,
            asset,
            object,
        };
        AssetDeliveryRecord {
            id: AssetDeliveryId::from_asset(asset),
            object: ObjectRecord {
                id: object,
                bucket: key.bucket(),
                key,
                sha256: Sha256Digest::compute(b"native bridge asset"),
                size_bytes: 19,
                media_type: "image/svg+xml".to_string(),
                category: objects::ObjectCategory::Asset,
                version: Some(reference.version),
                license: "CC BY 4.0".to_string(),
                provenance: "native bridge test".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1_000),
            },
            scope: AssetDeliveryScope::Catalog { asset, reference },
        }
    }

    #[tokio::test]
    async fn native_bridge_reproduces_only_with_exact_memory_catalog_assets() {
        let store = Arc::new(MemoryStore::default());
        let tenant = TenantId::from_uuid(uuid(1));
        let context = TenantContext::from_authenticated_session(tenant);
        let problem = ProblemId::from_uuid(uuid(2));
        let version = VersionId::from_uuid(uuid(3));
        let workspace = WorkspaceId::from_uuid(uuid(4));
        let asset = AssetId::from_uuid(uuid(5));
        let object = question_model::ObjectId::from_uuid(uuid(6));
        let publisher = UserId::from_uuid(uuid(7));
        let draft = DraftRecord {
            tenant,
            question: draft_question(workspace, asset),
            revises: None,
            derived_from: None,
        };
        let saved = store
            .upsert_draft(context, publisher, None, draft.clone())
            .await
            .expect("draft saves before publication");
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved.revision,
                    publication: ProblemVersionRef { problem, version },
                    published_source: QuestionSource::Native {
                        family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
                    },
                    source_artifact: None,
                    qti_promotion: None,
                    publisher,
                    scope: question_model::PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([
                        Capability::AlgorithmicGeneration,
                        Capability::ClientRendering,
                        Capability::ServerGrading,
                    ]),
                },
            )
            .await
            .expect("native question publishes");
        let reference = ProblemVersionRef { problem, version };
        store
            .register_asset_delivery(context, asset_record(reference, asset, object))
            .await
            .expect("exact version asset registers");

        let backend = NativeBackend::new(Arc::new(adapter_native::NativeAdapter::new()), store);
        let published = QuestionDefinition::from_draft(
            draft_question(workspace, asset),
            problem,
            version,
            QuestionSource::Native {
                family: adapter_native::peptide_bond_geometry::FAMILY_ID.to_string(),
            },
        );
        let issued = backend
            .issue(context, reference, &published, 37)
            .await
            .expect("native issue resolves only exact catalog assets");
        assert_eq!(issued.provenance.asset_objects, vec![object]);
        let attempt = QuestionAttempt {
            id: QuestionAttemptId::from_uuid(uuid(8)),
            tenant,
            run: RunId::from_uuid(uuid(9)),
            problem,
            question_version: version,
            assignment_position: 0,
            seed: 37,
            parameter_hash: issued.parameter_hash,
            response: None,
            result: None,
            timer: question_model::AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(1_000),
                deadline: None,
                submitted_at: None,
            },
            provenance: issued.provenance,
        };
        let envelope = backend
            .reproduce(context, reference, &published, &attempt)
            .await
            .expect("stored native attempt reproduces through memory asset resolver");
        let body = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(!body.contains("{{residue}}"));
        assert!(!body.contains("correct"));
        assert!(body.contains("alanine") || body.contains("glycine"));
        let outcome = backend
            .grade(
                context,
                reference,
                &published,
                &attempt,
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
            )
            .await
            .expect("native server grading remains behind the bridge");
        assert!(matches!(outcome, GradeOutcome::Graded(result) if result.correct));

        let wrong_reference = ProblemVersionRef {
            problem,
            version: VersionId::from_uuid(uuid(10)),
        };
        assert!(matches!(
            backend
                .reproduce(context, wrong_reference, &published, &attempt)
                .await,
            Err(RunBackendError::Invalid(_))
        ));

        let foreign_context =
            TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(11)));
        assert!(matches!(
            backend
                .reproduce(foreign_context, reference, &published, &attempt)
                .await,
            Err(RunBackendError::Invalid(_))
        ));

        let mut tampered = attempt.clone();
        tampered.provenance.asset_objects = vec![question_model::ObjectId::from_uuid(uuid(12))];
        assert!(matches!(
            backend
                .reproduce(context, reference, &published, &tampered)
                .await,
            Err(RunBackendError::Invalid(_))
        ));
    }
}
