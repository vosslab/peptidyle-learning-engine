//! PostgreSQL persistence for server-only iMathAS Question Backend Sessions.

use async_trait::async_trait;
use question_model::generation::QuestionSeed;
use question_model::{AccountId, QuestionGradingRule, Timestamp};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};

use super::Pool;
use super::connection::map_sqlx_error;
use crate::imathas_question_backend_session::StagedImathasResultReceipt;
use crate::{
    AutomatedGradingReceipt, AutomatedGradingReceiptChecksum, CommitStagedImathasResultGrading,
    ImathasGradingJobLease, ImathasQuestionBackendSession,
    ImathasQuestionBackendSessionAuthentication, ImathasQuestionBackendSessionChallenge,
    ImathasQuestionBackendSessionCreate, ImathasQuestionBackendSessionLease,
    ImathasQuestionBackendSessionReference, ImathasQuestionBackendSessionRestoreExpectation,
    ImathasQuestionBackendSessionStorageParts, ImathasQuestionBackendSessionStore,
    ImathasQuestionBackendStateCipher, ImathasQuestionBackendStateKeyId,
    ImathasQuestionBackendStateKeyRing, ImathasResponseChecksum,
    LoadedImathasQuestionBackendSession, QualifiedLaunchBindingDigest, SessionTokenHash,
    StageVerifiedImathasResult, StoreError,
};

/// PostgreSQL implementation of the durable iMathAS Question Backend Session boundary.
#[derive(Clone)]
pub struct PostgresImathasQuestionBackendSessionStore {
    pool: Pool,
    key_ring: std::sync::Arc<ImathasQuestionBackendStateKeyRing>,
}

impl PostgresImathasQuestionBackendSessionStore {
    /// Binds an already-attested application pool and server-owned key ring.
    pub fn new(pool: Pool, key_ring: std::sync::Arc<ImathasQuestionBackendStateKeyRing>) -> Self {
        Self { pool, key_ring }
    }

    async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<(Transaction<'_, Postgres>, AccountId), StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT account_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let account = session
            .map(|row| row.try_get("account_id").map(AccountId::from_uuid))
            .transpose()
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Forbidden)?;
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok((transaction, account))
    }
}

