//! Server-held grade bindings, correlation handles, and verified verdicts.

use sha2::{Digest, Sha256};

use objects::ObjectStoreError;
use question_model::generation::Seed;
use question_model::{
    GradingResult, QuestionAttemptId, QuestionTitleError, QuestionVersionReference,
};

use crate::cache::{binding_payload, constant_time_eq, hex};

/// Opaque server broker correlation. It is deliberately neither serializable
/// nor constructible from browser data. The server stores the opaque encoding
/// returned by [`CorrelationIssuer::begin`] and restores it only after MAC
/// validation.
#[derive(Clone, PartialEq, Eq)]
pub struct ServerCorrelation(pub(crate) String);

/// Adapter-owned server-only issuer for persisted grade correlation handles.
///
/// The secret comes from protected server configuration; its byte array is not
/// an HTTP shape and no browser input can construct an accepted restoration.
pub struct CorrelationIssuer {
    secret: [u8; 32],
}

impl CorrelationIssuer {
    /// Installs protected deployment configuration in the server composition.
    pub fn from_server_secret(secret: [u8; 32]) -> Self {
        Self { secret }
    }

    /// Begins a broker exchange after server-side authentication and attempt
    /// idempotency selection. The returned handle is safe to persist, not send
    /// to a browser or provider.
    pub fn begin(&self, binding: GradeBinding) -> PersistedCorrelation {
        let payload = binding_payload(&binding);
        let mac = self.mac(&payload);
        PersistedCorrelation(format!("{}.{}", hex(&payload), hex(&mac)))
    }

    /// Restores a previously persisted correlation, refusing altered, stale,
    /// wrong-binding, or non-canonical values before any provider call.
    pub fn restore(
        &self,
        binding: GradeBinding,
        persisted: &PersistedCorrelation,
    ) -> Result<ServerCorrelation, ImathasAdapterError> {
        let expected = self.begin(binding);
        if !constant_time_eq(expected.0.as_bytes(), persisted.0.as_bytes()) {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(ServerCorrelation(expected.0))
    }

    fn mac(&self, payload: &[u8]) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"ple:imathas:broker-correlation:v1");
        digest.update(self.secret);
        digest.update(payload);
        digest.finalize().into()
    }
}

/// Exact server-owned grade identity persisted alongside its idempotency row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradeBinding {
    pub attempt: QuestionAttemptId,
    pub question_version: QuestionVersionReference,
    pub seed: Seed,
}

/// Opaque database-persistable correlation encoding. It has no serde impl and
/// its inner string is inaccessible to clients.
#[derive(Clone, PartialEq, Eq)]
pub struct PersistedCorrelation(String);

impl PersistedCorrelation {
    /// Returns the bounded opaque value the protected broker row may store.
    /// This is intentionally not serde: callers must opt into this protected
    /// storage boundary rather than accidentally placing it in an HTTP DTO.
    pub fn to_storage_value(&self) -> String {
        self.0.clone()
    }

    /// Rehydrates an opaque value read from protected storage. This performs
    /// only canonical bounded syntax validation; callers must still call
    /// [`CorrelationIssuer::restore`] to validate the issuer MAC and exact
    /// attempt binding before a provider request.
    pub fn from_storage_value(value: &str) -> Result<Self, ImathasAdapterError> {
        const PAYLOAD_HEX_LEN: usize = (16 + 8 + 4 + 8) * 2;
        const MAC_HEX_LEN: usize = 32 * 2;
        const ENCODED_LEN: usize = PAYLOAD_HEX_LEN + 1 + MAC_HEX_LEN;
        if value.len() != ENCODED_LEN {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        let Some((payload, mac)) = value.split_once('.') else {
            return Err(ImathasAdapterError::InvalidCorrelation);
        };
        if payload.len() != PAYLOAD_HEX_LEN
            || mac.len() != MAC_HEX_LEN
            || !payload
                .bytes()
                .chain(mac.bytes())
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(ImathasAdapterError::InvalidCorrelation);
        }
        Ok(Self(value.to_owned()))
    }
}

impl std::fmt::Debug for PersistedCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersistedCorrelation(REDACTED)")
    }
}

impl std::fmt::Debug for ServerCorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServerCorrelation(REDACTED)")
    }
}

