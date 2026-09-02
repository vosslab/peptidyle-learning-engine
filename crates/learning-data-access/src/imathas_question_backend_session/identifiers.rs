use objects::Sha256Checksum;
use question_model::generation::QuestionSeed;
use question_model::{QuestionAttemptId, QuestionGradingRule, QuestionRevisionReference};
use uuid::Uuid;

use crate::StoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImathasQuestionBackendSessionReference(Uuid);

impl ImathasQuestionBackendSessionReference {
    pub fn generate() -> Result<Self, StoreError> {
        crate::random_uuid::random_uuid_v4(|_| {
            StoreError::Unavailable(
                "iMathAS Question Backend Session Reference randomness unavailable".into(),
            )
        })
        .map(Self)
    }

    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImathasResponseChecksum(Sha256Checksum);

/// Opaque iMathAS-signed bytes received only at the server/adapter boundary.
///
/// The token remains in memory only while the iMathAS adapter verifies it. It
/// has no serialization or durable representation; the verified Exchange
/// stores only its checksum.
#[derive(Clone, PartialEq, Eq)]
pub struct ImathasResultToken(Vec<u8>);

impl ImathasResultToken {
    pub const MAX_BYTES: usize = 8_192;

    /// Validates raw bytes received from a server-to-server iMathAS result.
    // ASVS 2.2.1: bound untrusted iMathAS input before verification or hashing.
    pub fn from_server_adapter_bytes(value: Vec<u8>) -> Result<Self, StoreError> {
        if value.is_empty() || value.len() > Self::MAX_BYTES {
            return Err(StoreError::InvalidRecord(
                "iMathAS Result Token must contain 1 through 8192 bytes".into(),
            ));
        }
        Ok(Self(value))
    }

    /// Gives the server-side iMathAS adapter the exact opaque bytes to verify.
    pub fn as_server_adapter_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for ImathasResultToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasResultToken")
            .field("bytes", &"[redacted]")
            .field("len", &self.0.len())
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImathasResultTokenChecksum(Sha256Checksum);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImathasResultChecksum(Sha256Checksum);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AutomatedGradingReceiptChecksum(Sha256Checksum);

macro_rules! secret_checksum {
    ($type:ident, $label:literal) => {
        impl $type {
            pub fn from_bytes(value: [u8; 32]) -> Self {
                Self(Sha256Checksum::from_bytes(value))
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                self.0.as_bytes()
            }
        }

        impl std::fmt::Debug for $type {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(concat!($label, "([redacted])"))
            }
        }
    };
}

secret_checksum!(ImathasResponseChecksum, "ImathasResponseChecksum");
secret_checksum!(ImathasResultChecksum, "ImathasResultChecksum");
secret_checksum!(
    AutomatedGradingReceiptChecksum,
    "AutomatedGradingReceiptChecksum"
);

/// One finite score produced by the contracted iMathAS profile.
///
/// The value is deliberately non-Serde: imathas_question_backend score evidence is accepted
/// only at the server boundary and is never a browser transport fact.
#[derive(Clone, Copy, PartialEq)]
pub struct ImathasNormalizedScore(f64);

impl ImathasNormalizedScore {
    pub fn try_from_f64(value: f64) -> Result<Self, StoreError> {
        if !value.is_finite()
            || !(0.0..=1.0).contains(&value)
            || value.to_bits() == (-0.0_f64).to_bits()
        {
            return Err(StoreError::InvalidRecord(
                "iMathAS normalized score must be finite, nonnegative zero, and within 0 through 1"
                    .into(),
            ));
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

impl std::fmt::Debug for ImathasNormalizedScore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasNormalizedScore([redacted])")
    }
}

/// The fixed first-profile iMathAS Result shape.
#[derive(Clone, PartialEq)]
pub struct ImathasResult {
    normalized_score: ImathasNormalizedScore,
}

impl ImathasResult {
    pub const fn new(normalized_score: ImathasNormalizedScore) -> Self {
        Self { normalized_score }
    }

    pub const fn normalized_score(&self) -> ImathasNormalizedScore {
        self.normalized_score
    }

    pub fn checksum(&self) -> ImathasResultChecksum {
        let mut bytes = b"ple:imathas-result:v1\0".to_vec();
        bytes.extend_from_slice(&self.normalized_score.value().to_bits().to_be_bytes());
        ImathasResultChecksum(Sha256Checksum::compute(&bytes))
    }
}

impl std::fmt::Debug for ImathasResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasResult([redacted])")
    }
}

impl ImathasResultTokenChecksum {
    /// Derives the receipt checksum after the adapter has accepted every
    /// imathas_question_backend protocol claim for this exact token.
    // ASVS 2.3.1: the verified Exchange transition accepts evidence only after
    // the imathas_question_backend-verification step completes.
    pub fn from_verified_token(token: &ImathasResultToken) -> Self {
        Self(Sha256Checksum::compute(token.as_server_adapter_bytes()))
    }