#[async_trait]
impl ImathasQuestionBackendSessionStore for PostgresImathasQuestionBackendSessionStore {
    async fn create_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        create: ImathasQuestionBackendSessionCreate,
    ) -> Result<ImathasQuestionBackendSessionReference, StoreError> {
        let reference = ImathasQuestionBackendSessionReference::generate()?;
        let create_parts = create.into_storage_parts(reference);
        let session = create_parts.session;
        let imathas_question_backend_state = create_parts.imathas_question_backend_state;
        let parts = session.storage_parts();
        let (mut transaction, resolved_account) = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        ensure_resolved_create_account(resolved_account, parts.account)?;
        let cipher = ImathasQuestionBackendStateCipher::seal(
            &self.key_ring,
            &session,
            &imathas_question_backend_state,
        )?;
        let persisted: uuid::Uuid = sqlx::query_scalar(
            "SELECT ple_api.create_imathas_question_backend_session(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, decode($10, 'hex'), $11, $12::numeric, \
                $13, $14::numeric, $15, $16, $17, convert_to($18, 'UTF8'), \
                to_timestamp($19::double precision / 1000.0), to_timestamp($20::double precision / 1000.0), \
                $21, $22, $23)",
        )
        .bind(parts.reference.as_uuid())
        .bind(parts.course.as_uuid())
        .bind(parts.assignment.as_uuid())
        .bind(parts.grading_context.question_attempt().as_uuid())
        .bind(parts.imathas_question_backend_binding.deployment_reference().as_str())
        .bind(parts.imathas_question_backend_binding.item_reference().as_str())
        .bind(parts.grading_context.question_revision().question_id.to_string())
        .bind(i32::try_from(parts.grading_context.question_revision().revision_number.get()).map_err(|_| {
            StoreError::InvalidRecord("Question Revision number exceeds PostgreSQL integer range".into())
        })?)
        .bind(parts.source_object.object.as_uuid())
        .bind(parts.source_object_checksum.as_str())
        .bind(parts.imathas_question_backend_binding.profile().as_str())
        .bind(parts.grading_context.question_seed().value().to_string())
        .bind(question_grading_rule_mode(&parts.question_grading_rule))
        .bind(question_grading_rule_points(&parts.question_grading_rule))
        .bind(parts.qualified_launch_binding_digest.as_str())
        .bind(parts.response_checksum.as_bytes().to_vec())
        .bind(parts.challenge.as_bytes().to_vec())
        .bind(parts.authentication.as_str())
        .bind(parts.issued_at.as_unix_millis())
        .bind(parts.expires_at.as_unix_millis())
        .bind(cipher.key_id().as_str())
        .bind(cipher.nonce().to_vec())
        .bind(cipher.ciphertext())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if persisted != reference.as_uuid() {
            return Err(StoreError::Unavailable(
                "database returned an invalid iMathAS Question Backend Session reference".into(),
            ));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(reference)
    }

    async fn load_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: ImathasQuestionBackendSessionReference,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
    ) -> Result<LoadedImathasQuestionBackendSession, StoreError> {
        let (mut transaction, _) = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = load_row(&mut transaction, reference, &expectation).await?;
        let (session, cipher) =
            decode_imathas_question_backend_session_row(&row, reference, &expectation)?;
        let imathas_question_backend_state = cipher.open(&self.key_ring, &session)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(LoadedImathasQuestionBackendSession::from_storage_parts(
            session,
            imathas_question_backend_state,
        ))
    }

    async fn lease_imathas_question_backend_session(
        &self,
        session_token_hash: SessionTokenHash,
        reference: ImathasQuestionBackendSessionReference,
        expectation: ImathasQuestionBackendSessionRestoreExpectation,
        lease_expires_at: Timestamp,
    ) -> Result<ImathasQuestionBackendSessionLease, StoreError> {
        let mut capability = [0_u8; 32];
        getrandom::fill(&mut capability).map_err(|_| {
            StoreError::Unavailable(
                "iMathAS Question Backend Session lease randomness unavailable".into(),
            )
        })?;
        let lease = ImathasQuestionBackendSessionLease::from_server_capability(
            reference,
            capability,
            lease_expires_at,
            expectation,
        );
        let lease_parts = lease.storage_parts();
        let context = lease_parts.restore;
        let (mut transaction, _) = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        sqlx::query(
            "SELECT ple_api.lease_imathas_question_backend_session(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, decode($10, 'hex'), $11, \
                $12::numeric, $13, $14::numeric, $15, $16, to_timestamp($17::double precision / 1000.0))",
        )
        .bind(lease_parts.reference.as_uuid())
        .bind(context.course.as_uuid())
        .bind(context.assignment.as_uuid())
        .bind(context.grading_context.question_attempt().as_uuid())
        .bind(context.imathas_question_backend_binding.deployment_reference().as_str())
        .bind(context.imathas_question_backend_binding.item_reference().as_str())
        .bind(
            context
                .grading_context
                .question_revision()
                .question_id
                .to_string(),
        )
        .bind(
            i32::try_from(
                context
                    .grading_context
                    .question_revision()
                    .revision_number
                    .get(),
            )
            .map_err(|_| {
                StoreError::InvalidRecord(
                    "Question Revision number exceeds PostgreSQL integer range".into(),
                )
            })?,
        )
        .bind(context.source_object.object.as_uuid())
        .bind(context.source_object_checksum.as_str())
        .bind(context.imathas_question_backend_binding.profile().as_str())
        .bind(context.grading_context.question_seed().value().to_string())
        .bind(question_grading_rule_mode(&context.question_grading_rule))
        .bind(question_grading_rule_points(&context.question_grading_rule))
        .bind(context.qualified_launch_binding_digest.as_str())
        .bind(lease_parts.capability_checksum.as_bytes().to_vec())
        .bind(lease_parts.expires_at.as_unix_millis())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(lease)
    }

    async fn stage_verified_imathas_result(
        &self,
        session_token_hash: SessionTokenHash,
        stage: StageVerifiedImathasResult,
    ) -> Result<StagedImathasResultReceipt, StoreError> {
        let transition_parts = stage.storage_parts();
        let lease = transition_parts.lease;
        let context = lease.restore;
        let (mut transaction, _) = self
            .begin_authenticated_application_transaction(session_token_hash)
            .await?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.stage_verified_imathas_result(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, decode($10, 'hex'), $11, \
                $12::numeric, $13, $14::numeric, $15, $16, $17, $18, $19, $20, $21, \
                $22, $23, to_timestamp($24::double precision / 1000.0))",
        )
        .bind(lease.reference.as_uuid())
        .bind(context.course.as_uuid())
        .bind(context.assignment.as_uuid())
        .bind(context.grading_context.question_attempt().as_uuid())
        .bind(
            context
                .imathas_question_backend_binding
                .deployment_reference()
                .as_str(),
        )
        .bind(
            context
                .imathas_question_backend_binding
                .item_reference()
                .as_str(),
        )
        .bind(
            context
                .grading_context
                .question_revision()
                .question_id
                .to_string(),
        )
        .bind(
            i32::try_from(
                context
                    .grading_context
                    .question_revision()
                    .revision_number
                    .get(),
            )
            .map_err(|_| {
                StoreError::InvalidRecord(
                    "Question Revision number exceeds PostgreSQL integer range".into(),
                )
            })?,
        )
        .bind(context.source_object.object.as_uuid())
        .bind(context.source_object_checksum.as_str())
        .bind(context.imathas_question_backend_binding.profile().as_str())
        .bind(context.grading_context.question_seed().value().to_string())
        .bind(question_grading_rule_mode(&context.question_grading_rule))
        .bind(question_grading_rule_points(&context.question_grading_rule))
        .bind(context.qualified_launch_binding_digest.as_str())
        .bind(lease.capability_checksum.as_bytes().to_vec())
        .bind(transition_parts.idempotency_key.as_str())
        .bind(
            transition_parts
                .imathas_result_token_checksum
                .as_bytes()
                .to_vec(),
        )
        .bind(transition_parts.imathas_result.normalized_score().value())
        .bind(transition_parts.imathas_result_checksum.as_bytes().to_vec())
        .bind(transition_parts.question_submission_id.as_uuid())
        .bind(transition_parts.grading_job_id.as_uuid())
        .bind(transition_parts.question_submission_grading_id.as_uuid())
        .bind(transition_parts.transitioned_at.as_unix_millis())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let receipt = StagedImathasResultReceipt::from_storage_parts(
            question_model::QuestionSubmissionId::from_uuid(
                row.try_get("submission_id").map_err(map_sqlx_error)?,
            ),
            crate::QuestionSubmissionGradingId::from_uuid(
                row.try_get("question_submission_grading_id")
                    .map_err(map_sqlx_error)?,
            ),
            crate::JobId::from_uuid(row.try_get("job_id").map_err(map_sqlx_error)?),
        );
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn claim_imathas_result_grading_job(
        &self,
        grading_job_id: crate::JobId,
        lease_expires_at: Timestamp,
    ) -> Result<ImathasGradingJobLease, StoreError> {
        let lease_token = random_uuid()?;
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_imathas_question_backend_grading_worker")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let claimed: bool = sqlx::query_scalar(
            "SELECT ple_api.claim_imathas_result_grading_job(\
                $1, $2, to_timestamp($3::double precision / 1000.0))",
        )
        .bind(grading_job_id.as_uuid())
        .bind(lease_token)
        .bind(lease_expires_at.as_unix_millis())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        if !claimed {
            return Err(StoreError::Conflict);
        }
        Ok(ImathasGradingJobLease::from_server_capability(
            grading_job_id,
            lease_token,
            lease_expires_at,
        ))
    }

    async fn commit_staged_imathas_result_grading(
        &self,
        command: CommitStagedImathasResultGrading,
    ) -> Result<AutomatedGradingReceipt, StoreError> {
        let (lease, committed_at) = command.storage_parts();
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_imathas_question_backend_grading_worker")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "SELECT * FROM ple_api.commit_imathas_result_grading(\
                $1, $2, to_timestamp($3::double precision / 1000.0))",
        )
        .bind(lease.job_id.as_uuid())
        .bind(lease.lease_token)
        .bind(committed_at.as_unix_millis())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let receipt = AutomatedGradingReceipt::from_storage_parts(
            crate::AutomatedGradingReceiptId::from_uuid(
                row.try_get("automated_grading_receipt_id")
                    .map_err(map_sqlx_error)?,
            ),
            AutomatedGradingReceiptChecksum::from_bytes(fixed_bytes::<32>(
                &row,
                "automated_grading_receipt_checksum",
            )?),
            question_model::GradingResult {
                correct: row.try_get("correct").map_err(map_sqlx_error)?,
                points_earned: row.try_get("points_earned").map_err(map_sqlx_error)?,
                points_possible: row.try_get("points_possible").map_err(map_sqlx_error)?,
            },
        );
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
}

