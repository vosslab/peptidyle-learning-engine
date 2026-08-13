//! Trusted persisted-source bridge for the WeBWorK adapter.
//!
//! The browser supplies neither a PG path nor an object identifier. Issuance
//! and explicit reproduction resolve the catalog-owned source binding before
//! reaching the adapter/renderer. First grade instead reads its
//! attempt-bound source artifact and protected issuance contracts, so it does
//! not depend on a later catalog lookup or render.

use std::sync::Arc;

use adapter_webwork::{WebworkAdapter, WebworkAdapterError, WebworkSource};
use learning_data_access::{
    CatalogSourceStore, IssuedWebworkGradingContract, PublishedSourceArtifact, Store, StoreError,
    TenantContext,
};
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectStore, ObjectStoreError};
use question_model::generation::Seed;
use question_model::{
    ActivityTimestamp, ProblemVersionRef, QuestionBackend, QuestionDefinition, SourceArtifact,
    StudentResponse,
};

use crate::run::RunBackendError;

mod replay_mapping;

pub(crate) use replay_mapping::{persist_replay_mapping, restore_replay_mapping};

/// Server-owned WeBWorK source resolver and adapter invocation boundary.
pub struct WebworkBackend<S, O, R> {
    sources: Arc<S>,
    objects: Arc<O>,
    adapter: Arc<WebworkAdapter<O, R>>,
}

impl<S, O, R> WebworkBackend<S, O, R> {
    /// Creates a bridge around independently configured source, object, and
    /// renderer dependencies.  The production composition root owns their
    /// construction; this module never reads environment values.
    pub fn new(sources: Arc<S>, objects: Arc<O>, adapter: Arc<WebworkAdapter<O, R>>) -> Self {
        Self {
            sources,
            objects,
            adapter,
        }
    }
}