    /// Reconstitutes the durable Exchange receipt from trusted private storage.
    #[allow(dead_code)] // Used when a Store reads verified Exchange receipts.
    pub(crate) fn from_storage_bytes(value: [u8; 32]) -> Self {
        Self(Sha256Checksum::from_bytes(value))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl std::fmt::Debug for ImathasResultTokenChecksum {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasResultTokenChecksum([redacted])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImathasLaunchBindingChecksum(String);

impl ImathasLaunchBindingChecksum {
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(StoreError::InvalidRecord(
                "iMathAS Launch Binding Checksum is invalid".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ImathasLaunchBindingChecksum {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasLaunchBindingChecksum([redacted])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImathasQuestionBackendSessionChallenge([u8; 32]);

impl ImathasQuestionBackendSessionChallenge {
    /// Mints a fresh, nonzero server-owned challenge.
    pub fn generate() -> Result<Self, StoreError> {
        loop {
            let mut value = [0_u8; 32];
            // ASVS 11.2.1 and 11.5.1: obtain challenge bytes from the OS CSPRNG.
            getrandom::fill(&mut value).map_err(|_| {
                StoreError::Unavailable("iMathAS Session Challenge randomness unavailable".into())
            })?;
            if let Ok(challenge) = Self::from_storage_bytes(value) {
                return Ok(challenge);
            }
        }
    }

    /// Reconstitutes a validated challenge read from trusted private storage.
    pub(crate) fn from_storage_bytes(value: [u8; 32]) -> Result<Self, StoreError> {
        // ASVS 2.2.1: reject the storage representation that violates the
        // server-owned challenge invariant before it enters the session model.
        if value.iter().all(|byte| *byte == 0) {
            return Err(StoreError::InvalidRecord(
                "iMathAS Session Challenge must not be all zero".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for ImathasQuestionBackendSessionChallenge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendSessionChallenge([redacted])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImathasQuestionBackendSessionAuthentication(String);

impl ImathasQuestionBackendSessionAuthentication {
    pub fn from_server_value(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        let Some((payload, mac)) = value.split_once('.') else {
            return Err(invalid_authentication());
        };
        if value.len() < 3
            || value.len() > 512
            || payload.len() % 2 != 0
            || mac.len() != 64
            || !payload
                .bytes()
                .chain(mac.bytes())
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(invalid_authentication());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn invalid_authentication() -> StoreError {
    StoreError::InvalidRecord("iMathAS Session Authentication is invalid".into())
}

impl std::fmt::Debug for ImathasQuestionBackendSessionAuthentication {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendSessionAuthentication([redacted])")
    }
}

/// The exact attempt, Question Revision, and Question Seed that one iMathAS
/// Question Backend Session grades.
///
/// This aggregate is server-only. Its three members travel together through
/// persistence, restoration, and imathas_question_backend verification so a caller cannot
/// accidentally bind a imathas_question_backend operation to facts from different Question
/// Attempts.
#[derive(Clone, PartialEq, Eq)]
pub struct ImathasGradingContext {
    question_attempt: QuestionAttemptId,
    question_revision: QuestionRevisionReference,
    pub(crate) question_seed: QuestionSeed,
}

impl ImathasGradingContext {
    pub fn new(
        question_attempt: QuestionAttemptId,
        question_revision: QuestionRevisionReference,
        question_seed: QuestionSeed,
    ) -> Self {
        Self {
            question_attempt,
            question_revision,
            question_seed,
        }
    }

    pub fn question_attempt(&self) -> QuestionAttemptId {
        self.question_attempt
    }

    pub fn question_revision(&self) -> &QuestionRevisionReference {
        &self.question_revision
    }

    pub fn question_seed(&self) -> QuestionSeed {
        self.question_seed
    }

    /// Returns the locked row-530 HMAC payload for this exact context.
    ///
    /// Version 1 is exactly: Question Attempt UUID bytes, canonical unprefixed
    /// Question ID UTF-8 bytes, Question Revision Number big-endian bytes, and
    /// Question Seed big-endian bytes. The canonical Question ID display has
    /// eight UTF-8 bytes (for example, `123-4567`) and is intentionally not
    /// length-prefixed. Keep this byte sequence stable while row-530
    /// authentication records exist.
    pub fn authentication_payload_v1(&self) -> Vec<u8> {
        let question_id = self.question_revision.question_id.to_string();
        let mut payload = Vec::with_capacity(16 + question_id.len() + 4 + 8);
        payload.extend_from_slice(self.question_attempt.as_uuid().as_bytes());
        payload.extend_from_slice(question_id.as_bytes());
        payload.extend_from_slice(&self.question_revision.revision_number.get().to_be_bytes());
        payload.extend_from_slice(&self.question_seed.value().to_be_bytes());
        payload
    }
}

/// Derives PLE's immutable grading fact from accepted iMathAS evidence.
pub fn derive_imathas_question_backend_grading_result(
    imathas_result: &ImathasResult,
    question_grading_rule: &QuestionGradingRule,
) -> Result<question_model::GradingResult, StoreError> {
    let normalized_score = imathas_result.normalized_score().value();
    match question_grading_rule {
        QuestionGradingRule::AllOrNothing { points } => Ok(question_model::GradingResult {
            correct: normalized_score == 1.0,
            points_earned: if normalized_score == 1.0 {
                *points
            } else {
                0.0
            },
            points_possible: *points,
        }),
        QuestionGradingRule::PartialCredit { points } => Ok(question_model::GradingResult {
            correct: normalized_score == 1.0,
            points_earned: points * normalized_score,
            points_possible: *points,
        }),
        QuestionGradingRule::Ungraded => Err(StoreError::InvalidRecord(
            "Ungraded Question Grading Rule cannot accept an iMathAS Result".into(),
        )),
    }
}

pub(crate) fn validate_question_grading_rule(rule: &QuestionGradingRule) -> Result<(), StoreError> {
    let points = match rule {
        QuestionGradingRule::AllOrNothing { points }
        | QuestionGradingRule::PartialCredit { points } => *points,
        QuestionGradingRule::Ungraded => return Ok(()),
    };
    if !points.is_finite() || points < 0.0 || points.to_bits() == (-0.0_f64).to_bits() {
        return Err(StoreError::InvalidRecord(
            "Question Grading Rule points must be finite, nonnegative zero, and nonnegative".into(),
        ));
    }
    Ok(())
}

impl std::fmt::Debug for ImathasGradingContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasGradingContext([redacted])")
    }
}