async fn load_row(
    transaction: &mut Transaction<'_, Postgres>,
    reference: ImathasQuestionBackendSessionReference,
    expectation: &ImathasQuestionBackendSessionRestoreExpectation,
) -> Result<PgRow, StoreError> {
    sqlx::query(
        "SELECT imathas_question_backend_session_id, imathas_item_reference, question_seed::text AS question_seed, \
                imathas_profile, qualified_launch_binding_digest, imathas_response_sha256, \
                imathas_question_backend_session_challenge, convert_from(imathas_question_backend_session_authentication, 'UTF8') AS authentication, \
                floor(extract(epoch FROM issued_at) * 1000)::bigint AS issued_at_millis, \
                floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis, \
                imathas_question_backend_state_key_id, imathas_question_backend_state_nonce, imathas_question_backend_state_ciphertext \
         FROM ple_api.load_imathas_question_backend_session(\
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, decode($11, 'hex'), $12, \
             $13::numeric, $14, $15::numeric, $16)",
    )
    .bind(reference.as_uuid())
    .bind(expectation.storage_parts().account.as_uuid())
    .bind(expectation.storage_parts().course.as_uuid())
    .bind(expectation.storage_parts().assignment.as_uuid())
    .bind(expectation.storage_parts().grading_context.question_attempt().as_uuid())
    .bind(
        expectation
            .storage_parts()
            .imathas_question_backend_binding
            .deployment_reference()
            .as_str(),
    )
    .bind(
        expectation
            .storage_parts()
            .imathas_question_backend_binding
            .item_reference()
            .as_str(),
    )
    .bind(expectation.storage_parts().grading_context.question_revision().question_id.to_string())
    .bind(i32::try_from(expectation.storage_parts().grading_context.question_revision().revision_number.get()).map_err(|_| {
        StoreError::InvalidRecord("Question Revision number exceeds PostgreSQL integer range".into())
    })?)
    .bind(expectation.storage_parts().source_object.object.as_uuid())
    .bind(expectation.storage_parts().source_object_checksum.as_str())
    .bind(
        expectation
            .storage_parts()
            .imathas_question_backend_binding
            .profile()
            .as_str(),
    )
    .bind(expectation.storage_parts().grading_context.question_seed().value().to_string())
    .bind(question_grading_rule_mode(
        &expectation.storage_parts().question_grading_rule,
    ))
    .bind(question_grading_rule_points(
        &expectation.storage_parts().question_grading_rule,
    ))
    .bind(expectation.storage_parts().qualified_launch_binding_digest.as_str())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)
}

