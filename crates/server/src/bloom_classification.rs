//! Server-owned Bloom Classification and publication-preparation boundary.
//!
//! Protected semantic source remains inside this module. Provider output is
//! coupled to the exact candidate before persistence mints a one-use receipt.

use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use async_trait::async_trait;
use learning_data_access::{
    BloomClassificationPreparationStore, BloomPreparationCandidate, BloomPreparationReceiptId,
    PrepareBloomClassificationInput, StoreError,
};
use objects::Sha256Checksum;
use question_model::{BloomClassification, QuestionRevisionReference};
use regex_lite::Regex;

/// Native Ollama implementation of the private classifier contract.
pub mod ollama;

/// Maximum protected semantic evidence admitted to one classification request.
pub const MAX_BLOOM_EVIDENCE_BYTES: usize = 1024 * 1024;

/// Verified protected image evidence required to interpret a Question.
pub struct BloomProtectedAssetEvidence {
    bytes: Arc<[u8]>,
    checksum: Sha256Checksum,
    media_type: String,
}

impl BloomProtectedAssetEvidence {
    /// Admits one exact verified still image without making it browser-visible.
    pub fn new(
        bytes: Vec<u8>,
        checksum: Sha256Checksum,
        media_type: String,
    ) -> Result<Self, BloomClassificationError> {
        if bytes.is_empty()
            || bytes.len() > MAX_BLOOM_EVIDENCE_BYTES
            || Sha256Checksum::compute(&bytes) != checksum
            || !matches!(
                media_type.as_str(),
                "image/jpeg" | "image/png" | "image/webp"
            )
        {
            return Err(BloomClassificationError::UnsupportedEvidence);
        }
        Ok(Self {
            bytes: Arc::from(bytes),
            checksum,
            media_type,
        })
    }
}

/// Exact immutable Question evidence admitted for one provider call.
pub struct BloomQuestionEvidence {
    source_bytes: Arc<[u8]>,
    source_checksum: Sha256Checksum,
    source_media_type: String,
    protected_assets: Vec<BloomProtectedAssetEvidence>,
}

impl BloomQuestionEvidence {
    /// Validates exact source bytes and any required protected visual evidence.
    pub fn new(
        source_bytes: Vec<u8>,
        source_checksum: Sha256Checksum,
        source_media_type: String,
        protected_assets: Vec<BloomProtectedAssetEvidence>,
    ) -> Result<Self, BloomClassificationError> {
        if source_bytes.is_empty()
            || Sha256Checksum::compute(&source_bytes) != source_checksum
            || !matches!(
                source_media_type.as_str(),
                "application/vnd.peptidyle.question+json" | "text/x-wework-pg"
            )
            || std::str::from_utf8(&source_bytes).is_err()
        {
            return Err(BloomClassificationError::UnsupportedEvidence);
        }
        // ASVS 2.2.1 and 2.2.2: the trusted service boundary admits only
        // complete visual evidence before the provider can observe the source.
        validate_visual_evidence(&source_bytes, &source_media_type)?;
        let evidence_bytes = protected_assets
            .iter()
            .try_fold(source_bytes.len(), |total, asset| {
                total.checked_add(asset.bytes.len())
            });
        if evidence_bytes.is_none_or(|total| total > MAX_BLOOM_EVIDENCE_BYTES) {
            return Err(BloomClassificationError::InputTooLarge);
        }
        Ok(Self {
            source_bytes: Arc::from(source_bytes),
            source_checksum,
            source_media_type,
            protected_assets,
        })
    }
}

fn validate_visual_evidence(
    source_bytes: &[u8],
    source_media_type: &str,
) -> Result<(), BloomClassificationError> {
    let source = std::str::from_utf8(source_bytes)
        .map_err(|_| BloomClassificationError::UnsupportedEvidence)?;
    match source_media_type {
        adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE => {
            let document = adapter_ple::question_json::PleQuestionJsonDocument::parse(source_bytes)
                .map_err(|_| BloomClassificationError::UnsupportedEvidence)?;
            if document.has_external_image_resource() {
                return Err(BloomClassificationError::UnsupportedEvidence);
            }
        }
        "text/x-wework-pg" if webwork_has_visual_dependency(source) => {
            return Err(BloomClassificationError::UnsupportedEvidence);
        }
        "text/x-wework-pg" => {}
        _ => return Err(BloomClassificationError::UnsupportedEvidence),
    }
    Ok(())
}

fn webwork_has_visual_dependency(source: &str) -> bool {
    static VISUAL_OR_RESOURCE_CONSTRUCT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i:<img|<svg|!\[|image\s*\(|background-image|\.(?:png|jpe?g|gif|svg|webp)|pggraphmacros\.pl|pgtikz\.pl|lateximage|gd::)",
        )
        .expect("fixed visual-dependency expression")
    });
    VISUAL_OR_RESOURCE_CONSTRUCT.is_match(source)
}

