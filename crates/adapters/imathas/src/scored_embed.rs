//! The first concrete, deliberately narrow iMathAS scored-embed profile.
//!
//! This module verifies a score only after the server-side broker has matched
//! it to an exact, single-use launch ledger.  iMathAS result JWTs authenticate
//! the provider response, but the upstream protocol does not carry PLE's
//! account, attempt, version, nonce, or idempotency claims. Consequently a
//! valid JWT alone is never a grade.

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::{ActivityTimestamp, GradingResult, QuestionRevisionReference};
use serde::Deserialize;
use serde::de::IgnoredAny;
use sha2::{Digest, Sha256};

use crate::{
    GradeBinding, ImathasAdapterError, ProviderFailure, ServerCorrelation, VerifiedProviderGrade,
    constant_time_eq,
};

/// The only currently supported server-graded iMathAS profile.
pub const SCORED_EMBED_BROKER_PROFILE_ID: &str = "imathas_scored_embed_broker_v1";
/// The bounded iMathAS scored-embed seed range documented by the provider.
pub const PROVIDER_SEED_VARIANTS: u64 = 9_999;
const MAX_RESULT_TOKEN_BYTES: usize = 8_192;
const MAX_RESULT_BODY_BYTES: usize = 4_096;
const MAX_QUESTION_ID_BYTES: usize = 128;

/// Maps PLE's larger seed space into iMathAS's documented `1..=9999` range.
///
/// This is deterministic but not injective.  The resulting 9,999-variant
/// collision space is an explicit authoring and capability limitation, not a
/// claim that iMathAS can reproduce every PLE seed uniquely.
pub fn normalize_provider_seed(ple_seed: question_model::generation::QuestionSeed) -> u16 {
    (ple_seed.value() % PROVIDER_SEED_VARIANTS + 1) as u16
}

/// Deployment confirmation required before this profile can grade a published
/// question.  It intentionally contains no host, URL, credential, or JWT.
#[derive(Clone, PartialEq, Eq)]
pub struct ScoredEmbedProfileConfig {
    provider_key: String,
    frozen_execution_target: bool,
    source_digest_revalidation: bool,
    contracted_scored_embed: bool,
}

impl std::fmt::Debug for ScoredEmbedProfileConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScoredEmbedProfileConfig")
            .field("provider_key", &self.provider_key)
            .field("frozen_execution_target", &self.frozen_execution_target)
            .field(
                "source_digest_revalidation",
                &self.source_digest_revalidation,
            )
            .field("contracted_scored_embed", &self.contracted_scored_embed)
            .finish()
    }
}

impl ScoredEmbedProfileConfig {
    /// Creates the profile only for an explicitly contracted/self-hosted
    /// provider where the operator guarantees a frozen execution target and
    /// revalidates its source digest at every launch.
    pub fn contracted_self_hosted(
        provider_key: impl Into<String>,
        frozen_execution_target: bool,
        source_digest_revalidation: bool,
    ) -> Result<Self, ScoredEmbedFailure> {
        let provider_key = provider_key.into();
        if !valid_provider_key(&provider_key)
            || !frozen_execution_target
            || !source_digest_revalidation
        {
            return Err(ScoredEmbedFailure::UnsupportedProfile);
        }
        Ok(Self {
            provider_key,
            frozen_execution_target,
            source_digest_revalidation,
            contracted_scored_embed: true,
        })
    }

    /// Generic hosted MyOpenMath has no confirmed immutable execution target
    /// or server-grade correlation contract, so published server grading is
    /// deliberately refused.  A separate ungraded sandbox profile may use a
    /// public practice embed, but it is outside this module.
    pub fn generic_myopenmath_hosted(provider_key: impl Into<String>) -> Self {
        Self {
            provider_key: provider_key.into(),
            frozen_execution_target: false,
            source_digest_revalidation: false,
            contracted_scored_embed: false,
        }
    }

    /// The opaque deployment selector used to bind a ledger to one provider.
    pub fn provider_key(&self) -> &str {
        &self.provider_key
    }

