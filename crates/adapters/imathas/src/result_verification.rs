//! The first concrete, deliberately narrow iMathAS grading profile.
//!
//! This module verifies a score only after the server-side iMathAS Question Backend has matched
//! it to an exact, single-use iMathAS Question Backend Session. The iMathAS
//! grading profile carries the signed iMathAS Session Challenge and Qualified Launch Binding
//! Digest. The unextended upstream iMathAS protocol omits PLE account, attempt,
//! version, and idempotency facts; consequently a valid JWT alone is never a
//! grade.

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::{QuestionRevisionReference, Timestamp};
use serde::Deserialize;
use serde::de::IgnoredAny;
use sha2::{Digest, Sha256};

use crate::{ImathasAdapterError, ImathasQuestionBackendFailure, VerifiedImathasResult};

/// The only currently supported server-graded iMathAS profile.
pub const IMATHAS_GRADING_PROFILE_ID: &str = "imathas_remote_grading_v1";
/// The bounded iMathAS seed range documented by iMathAS.
pub const IMATHAS_SEED_VARIANTS: u64 = 9_999;
const MAX_RESULT_TOKEN_BYTES: usize = 8_192;
const MAX_RESULT_BODY_BYTES: usize = 4_096;
const MAX_QUESTION_ID_BYTES: usize = 128;

/// Maps PLE's larger seed space into iMathAS's documented `1..=9999` range.
///
/// This is deterministic but not injective.  The resulting 9,999-variant
/// collision space is an explicit authoring and capability limitation, not a
/// claim that iMathAS can reproduce every PLE seed uniquely.
pub fn normalize_imathas_seed(ple_seed: question_model::generation::QuestionSeed) -> u16 {
    (ple_seed.value() % IMATHAS_SEED_VARIANTS + 1) as u16
}

/// Deployment confirmation required before this profile can grade a published
/// question.  It intentionally contains no host, URL, credential, or JWT.
#[derive(Clone, PartialEq, Eq)]
pub struct ImathasGradingProfile {
    deployment_reference: String,
    frozen_execution_target: bool,
    source_object_checksum_revalidation: bool,
    grading_enabled: bool,
}

impl std::fmt::Debug for ImathasGradingProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasGradingProfile")
            .field("deployment_reference", &self.deployment_reference)
            .field("frozen_execution_target", &self.frozen_execution_target)
            .field(
                "source_object_checksum_revalidation",
                &self.source_object_checksum_revalidation,
            )
            .field("grading_enabled", &self.grading_enabled)
            .finish()
    }
}

impl ImathasGradingProfile {
    /// Creates the profile only for an explicit iMathAS grading deployment
    /// where the operator guarantees a frozen execution target and
    /// revalidates its Source Object Checksum at every launch.
    pub fn grading_deployment(
        deployment_reference: impl Into<String>,
        frozen_execution_target: bool,
        source_object_checksum_revalidation: bool,
    ) -> Result<Self, ImathasGradingFailure> {
        let deployment_reference = deployment_reference.into();
        if !valid_deployment_reference(&deployment_reference)
            || !frozen_execution_target
            || !source_object_checksum_revalidation
        {
            return Err(ImathasGradingFailure::UnsupportedProfile);
        }
        Ok(Self {
            deployment_reference,
            frozen_execution_target,
            source_object_checksum_revalidation,
            grading_enabled: true,
        })
    }

    /// An unverified hosted MyOpenMath deployment has no confirmed immutable execution target
    /// or iMathAS Question Backend Session authentication contract, so grading is
    /// unavailable. A separate ungraded practice display is outside this module.
    pub fn unverified_myopenmath_hosted(deployment_reference: impl Into<String>) -> Self {
        Self {
            deployment_reference: deployment_reference.into(),
            frozen_execution_target: false,
            source_object_checksum_revalidation: false,
            grading_enabled: false,
        }
    }

    /// The opaque deployment selector used to bind this grading profile to one iMathAS deployment.
    pub fn deployment_reference(&self) -> &str {
        &self.deployment_reference
    }

    /// Whether this deployment may expose `serverGrading` for this profile.
    pub fn allows_grading(&self) -> bool {
        self.grading_enabled
            && self.frozen_execution_target
            && self.source_object_checksum_revalidation
    }
}

/// Redacted classification for an iMathAS grading refusal. No variant carries an
/// upstream token, response body, answer, source, URL, or secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImathasGradingFailure {
    UnsupportedProfile,
    InvalidLimits,
    InvalidSessionBinding,
    ExpiredSessionBinding,
    InvalidResult,
    InvalidSignature,
    MissingLaunchBinding,
    WrongLaunchBinding,
    WrongQuestion,
    InvalidScore,
    BackendUnavailable,
}