/// One exact ordered Question Revision and its complete semantic evidence.
pub struct BloomPoolMemberEvidence {
    question_revision: QuestionRevisionReference,
    question: BloomQuestionEvidence,
}

impl BloomPoolMemberEvidence {
    /// Couples a member reference to the exact source evidence classified for it.
    pub fn new(
        question_revision: QuestionRevisionReference,
        question: BloomQuestionEvidence,
    ) -> Self {
        Self {
            question_revision,
            question,
        }
    }
}

/// Exact intended Pool Revision evidence. Pool publication orchestration is a
/// later slice; this type only closes the provider contract now.
pub struct BloomPoolEvidence {
    title: String,
    description: String,
    members: Vec<BloomPoolMemberEvidence>,
}

impl BloomPoolEvidence {
    /// Admits one complete intended Pool Revision for classification.
    pub fn new(
        title: String,
        description: String,
        members: Vec<BloomPoolMemberEvidence>,
    ) -> Result<Self, BloomClassificationError> {
        if title.trim().is_empty() || description.trim().is_empty() || members.is_empty() {
            return Err(BloomClassificationError::UnsupportedEvidence);
        }
        let evidence_bytes = members.iter().try_fold(
            title.len().saturating_add(description.len()),
            |total, member| {
                let question_bytes = member.question.protected_assets.iter().try_fold(
                    member.question.source_bytes.len(),
                    |question_total, asset| question_total.checked_add(asset.bytes.len()),
                )?;
                total.checked_add(question_bytes)
            },
        );
        if evidence_bytes.is_none_or(|total| total > MAX_BLOOM_EVIDENCE_BYTES) {
            return Err(BloomClassificationError::InputTooLarge);
        }
        Ok(Self {
            title,
            description,
            members,
        })
    }
}

/// Closed protected candidate kinds accepted by a Bloom provider.
pub enum BloomClassificationCandidate {
    /// One immutable Question source and its required protected assets.
    Question(BloomQuestionEvidence),
    /// One complete intended Pool Revision and every ordered member source.
    Pool(BloomPoolEvidence),
}

impl BloomClassificationCandidate {
    fn preparation_candidate(&self) -> BloomPreparationCandidate {
        match self {
            Self::Question(question) => BloomPreparationCandidate::Question {
                source_checksum: question.source_checksum,
            },
            Self::Pool(pool) => BloomPreparationCandidate::Pool {
                title: pool.title.clone(),
                description: pool.description.clone(),
                members: pool
                    .members
                    .iter()
                    .map(|member| member.question_revision.clone())
                    .collect(),
            },
        }
    }
}

/// Non-sensitive provider metadata retained with a classified candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BloomClassifierProvenance {
    provider_kind: String,
    model_profile: String,
    prompt_revision: String,
    elapsed: Duration,
}

impl BloomClassifierProvenance {
    /// Creates bounded operational provenance without source or response text.
    pub fn new(
        provider_kind: String,
        model_profile: String,
        prompt_revision: String,
        elapsed: Duration,
    ) -> Result<Self, BloomClassificationError> {
        for value in [&provider_kind, &model_profile, &prompt_revision] {
            if value.is_empty()
                || value.len() > 200
                || value.chars().any(|character| character.is_control())
            {
                return Err(BloomClassificationError::InvalidResponse);
            }
        }
        Ok(Self {
            provider_kind,
            model_profile,
            prompt_revision,
            elapsed,
        })
    }

    /// Returns the provider family without exposing protected evidence.
    pub fn provider_kind(&self) -> &str {
        &self.provider_kind
    }

    /// Returns the selected non-secret model profile.
    pub fn model_profile(&self) -> &str {
        &self.model_profile
    }

    /// Returns the fixed reviewed prompt revision.
    pub fn prompt_revision(&self) -> &str {
        &self.prompt_revision
    }

    /// Returns provider-call elapsed time.
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

/// Strict provider result before it is coupled to semantic candidate facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BloomClassifierOutput {
    classification: BloomClassification,
    provenance: BloomClassifierProvenance,
}

impl BloomClassifierOutput {
    /// Builds one complete two-dimensional result from a trusted provider adapter.
    pub const fn new(
        classification: BloomClassification,
        provenance: BloomClassifierProvenance,
    ) -> Self {
        Self {
            classification,
            provenance,
        }
    }
}

/// Server-side provider contract. Candidates deliberately have no browser
/// serialization and no `Debug` implementation.
#[async_trait]
pub trait BloomClassifier: Send + Sync {
    /// Verifies that the selected provider/model is available without source evidence.
    async fn preflight(&self) -> Result<(), BloomClassificationError> {
        Ok(())
    }

    /// Classifies one exact complete semantic candidate.
    async fn classify(
        &self,
        candidate: &BloomClassificationCandidate,
    ) -> Result<BloomClassifierOutput, BloomClassificationError>;
}

/// Fail-closed provider used when no installation-owned configuration exists.
pub struct NotConfiguredBloomClassifier;

#[async_trait]
impl BloomClassifier for NotConfiguredBloomClassifier {
    async fn preflight(&self) -> Result<(), BloomClassificationError> {
        Err(BloomClassificationError::NotConfigured)
    }