    /// Whether this deployment may expose `serverGrading` for this profile.
    pub fn allows_published_server_grading(&self) -> bool {
        self.contracted_scored_embed
            && self.frozen_execution_target
            && self.source_digest_revalidation
    }
}

/// Redacted classification for a scored-embed refusal.  No variant carries an
/// upstream token, response body, answer, source, URL, or secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoredEmbedFailure {
    UnsupportedProfile,
    InvalidLedger,
    StaleLedger,
    DuplicateResult,
    InvalidResult,
    InvalidSignature,
    MissingLaunchBinding,
    WrongLaunchBinding,
    WrongQuestion,
    InvalidScore,
    ProviderUnavailable,
}

impl ScoredEmbedFailure {
    /// Maps only transport-independent failures into the adapter's existing
    /// question-local retry/degraded classification.
    pub fn into_adapter_error(self) -> ImathasAdapterError {
        match self {
            Self::UnsupportedProfile => ImathasAdapterError::UnsupportedProfile,
            Self::InvalidLedger | Self::StaleLedger | Self::DuplicateResult => {
                ImathasAdapterError::InvalidCorrelation
            }
            Self::InvalidResult
            | Self::InvalidSignature
            | Self::MissingLaunchBinding
            | Self::WrongLaunchBinding
            | Self::WrongQuestion
            | Self::InvalidScore => ImathasAdapterError::VerificationRefused,
            Self::ProviderUnavailable => {
                ImathasAdapterError::Provider(ProviderFailure::Unavailable)
            }
        }
    }
}

/// Server-only, exact launch state.  The broker must persist this alongside
/// its own idempotency transaction; this in-memory type makes no durability
/// claim and deliberately has no serde implementation.
pub struct ScoredEmbedLaunchLedger {
    binding: GradeBinding,
    provider_key: String,
    provider_question_id: String,
    source_digest: String,
    profile: String,
    provider_seed: u16,
    expires_at: ActivityTimestamp,
    correlation: ServerCorrelation,
    nonce: ScoredEmbedNonce,
    binding_digest: String,
    consumed: bool,
}

impl std::fmt::Debug for ScoredEmbedLaunchLedger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScoredEmbedLaunchLedger")
            .field("binding", &self.binding)
            .field("provider_key", &self.provider_key)
            .field("provider_question_id", &self.provider_question_id)
            .field("source_digest", &"REDACTED")
            .field("profile", &self.profile)
            .field("provider_seed", &self.provider_seed)
            .field("expires_at", &self.expires_at)
            .field("correlation", &"REDACTED")
            .field("nonce", &"REDACTED")
            .field("binding_digest", &"REDACTED")
            .field("consumed", &self.consumed)
            .finish()
    }
}