impl ImathasGradingFailure {
    /// Maps only transport-independent failures into the adapter's existing
    /// question-local retry/degraded classification.
    pub fn into_adapter_error(self) -> ImathasAdapterError {
        match self {
            Self::UnsupportedProfile => ImathasAdapterError::UnsupportedProfile,
            Self::InvalidLimits | Self::InvalidSessionBinding | Self::ExpiredSessionBinding => {
                ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication
            }
            Self::InvalidResult
            | Self::InvalidSignature
            | Self::MissingLaunchBinding
            | Self::WrongLaunchBinding
            | Self::WrongQuestion
            | Self::InvalidScore => ImathasAdapterError::VerificationRefused,
            Self::BackendUnavailable => {
                ImathasAdapterError::QuestionBackend(ImathasQuestionBackendFailure::Unavailable)
            }
        }
    }
}

/// Complete server-held identity for a cached iMathAS render.
///
/// The storage address belongs to the cache implementation and is deliberately
/// absent from this application record.
#[derive(Clone, PartialEq, Eq)]
pub struct ImathasRenderCacheEntry {
    question_revision: QuestionRevisionReference,
    imathas_seed: u16,
    profile: String,
    payload_digest: String,
    expires_at: Timestamp,
}

impl std::fmt::Debug for ImathasRenderCacheEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasRenderCacheEntry")
            .field("question_revision", &self.question_revision)
            .field("imathas_seed", &self.imathas_seed)
            .field("profile", &self.profile)
            .field("payload_digest", &"REDACTED")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl ImathasRenderCacheEntry {
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn new(
        question_revision: QuestionRevisionReference,
        imathas_seed: u16,
        profile: String,
        payload_digest: String,
        expires_at: Timestamp,
    ) -> Self {
        Self {
            question_revision,
            imathas_seed,
            profile,
            payload_digest,
            expires_at,
        }
    }
}

/// Server-private iMathAS Result Verifier. The credential is accepted at
/// composition time and has no getter or serializable representation.
pub struct ImathasResultVerifier {
    profile: ImathasGradingProfile,
    signing_secret: Vec<u8>,
}

/// Crate-local witness that this module completed iMathAS Result Verification
/// and exact launch-binding checks.
pub(crate) struct ImathasResultVerificationSeal(());

impl ImathasResultVerificationSeal {
    fn after_verified_result_verification() -> Self {
        Self(())
    }
}

impl std::fmt::Debug for ImathasResultVerifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasResultVerifier")
            .field("profile", &self.profile)
            .field("signing_secret", &"REDACTED")
            .finish()
    }
}

impl ImathasResultVerifier {
    /// Installs one iMathAS deployment's protected HS256 result-verification secret.
    pub fn new(
        profile: ImathasGradingProfile,
        signing_secret: impl AsRef<[u8]>,
    ) -> Result<Self, ImathasGradingFailure> {
        let signing_secret = signing_secret.as_ref();
        if !profile.allows_grading() || signing_secret.is_empty() {
            return Err(ImathasGradingFailure::UnsupportedProfile);
        }
        Ok(Self {
            profile,
            signing_secret: signing_secret.to_vec(),
        })
    }

