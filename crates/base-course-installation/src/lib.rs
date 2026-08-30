//! Deterministic installation of the ordinary live-demo Base Course.
//!
//! The caller owns PostgreSQL configuration and migrations. This crate borrows
//! an LDA pool and configured Store, then returns one owned serializable result.

mod accounts;
mod activity;
mod error;
mod installation;
mod publication;
mod receipt;
mod records;

pub use error::BaseCourseInstallError;
pub use installation::install;
pub use receipt::{
    BaseCourseAction, BaseCourseInstallOutput, BaseCourseInstallStateOutput, BaseCourseManifest,
};

use std::collections::BTreeSet;

use question_model::AccountId;

/// Narrow host capability used to turn the deterministic Mary response into
/// ordinary accepted Student work.  The installer owns the recipe, while the
/// host owns the server application and execution composition.
#[async_trait::async_trait]
pub trait AcceptedSubmissionSeedExecutor: Send + Sync {
    async fn execute_seed_submission(
        &self,
        request: AcceptedSubmissionSeedRequest,
    ) -> Result<AcceptedSubmissionSeedOutcome, learning_data_access::StoreError>;
}

/// Server-private deterministic input for one installed completed attempt.
#[derive(Clone)]
pub struct AcceptedSubmissionSeedRequest {
    /// The deterministic Student Account whose enrollment owns this work.
    ///
    /// The host validates the Account's exact membership and enrollment chain
    /// before accepting the response.
    pub student_account: AccountId,
    pub binding: learning_data_access::StudentWorkRoutingBinding,
    pub attempt: question_model::QuestionAttemptId,
    pub response: question_model::StudentResponse,
    pub idempotency_key: learning_data_access::SubmissionIdempotencyKey,
}

impl std::fmt::Debug for AcceptedSubmissionSeedRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AcceptedSubmissionSeedRequest")
            .field("student_account", &self.student_account)
            .field("binding", &self.binding)
            .field("attempt", &self.attempt)
            .field("response", &"[SERVER-ONLY]")
            .field("idempotency_key", &"[REDACTED]")
            .finish()
    }
}

/// Answer-free result of submitting one deterministic recipe response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptedSubmissionSeedOutcome {
    Completed,
    PendingRecovery,
}

/// Validated identities used by the deterministic Base Course recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseCourseParticipants {
    primary_instructor: AccountId,
    mary: AccountId,
    jack: AccountId,
    approval_candidate: AccountId,
    sysadmin: AccountId,
}

impl BaseCourseParticipants {
    /// Validates that the five ordinary accounts have distinct identities.
    ///
    /// # Errors
    ///
    /// Returns [`BaseCourseInstallError::Request`] when any participant identity repeats.
    pub fn try_new(
        primary_instructor: AccountId,
        mary: AccountId,
        jack: AccountId,
        approval_candidate: AccountId,
        sysadmin: AccountId,
    ) -> Result<Self, BaseCourseInstallError> {
        let distinct = [primary_instructor, mary, jack, approval_candidate, sysadmin]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if distinct.len() != 5 {
            return Err(BaseCourseInstallError::request(
                "the primary Instructor, Mary, Jack, approval candidate, and Sysadmin must identify five distinct accounts",
            ));
        }
        Ok(Self {
            primary_instructor,
            mary,
            jack,
            approval_candidate,
            sysadmin,
        })
    }

    pub(crate) fn primary_instructor(self) -> AccountId {
        self.primary_instructor
    }

    pub(crate) fn mary(self) -> AccountId {
        self.mary
    }

    pub(crate) fn jack(self) -> AccountId {
        self.jack
    }

    pub(crate) fn approval_candidate(self) -> AccountId {
        self.approval_candidate
    }

    pub(crate) fn sysadmin(self) -> AccountId {
        self.sysadmin
    }
}

/// The only two installation calls. Only `Install` can carry receipt bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseCourseInstallPhase {
    /// Claim or resume the generation and return its canonical storage receipt.
    Prepare,
    /// Verify the exact prepared receipt, converge records, and complete installation.
    Install { storage_receipt_json: String },
}

/// One validated installation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCourseInstallRequest {
    participants: BaseCourseParticipants,
    phase: BaseCourseInstallPhase,
}

impl BaseCourseInstallRequest {
    /// Combines validated participants with a closed installation phase.
    pub fn new(participants: BaseCourseParticipants, phase: BaseCourseInstallPhase) -> Self {
        Self {
            participants,
            phase,
        }
    }

    pub(crate) fn into_parts(self) -> (BaseCourseParticipants, BaseCourseInstallPhase) {
        (self.participants, self.phase)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(value))
    }

    #[test]
    fn participants_require_five_distinct_account_identities() {
        let valid = BaseCourseParticipants::try_new(account(2), account(3), account(4), account(5), account(6));
        let duplicate_mary =
            BaseCourseParticipants::try_new(account(2), account(2), account(4), account(5), account(6));
        let duplicate_sysadmin =
            BaseCourseParticipants::try_new(account(2), account(3), account(4), account(5), account(5));

        assert!(valid.is_ok());
        assert!(duplicate_mary.is_err());
        assert!(duplicate_sysadmin.is_err());
    }

    #[test]
    fn install_receipt_exists_only_in_the_install_phase() {
        let participants =
            BaseCourseParticipants::try_new(account(2), account(3), account(4), account(5), account(6)).unwrap();
        let prepare = BaseCourseInstallRequest::new(participants, BaseCourseInstallPhase::Prepare);
        let install = BaseCourseInstallRequest::new(
            participants,
            BaseCourseInstallPhase::Install {
                storage_receipt_json: "receipt".to_string(),
            },
        );

        assert!(matches!(
            prepare.into_parts().1,
            BaseCourseInstallPhase::Prepare
        ));
        assert!(matches!(
            install.into_parts().1,
            BaseCourseInstallPhase::Install { storage_receipt_json } if storage_receipt_json == "receipt"
        ));
    }
}