impl ScoredEmbedLaunchLedger {
    /// Builds exact server-held state after the broker has authenticated the
    /// attempt, verified the frozen execution target, and revalidated the
    /// pinned source digest.  `expires_at` is mandatory because provider
    /// result JWTs are allowed to omit `exp` by the upstream protocol.
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        profile: &ScoredEmbedProfileConfig,
        binding: GradeBinding,
        provider_question_id: impl Into<String>,
        source_digest: impl Into<String>,
        expires_at: ActivityTimestamp,
        correlation: ServerCorrelation,
        nonce: ScoredEmbedNonce,
    ) -> Result<Self, ScoredEmbedFailure> {
        let provider_question_id = provider_question_id.into();
        let source_digest = source_digest.into();
        if !profile.allows_published_server_grading()
            || !valid_question_id(&provider_question_id)
            || !valid_sha256(&source_digest)
            || expires_at.as_unix_millis() <= 0
        {
            return Err(ScoredEmbedFailure::InvalidLedger);
        }
        let binding_digest = launch_binding_digest(
            &binding,
            &provider_question_id,
            &source_digest,
            normalize_provider_seed(binding.seed),
            &correlation,
        );
        Ok(Self {
            binding: binding.clone(),
            provider_key: profile.provider_key.clone(),
            provider_question_id,
            source_digest,
            profile: SCORED_EMBED_BROKER_PROFILE_ID.into(),
            provider_seed: normalize_provider_seed(binding.seed),
            expires_at,
            correlation,
            nonce,
            binding_digest,
            consumed: false,
        })
    }

    /// Records both seed values in private launch/cache validation.
    pub fn provider_seed(&self) -> u16 {
        self.provider_seed
    }
    /// Original PLE seed retained for exact attempt replay.
    pub fn ple_seed(&self) -> question_model::generation::QuestionSeed {
        self.binding.seed
    }
    /// The complete server-held cache entry identity for this provider render.
    ///
    /// It omits the attempt binding so identical published Question Revisions may
    /// reuse rendered content, while binding the exact provider payload and its
    /// validity window.
    pub fn external_question_provider_cache_entry(&self) -> ExternalQuestionProviderCacheEntry {
        ExternalQuestionProviderCacheEntry {
            question_revision: self.binding.question_revision.clone(),
            provider_seed: self.provider_seed,
            profile: self.profile.clone(),
            payload_digest: self.source_digest.clone(),
            expires_at: self.expires_at,
        }
    }

    pub(crate) fn correlation(&self) -> &ServerCorrelation {
        &self.correlation
    }

    pub(crate) fn ensure_eligible_at(
        &self,
        now: ActivityTimestamp,
    ) -> Result<(), ScoredEmbedFailure> {
        if self.consumed {
            return Err(ScoredEmbedFailure::DuplicateResult);
        }
        if now.as_unix_millis() > self.expires_at.as_unix_millis() {
            return Err(ScoredEmbedFailure::StaleLedger);
        }
        Ok(())
    }

    pub(crate) fn storage_parts(&self) -> LaunchLedgerStorageParts {
        LaunchLedgerStorageParts {
            binding: self.binding.clone(),
            provider_key: self.provider_key.clone(),
            provider_question_id: self.provider_question_id.clone(),
            source_digest: self.source_digest.clone(),
            profile: self.profile.clone(),
            provider_seed: self.provider_seed,
            expires_at: self.expires_at,
            correlation: self.correlation.0.clone(),
            nonce: self.nonce.0,
            binding_digest: self.binding_digest.clone(),
            consumed: self.consumed,
        }
    }

    pub(crate) fn from_storage_parts(
        parts: LaunchLedgerStorageParts,
    ) -> Result<Self, ScoredEmbedFailure> {
        if !valid_provider_key(&parts.provider_key)
            || !valid_question_id(&parts.provider_question_id)
            || !valid_sha256(&parts.source_digest)
            || parts.profile != SCORED_EMBED_BROKER_PROFILE_ID
            || parts.provider_seed != normalize_provider_seed(parts.binding.seed)
            || parts.expires_at.as_unix_millis() <= 0
            || parts.nonce.iter().all(|byte| *byte == 0)
            || !valid_sha256(&parts.binding_digest)
        {
            return Err(ScoredEmbedFailure::InvalidLedger);
        }
        let correlation = ServerCorrelation(parts.correlation);
        let expected = launch_binding_digest(
            &parts.binding,
            &parts.provider_question_id,
            &parts.source_digest,
            parts.provider_seed,
            &correlation,
        );
        if !constant_time_eq(expected.as_bytes(), parts.binding_digest.as_bytes()) {
            return Err(ScoredEmbedFailure::InvalidLedger);
        }
        Ok(Self {
            binding: parts.binding,
            provider_key: parts.provider_key,
            provider_question_id: parts.provider_question_id,
            source_digest: parts.source_digest,
            profile: parts.profile,
            provider_seed: parts.provider_seed,
            expires_at: parts.expires_at,
            correlation,
            nonce: ScoredEmbedNonce(parts.nonce),
            binding_digest: parts.binding_digest,
            consumed: parts.consumed,
        })
    }

    /// Private signed-launch claims for the contracted provider extension.
    /// These are sent only in the protected server-to-provider signed launch
    /// contract, never in a PLE browser DTO, URL, log, or Debug output.
    pub fn signed_launch_claims(&self) -> ScoredEmbedLaunchClaims {
        ScoredEmbedLaunchClaims {
            nonce: self.nonce.encoded(),
            binding_digest: self.binding_digest.clone(),
        }
    }
}