impl<S, O, R> WebworkBackend<S, O, R>
where
    S: CatalogSourceStore + Store + Send + Sync + 'static,
    O: ObjectStore + Send + Sync + 'static,
    R: adapter_webwork::renderer_contract::WebworkRenderer + Send + Sync + 'static,
{
    /// Issues a key-free WebWork instance from the only permitted source path.
    pub async fn issue(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        seed: u64,
    ) -> Result<adapter_webwork::WebworkIssuedAttempt, RunBackendError> {
        validate_reference(reference, question)?;
        let (source, created_at) = self.resolve_source(context, reference, question).await?;
        self.adapter
            .issue(question, Seed::new(seed), &source, created_at)
            .await
            .map_err(map_adapter_error)
    }

    /// Reproduces the exact key-free cached output without asking the private
    /// renderer to rebuild issuance-only controls.
    pub async fn reproduce(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
        attempt: &question_model::QuestionAttempt,
    ) -> Result<adapter_webwork::WebworkIssuedAttempt, RunBackendError> {
        validate_attempt_reference(reference, question, attempt)?;
        let (source, _) = self.resolve_source(context, reference, question).await?;
        let issued = self
            .adapter
            .reproduce(question, Seed::new(attempt.seed), &source)
            .await
            .map_err(map_adapter_error)?;
        validate_issued_attempt(attempt, &issued)?;
        Ok(issued)
    }

    /// Grades from the complete attempt-bound private replay state.
    ///
    /// WeBWorK replay state and its presentation binding are required for
    /// every issued WeBWorK attempt. Their absence is an unavailable immutable
    /// grading authority, never permission to reissue from current source.
    pub async fn grade(
        &self,
        context: TenantContext,
        actor: question_model::UserId,
        reference: ProblemVersionRef,
        attempt: &question_model::QuestionAttempt,
        grading_contract: &IssuedWebworkGradingContract,
        response: &StudentResponse,
    ) -> Result<grading::GradeOutcome, RunBackendError> {
        let question = grading_contract.question();
        validate_attempt_reference(reference, question, attempt).map_err(|_| {
            RunBackendError::Unavailable("WeBWorK issued grading contract is unavailable".into())
        })?;
        validate_active_renderer(attempt, self.adapter.renderer_identity())?;
        let binding = self
            .sources
            .get_attempt_presentation_binding(context, actor, attempt.id)
            .await
            .map_err(map_store_error)?;
        let state = self
            .sources
            .get_webwork_grade_replay_state(context, actor, attempt.id)
            .await
            .map_err(map_store_error)?;
        let (state, binding) = match (state, binding) {
            (Some(state), Some(binding)) => {
                validate_replay_state(attempt, binding, &state)?;
                (state, binding)
            }
            (None, _) => {
                return Err(RunBackendError::Unavailable(
                    "WeBWorK immutable grade replay state is missing".into(),
                ));
            }
            (Some(_), None) => {
                return Err(RunBackendError::Unavailable(
                    "WeBWorK immutable grade replay binding is missing".into(),
                ));
            }
        };
        let snapshot = self
            .sources
            .get_attempt_presentation_snapshot(context, actor, attempt.id)
            .await
            .map_err(map_store_error)?;
        let snapshot = snapshot.ok_or_else(|| {
            RunBackendError::Unavailable("WeBWorK issued presentation snapshot is missing".into())
        })?;
        let grading_envelope = self
            .sources
            .get_attempt_grading_envelope(context, actor, attempt.id)
            .await
            .map_err(map_store_error)?;
        let grading_envelope = grading_envelope.ok_or_else(|| {
            RunBackendError::Unavailable("WeBWorK issued grading envelope is missing".into())
        })?;
        let presentation = question_model::presentation::reproduce_presentation_v1(
            &grading_envelope,
            &snapshot.asset_bindings,
            binding,
        )
        .map_err(|_| {
            RunBackendError::Unavailable(
                "WeBWorK issued presentation contract is unavailable".into(),
            )
        })?;
        if presentation.envelope != snapshot.envelope {
            return Err(RunBackendError::Unavailable(
                "WeBWorK issued presentation contract is unavailable".into(),
            ));
        }
        let replay = restore_replay_mapping(state.mapping, &presentation)?;
        // Grade from the immutable artifact retained in the attempt provenance,
        // not a current catalog lookup or a renderer reproduction.
        let source = WebworkSource::resolve(
            self.objects.as_ref(),
            state.problem,
            state.version,
            state.source_artifact,
        )
        .await
        .map_err(|_| {
            RunBackendError::Unavailable("WeBWorK issued source artifact is unavailable".into())
        })?;
        self.adapter
            .grade(
                question,
                Seed::new(attempt.seed),
                &source,
                response,
                &replay,
            )
            .await
            .map_err(map_adapter_error)
    }

    async fn resolve_source(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
        question: &QuestionDefinition,
    ) -> Result<(WebworkSource, ActivityTimestamp), RunBackendError> {
        let artifact = self
            .sources
            .catalog_source_artifact(context, reference)
            .await
            .map_err(map_store_error)?
            .ok_or_else(|| {
                RunBackendError::Invalid("published WeBWorK source is unavailable".to_string())
            })?;
        let source_artifact = validate_source_artifact(reference, question, &artifact)?;
        let created_at = artifact.object.created_at;
        let source = WebworkSource::resolve(
            self.objects.as_ref(),
            reference.problem,
            reference.version,
            source_artifact,
        )
        .await
        .map_err(map_adapter_error)?;
        Ok((source, created_at))
    }
}

fn validate_active_renderer(
    attempt: &question_model::QuestionAttempt,
    active: &adapter_webwork::renderer_contract::RendererIdentity,
) -> Result<(), RunBackendError> {
    let Some(issued) = attempt.provenance.renderer.as_ref() else {
        return Err(RunBackendError::Unavailable(
            "WeBWorK attempt omitted its renderer identity".into(),
        ));
    };
    if issued.id != active.id || issued.version != active.version {
        return Err(RunBackendError::Unavailable(
            "configured WeBWorK renderer does not match the issued attempt".into(),
        ));
    }
    Ok(())
}

fn validate_replay_state(
    attempt: &question_model::QuestionAttempt,
    binding: question_model::PresentationBindingV1,
    state: &learning_data_access::WebworkGradeReplayStateV1,
) -> Result<(), RunBackendError> {
    if state.problem != attempt.problem
        || state.version != attempt.question_version
        || state.seed != attempt.seed
        || attempt.provenance.source_artifact.as_ref() != Some(&state.source_artifact)
        || attempt.provenance.renderer.as_ref() != Some(&state.renderer)
        || state.presentation_digest != binding.digest()
    {
        return Err(RunBackendError::Unavailable(
            "WeBWorK replay does not match the issued attempt".into(),
        ));
    }
    Ok(())
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
    if !matches!(
        question.source,
        question_model::QuestionSource::Webwork { .. }
    ) {
        return Err(RunBackendError::Unsupported(
            "published question is not backed by WeBWorK".to_string(),
        ));
    }
    Ok(())
}