/// Authenticated provider verdict. The fields are private and this type has no
/// serde implementation, so an HTTP/browser payload cannot deserialize into it.
#[derive(Clone, PartialEq)]
pub struct VerifiedProviderGrade {
    pub(crate) result: GradingResult,
    pub(crate) attempt: QuestionAttemptId,
    pub(crate) question_version: QuestionVersionReference,
    pub(crate) seed: Seed,
    pub(crate) correlation: String,
}

impl std::fmt::Debug for VerifiedProviderGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedProviderGrade")
            .field("result", &self.result)
            .field("attempt", &self.attempt)
            .field("question_version", &self.question_version)
            .field("seed", &self.seed)
            .field("correlation", &"REDACTED")
            .finish()
    }
}

impl VerifiedProviderGrade {
    /// Server-only verified result; this type is non-serde and can only be
    /// obtained from the sealed contracted verifier.
    pub fn result(&self) -> GradingResult {
        self.result
    }

    /// Exact identity authenticated by the provider verifier.
    pub fn binding(&self) -> GradeBinding {
        GradeBinding {
            attempt: self.attempt,
            question_version: self.question_version.clone(),
            seed: self.seed,
        }
    }

    /// Provider implementations use this only after their signature/audience/
    /// expiry/nonce verification succeeds.
    #[cfg(test)]
    pub(crate) fn verified(
        result: GradingResult,
        attempt: QuestionAttemptId,
        question_version: QuestionVersionReference,
        seed: Seed,
        correlation: &ServerCorrelation,
    ) -> Self {
        Self {
            result,
            attempt,
            question_version,
            seed,
            correlation: correlation.0.clone(),
        }
    }

    /// The scored-embed verifier is the only production constructor. Its
    /// result token has already passed signature, expiry, exact question, and
    /// single-use server-ledger checks before this sealed grade exists.
    pub(crate) fn from_scored_embed(
        result: GradingResult,
        binding: GradeBinding,
        correlation: &ServerCorrelation,
    ) -> Self {
        Self {
            result,
            attempt: binding.attempt,
            question_version: binding.question_version,
            seed: binding.seed,
            correlation: correlation.0.clone(),
        }
    }
}

/// Provider-local failures. They are deliberately classified as unavailable or
/// invalid rather than a student correctness decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderFailure {
    Unavailable,
    Timeout,
    UnsupportedProfile,
    Authentication,
    Correlation,
    InvalidResponse,
}

/// Adapter failures suitable for a backend-local retry/degraded state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImathasAdapterError {
    UnsupportedSource,
    InvalidDraft,
    UnsupportedProfile,
    SourceChecksumMismatch,
    UntrustedSource,
    SourceDoesNotMatchQuestion,
    InvalidCache,
    InvalidProviderRender,
    InvalidTitle(QuestionTitleError),
    InvalidCorrelation,
    VerificationRefused,
    Provider(ProviderFailure),
    ObjectStore(ObjectStoreError),
}

impl std::fmt::Display for ImathasAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSource => f.write_str("question source is not iMathAS"),
            Self::InvalidDraft => f.write_str("invalid private iMathAS draft locator"),
            Self::UnsupportedProfile => f.write_str("unsupported iMathAS integration profile"),
            Self::SourceChecksumMismatch => f.write_str("iMathAS snapshot checksum mismatch"),
            Self::UntrustedSource => {
                f.write_str("iMathAS source was not resolved through its immutable object")
            }
            Self::SourceDoesNotMatchQuestion => {
                f.write_str("iMathAS source does not match its published question")
            }
            Self::InvalidCache => f.write_str("invalid iMathAS render cache"),
            Self::InvalidProviderRender => f.write_str("invalid iMathAS provider render"),
            Self::InvalidTitle(error) => write!(f, "invalid iMathAS question title: {error}"),
            Self::InvalidCorrelation => f.write_str("invalid server-held iMathAS correlation"),
            Self::VerificationRefused => {
                f.write_str("iMathAS verified grade did not match its server-held binding")
            }
            Self::Provider(_) => f.write_str("iMathAS provider unavailable or rejected request"),
            Self::ObjectStore(value) => value.fmt(f),
        }
    }
}

impl std::error::Error for ImathasAdapterError {}