/// Private fields carried inside an authenticated launch-session storage blob.
/// It intentionally has no serde or Debug representation.
pub(crate) struct LaunchLedgerStorageParts {
    pub(crate) binding: GradeBinding,
    pub(crate) provider_key: String,
    pub(crate) provider_question_id: String,
    pub(crate) source_digest: String,
    pub(crate) profile: String,
    pub(crate) provider_seed: u16,
    pub(crate) expires_at: ActivityTimestamp,
    pub(crate) correlation: String,
    pub(crate) nonce: [u8; 32],
    pub(crate) binding_digest: String,
    pub(crate) consumed: bool,
}

/// High-entropy server-generated nonce for one provider launch. The broker
/// must use cryptographically random bytes; this type rejects the obvious
/// empty/all-zero placeholder and is never serializable.
#[derive(Clone, PartialEq, Eq)]
pub struct ScoredEmbedNonce([u8; 32]);

impl ScoredEmbedNonce {
    /// Wraps 256 bits generated by the server's CSPRNG.
    pub fn from_server_random(value: [u8; 32]) -> Result<Self, ScoredEmbedFailure> {
        if value.iter().all(|byte| *byte == 0) {
            return Err(ScoredEmbedFailure::InvalidLedger);
        }
        Ok(Self(value))
    }

    fn encoded(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0)
    }
}

impl std::fmt::Debug for ScoredEmbedNonce {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ScoredEmbedNonce(REDACTED)")
    }
}

/// Server-private claims that a contracted/self-hosted provider must copy from
/// the signed launch JWT into its signed result JWT. Stock scored embeds do
/// not document these claims, so a provider unable to echo them is unavailable
/// for this profile rather than weakly compatible.
pub struct ScoredEmbedLaunchClaims {
    nonce: String,
    binding_digest: String,
}

impl ScoredEmbedLaunchClaims {
    /// Opaque, per-launch correlation claim for the provider's signed result.
    pub fn nonce(&self) -> &str {
        &self.nonce
    }
    /// Signed digest of exact attempt/problem/version/seed/source/profile binding.
    pub fn binding_digest(&self) -> &str {
        &self.binding_digest
    }
}

impl std::fmt::Debug for ScoredEmbedLaunchClaims {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ScoredEmbedLaunchClaims(REDACTED)")
    }
}

/// Complete server-held identity for a cached external-provider render.
///
/// The storage address belongs to the cache implementation and is deliberately
/// absent from this application record.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalQuestionProviderCacheEntry {
    question_revision: QuestionRevisionReference,
    provider_seed: u16,
    profile: String,
    payload_digest: String,
    expires_at: ActivityTimestamp,
}

impl std::fmt::Debug for ExternalQuestionProviderCacheEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExternalQuestionProviderCacheEntry")
            .field("question_revision", &self.question_revision)
            .field("provider_seed", &self.provider_seed)
            .field("profile", &self.profile)
            .field("payload_digest", &"REDACTED")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Server-private scored-embed result verifier.  The credential is accepted at
/// composition time and has no getter or serializable representation.
pub struct ScoredEmbedResultVerifier {
    profile: ScoredEmbedProfileConfig,
    signing_secret: Vec<u8>,
}

impl std::fmt::Debug for ScoredEmbedResultVerifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScoredEmbedResultVerifier")
            .field("profile", &self.profile)
            .field("signing_secret", &"REDACTED")
            .finish()
    }
}

impl ScoredEmbedResultVerifier {
    /// Installs one provider's protected HS256 result-verification secret.
    pub fn new(
        profile: ScoredEmbedProfileConfig,
        signing_secret: impl AsRef<[u8]>,
    ) -> Result<Self, ScoredEmbedFailure> {
        let signing_secret = signing_secret.as_ref();
        if !profile.allows_published_server_grading() || signing_secret.is_empty() {
            return Err(ScoredEmbedFailure::UnsupportedProfile);
        }
        Ok(Self {
            profile,
            signing_secret: signing_secret.to_vec(),
        })
    }

