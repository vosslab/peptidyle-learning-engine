//! Locked verification for active rehearsals affected by source removal.
//!
//! Callers first lock their source facts in the shared order: course,
//! assignment(s), Instructor membership(s).  This module then locks matching
//! active rehearsal runs by stable UUID, verifies every private aggregate, and
//! returns only an opaque count.  It never projects a learner identity or a
//! rehearsal subject.  ASVS 1.2.4, 2.3.3, 8.2.2, and 8.4.1 apply.

use question_model::TenantId;
use sqlx::{Postgres, Transaction};

use super::super::*;
use super::{LockedRehearsalSourceWitness, RehearsalSourceSelector, hydration, integrity, rows};

/// Verify every active rehearsal locked by the matching broker prepare.
///
/// The broker owns source/run locks; this application-role function only
/// plain-reads its exact opaque UUID witness before private hydration.
pub(super) async fn verify_prelocked_source_aggregates(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    selector: RehearsalSourceSelector,
    locked_rehearsal_count: i64,
    locked_rehearsal_run_ids: Vec<sqlx::types::Uuid>,
) -> Result<LockedRehearsalSourceWitness, StoreError> {
    let locked_count = validate_witness(locked_rehearsal_count, &locked_rehearsal_run_ids)?;
    let locators = read_prelocked_locators(tx, tenant, &locked_rehearsal_run_ids).await?;
    if locators.len() != locked_rehearsal_run_ids.len()
        || locators
            .iter()
            .map(|locator| locator.id.as_uuid())
            .ne(locked_rehearsal_run_ids.iter().copied())
    {
        return Err(StoreError::InvalidRecord(
            "locked rehearsal witness does not match persisted rows".into(),
        ));
    }
    for locator in locators {
        if !selector.matches(&locator) || !locator.lifecycle.is_active() {
            return Err(StoreError::InvalidRecord(
                "invalid locked rehearsal source scope".into(),
            ));
        }
        let aggregate = hydration::load_locked_source_aggregate(tx, locator).await?;
        integrity::require_active(&aggregate.run)?;
    }
    Ok(LockedRehearsalSourceWitness {
        count: locked_count,
    })
}

async fn read_prelocked_locators(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    locked_rehearsal_run_ids: &[sqlx::types::Uuid],
) -> Result<Vec<rows::RunLocator>, StoreError> {
    sqlx::query(PRELOCKED_LOCATORS)
        .bind(tenant.as_uuid())
        .bind(locked_rehearsal_run_ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_sqlx_error)?
        .iter()
        .map(rows::decode_locator)
        .collect()
}

fn validate_witness(
    locked_rehearsal_count: i64,
    locked_rehearsal_run_ids: &[sqlx::types::Uuid],
) -> Result<u64, StoreError> {
    let count = u64::try_from(locked_rehearsal_count)
        .map_err(|_| StoreError::InvalidRecord("invalid locked rehearsal witness count".into()))?;
    if usize::try_from(count).ok() != Some(locked_rehearsal_run_ids.len())
        || locked_rehearsal_run_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(StoreError::InvalidRecord(
            "invalid locked rehearsal witness".into(),
        ));
    }
    Ok(count)
}

impl RehearsalSourceSelector {
    fn matches(&self, locator: &rows::RunLocator) -> bool {
        match self {
            Self::Course { course } => locator.course == *course,
            Self::Assignment { course, assignment } => {
                locator.course == *course && locator.assignment_id == *assignment
            }
            Self::DirectInstructorMembership { course, membership } => {
                locator.course == *course && locator.owner == *membership
            }
        }
    }
}

const PRELOCKED_LOCATORS: &str = concat!(
    "SELECT tenant_id, rehearsal_run_id, rehearsal_reference, course_id, assignment_id, ",
    "assignment_reference, direct_instructor_membership_id, actor_id, assignment_revision, ",
    "lifecycle, (extract(epoch FROM started_at) * 1000)::bigint AS started_at_millis, ",
    "(extract(epoch FROM updated_at) * 1000)::bigint AS updated_at_millis, ",
    "CASE WHEN terminal_at IS NULL THEN NULL ELSE ",
    "(extract(epoch FROM terminal_at) * 1000)::bigint END AS terminal_at_millis, ",
    "evidence_head_digest, evidence_length FROM rehearsal_run WHERE tenant_id=$1 ",
    "AND rehearsal_run_id = ANY($2::uuid[]) ORDER BY rehearsal_run_id"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelocked_query_is_plain_answer_free_read() {
        assert!(PRELOCKED_LOCATORS.contains("tenant_id=$1"));
        assert!(PRELOCKED_LOCATORS.contains("rehearsal_run_id = ANY($2::uuid[])"));
        assert!(PRELOCKED_LOCATORS.contains("ORDER BY rehearsal_run_id"));
        assert!(!PRELOCKED_LOCATORS.contains("FOR UPDATE"));
        assert!(!PRELOCKED_LOCATORS.contains("subject_payload"));
    }

    #[test]
    fn witness_validation_rejects_negative_mismatch_duplicate_and_unsorted_ids() {
        let one = sqlx::types::Uuid::from_u128(1);
        let two = sqlx::types::Uuid::from_u128(2);
        assert!(validate_witness(-1, &[]).is_err());
        assert!(validate_witness(2, &[one]).is_err());
        assert!(validate_witness(2, &[one, one]).is_err());
        assert!(validate_witness(2, &[two, one]).is_err());
        assert_eq!(validate_witness(2, &[one, two]).expect("valid witness"), 2);
    }
}
