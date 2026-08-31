//! Immutable iMathAS source resolution, issue caching, and verified grading.

use std::collections::BTreeSet;

use objects::{ObjectStore, ObjectStoreError, PutObject};
use question_model::capability::{Capability, QuestionBackendCapabilities};
use question_model::generation::QuestionSeed;
use question_model::{
    ActivityTimestamp, GradingResult, QuestionAttemptId, QuestionAttemptReproductionDetails,
    QuestionPresentation, QuestionRendererVersion, QuestionSource, QuestionVersion,
    QuestionVersionReference, SourceObjectReference,
};
use sha2::{Digest, Sha256};

use crate::cache::{
    CachedRender, backend_version, decode_cache, grader_version, parameter_hash, render_key,
    validate_cache,
};
use crate::external_question_provider;
use crate::{
    ADAPTER_ID, ADAPTER_VERSION, GRADING_ID, GRADING_VERSION, GradeBinding, ImathasAdapterError,
    ImathasDraftQuestionSource, ImathasProvider, PreparedSnapshot, ProviderGradeRequest,
    ProviderRenderRequest, ServerCorrelation, SupportedProfile, VerifiedProviderGrade, hex,
    verify_binding,
};

/// Exact immutable source loaded through trusted storage.
#[derive(Clone)]
pub struct ImathasSource {
    pub(crate) question_version: QuestionVersionReference,
    pub(crate) artifact: SourceObjectReference,
    pub(crate) provider: String,
    pub(crate) item_ref: String,
    pub(crate) profile: String,
    pub(crate) bytes: Vec<u8>,
}

impl std::fmt::Debug for ImathasSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasSource")
            .field("source", &"[SERVER-ONLY]")
            .finish_non_exhaustive()
    }
}

impl ImathasSource {
    pub fn artifact(&self) -> &SourceObjectReference {
        &self.artifact
    }
}

/// Key-free issued external-tool question.
#[derive(Debug, Clone, PartialEq)]
pub struct ImathasIssuedAttempt {
    pub envelope: QuestionPresentation,
    pub parameter_hash: String,
    pub reproduction_details: QuestionAttemptReproductionDetails,
    pub cache_hit: bool,
}

/// Server-only verified grade receipt. The attempt store persists the first
/// receipt under its own idempotency key and returns it on replay; this adapter
/// intentionally performs no process-local grade caching.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedGradeReceipt {
    result: GradingResult,
    binding: GradeBinding,
}

impl VerifiedGradeReceipt {
    /// The result accepted from the authenticated provider verifier.
    pub fn result(&self) -> GradingResult {
        self.result
    }

    /// Exact identity the API/store must use when persisting the first receipt.
    pub fn binding(&self) -> GradeBinding {
        self.binding.clone()
    }
}

/// iMathAS adapter with immutable source and deterministic, browser-safe cache.
pub struct ImathasAdapter<S, P> {
    store: S,
    provider: P,
    profiles: BTreeSet<String>,
}

impl<S: ObjectStore, P: ImathasProvider> ImathasAdapter<S, P> {
    pub fn new(
        store: S,
        provider: P,
        profiles: impl IntoIterator<Item = SupportedProfile>,
    ) -> Self {
        Self {
            store,
            provider,
            profiles: profiles.into_iter().map(|profile| profile.name).collect(),
        }
    }