    /// Verifies a bounded provider response after an allowlisted broker proxy
    /// receives it server-to-server.  Callers must never pass browser
    /// `postMessage` content here.  On success the ledger is consumed; durable
    /// replay/idempotency belongs to the caller's broker transaction.
    pub fn verify_result(
        &self,
        ledger: &mut ScoredEmbedLaunchLedger,
        result_token: &str,
        now: ActivityTimestamp,
    ) -> Result<VerifiedProviderGrade, ScoredEmbedFailure> {
        if !self.profile.allows_published_server_grading()
            || ledger.provider_key != self.profile.provider_key
            || ledger.profile != SCORED_EMBED_BROKER_PROFILE_ID
            || ledger.provider_seed != normalize_provider_seed(ledger.binding.seed)
            || !valid_sha256(&ledger.source_digest)
        {
            return Err(ScoredEmbedFailure::InvalidLedger);
        }
        ledger.ensure_eligible_at(now)?;
        let claims = verify_hs256(result_token, &self.signing_secret)?;
        if claims.question_id != ledger.provider_question_id {
            return Err(ScoredEmbedFailure::WrongQuestion);
        }
        let Some(nonce) = claims.nonce else {
            return Err(ScoredEmbedFailure::MissingLaunchBinding);
        };
        let Some(binding_digest) = claims.binding_digest else {
            return Err(ScoredEmbedFailure::MissingLaunchBinding);
        };
        let expected_nonce = ledger.nonce.encoded();
        if !constant_time_eq(nonce.as_bytes(), expected_nonce.as_bytes())
            || !constant_time_eq(binding_digest.as_bytes(), ledger.binding_digest.as_bytes())
        {
            return Err(ScoredEmbedFailure::WrongLaunchBinding);
        }
        if !claims.score.is_finite() || !(0.0..=1.0).contains(&claims.score) {
            return Err(ScoredEmbedFailure::InvalidScore);
        }
        if let Some(exp) = claims.exp {
            let Some(expiry_ms) = exp.checked_mul(1_000) else {
                return Err(ScoredEmbedFailure::InvalidResult);
            };
            if now.as_unix_millis() > expiry_ms {
                return Err(ScoredEmbedFailure::StaleLedger);
            }
        }
        // Upstream permits no exp. The mandatory, exact broker ledger expiry
        // above and its server-side receipt are therefore the fallback, not an
        // invented JWT guarantee.
        ledger.consumed = true;
        Ok(VerifiedProviderGrade::from_scored_embed(
            GradingResult {
                correct: claims.score >= 1.0,
                points_earned: claims.score,
                points_possible: 1.0,
            },
            ledger.binding.clone(),
            &ledger.correlation,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JwtHeader {
    alg: String,
    #[serde(default)]
    typ: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ResultQuestionId {
    Text(String),
    Number(u64),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultClaims {
    #[serde(rename = "id")]
    question_id: ResultQuestionId,
    score: f64,
    #[serde(default)]
    exp: Option<i64>,
    /// Required contracted-profile extension: exact opaque launch nonce.
    #[serde(default, rename = "ple_nonce")]
    nonce: Option<String>,
    /// Required contracted-profile extension: signed exact ledger binding.
    #[serde(default, rename = "ple_binding")]
    binding_digest: Option<String>,
    // These official fields can contain response, correct-answer, or provider
    // diagnostics. Deserialize only to discard them immediately.
    #[serde(default)]
    raw: Option<IgnoredAny>,
    #[serde(default)]
    allans: Option<IgnoredAny>,
    #[serde(default)]
    redisplay: Option<IgnoredAny>,
    #[serde(default)]
    errors: Option<IgnoredAny>,
}

struct VerifiedClaims {
    question_id: String,
    score: f64,
    exp: Option<i64>,
    nonce: Option<String>,
    binding_digest: Option<String>,
}

fn verify_hs256(token: &str, secret: &[u8]) -> Result<VerifiedClaims, ScoredEmbedFailure> {
    if token.is_empty() || token.len() > MAX_RESULT_TOKEN_BYTES || !token.is_ascii() {
        return Err(ScoredEmbedFailure::InvalidResult);
    }
    let mut sections = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) = (
        sections.next(),
        sections.next(),
        sections.next(),
        sections.next(),
    ) else {
        return Err(ScoredEmbedFailure::InvalidResult);
    };
    if header.is_empty() || payload.is_empty() || signature.is_empty() {
        return Err(ScoredEmbedFailure::InvalidResult);
    }
    let decode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_bytes = decode
        .decode(header)
        .map_err(|_| ScoredEmbedFailure::InvalidResult)?;
    let payload_bytes = decode
        .decode(payload)
        .map_err(|_| ScoredEmbedFailure::InvalidResult)?;
    if header_bytes.len() > MAX_RESULT_BODY_BYTES || payload_bytes.len() > MAX_RESULT_BODY_BYTES {
        return Err(ScoredEmbedFailure::InvalidResult);
    }
    let parsed_header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| ScoredEmbedFailure::InvalidResult)?;
    if parsed_header.alg != "HS256" || parsed_header.typ.as_deref().is_some_and(|typ| typ != "JWT")
    {
        return Err(ScoredEmbedFailure::InvalidResult);
    }
    let signature = decode
        .decode(signature)
        .map_err(|_| ScoredEmbedFailure::InvalidResult)?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret).map_err(|_| ScoredEmbedFailure::InvalidSignature)?;
    mac.update(header.as_bytes());
    mac.update(b".");
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| ScoredEmbedFailure::InvalidSignature)?;
    let claims: ResultClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| ScoredEmbedFailure::InvalidResult)?;
    let ResultClaims {
        question_id,
        score,
        exp,
        nonce,
        binding_digest,
        raw,
        allans,
        redisplay,
        errors,
    } = claims;
    // Touch these answer-bearing fields solely to make their discard explicit.
    // They never leave this scope, enter a receipt, cache, or Debug output.
    let _ = (raw, allans, redisplay, errors);
    let question_id = match question_id {
        ResultQuestionId::Text(value) => value,
        ResultQuestionId::Number(value) => value.to_string(),
    };
    if !valid_question_id(&question_id) {
        return Err(ScoredEmbedFailure::InvalidResult);
    }
    Ok(VerifiedClaims {
        question_id,
        score,
        exp,
        nonce: nonce.filter(|value| valid_nonce(value)),
        binding_digest: binding_digest.filter(|value| valid_sha256(value)),
    })
}

fn valid_provider_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_question_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_QUESTION_ID_BYTES
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_nonce(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn launch_binding_digest(
    binding: &GradeBinding,
    provider_question_id: &str,
    source_digest: &str,
    provider_seed: u16,
    correlation: &ServerCorrelation,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ple:imathas:scored-embed-binding:v1");
    digest.update(binding.attempt.as_uuid().as_bytes());
    digest.update(binding.question_revision.question_id.to_string().as_bytes());
    digest.update(
        binding
            .question_revision
            .revision_number
            .get()
            .to_be_bytes(),
    );
    digest.update(binding.seed.value().to_be_bytes());
    digest.update(provider_seed.to_be_bytes());
    digest.update(SCORED_EMBED_BROKER_PROFILE_ID.as_bytes());
    digest.update(provider_question_id.as_bytes());
    digest.update(source_digest.as_bytes());
    digest.update(correlation.0.as_bytes());
    crate::hex(digest.finalize().as_slice())
}

#[cfg(test)]
mod tests {
    use hmac::{Hmac, KeyInit, Mac};
    use question_model::{
        QuestionAttemptId, QuestionId, QuestionRevisionNumber, QuestionRevisionReference,
        generation::QuestionSeed,
    };
    use uuid::Uuid;

    use super::*;
    use crate::CorrelationIssuer;

    fn profile() -> ScoredEmbedProfileConfig {
        ScoredEmbedProfileConfig::contracted_self_hosted("self-hosted-imathas", true, true).unwrap()
    }

    fn binding() -> GradeBinding {
        GradeBinding {
            attempt: QuestionAttemptId::from_uuid(Uuid::from_u128(2)),
            question_revision: QuestionRevisionReference {
                question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
                revision_number: QuestionRevisionNumber::new(4).expect("positive version"),
            },
            seed: QuestionSeed::new(10_001),
        }
    }

    fn ledger_with(
        binding: GradeBinding,
        source_digest: String,
        nonce: [u8; 32],
        expires_at: i64,
    ) -> ScoredEmbedLaunchLedger {
        let correlation = CorrelationIssuer::from_server_secret([9; 32])
            .restore(
                binding.clone(),
                &CorrelationIssuer::from_server_secret([9; 32]).begin(binding.clone()),
            )
            .unwrap();
        ScoredEmbedLaunchLedger::begin(
            &profile(),
            binding,
            "17",
            source_digest,
            ActivityTimestamp::from_unix_millis(expires_at),
            correlation,
            ScoredEmbedNonce::from_server_random(nonce).unwrap(),
        )
        .unwrap()
    }

    fn ledger(expires_at: i64) -> ScoredEmbedLaunchLedger {
        ledger_with(binding(), "a".repeat(64), [7; 32], expires_at)
    }

    fn token(payload: &str, secret: &[u8]) -> String {
        let encode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let header = encode.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = encode.encode(payload);
        let signed = format!("{header}.{payload}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(signed.as_bytes());
        format!("{signed}.{}", encode.encode(mac.finalize().into_bytes()))
    }

    fn bound_token(
        ledger: &ScoredEmbedLaunchLedger,
        score: f64,
        extra: &str,
        secret: &[u8],
    ) -> String {
        let claims = ledger.signed_launch_claims();
        token(
            &format!(
                r#"{{"id":17,"score":{score},"ple_nonce":"{}","ple_binding":"{}"{extra}}}"#,
                claims.nonce(),
                claims.binding_digest(),
            ),
            secret,
        )
    }

    #[test]
    fn profile_refuses_generic_hosted_and_requires_frozen_revalidated_execution() {
        assert!(
            !ScoredEmbedProfileConfig::generic_myopenmath_hosted("myopenmath_hosted")
                .allows_published_server_grading()
        );
        assert!(ScoredEmbedProfileConfig::contracted_self_hosted("provider", false, true).is_err());
        assert!(ScoredEmbedProfileConfig::contracted_self_hosted("provider", true, false).is_err());
    }

    #[test]
    fn provider_cache_entry_binds_render_input_without_attempt_identity() {
        assert_eq!(normalize_provider_seed(QuestionSeed::new(0)), 1);
        assert_eq!(normalize_provider_seed(QuestionSeed::new(9_998)), 9_999);
        assert_eq!(normalize_provider_seed(QuestionSeed::new(9_999)), 1);
        let ledger = ledger(20_000);
        assert_eq!(ledger.ple_seed(), QuestionSeed::new(10_001));
        assert_eq!(ledger.provider_seed(), 3);
        let entry = ledger.external_question_provider_cache_entry();
        let debug = format!("{entry:?}");
        assert!(!debug.contains(&binding().attempt.to_string()));
        assert!(!debug.contains(&"a".repeat(64)));

        assert_ne!(
            entry,
            ledger_with(binding(), "b".repeat(64), [7; 32], 20_000)
                .external_question_provider_cache_entry()
        );
        assert_ne!(
            entry,
            ledger_with(binding(), "a".repeat(64), [7; 32], 20_001)
                .external_question_provider_cache_entry()
        );
    }

    #[test]
    fn valid_signed_result_consumes_exact_ledger_and_discards_answer_fields() {
        let secret = b"provider-secret";
        let verifier = ScoredEmbedResultVerifier::new(profile(), secret).unwrap();
        let mut ledger = ledger(20_000);
        let first_result = bound_token(
            &ledger,
            0.75,
            r#", "raw":"student answer","allans":["answer"],"redisplay":{"answer":"x"},"errors":"details""#,
            secret,
        );
        let result = verifier
            .verify_result(
                &mut ledger,
                &first_result,
                ActivityTimestamp::from_unix_millis(10_000),
            )
            .unwrap();
        assert_eq!(result.result.points_earned, 0.75);
        assert!(!result.result.correct);
        let replay = bound_token(&ledger, 1.0, "", secret);
        assert_eq!(
            verifier.verify_result(
                &mut ledger,
                &replay,
                ActivityTimestamp::from_unix_millis(10_000),
            ),
            Err(ScoredEmbedFailure::DuplicateResult)
        );
        assert!(!format!("{result:?}").contains("student answer"));
    }

    #[test]
    fn forged_wrong_expired_and_wrong_question_results_refuse() {
        let secret = b"provider-secret";
        let verifier = ScoredEmbedResultVerifier::new(profile(), secret).unwrap();
        let expired = ledger(20_000);
        let valid = bound_token(&expired, 1.0, ",\"exp\":11", secret);
        assert_eq!(
            verifier.verify_result(
                &mut ledger(20_000),
                &valid,
                ActivityTimestamp::from_unix_millis(12_000)
            ),
            Err(ScoredEmbedFailure::StaleLedger)
        );
        assert_eq!(
            verifier.verify_result(
                &mut ledger(20_000),
                &token(r#"{"id":"17","score":1.0}"#, secret),
                ActivityTimestamp::from_unix_millis(10_000),
            ),
            Err(ScoredEmbedFailure::MissingLaunchBinding)
        );
        assert_eq!(
            verifier.verify_result(
                &mut ledger(20_000),
                &token(
                    r#"{"id":"99","score":1.0,"ple_nonce":"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc","ple_binding":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
                    secret,
                ),
                ActivityTimestamp::from_unix_millis(10_000),
            ),
            Err(ScoredEmbedFailure::WrongQuestion)
        );
        assert_eq!(
            verifier.verify_result(
                &mut ledger(20_000),
                &bound_token(&ledger(20_000), 1.0, "", b"wrong-secret"),
                ActivityTimestamp::from_unix_millis(10_000),
            ),
            Err(ScoredEmbedFailure::InvalidSignature)
        );
    }

    #[test]
    fn signed_result_cannot_replay_across_same_item_ledgers() {
        let secret = b"provider-secret";
        let verifier = ScoredEmbedResultVerifier::new(profile(), secret).unwrap();
        let mut exact = ledger(20_000);
        let result = bound_token(&exact, 1.0, "", secret);
        assert!(
            verifier
                .verify_result(
                    &mut exact,
                    &result,
                    ActivityTimestamp::from_unix_millis(10_000)
                )
                .is_ok()
        );

        let base = binding();
        for changed in [
            GradeBinding {
                question_revision: QuestionRevisionReference {
                    question_id: base.question_revision.question_id.clone(),
                    revision_number: QuestionRevisionNumber::new(44).expect("positive version"),
                },
                ..base.clone()
            },
            GradeBinding {
                seed: QuestionSeed::new(10_002),
                ..base.clone()
            },
        ] {
            let mut other = ledger_with(changed, "a".repeat(64), [8; 32], 20_000);
            assert_eq!(
                verifier.verify_result(
                    &mut other,
                    &result,
                    ActivityTimestamp::from_unix_millis(10_000)
                ),
                Err(ScoredEmbedFailure::WrongLaunchBinding)
            );
        }
        let mut source_changed = ledger_with(base, "b".repeat(64), [9; 32], 20_000);
        assert_eq!(
            verifier.verify_result(
                &mut source_changed,
                &result,
                ActivityTimestamp::from_unix_millis(10_000)
            ),
            Err(ScoredEmbedFailure::WrongLaunchBinding)
        );
    }

    #[test]
    fn recorded_fixture_is_redacted_and_never_a_grade_transport() {
        let fixture = include_str!("../tests/fixtures/scored_embed_recorded_redacted.json");
        for forbidden in ["provider-secret", "student answer", "raw-answer", "eyJ"] {
            assert!(!fixture.contains(forbidden));
        }
        let json: serde_json::Value = serde_json::from_str(fixture).unwrap();
        assert_eq!(json["profile"], SCORED_EMBED_BROKER_PROFILE_ID);
        assert_eq!(json["expected"]["browserMessageGrades"], false);
        assert_eq!(json["expected"]["crossLedgerReplayRejected"], true);
        assert_eq!(json["launch"]["providerSeed"], 3);
    }
}
