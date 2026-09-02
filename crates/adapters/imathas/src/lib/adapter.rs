//! Immutable iMathAS source resolution, issue caching, and verified grading.

use std::collections::BTreeSet;

use objects::{ObjectStore, ObjectStoreError, PutObject, ResolvedQuestionSource};
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::generation::QuestionSeed;
use question_model::{
    QuestionAttemptReproductionDetails, QuestionBackendLocator, QuestionRendererVersion,
    QuestionRevision, QuestionRevisionReference, QuestionVariationPresentation,
    SourceObjectChecksum, SourceObjectReference, Timestamp,
};
use sha2::{Digest, Sha256};

use crate::cache::{
    CachedRender, backend_version, decode_cache, grader_version, parameter_hash, render_key,
    validate_cache,
};
use crate::imathas_question_backend;
use crate::{
    ADAPTER_ID, ADAPTER_VERSION, GRADING_ID, GRADING_VERSION, ImathasAdapterError,
    ImathasQuestionLocation, ImathasRenderRequest, ImathasResultRequest, PreparedSnapshot,
    QuestionBackend, SupportedImathasProfile, VerifiedImathasQuestionBackendResult, hex,
    verify_binding,
};

/// Exact immutable source loaded through trusted storage.
#[derive(Clone)]
pub struct ResolvedImathasQuestionSource {
    resolved: ResolvedQuestionSource,
    pub(crate) binding: question_model::ImathasQuestionBackendBinding,
}

impl std::fmt::Debug for ResolvedImathasQuestionSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResolvedImathasQuestionSource")
            .field("source", &"[SERVER-ONLY]")
            .finish_non_exhaustive()
    }
}

impl ResolvedImathasQuestionSource {
    /// Resolves an iMathAS Question Source from its exact immutable object.
    pub async fn resolve<S: ObjectStore>(
        store: &S,
        question: &QuestionRevision,
        source_object_reference: SourceObjectReference,
        source_object_checksum: SourceObjectChecksum,
    ) -> Result<Self, ImathasAdapterError> {
        let QuestionBackendLocator::Imathas { binding } = &question.backend_locator else {
            return Err(ImathasAdapterError::UnsupportedSource);
        };
        let question_revision = QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        };
        let resolved = ResolvedQuestionSource::resolve(
            store,
            question_revision,
            source_object_reference,
            source_object_checksum,
        )
        .await
        .map_err(ImathasAdapterError::QuestionSourceResolution)?;
        Ok(Self {
            resolved,
            binding: binding.clone(),
        })
    }

    pub fn artifact(&self) -> &SourceObjectReference {
        self.resolved.source_object_reference()
    }

    /// Exact Question Revision that owns the immutable iMathAS snapshot.
    pub fn question_revision(&self) -> &QuestionRevisionReference {
        self.resolved.question_revision()
    }

    /// SHA-256 evidence for the immutable source bytes.
    pub fn source_object_checksum(&self) -> &SourceObjectChecksum {
        self.resolved.source_object_checksum()
    }

    /// Immutable iMathAS snapshot bytes verified by the Object Store.
    pub fn bytes(&self) -> &[u8] {
        self.resolved.bytes()
    }

    /// Exact iMathAS backend binding pinned by this Question Revision.
    pub fn binding(&self) -> &question_model::ImathasQuestionBackendBinding {
        &self.binding
    }
}

/// Key-free issued iMathAS Question Backend response control.
#[derive(Debug, Clone, PartialEq)]
pub struct ImathasIssuedAttempt {
    pub envelope: QuestionVariationPresentation,
    pub parameter_hash: String,
    pub reproduction_details: QuestionAttemptReproductionDetails,
    pub cache_hit: bool,
}

/// iMathAS adapter with immutable source and deterministic, browser-safe cache.
pub struct ImathasAdapter<S, P> {
    store: S,
    question_backend: P,
    profiles: BTreeSet<question_model::ImathasProfile>,
}

impl<S: ObjectStore, P: QuestionBackend> ImathasAdapter<S, P> {
    pub fn new(
        store: S,
        question_backend: P,
        profiles: impl IntoIterator<Item = SupportedImathasProfile>,
    ) -> Self {
        Self {
            store,
            question_backend,
            profiles: profiles
                .into_iter()
                .map(|profile| profile.profile)
                .collect(),
        }
    }