    /// Snapshot a draft before publication. This neither knows nor mints a published identity.
    pub async fn prepare_snapshot(
        &self,
        draft: &question_model::DraftQuestionSource,
    ) -> Result<PreparedSnapshot, ImathasAdapterError> {
        let locator = ImathasDraftQuestionSource::from_draft(draft)?;
        let (bytes, profile) = self
            .provider
            .snapshot(&locator)
            .await
            .map_err(ImathasAdapterError::Provider)?;
        if bytes.is_empty() || !self.profiles.contains(profile.name()) {
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
        source: &QuestionSource,
        profile: &SupportedProfile,
    ) -> Result<QuestionBackendCapabilities, ImathasAdapterError> {
        let QuestionSource::Imathas {
            integration_profile,
            ..
        } = source
        else {
            return Err(ImathasAdapterError::UnsupportedSource);
        };
        if integration_profile != profile.name() || !self.profiles.contains(profile.name()) {
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

    /// Issues an external-tool marker and safe provider prompt. Repeated exact
    /// version/seed requests are served from immutable cache storage.
    pub async fn issue(
        &self,
        question: &QuestionVersion,
        seed: QuestionSeed,
        source: &ImathasSource,
        created_at: ActivityTimestamp,
    ) -> Result<ImathasIssuedAttempt, ImathasAdapterError> {
        question
            .metadata
            .validate_title()
            .map_err(ImathasAdapterError::InvalidTitle)?;
        verify_binding(question, source)?;
        if !self.profiles.contains(&source.profile) {
            return Err(ImathasAdapterError::UnsupportedProfile);
        }
        let question_version = QuestionVersionReference {
            question_id: question.question_id.clone(),
            version_number: question.version_number,
        };
        let key = render_key(&question_version, seed);
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
            .provider
            .render(ProviderRenderRequest {
                snapshot: &source.bytes,
                profile: &source.profile,
                question_version: question_version.clone(),
                seed,
            })
            .await
            .map_err(ImathasAdapterError::Provider)?;
        if question_model::validate_question_title(&safe.title).is_err() {
            return Err(ImathasAdapterError::InvalidProviderRender);
        }
        let record = CachedRender {
            schema: 1,
            source: source.artifact.clone(),
            provider: source.provider.clone(),
            profile: source.profile.clone(),
            envelope: QuestionPresentation {
                variation: question_model::QuestionVariation::static_variation(
                    question_version.clone(),
                    seed,
                ),
                title: safe.title,
                prompt: safe.prompt,
                response: question_model::QuestionResponseFormat::ExternalTool {},
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
                license: None,
                provenance: "safe iMathAS external-tool render cache".into(),
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
        source: &ImathasSource,
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
                    version: source.profile.clone(),
                }),
                generator: None,
                source_object_reference: Some(source.artifact.clone()),
                asset_objects: Vec::new(),
                grader: grader_version(GRADING_ID, GRADING_VERSION),
                rendered_question_sha256: hash,
            },
            envelope: cached.envelope,
            cache_hit,
        })
    }

    /// Accepts only a provider-verifier result matching every server-held binding.
    pub async fn grade(
        &self,
        question: &QuestionVersion,
        source: &ImathasSource,
        attempt: QuestionAttemptId,
        seed: QuestionSeed,
        correlation: &ServerCorrelation,
    ) -> Result<VerifiedGradeReceipt, ImathasAdapterError> {
        verify_binding(question, source)?;
        let question_version = QuestionVersionReference {
            question_id: question.question_id.clone(),
            version_number: question.version_number,
        };
        let verdict = self
            .provider
            .verify_grade(ProviderGradeRequest {
                snapshot: &source.bytes,
                profile: &source.profile,
                attempt,
                question_version: question_version.clone(),
                seed,
                correlation,
            })
            .await
            .map_err(ImathasAdapterError::Provider)?;
        if verdict.attempt != attempt
            || verdict.question_version != question_version
            || verdict.seed != seed
            || verdict.correlation != correlation.0
        {
            return Err(ImathasAdapterError::VerificationRefused);
        }
        Ok(VerifiedGradeReceipt {
            result: verdict.result,
            binding: GradeBinding {
                attempt,
                question_version,
                seed,
            },
        })
    }
}

// The protected scored-embed path is deliberately an opt-in extension. A
// render-only provider cannot acquire launch or result-verification ability.
impl<S, T> ImathasAdapter<S, external_question_provider::ContractedScoredEmbedProvider<T>>
where
    S: ObjectStore,
    T: external_question_provider::ScoredEmbedTransport,
{
    pub fn contracted_provider_key(&self) -> &str {
        self.provider.provider_key()
    }
    pub fn contracted_launch_lifetime_millis(&self) -> u32 {
        self.provider.launch_lifetime_millis()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn begin_contracted_launch(
        &self,
        question: &QuestionVersion,
        source: &ImathasSource,
        attempt: QuestionAttemptId,
        seed: QuestionSeed,
        correlation: ServerCorrelation,
        nonce: crate::scored_embed::ScoredEmbedNonce,
        now: ActivityTimestamp,
    ) -> Result<external_question_provider::ContractedLaunchSession, ImathasAdapterError> {
        self.provider
            .begin_launch(question, source, attempt, seed, correlation, nonce, now)
            .await
    }

    pub async fn retrieve_contracted_grade(
        &self,
        session: &mut external_question_provider::ContractedLaunchSession,
        now: ActivityTimestamp,
    ) -> Result<VerifiedProviderGrade, ImathasAdapterError> {
        self.provider.retrieve_and_verify(session, now).await
    }

    pub async fn proxy_contracted_activity(
        &self,
        session: &external_question_provider::ContractedLaunchSession,
        method: external_question_provider::ProxyMethod,
        body: &[u8],
        now: ActivityTimestamp,
    ) -> Result<external_question_provider::ProxyResponse, ImathasAdapterError> {
        self.provider
            .proxy_activity(session, method, body, now)
            .await
    }
}
