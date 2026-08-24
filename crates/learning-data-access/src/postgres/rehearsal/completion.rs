//! Completion after a verified dispatched claim.

use domain::{
    RehearsalEvidencePayload, RehearsalValidatedSubmissionEvidence, evidence_entry_digest,
    private_payload_digest, validate_claim_completion, verify_rehearsal_claim_completion_proof,
};

use super::super::*;
use super::{auth, hydration, integrity};

const COMPLETE_CLAIM_SQL: &str =
    "SELECT ple_rehearsal_complete_claim($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)";

pub(super) async fn complete(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::CompleteRehearsalSubmissionCommand,
) -> Result<crate::RehearsalSubmissionReceipt, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let witness = auth::prepare_operation(&mut tx, tenant, command.locator).await?;
    let source = witness.source();
    let aggregate = hydration::load_authorized(&mut tx, tenant, command.locator, &source).await?;
    if aggregate.run.id != witness.run {
        return Err(StoreError::NotFound);
    }
    integrity::require_active(&aggregate.run)?;
    let claim_id = command.handle.claim();
    let operation = command.handle.operation();
    let generation = command.handle.generation();
    let claim = aggregate.claim(claim_id).ok_or(StoreError::NotFound)?;
    let expected = claim.snapshot.operation() == operation
        && claim.snapshot.generation() == generation
        && claim.snapshot.state() == domain::RehearsalSubmissionClaimState::GradingDispatched;
    if !expected {
        return Err(StoreError::Conflict);
    }
    validate_claim_completion(aggregate.run.receipt.lifecycle, true, command.handle)
        .map_err(|_| StoreError::Conflict)?;
    let frozen = aggregate
        .frozen(claim.root.sealed_request().attempt())
        .ok_or(StoreError::NotFound)?;
    let millis: i64 = sqlx::query_scalar("SELECT ple_rehearsal_now_millis()")
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
    let accepted_at = question_model::ActivityTimestamp::from_unix_millis(millis);
    let evidence = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        &claim.root,
        claim.root.sealed_request().clone(),
        frozen,
        command.grading,
        accepted_at,
    )
    .map_err(invalid)?;
    let payload = RehearsalEvidencePayload::AcceptedSubmission(evidence);
    let sequence =
        aggregate.run.head.length().checked_add(1).ok_or_else(|| {
            StoreError::Unavailable("rehearsal evidence sequence exhausted".into())
        })?;
    let entry_digest = evidence_entry_digest(
        sequence,
        payload.kind(),
        aggregate.run.head.digest(),
        private_payload_digest(&payload),
        accepted_at,
    );
    let staged = {
        let mut values = aggregate.evidence.clone();
        values.push(domain::RehearsalEvidenceChainEntry {
            record: question_model::RehearsalEvidenceRecord {
                sequence,
                kind: payload.kind(),
                previous_digest: Some(aggregate.run.head.digest()),
                digest: entry_digest,
                recorded_at: accepted_at,
            },
            payload: payload.clone(),
        });
        values
    };
    let head = aggregate
        .run
        .head
        .advance(&staged.last().expect("new entry").record)
        .map_err(invalid)?;
    let proof =
        verify_rehearsal_claim_completion_proof(aggregate.genesis(), head, &claim.root, &staged)
            .map_err(invalid)?;
    let outcome = proof.replay_receipt();
    let receipt_digest = domain::rehearsal::persistence::public_outcome_digest(&outcome);
    let stored: bool = sqlx::query_scalar(COMPLETE_CLAIM_SQL)
        .bind(tenant.as_uuid())
        .bind(command.locator.actor.as_uuid())
        .bind(command.locator.course.as_uuid())
        .bind(source.assignment.as_uuid())
        .bind(revision(command.locator.revision)?)
        .bind(aggregate.run.id.as_uuid())
        .bind(claim_id.as_uuid())
        .bind(operation.as_uuid())
        .bind(aggregate.run.head.digest().as_bytes().to_vec())
        .bind(i64::from(aggregate.run.head.length()))
        .bind(entry_digest.as_bytes().to_vec())
        .bind(domain::rehearsal::persistence::encode_evidence_payload(
            &payload,
        ))
        .bind(private_payload_digest(&payload).as_bytes().to_vec())
        .bind(millis)
        .bind(serde_json::to_value(&outcome).map_err(|_| {
            StoreError::InvalidRecord("rehearsal receipt serialization failed".into())
        })?)
        .bind(receipt_digest.as_bytes().to_vec())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
    if !stored {
        return Err(StoreError::Conflict);
    }
    let after = hydration::load_authorized(&mut tx, tenant, command.locator, &source).await?;
    let completed = after.claim(claim_id).ok_or(StoreError::NotFound)?;
    if completed.snapshot.state() != domain::RehearsalSubmissionClaimState::Completed
        || completed.outcome.as_ref() != Some(&outcome)
        || completed.snapshot.operation() != operation
        || completed.snapshot.generation() != generation
    {
        return Err(StoreError::InvalidRecord(
            "rehearsal completion did not verify after mutation".into(),
        ));
    }
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(crate::RehearsalSubmissionReceipt {
        outcome,
        replayed: false,
    })
}

fn revision(value: question_model::TeachingOperationRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value())
        .map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))
}
fn invalid(_error: impl std::fmt::Debug) -> StoreError {
    StoreError::InvalidRecord("invalid rehearsal completion".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_capability_has_exactly_sixteen_bound_parameters() {
        assert!(COMPLETE_CLAIM_SQL.ends_with("$16)"));
        assert!(!COMPLETE_CLAIM_SQL.contains("$17"));
        assert_eq!(COMPLETE_CLAIM_SQL.matches('$').count(), 16);
    }
}