    /// Snapshot a draft before publication. This neither knows nor mints a published identity.
    pub async fn prepare_snapshot(
        &self,
        draft: &question_model::DraftQuestionBackendLocator,
    ) -> Result<PreparedSnapshot, ImathasAdapterError> {
        let locator = ImathasQuestionLocation::from_draft_backend_locator(draft)?;
        let (bytes, profile) = self
            .question_backend
            .snapshot(&locator)
            .await
            .map_err(ImathasAdapterError::QuestionBackend)?;
        if bytes.is_empty() || !self.profiles.contains(profile.profile()) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        Ok(PreparedSnapshot {
            sha256: hex(Sha256::digest(&bytes).as_slice()),
            bytes,
            profile,
        })
    }

    /// Derives only capabilities actually delivered by the pinned profile.
    pub fn capabilities(
        &self,
        source: &QuestionBackendLocator,
        profile: &SupportedImathasProfile,
    ) -> Result<QuestionBackendCapabilities, ImathasAdapterError> {
        let QuestionBackendLocator::Imathas { binding } = source else {
            return Err(ImathasAdapterError::UnsupportedSource);
        };
        if binding.profile() != profile.profile() || !self.profiles.contains(profile.profile()) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let mut values = vec![Capability::QuestionAttemptTimeLimit];
        if profile.deterministic_seeded_render {
            values.push(Capability::AlgorithmicGeneration);
        }
        if profile.verified_server_grading {
            values.push(Capability::ServerGrading);
        }
        if profile.partial_credit {
            values.push(Capability::PartialCredit);
        }
        Ok(QuestionBackendCapabilities::from_iter(values))
    }

    /// Issues a iMathAS Question Backend marker and safe iMathAS prompt. Repeated exact
    /// version/seed requests are served from immutable cache storage.
    pub async fn issue(
        &self,
        question: &QuestionRevision,
        seed: QuestionSeed,
        source: &ResolvedImathasQuestionSource,
        created_at: Timestamp,
    ) -> Result<ImathasIssuedAttempt, ImathasAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(ImathasAdapterError::InvalidTitle)?;
        verify_binding(question, source)?;
        if !self.profiles.contains(source.binding.profile()) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let question_revision = QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        };
        let key = render_key(&question_revision, seed);
        match self.store.get(&key).await {
            Ok(stored) => {
                let cached = decode_cache(&stored.bytes)?;
                validate_cache(&cached, question, seed, source)?;
                return self.issued(cached, source, true);
            }
            Err(ObjectStoreError::NotFound) => {}
            Err(error) => return Err(ImathasAdapterError::ObjectStore(error)),
        }
        let safe = self
            .question_backend
            .render(ImathasRenderRequest {
                snapshot: source.bytes(),
                profile: source.binding.profile().as_str(),
                question_revision: question_revision.clone(),
                seed,
            })
            .await
            .map_err(ImathasAdapterError::QuestionBackend)?;
        if question_model::validate_question_title(&safe.title).is_err() {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendRender);
        }
        let record = CachedRender {
            schema: 1,
            source: source.artifact().clone(),
            source_object_checksum: source.source_object_checksum().clone(),
            binding: source.binding.clone(),
            envelope: QuestionVariationPresentation {
                variation: question_model::QuestionVariation::static_variation(
                    question_revision.clone(),
                    seed,
                ),
                title: safe.title,
                prompt: safe.prompt,
                response: question_model::QuestionResponseFormat::ImathasQuestionBackend {},
            },
        };
        validate_cache(&record, question, seed, source)?;
        let bytes = serde_json::to_vec(&record).map_err(|_| ImathasAdapterError::InvalidCache)?;
        match self
            .store
            .put(PutObject {
                address: key.clone(),
                bytes,
                media_type: "application/vnd.peptidyle.imathas-render+json".into(),
                created_at,
            })
            .await
        {
            Ok(_) => self.issued(record, source, false),
            Err(ObjectStoreError::AlreadyExists) => {
                let stored = self
                    .store
                    .get(&key)
                    .await
                    .map_err(ImathasAdapterError::ObjectStore)?;
                let cached = decode_cache(&stored.bytes)?;
                validate_cache(&cached, question, seed, source)?;
                self.issued(cached, source, true)
            }
            Err(error) => Err(ImathasAdapterError::ObjectStore(error)),
        }
    }

    fn issued(
        &self,
        cached: CachedRender,
        source: &ResolvedImathasQuestionSource,
        cache_hit: bool,
    ) -> Result<ImathasIssuedAttempt, ImathasAdapterError> {
        let hash = hex(Sha256::digest(
            serde_json::to_vec(&cached).map_err(|_| ImathasAdapterError::InvalidCache)?,
        )
        .as_slice());
        Ok(ImathasIssuedAttempt {
            parameter_hash: parameter_hash(cached.envelope.variation.seed),
            reproduction_details: QuestionAttemptReproductionDetails {
                backend: backend_version(ADAPTER_ID, ADAPTER_VERSION),
                renderer_version: Some(QuestionRendererVersion {
                    name: "imathas-profile".to_string(),
                    version: source.binding.profile().as_str().to_owned(),
                }),
                generator: None,
                source_object_reference: Some(source.artifact().clone()),
                source_object_checksum: Some(source.source_object_checksum().clone()),
                asset_objects: Vec::new(),
                grader: grader_version(GRADING_ID, GRADING_VERSION),
                rendered_question_sha256: hash,
            },
            envelope: cached.envelope,
            cache_hit,
        })
    }

    /// Accepts only an iMathAS-verifier result matching every server-held binding.
    pub async fn verify_imathas_question_backend_result(
        &self,
        question: &QuestionRevision,
        source: &ResolvedImathasQuestionSource,
        grading_context: &learning_data_access::ImathasQuestionBackendGradingContext,
        launch_session_authentication: &learning_data_access::ImathasQuestionBackendSessionAuthentication,
    ) -> Result<VerifiedImathasQuestionBackendResult, ImathasAdapterError> {
        verify_binding(question, source)?;
        let question_revision = QuestionRevisionReference {
            question_id: question.question_id.clone(),
            revision_number: question.revision_number,
        };
        if grading_context.question_revision() != &question_revision {
            return Err(ImathasAdapterError::VerificationRefused);
        }
        let verdict = self
            .question_backend
            .verify_result(ImathasResultRequest {
                snapshot: source.bytes(),
                profile: source.binding.profile().as_str(),
                grading_context,
                launch_session_authentication,
            })
            .await
            .map_err(ImathasAdapterError::QuestionBackend)?;
        if verdict.grading_context != *grading_context
            || verdict.launch_session_authentication != *launch_session_authentication
        {
            return Err(ImathasAdapterError::VerificationRefused);
        }
        Ok(verdict)
    }
}