    async fn classify(
        &self,
        _candidate: &BloomClassificationCandidate,
    ) -> Result<BloomClassifierOutput, BloomClassificationError> {
        Err(BloomClassificationError::NotConfigured)
    }
}

/// Candidate-bound result used to mint receipts without another provider call.
pub struct ClassifiedBloomCandidate {
    preparation_candidate: BloomPreparationCandidate,
    classification: BloomClassification,
    provenance: BloomClassifierProvenance,
}

impl ClassifiedBloomCandidate {
    /// Returns safe operational provenance only.
    pub const fn provenance(&self) -> &BloomClassifierProvenance {
        &self.provenance
    }
}

/// Couples classification and one-use receipt preparation for publication.
pub struct BloomPublicationPreparation {
    classifier: Arc<dyn BloomClassifier>,
    receipts: Arc<dyn BloomClassificationPreparationStore>,
}

impl BloomPublicationPreparation {
    /// Creates the server-owned service from separate provider and persistence owners.
    pub fn new(
        classifier: Arc<dyn BloomClassifier>,
        receipts: Arc<dyn BloomClassificationPreparationStore>,
    ) -> Self {
        Self {
            classifier,
            receipts,
        }
    }

    /// Performs a source-free provider/model readiness check.
    pub async fn preflight(&self) -> Result<(), BloomClassificationError> {
        self.classifier.preflight().await
    }

    /// Calls the provider once, then seals its output to exact preparation facts.
    pub async fn classify(
        &self,
        candidate: &BloomClassificationCandidate,
    ) -> Result<ClassifiedBloomCandidate, BloomClassificationError> {
        let output = match self.classifier.classify(candidate).await {
            Ok(output) => output,
            Err(error) => {
                // ASVS V14.2.1 and V15.3.2: only a closed reason code enters
                // the request span; candidate and raw provider data never do.
                tracing::warn!(
                    event = "bloom_classification",
                    outcome = "failed",
                    reason = error.reason_code()
                );
                return Err(error);
            }
        };
        tracing::info!(
            event = "bloom_classification",
            outcome = "classified",
            provider_kind = output.provenance.provider_kind(),
            model_profile = output.provenance.model_profile(),
            prompt_revision = output.provenance.prompt_revision(),
            elapsed_milliseconds = output.provenance.elapsed().as_millis()
        );
        Ok(ClassifiedBloomCandidate {
            preparation_candidate: candidate.preparation_candidate(),
            classification: output.classification,
            provenance: output.provenance,
        })
    }

    /// Mints one fresh one-use receipt for an already classified exact candidate.
    pub async fn prepare_receipt(
        &self,
        classified: &ClassifiedBloomCandidate,
    ) -> Result<BloomPreparationReceiptId, BloomClassificationError> {
        self.receipts
            .prepare_bloom_classification(PrepareBloomClassificationInput {
                candidate: classified.preparation_candidate.clone(),
                classification: classified.classification,
            })
            .await
            .map_err(BloomClassificationError::Store)
    }
}

/// Redacted failures at the private classification boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum BloomClassificationError {
    /// No provider/model was selected for this installation.
    NotConfigured,
    /// The bounded provider concurrency slot was already occupied.
    Busy,
    /// The bounded provider call exceeded its deadline.
    Timeout,
    /// The local provider or selected model could not serve the request.
    Unavailable,
    /// Complete semantic evidence exceeded the fixed admission limit.
    InputTooLarge,
    /// Required evidence could not be represented by this provider path.
    UnsupportedEvidence,
    /// The provider reply violated the strict response contract.
    InvalidResponse,
    /// One-use receipt persistence failed after classification completed.
    Store(StoreError),
}

impl BloomClassificationError {
    /// Returns a fixed operator-safe reason code with no provider or source text.
    pub const fn reason_code(&self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::Busy => "busy",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::InputTooLarge => "input_too_large",
            Self::UnsupportedEvidence => "unsupported_evidence",
            Self::InvalidResponse => "invalid_response",
            Self::Store(_) => "receipt_store",
        }
    }
}

impl std::fmt::Display for BloomClassificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => formatter.write_str("Bloom Classification is not configured"),
            Self::Busy => formatter.write_str("Bloom Classification provider is busy"),
            Self::Timeout => formatter.write_str("Bloom Classification provider timed out"),
            Self::Unavailable => {
                formatter.write_str("Bloom Classification provider is unavailable")
            }
            Self::InputTooLarge => {
                formatter.write_str("Bloom Classification evidence is too large")
            }
            Self::UnsupportedEvidence => {
                formatter.write_str("Bloom Classification evidence is unsupported")
            }
            Self::InvalidResponse => {
                formatter.write_str("Bloom Classification provider returned an invalid response")
            }
            Self::Store(error) => write!(formatter, "Bloom Classification receipt failed: {error}"),
        }
    }
}

impl std::error::Error for BloomClassificationError {}