fn validate_attempt_reference(
    reference: ProblemVersionRef,
    question: &QuestionDefinition,
    attempt: &question_model::QuestionAttempt,
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
    question: &QuestionDefinition,
    artifact: &PublishedSourceArtifact,
) -> Result<SourceArtifact, RunBackendError> {
    validate_reference(reference, question)?;
    let object = &artifact.object;
    let expected_key = ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object: object.id,
    };
    if artifact.reference != reference
        || artifact.backend != QuestionBackend::Webwork
        || object.key != expected_key
        || object.bucket != Bucket::PrivateContent
        || object.category != ObjectCategory::Source
        || object.version != Some(reference.version)
    {
        return Err(RunBackendError::Invalid(
            "published WeBWorK source binding is invalid".to_string(),
        ));
    }
    Ok(SourceArtifact {
        object: object.id,
        sha256: object.sha256.to_string(),
    })
}

/// Reproduction and grading compare the complete persisted record, not only
/// an adapter id.  This rejects a changed source, renderer, parameters, or
/// rendered question instead of silently handing out a fresh prompt.
pub(crate) fn validate_issued_attempt(
    attempt: &question_model::QuestionAttempt,
    issued: &adapter_webwork::WebworkIssuedAttempt,
) -> Result<(), RunBackendError> {
    if attempt.parameter_hash != issued.parameter_hash || attempt.provenance != issued.provenance {
        return Err(RunBackendError::Invalid(
            "persisted WeBWorK attempt provenance does not reproduce".to_string(),
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

fn map_adapter_error(error: WebworkAdapterError) -> RunBackendError {
    match error {
        WebworkAdapterError::Renderer(
            adapter_webwork::renderer_contract::RendererFailure::Unavailable
            | adapter_webwork::renderer_contract::RendererFailure::TimedOut
            | adapter_webwork::renderer_contract::RendererFailure::ResourceExhausted,
        )
        | WebworkAdapterError::ObjectStore(ObjectStoreError::Unavailable(_)) => {
            RunBackendError::Unavailable("question backend is temporarily unavailable".to_string())
        }
        WebworkAdapterError::UnsupportedSource => RunBackendError::Unsupported(error.to_string()),
        other => RunBackendError::Invalid(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use learning_data_access::{CatalogStore, DraftRecord, PublishDraftCommand, Store};
    use objects::PutObject;
    use objects::memory::MemoryObjectStore;
    use question_model::answer::SelectionCardinality;
    use question_model::envelope::ContentBlock;
    use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
    use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
    use question_model::taxonomy::License;
    use question_model::{
        AttemptTimerRecord, BackendCapabilities, DraftQuestionDefinition, DraftQuestionSource,
        GradingDefinition, ProblemId, QuestionAttempt, QuestionAttemptId, QuestionMetadata,
        QuestionSource, RunId, TenantId, UserId, VersionId, WorkspaceId,
    };
    use uuid::Uuid;

    use super::*;
    use crate::composite_backend::CompositeBackend;
    use crate::native_backend::NativeBackend;
    use crate::run::RunBackend;

    #[path = "persisted_replay.rs"]
    mod persisted_replay;

    const OPL: &str = concat!(
        "## Recorded OPL-style example: a small multiple-choice PG question.\n",
        "DOCUMENT();\n",
        "loadMacros(\"PGstandard.pl\", \"PGchoicemacros.pl\");\n",
        "BEGIN_TEXT\n",
        "Which molecule is water?\n",
        "END_TEXT\n",
        "$showPartialCorrectAnswers = 0;\n",
        "ANS(str_cmp(\"H2O\"));\n",
        "ENDDOCUMENT();\n",
    );

    #[derive(Clone)]
    struct RecordedRenderer {
        renders: Arc<AtomicUsize>,
        grades: Arc<AtomicUsize>,
        unavailable: Arc<AtomicBool>,
        identity: adapter_webwork::renderer_contract::RendererIdentity,
    }

    #[async_trait]
    impl adapter_webwork::renderer_contract::WebworkRenderer for RecordedRenderer {
        fn identity(&self) -> &adapter_webwork::renderer_contract::RendererIdentity {
            &self.identity
        }

        async fn render(
            &self,
            request: adapter_webwork::renderer_contract::RenderRequest<'_>,
        ) -> Result<
            adapter_webwork::renderer_contract::RenderedWebworkQuestion,
            adapter_webwork::renderer_contract::RendererFailure,
        > {
            self.renders.fetch_add(1, Ordering::SeqCst);
            if self.unavailable.load(Ordering::SeqCst) {
                return Err(adapter_webwork::renderer_contract::RendererFailure::Unavailable);
            }
            if request.pg_source != OPL.as_bytes() || request.pg_path != "Library/OPL/select-one.pg"
            {
                return Err(
                    adapter_webwork::renderer_contract::RendererFailure::InvalidOutput(
                        "unexpected recorded source".to_string(),
                    ),
                );
            }
            Ok(
                adapter_webwork::renderer_contract::RenderedWebworkQuestion {
                    envelope: question_envelope(request.seed),
                    html: "<p>Which molecule is water?</p>".to_string(),
                    renderer: self.identity.clone(),
                    replay: Some(
                        adapter_webwork::renderer_contract::WebworkReplayMappingV1::SingleChoice {
                            controls: BTreeMap::from([
                                (
                                    ChoiceId::new("salt"),
                                    adapter_webwork::renderer_contract::UpstreamControlV1 {
                                        field: "AnSwEr0001".into(),
                                        value: "0".into(),
                                    },
                                ),
                                (
                                    ChoiceId::new("water"),
                                    adapter_webwork::renderer_contract::UpstreamControlV1 {
                                        field: "AnSwEr0001".into(),
                                        value: "1".into(),
                                    },
                                ),
                            ]),
                        },
                    ),
                },
            )
        }

        async fn grade(
            &self,
            request: adapter_webwork::renderer_contract::GradeRequest<'_>,
        ) -> Result<grading::GradeOutcome, adapter_webwork::renderer_contract::RendererFailure>
        {
            self.grades.fetch_add(1, Ordering::SeqCst);
            if request.pg_source != OPL.as_bytes() {
                return Err(
                    adapter_webwork::renderer_contract::RendererFailure::InvalidOutput(
                        "unexpected recorded source".to_string(),
                    ),
                );
            }
            Ok(grading::GradeOutcome::Graded(
                question_model::AttemptResult {
                    correct: matches!(request.response, StudentResponse::MultipleChoice { selected } if selected == &vec![ChoiceId::new("water")]),
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
            ))
        }
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn reference() -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(id(10)),
            version: VersionId::from_uuid(id(11)),
        }
    }

    fn draft() -> DraftQuestionDefinition {
        DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id(12)),
            source: DraftQuestionSource::Webwork {
                pg_path: "Library/OPL/select-one.pg".to_string(),
            },
            prompt: Vec::new(),
            response: ResponseDefinition::MultipleChoice {
                choices: vec![
                    ChoiceOption {
                        id: ChoiceId::new("water"),
                        body: vec![ContentBlock::Text {
                            markdown: "H2O".to_string(),
                        }],
                    },
                    ChoiceOption {
                        id: ChoiceId::new("salt"),
                        body: vec![ContentBlock::Text {
                            markdown: "NaCl".to_string(),
                        }],
                    },
                ],
                selection: SelectionCardinality::ExactlyOne,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: Some(2),
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: question_model::generation::RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Recorded OPL".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    fn question_envelope(seed: u64) -> question_model::QuestionEnvelope {
        question_model::QuestionEnvelope {
            version: reference().version,
            seed: Seed::new(seed),
            title: "untrusted title".to_string(),
            prompt: vec![ContentBlock::Text {
                markdown: "Which molecule is water?".to_string(),
            }],
            response: ResponseDefinition::MultipleChoice {
                choices: vec![
                    ChoiceOption {
                        id: ChoiceId::new("water"),
                        body: vec![ContentBlock::Text {
                            markdown: "H2O".to_string(),
                        }],
                    },
                    ChoiceOption {
                        id: ChoiceId::new("salt"),
                        body: vec![ContentBlock::Text {
                            markdown: "NaCl".to_string(),
                        }],
                    },
                ],
                selection: SelectionCardinality::ExactlyOne,
            },
        }
    }

    async fn fixture() -> (
        WebworkBackend<
            learning_data_access::in_memory::MemoryStore,
            MemoryObjectStore,
            RecordedRenderer,
        >,
        TenantContext,
        QuestionDefinition,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<AtomicBool>,
    ) {
        let source_store = Arc::new(learning_data_access::in_memory::MemoryStore::default());
        let objects = Arc::new(MemoryObjectStore::default());
        let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(13)));
        let reference = reference();
        let object = question_model::ObjectId::from_uuid(id(14));
        let publisher = UserId::from_uuid(id(15));
        let record = objects
            .put(PutObject {
                key: ObjectKey::ProblemSource {
                    problem: reference.problem,
                    version: reference.version,
                    object,
                },
                bytes: OPL.as_bytes().to_vec(),
                media_type: "text/x-wework-pg".to_string(),
                license: "CC-BY-SA-4.0".to_string(),
                provenance: "recorded OPL".to_string(),
                created_at: ActivityTimestamp::from_unix_millis(1),
            })
            .await
            .expect("source object stores");
        let draft = DraftRecord {
            tenant: context.tenant_id(),
            question: draft(),
            revises: None,
            derived_from: None,
        };
        let saved = source_store
            .upsert_draft(context, publisher, None, draft.clone())
            .await
            .expect("draft stores");
        source_store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: saved.revision,
                    publication: reference,
                    published_source: QuestionSource::Webwork {
                        pg_path: "Library/OPL/select-one.pg".to_string(),
                    },
                    source_artifact: Some(PublishedSourceArtifact {
                        reference,
                        backend: QuestionBackend::Webwork,
                        object: record,
                    }),
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: question_model::PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([
                        question_model::Capability::AlgorithmicGeneration,
                        question_model::Capability::ServerGrading,
                    ]),
                },
            )
            .await
            .expect("source-backed WebWork publishes");
        let renders = Arc::new(AtomicUsize::new(0));
        let grades = Arc::new(AtomicUsize::new(0));
        let unavailable = Arc::new(AtomicBool::new(false));
        let renderer = RecordedRenderer {
            renders: Arc::clone(&renders),
            grades: Arc::clone(&grades),
            unavailable: Arc::clone(&unavailable),
            identity: adapter_webwork::renderer_contract::RendererIdentity {
                id: "recorded-opl".to_string(),
                version: "1".to_string(),
            },
        };
        let adapter = Arc::new(WebworkAdapter::new(objects.as_ref().clone(), renderer));
        let question = QuestionDefinition::from_draft(
            draft.question,
            reference.problem,
            reference.version,
            QuestionSource::Webwork {
                pg_path: "Library/OPL/select-one.pg".to_string(),
            },
        );
        (
            WebworkBackend::new(source_store, objects, adapter),
            context,
            question,
            renders,
            grades,
            unavailable,
        )
    }

    fn attempt(issued: &adapter_webwork::WebworkIssuedAttempt) -> QuestionAttempt {
        let reference = reference();
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(id(16)),
            tenant: TenantId::from_uuid(id(13)),
            run: RunId::from_uuid(id(17)),
            problem: reference.problem,
            question_version: reference.version,
            assignment_position: 0,
            seed: issued.envelope.seed.value(),
            parameter_hash: issued.parameter_hash.clone(),
            response: None,
            status: question_model::AttemptStatus::InProgress,
            result: None,
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(1),
                deadline: None,
                submitted_at: None,
            },
            provenance: issued.provenance.clone(),
            issued_capability: question_model::IssuedAttemptCapabilityV1::WebworkPresentation,
        }
    }

    #[tokio::test]
    async fn issue_and_attempt_reproduction_use_distinct_cache_boundaries() {
        let (backend, context, question, renders, grades, _unavailable) = fixture().await;
        let issued = backend
            .issue(context, reference(), &question, 99)
            .await
            .expect("issues OPL fixture");
        assert_eq!(renders.load(Ordering::SeqCst), 1);
        assert!(
            !serde_json::to_string(&issued.envelope)
                .expect("envelope serializes")
                .contains("correct")
        );
        let stored = attempt(&issued);
        let composite = CompositeBackend::new(
            NativeBackend::new(
                Arc::new(adapter_native::NativeAdapter::new()),
                Arc::new(learning_data_access::in_memory::MemoryStore::default()),
            ),
            backend,
        );
        let replay = composite
            .reproduce(context, reference(), &question, &stored)
            .await
            .expect("cache replays");
        assert_eq!(issued.envelope, replay);
        assert_eq!(renders.load(Ordering::SeqCst), 1, "replay is a cache hit");
        assert_eq!(grades.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn prefetch_issue_is_exact_cache_backed_and_renderer_outage_is_question_local() {
        let (backend, context, question, renders, grades, unavailable) = fixture().await;

        // `RunBackend::issue` is the prefetch boundary: it returns the exact
        // envelope that the reservation will expose plus the hash/provenance
        // later recorded on the attempt, without creating an attempt itself.
        let first = backend
            .issue(context, reference(), &question, 101)
            .await
            .expect("prefetch issues a safe envelope");
        // The adapter's provenance hash covers its complete safe cached render
        // (envelope plus sanitized renderer markup), so it must remain opaque
        // to this backend boundary but be canonical and stable on replay.
        assert_eq!(first.provenance.rendered_question_sha256.len(), 64);
        assert!(
            first
                .provenance
                .rendered_question_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        );
        assert_eq!(first.envelope.seed, Seed::new(101));
        assert_eq!(first.envelope.version, reference().version);
        assert_eq!(renders.load(Ordering::SeqCst), 1);
        let wire = serde_json::to_string(&first.envelope).expect("safe envelope serializes");
        for forbidden in ["Library/OPL", "answer", "pg_source", "correct"] {
            assert!(
                !wire.contains(forbidden),
                "prefetch envelope leaks {forbidden}"
            );
        }

        let replay = backend
            .issue(context, reference(), &question, 101)
            .await
            .expect("same reservation reproduces exactly");
        assert_eq!(first.envelope, replay.envelope);
        assert_eq!(first.parameter_hash, replay.parameter_hash);
        assert_eq!(first.provenance, replay.provenance);
        assert_eq!(
            renders.load(Ordering::SeqCst),
            2,
            "a new issuance rebuilds private replay even on a safe-cache hit"
        );

        unavailable.store(true, Ordering::SeqCst);
        assert!(matches!(
            backend.issue(context, reference(), &question, 102).await,
            Err(RunBackendError::Unavailable(_))
        ));
        assert_eq!(grades.load(Ordering::SeqCst), 0, "prefetch never grades");
        assert_eq!(
            renders.load(Ordering::SeqCst),
            3,
            "one failed renderer call"
        );

        // An already issued attempt still reproduces from its safe cache while
        // the renderer is unavailable. A new reservation cannot, because it
        // must capture private replay state for later one-call grading.
        let stored = attempt(&first);
        let cached_during_outage = backend
            .reproduce(context, reference(), &question, &stored)
            .await
            .expect("issued attempt reproduction survives renderer outage");
        assert_eq!(cached_during_outage.envelope, first.envelope);
        assert_eq!(renders.load(Ordering::SeqCst), 3);
        assert!(matches!(
            backend.issue(context, reference(), &question, 101).await,
            Err(RunBackendError::Unavailable(_))
        ));
        assert_eq!(renders.load(Ordering::SeqCst), 4);
        unavailable.store(false, Ordering::SeqCst);
        let recovered = backend
            .issue(context, reference(), &question, 102)
            .await
            .expect("later retry recovers without grading");
        assert_eq!(recovered.envelope.seed, Seed::new(102));
        assert_eq!(grades.load(Ordering::SeqCst), 0);
        assert_eq!(renders.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn tampered_provenance_and_foreign_tenant_refuse_before_grading() {
        let (backend, context, question, _renders, grades, _unavailable) = fixture().await;
        let issued = backend
            .issue(context, reference(), &question, 99)
            .await
            .expect("issues");
        let stored = attempt(&issued);
        let grading_contract = IssuedWebworkGradingContract::new(question.clone())
            .expect("fixture WebWork definition is valid");
        for tampered in [
            {
                let mut value = stored.clone();
                value.parameter_hash = "tampered".to_string();
                value
            },
            {
                let mut value = stored.clone();
                value
                    .provenance
                    .source_artifact
                    .as_mut()
                    .expect("WeBWorK provenance has a source")
                    .object = question_model::ObjectId::from_uuid(id(19));
                value
            },
            {
                let mut value = stored.clone();
                value
                    .provenance
                    .renderer
                    .as_mut()
                    .expect("WeBWorK provenance has a renderer")
                    .version = "tampered".to_string();
                value
            },
            {
                let mut value = stored.clone();
                value.provenance.rendered_question_sha256 = "tampered".to_string();
                value
            },
        ] {
            assert!(
                backend
                    .grade(
                        context,
                        UserId::from_uuid(id(15)),
                        reference(),
                        &tampered,
                        &grading_contract,
                        &StudentResponse::MultipleChoice {
                            selected: vec![ChoiceId::new("water")]
                        }
                    )
                    .await
                    .is_err(),
                "a synthetic, unpersisted tampered attempt must never reach grading"
            );
        }
        assert_eq!(grades.load(Ordering::SeqCst), 0);
        let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(id(18)));
        assert!(matches!(
            backend.issue(foreign, reference(), &question, 99).await,
            Err(RunBackendError::Invalid(_))
        ));
    }
}
