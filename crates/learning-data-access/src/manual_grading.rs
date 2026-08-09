//! Server-only Store contract for response-bearing manual evaluation.

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{
    ActivityTimestamp, QuestionAttemptId, ScoringGeneration, StudentResponse, TenantId, UserId,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use uuid::Uuid;

use crate::{StoreError, SubmissionIdempotencyKey, SubmissionRecord, TenantContext};

/// Trusted server request to persist a real learner response that requires an
/// instructor evaluation. It deliberately carries no numeric result.
#[derive(Clone, PartialEq)]
pub struct SubmitPendingManualQuestionAttemptCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub response: StudentResponse,
    pub idempotency_key: SubmissionIdempotencyKey,
}

impl std::fmt::Debug for SubmitPendingManualQuestionAttemptCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitPendingManualQuestionAttemptCommand")
            .field("actor", &self.actor)
            .field("attempt", &self.attempt)
            .field("response", &self.response)
            .field("idempotency_key", &self.idempotency_key)
            .finish()
    }
}

/// Stable idempotency identity for one authorized manual evaluation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManualGradeActionId(Uuid);

impl ManualGradeActionId {
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    pub fn generate() -> Result<Self, StoreError> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| {
            StoreError::Unavailable(format!(
                "manual grade action ID randomness unavailable: {error}"
            ))
        })?;
        Ok(Self(Uuid::from_bytes(bytes)))
    }
}

/// Positive optimistic-concurrency token for one current evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvaluationRevision(u64);

impl EvaluationRevision {
    pub const INITIAL: Self = Self(1);

    pub fn from_u64(value: u64) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0.checked_add(1).map(Self).ok_or(StoreError::Conflict)
    }
}

/// Exact normalized credit for the PostgreSQL `NUMERIC(16, 12)` boundary.
#[derive(Clone, PartialEq)]
pub struct ManualCredit {
    value: Decimal,
    canonical_decimal: String,
}

impl ManualCredit {
    pub fn new(value: Decimal) -> Result<Self, StoreError> {
        if value.scale() > 12 || !(Decimal::from(-1_000)..=Decimal::from(1_000)).contains(&value) {
            return Err(StoreError::InvalidRecord(
                "manual credit must have at most 12 fractional digits and be between -1000 and 1000"
                    .to_string(),
            ));
        }
        let value = value.normalize();
        Ok(Self {
            value,
            canonical_decimal: value.to_string(),
        })
    }

    pub fn parse(value: &str) -> Result<Self, StoreError> {
        Decimal::from_str(value)
            .map_err(|_| StoreError::InvalidRecord("manual credit must be a decimal".to_string()))
            .and_then(Self::new)
    }

    pub fn as_decimal(&self) -> Decimal {
        self.value
    }

    pub(crate) fn try_as_f64(&self) -> Result<f64, StoreError> {
        self.value
            .to_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                StoreError::InvalidRecord(
                    "manual credit cannot be represented by the current result projection"
                        .to_string(),
                )
            })
    }

    pub fn as_canonical_decimal(&self) -> &str {
        &self.canonical_decimal
    }
}

impl std::fmt::Debug for ManualCredit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ManualCredit")
            .field(&self.canonical_decimal)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualEvaluationStatus {
    NeedsManualGrading,
    Graded,
}

/// One mutable current manual evaluation, never a grade-history entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ManualEvaluationRecord {
    pub tenant: TenantId,
    pub attempt: QuestionAttemptId,
    pub revision: EvaluationRevision,
    pub status: ManualEvaluationStatus,
    pub credit: Option<ManualCredit>,
    pub evaluated_at: ActivityTimestamp,
}

#[derive(Clone, PartialEq)]
pub struct SetManualGradeCommand {
    pub action: ManualGradeActionId,
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
    pub expected_revision: EvaluationRevision,
    pub credit: ManualCredit,
}

impl std::fmt::Debug for SetManualGradeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetManualGradeCommand")
            .field("action", &self.action)
            .field("actor", &self.actor)
            .field("attempt", &self.attempt)
            .field("expected_revision", &self.expected_revision)
            .field("credit", &"[redacted]")
            .finish()
    }
}

/// Minimal replay-safe receipt for one manual action. It deliberately carries
/// no credit, result, response, rubric, or prior evaluation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualGradeReceipt {
    pub action: ManualGradeActionId,
    pub attempt: QuestionAttemptId,
    pub resulting_revision: EvaluationRevision,
    pub scoring_generation: ScoringGeneration,
    pub occurred_at: ActivityTimestamp,
}

pub(crate) fn request_digest(command: &SetManualGradeCommand) -> Sha256Digest {
    let mut bytes = Vec::with_capacity(16 + 16 + 8 + command.credit.as_canonical_decimal().len());
    bytes.extend_from_slice(command.actor.as_uuid().as_bytes());
    bytes.extend_from_slice(command.attempt.as_uuid().as_bytes());
    bytes.extend_from_slice(&command.expected_revision.as_u64().to_be_bytes());
    bytes.extend_from_slice(command.credit.as_canonical_decimal().as_bytes());
    Sha256Digest::compute(&bytes)
}

/// Server-only persistence boundary for response-bearing manual evaluation.
#[async_trait]
pub trait ManualGradingStore: Send + Sync {
    async fn submit_pending_manual_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitPendingManualQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError>;

    async fn get_manual_evaluation_for_edit(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
    ) -> Result<Option<ManualEvaluationRecord>, StoreError>;

    async fn set_manual_grade(
        &self,
        context: TenantContext,
        command: SetManualGradeCommand,
    ) -> Result<ManualGradeReceipt, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::ManualCredit;

    #[test]
    fn manual_credit_parses_and_canonicalizes_decimal_input() {
        let negative_zero = ManualCredit::parse("-0.000").expect("negative zero is valid");
        assert_eq!(negative_zero.as_canonical_decimal(), "0");
        let trimmed = ManualCredit::parse("1.230000000000").expect("bounded decimal is valid");
        assert_eq!(trimmed.as_canonical_decimal(), "1.23");
        assert_eq!(trimmed.as_decimal().to_string(), "1.23");
    }

    #[test]
    fn manual_credit_rejects_out_of_range_or_overprecise_input() {
        assert!(ManualCredit::parse("1000.000000000001").is_err());
        assert!(ManualCredit::parse("1000.0000000000001").is_err());
        assert!(ManualCredit::parse("not-a-credit").is_err());
    }
}
