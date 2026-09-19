//! PostgreSQL persistence for one-use, SQL-fingerprinted Bloom receipts.

use async_trait::async_trait;

use super::{Pool, connection::map_sqlx_error};
use crate::{
    BloomClassificationPreparationStore, BloomPreparationCandidate, BloomPreparationReceiptId,
    PrepareBloomClassificationInput, StoreError,
};

const BLOOM_PREPARATION_RECEIPT_ATTEMPTS: usize = 8;
const BLOOM_PREPARATION_RECEIPT_PRIMARY_KEY: &str = "bloom_preparation_receipt_pkey";

#[derive(Debug, Clone, PartialEq)]
enum PrepareReceiptAttemptError {
    ReceiptIdCollision,
    Store(StoreError),
}

#[async_trait]
trait BloomReceiptPreparer {
    async fn prepare_once(
        &self,
        receipt_id: uuid::Uuid,
        input: &PrepareBloomClassificationInput,
    ) -> Result<(), PrepareReceiptAttemptError>;
}

/// Binds the ordinary application pool to the two typed Bloom-preparation procedures.
#[derive(Clone)]
pub struct PostgresBloomClassificationPreparationStore {
    pool: Pool,
}

impl PostgresBloomClassificationPreparationStore {
    /// Creates a Store over the ordinary attested application pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BloomReceiptPreparer for PostgresBloomClassificationPreparationStore {
    async fn prepare_once(
        &self,
        receipt_id: uuid::Uuid,
        input: &PrepareBloomClassificationInput,
    ) -> Result<(), PrepareReceiptAttemptError> {
        let mut transaction = self.pool.begin().await.map_err(map_prepare_store_error)?;
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_prepare_store_error)?;