// A render-only iMathAS Question Backend cannot acquire iMathAS Question Backend
// Launch or Result Verification ability.
impl<S, T> ImathasAdapter<S, imathas_question_backend::ImathasQuestionBackend<T>>
where
    S: ObjectStore,
    T: imathas_question_backend::ImathasQuestionBackendTransport,
{
    pub fn imathas_question_backend_deployment_reference(&self) -> &str {
        self.question_backend.deployment_reference()
    }
    pub fn imathas_question_backend_launch_lifetime_millis(&self) -> u32 {
        self.question_backend.launch_lifetime_millis()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn prepare_imathas_question_backend_launch(
        &self,
        question: &QuestionRevision,
        source: &ResolvedImathasQuestionSource,
        validation: &learning_data_access::ImathasQuestionBackendLaunchPreparationValidation,
        now: Timestamp,
    ) -> Result<imathas_question_backend::ImathasLaunchPreparation, ImathasAdapterError> {
        self.question_backend
            .prepare_imathas_question_backend_launch(question, source, validation, now)
            .await
    }

    pub async fn retrieve_verified_imathas_question_backend_result(
        &self,
        validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
        imathas_launch_state: &imathas_question_backend::ImathasLaunchState,
        now: Timestamp,
    ) -> Result<VerifiedImathasQuestionBackendResult, ImathasAdapterError> {
        self.question_backend
            .retrieve_and_verify(validation, imathas_launch_state, now)
            .await
    }

    pub async fn proxy_imathas_question_backend_activity(
        &self,
        validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
        imathas_launch_state: &imathas_question_backend::ImathasLaunchState,
        method: imathas_question_backend::ProxyMethod,
        body: &[u8],
        now: Timestamp,
    ) -> Result<imathas_question_backend::ProxyResponse, ImathasAdapterError> {
        self.question_backend
            .proxy_activity(validation, imathas_launch_state, method, body, now)
            .await
    }
}