    /// Verifies a bounded iMathAS response after an allowlisted server proxy
    /// receives it server-to-server.  Callers must never pass browser
    /// `postMessage` content here. On success it produces a verified iMathAS
    /// result; LDA Store consumption and durable replay/idempotency remain the
    /// caller's Session transaction.
    pub fn verify_result(
        &self,
        validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
        result_token: &learning_data_access::ImathasResultToken,
        now: Timestamp,
    ) -> Result<VerifiedImathasResult, ImathasGradingFailure> {
        if !self.profile.allows_grading()
            || validation
                .imathas_question_backend_binding
                .deployment_reference()
                .as_str()
                != self.profile.deployment_reference()
            || validation
                .imathas_question_backend_binding
                .profile()
                .as_str()
                != IMATHAS_GRADING_PROFILE_ID
        {
            return Err(ImathasGradingFailure::InvalidSessionBinding);
        }
        if now >= validation.expires_at {
            return Err(ImathasGradingFailure::ExpiredSessionBinding);
        }
        let result_token_text = std::str::from_utf8(result_token.as_server_adapter_bytes())
            .map_err(|_| ImathasGradingFailure::InvalidResult)?;
        let claims = verify_hs256(result_token_text, &self.signing_secret)?;
        if claims.question_id
            != validation
                .imathas_question_backend_binding
                .item_reference()
                .as_str()
        {
            return Err(ImathasGradingFailure::WrongQuestion);
        }
        let Some(challenge) = claims.challenge else {
            return Err(ImathasGradingFailure::MissingLaunchBinding);
        };
        let Some(binding_digest) = claims.binding_digest else {
            return Err(ImathasGradingFailure::MissingLaunchBinding);
        };
        let expected_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(validation.challenge.as_bytes());
        let expected_binding_digest = validation.qualified_launch_binding_digest.as_str();
        if !crate::constant_time_eq(challenge.as_bytes(), expected_challenge.as_bytes())
            || !crate::constant_time_eq(
                binding_digest.as_bytes(),
                expected_binding_digest.as_bytes(),
            )
        {
            return Err(ImathasGradingFailure::WrongLaunchBinding);
        }
        let normalized_score =
            learning_data_access::ImathasNormalizedScore::try_from_f64(claims.score)
                .map_err(|_| ImathasGradingFailure::InvalidScore)?;
        if let Some(exp) = claims.exp {
            let Some(expiry_ms) = exp.checked_mul(1_000) else {
                return Err(ImathasGradingFailure::InvalidResult);
            };
            if now.as_unix_millis() > expiry_ms {
                return Err(ImathasGradingFailure::ExpiredSessionBinding);
            }
        }
        // Upstream permits no exp. The mandatory, exact session expiry
        // above and its server-side receipt are therefore the fallback, not an
        // invented JWT guarantee.
        let imathas_result_token_checksum =
            learning_data_access::ImathasResultTokenChecksum::from_verified_token(result_token);
        Ok(VerifiedImathasResult::from_result_verification(
            ImathasResultVerificationSeal::after_verified_result_verification(),
            learning_data_access::ImathasResult::new(normalized_score),
            validation.grading_context.clone(),
            &validation.authentication,
            imathas_result_token_checksum,
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
    /// Required iMathAS grading profile extension: exact opaque iMathAS Session Challenge.
    #[serde(default, rename = "ple_launch_challenge")]
    challenge: Option<String>,
    /// Required iMathAS grading profile extension: signed exact iMathAS Session binding.
    #[serde(default, rename = "ple_binding")]
    binding_digest: Option<String>,
    // These official fields can contain response, correct-answer, or iMathAS
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
    challenge: Option<String>,
    binding_digest: Option<String>,
}

fn verify_hs256(token: &str, secret: &[u8]) -> Result<VerifiedClaims, ImathasGradingFailure> {
    if token.is_empty() || token.len() > MAX_RESULT_TOKEN_BYTES || !token.is_ascii() {
        return Err(ImathasGradingFailure::InvalidResult);
    }
    let mut sections = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) = (
        sections.next(),
        sections.next(),
        sections.next(),
        sections.next(),
    ) else {
        return Err(ImathasGradingFailure::InvalidResult);
    };
    if header.is_empty() || payload.is_empty() || signature.is_empty() {
        return Err(ImathasGradingFailure::InvalidResult);
    }
    let decode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_bytes = decode
        .decode(header)
        .map_err(|_| ImathasGradingFailure::InvalidResult)?;
    let payload_bytes = decode
        .decode(payload)
        .map_err(|_| ImathasGradingFailure::InvalidResult)?;
    if header_bytes.len() > MAX_RESULT_BODY_BYTES || payload_bytes.len() > MAX_RESULT_BODY_BYTES {
        return Err(ImathasGradingFailure::InvalidResult);
    }
    let parsed_header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| ImathasGradingFailure::InvalidResult)?;
    if parsed_header.alg != "HS256" || parsed_header.typ.as_deref().is_some_and(|typ| typ != "JWT")
    {
        return Err(ImathasGradingFailure::InvalidResult);
    }
    let signature = decode
        .decode(signature)
        .map_err(|_| ImathasGradingFailure::InvalidResult)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .map_err(|_| ImathasGradingFailure::InvalidSignature)?;
    mac.update(header.as_bytes());
    mac.update(b".");
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| ImathasGradingFailure::InvalidSignature)?;
    let claims: ResultClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| ImathasGradingFailure::InvalidResult)?;
    let ResultClaims {
        question_id,
        score,
        exp,
        challenge,
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
        return Err(ImathasGradingFailure::InvalidResult);
    }
    Ok(VerifiedClaims {
        question_id,
        score,
        exp,
        challenge: challenge.filter(|value| valid_launch_challenge(value)),
        binding_digest: binding_digest.filter(|value| valid_sha256(value)),
    })
}

pub(crate) fn valid_deployment_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(crate) fn valid_question_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_QUESTION_ID_BYTES
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

pub(crate) fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_launch_challenge(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(crate) fn launch_binding_digest(
    grading_context: &learning_data_access::ImathasGradingContext,
    imathas_item_reference: &str,
    source_object_checksum: &str,
    imathas_seed: u16,
    launch_session_authentication: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ple:imathas:imathas-question-backend-launch-binding:v1");
    digest.update(grading_context.question_attempt().as_uuid().as_bytes());
    digest.update(
        grading_context
            .question_revision()
            .question_id
            .to_string()
            .as_bytes(),
    );
    digest.update(
        grading_context
            .question_revision()
            .revision_number
            .get()
            .to_be_bytes(),
    );
    digest.update(grading_context.question_seed().value().to_be_bytes());
    digest.update(imathas_seed.to_be_bytes());
    digest.update(IMATHAS_GRADING_PROFILE_ID.as_bytes());
    digest.update(imathas_item_reference.as_bytes());
    digest.update(source_object_checksum.as_bytes());
    digest.update(launch_session_authentication.as_bytes());
    crate::hex(digest.finalize().as_slice())
}