        let classification = input.classification;
        match &input.candidate {
            BloomPreparationCandidate::Question { source_checksum } => {
                // ASVS 1.2.4: immutable semantic source content stays bound;
                // SQL owns fingerprint serialization and receipt persistence.
                sqlx::query(
                    "SELECT ple_api.prepare_question_revision_bloom_classification(\
                     $1, $2, $3, $4)",
                )
                .bind(receipt_id)
                .bind(source_checksum.to_string())
                .bind(classification.cognitive_process.as_str())
                .bind(classification.knowledge_dimension.as_str())
                .execute(&mut *transaction)
                .await
                .map_err(map_prepare_sql_error)?;
            }
            BloomPreparationCandidate::Pool {
                title,
                description,
                members,
            } => {
                let member_question_ids = members
                    .iter()
                    .map(|member| member.question_id.as_str().to_owned())
                    .collect::<Vec<_>>();
                let member_revision_numbers = members
                    .iter()
                    .map(|member| i32::try_from(member.revision_number.get()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| {
                        PrepareReceiptAttemptError::Store(StoreError::InvalidRecord(
                            "Question Revision Number exceeds PostgreSQL integer".to_owned(),
                        ))
                    })?;
                // Preserve vector order: the SQL fingerprint binds current Pool membership order.
                sqlx::query(
                    "SELECT ple_api.prepare_question_pool_bloom_classification(\
                     $1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(receipt_id)
                .bind(title)
                .bind(description)
                .bind(member_question_ids)
                .bind(member_revision_numbers)
                .bind(classification.cognitive_process.as_str())
                .bind(classification.knowledge_dimension.as_str())
                .execute(&mut *transaction)
                .await
                .map_err(map_prepare_sql_error)?;
            }
        }
        transaction
            .commit()
            .await
            .map_err(map_prepare_store_error)?;
        Ok(())
    }
}

#[async_trait]
impl BloomClassificationPreparationStore for PostgresBloomClassificationPreparationStore {
    async fn prepare_bloom_classification(
        &self,
        input: PrepareBloomClassificationInput,
    ) -> Result<BloomPreparationReceiptId, StoreError> {
        prepare_with_fresh_receipt_ids(self, input, || {
            crate::random_uuid::random_uuid_v4(|error| {
                StoreError::Unavailable(format!(
                    "Bloom Classification receipt randomness unavailable: {error}"
                ))
            })
        })
        .await
    }
}

async fn prepare_with_fresh_receipt_ids<P>(
    preparer: &P,
    input: PrepareBloomClassificationInput,
    mut issue_receipt_id: impl FnMut() -> Result<uuid::Uuid, StoreError>,
) -> Result<BloomPreparationReceiptId, StoreError>
where
    P: BloomReceiptPreparer + ?Sized,
{
    for _ in 0..BLOOM_PREPARATION_RECEIPT_ATTEMPTS {
        let receipt_id = issue_receipt_id()?;
        // ASVS 2.3.3: each literal identifier attempt owns a fresh transaction;
        // only its primary-key collision is safe to roll back and retry.
        match preparer.prepare_once(receipt_id, &input).await {
            Ok(()) => return Ok(BloomPreparationReceiptId::from_uuid(receipt_id)),
            Err(PrepareReceiptAttemptError::ReceiptIdCollision) => continue,
            Err(PrepareReceiptAttemptError::Store(error)) => return Err(error),
        }
    }
    Err(StoreError::AlreadyExists)
}

fn map_prepare_store_error(error: sqlx::Error) -> PrepareReceiptAttemptError {
    PrepareReceiptAttemptError::Store(map_sqlx_error(error))
}

fn map_prepare_sql_error(error: sqlx::Error) -> PrepareReceiptAttemptError {
    if let sqlx::Error::Database(database_error) = &error
        && is_receipt_id_collision(
            database_error.code().as_deref(),
            database_error.constraint(),
        )
    {
        return PrepareReceiptAttemptError::ReceiptIdCollision;
    }
    map_prepare_store_error(error)
}

fn is_receipt_id_collision(code: Option<&str>, constraint: Option<&str>) -> bool {
    code == Some("23505") && constraint == Some(BLOOM_PREPARATION_RECEIPT_PRIMARY_KEY)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use objects::Sha256Checksum;
    use question_model::{BloomClassification, BloomCognitiveProcess, BloomKnowledgeDimension};

    use super::*;

    struct CollisionThenSuccess {
        attempts: Mutex<Vec<(uuid::Uuid, PrepareBloomClassificationInput)>>,
    }

    #[async_trait]
    impl BloomReceiptPreparer for CollisionThenSuccess {
        async fn prepare_once(
            &self,
            receipt_id: uuid::Uuid,
            input: &PrepareBloomClassificationInput,
        ) -> Result<(), PrepareReceiptAttemptError> {
            let mut attempts = self.attempts.lock().expect("attempt capture lock");
            attempts.push((receipt_id, input.clone()));
            if attempts.len() == 1 {
                Err(PrepareReceiptAttemptError::ReceiptIdCollision)
            } else {
                Ok(())
            }
        }
    }

    fn input() -> PrepareBloomClassificationInput {
        PrepareBloomClassificationInput {
            candidate: BloomPreparationCandidate::Question {
                source_checksum: Sha256Checksum::compute(b"exact source"),
            },
            classification: BloomClassification {
                cognitive_process: BloomCognitiveProcess::Analyze,
                knowledge_dimension: BloomKnowledgeDimension::ConceptualKnowledge,
            },
        }
    }

    #[tokio::test]
    async fn literal_receipt_uuid_collision_retries_with_fresh_uuid_and_same_input() {
        let collision = uuid::Uuid::from_u128(1);
        let accepted = uuid::Uuid::from_u128(2);
        let mut receipt_ids = [collision, accepted].into_iter();
        let preparer = CollisionThenSuccess {
            attempts: Mutex::new(Vec::new()),
        };
        let input = input();

        let receipt = prepare_with_fresh_receipt_ids(&preparer, input.clone(), || {
            receipt_ids
                .next()
                .ok_or_else(|| StoreError::Unavailable("test receipt IDs exhausted".to_owned()))
        })
        .await
        .expect("second receipt UUID is accepted");

        assert_eq!(receipt.as_uuid(), accepted);
        assert_eq!(
            preparer
                .attempts
                .into_inner()
                .expect("attempt capture lock"),
            vec![(collision, input.clone()), (accepted, input)]
        );
        assert!(is_receipt_id_collision(
            Some("23505"),
            Some(BLOOM_PREPARATION_RECEIPT_PRIMARY_KEY)
        ));
        assert!(!is_receipt_id_collision(
            Some("23505"),
            Some("published_question_pkey")
        ));
    }
}