fn decode_imathas_question_backend_session_row(
    row: &PgRow,
    reference: ImathasQuestionBackendSessionReference,
    expectation: &ImathasQuestionBackendSessionRestoreExpectation,
) -> Result<
    (
        ImathasQuestionBackendSession,
        ImathasQuestionBackendStateCipher,
    ),
    StoreError,
> {
    let stored_reference: uuid::Uuid = row
        .try_get("imathas_question_backend_session_id")
        .map_err(map_sqlx_error)?;
    if stored_reference != reference.as_uuid() {
        return Err(invalid_stored_session());
    }
    let imathas_item = question_model::ImathasItemReference::new(
        row.try_get::<String, _>("imathas_item_reference")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid_stored_session())?;
    let restore = expectation.storage_parts();
    if imathas_item != *restore.imathas_question_backend_binding.item_reference() {
        return Err(invalid_stored_session());
    }
    let question_seed = row
        .try_get::<String, _>("question_seed")
        .map_err(map_sqlx_error)?
        .parse::<u64>()
        .map(QuestionSeed::new)
        .map_err(|_| invalid_stored_session())?;
    if question_seed != restore.grading_context.question_seed() {
        return Err(invalid_stored_session());
    }
    let profile = question_model::ImathasProfile::new(
        row.try_get::<String, _>("imathas_profile")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid_stored_session())?;
    let digest = QualifiedLaunchBindingDigest::parse(
        row.try_get::<String, _>("qualified_launch_binding_digest")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid_stored_session())?;
    let response = fixed_bytes::<32>(row, "imathas_response_sha256")?;
    let challenge = ImathasQuestionBackendSessionChallenge::from_storage_bytes(fixed_bytes::<32>(
        row,
        "imathas_question_backend_session_challenge",
    )?)
    .map_err(|_| invalid_stored_session())?;
    let authentication = ImathasQuestionBackendSessionAuthentication::from_server_value(
        row.try_get::<String, _>("authentication")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid_stored_session())?;
    let issued_at =
        Timestamp::from_unix_millis(row.try_get("issued_at_millis").map_err(map_sqlx_error)?);
    let expires_at =
        Timestamp::from_unix_millis(row.try_get("expires_at_millis").map_err(map_sqlx_error)?);
    let key_id = ImathasQuestionBackendStateKeyId::parse(
        row.try_get::<String, _>("imathas_question_backend_state_key_id")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| invalid_stored_session())?;
    let nonce = fixed_bytes::<24>(row, "imathas_question_backend_state_nonce")?;
    let ciphertext: Vec<u8> = row
        .try_get("imathas_question_backend_state_ciphertext")
        .map_err(map_sqlx_error)?;
    let parts = ImathasQuestionBackendSessionStorageParts {
        reference,
        account: restore.account,
        course: restore.course,
        assignment: restore.assignment,
        grading_context: restore.grading_context,
        question_grading_rule: restore.question_grading_rule,
        imathas_question_backend_binding: question_model::ImathasQuestionBackendBinding::new(
            restore
                .imathas_question_backend_binding
                .deployment_reference()
                .clone(),
            imathas_item,
            profile,
        ),
        source_object: restore.source_object,
        source_object_checksum: restore.source_object_checksum,
        response_checksum: ImathasResponseChecksum::from_bytes(response),
        challenge,
        authentication,
        qualified_launch_binding_digest: digest,
        issued_at,
        expires_at,
        revoked_at: None,
        consumed_at: None,
        lease_expires_at: None,
        lease_active: false,
    };
    let session = ImathasQuestionBackendSession::from_row_parts(parts)
        .map_err(|_| invalid_stored_session())?;
    let cipher = ImathasQuestionBackendStateCipher::from_row_parts(key_id, nonce, ciphertext)
        .map_err(|_| invalid_stored_session())?;
    Ok((session, cipher))
}

fn fixed_bytes<const N: usize>(row: &PgRow, column: &str) -> Result<[u8; N], StoreError> {
    let bytes: Vec<u8> = row.try_get(column).map_err(map_sqlx_error)?;
    bytes.try_into().map_err(|_| invalid_stored_session())
}

fn invalid_stored_session() -> StoreError {
    StoreError::Unavailable("stored iMathAS Question Backend Session is invalid".into())
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("iMathAS grading UUID randomness unavailable".into())
    })
}

fn ensure_resolved_create_account(
    resolved_account: AccountId,
    create_account: AccountId,
) -> Result<(), StoreError> {
    if resolved_account == create_account {
        Ok(())
    } else {
        Err(StoreError::Forbidden)
    }
}

fn question_grading_rule_mode(rule: &QuestionGradingRule) -> &'static str {
    match rule {
        QuestionGradingRule::AllOrNothing { .. } => "all_or_nothing",
        QuestionGradingRule::PartialCredit { .. } => "partial_credit",
        QuestionGradingRule::Ungraded => "ungraded",
    }
}

fn question_grading_rule_points(rule: &QuestionGradingRule) -> Option<f64> {
    match rule {
        QuestionGradingRule::AllOrNothing { points }
        | QuestionGradingRule::PartialCredit { points } => Some(*points),
        QuestionGradingRule::Ungraded => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_refuses_an_account_other_than_the_resolved_session_account() {
        let resolved = AccountId::from_uuid(uuid::Uuid::from_u128(1));
        let different_create_account = AccountId::from_uuid(uuid::Uuid::from_u128(2));

        assert_eq!(
            ensure_resolved_create_account(resolved, different_create_account),
            Err(StoreError::Forbidden)
        );
        assert_eq!(ensure_resolved_create_account(resolved, resolved), Ok(()));
    }
}
