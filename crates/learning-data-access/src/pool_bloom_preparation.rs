//! Request-owned trusted receipts for the Pools created by one mutation.

use std::collections::VecDeque;

use crate::{BloomPreparationReceiptId, StoreError};

/// One independently prepared receipt for each newly materialized Question Pool.
///
/// Supply receipts in materialization order: Blueprint module, Assessment, Entry,
/// followed by each daughter Course in the locked database result order. Retained
/// unchanged Pools consume no receipt. PostgreSQL verifies the exact candidate
/// fingerprint and one-use constraint; order alone never grants publication.
#[derive(Debug, Clone, Default)]
pub struct PoolBloomPreparationReceipts(VecDeque<BloomPreparationReceiptId>);

impl PoolBloomPreparationReceipts {
    /// Owns receipts prepared by trusted orchestration before this mutation.
    pub fn new(receipts: impl IntoIterator<Item = BloomPreparationReceiptId>) -> Self {
        Self(receipts.into_iter().collect())
    }

    /// Takes one prepared receipt, refusing to publish an unclassified Pool.
    pub fn take_next(&mut self) -> Result<BloomPreparationReceiptId, StoreError> {
        self.0.pop_front().ok_or_else(|| {
            StoreError::Unavailable(
                "Question Pool Bloom classification preparation is unavailable".to_owned(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_pool_requires_a_prepared_receipt_position() {
        let receipt = BloomPreparationReceiptId::from_uuid(uuid::Uuid::from_u128(1));
        let mut receipts = PoolBloomPreparationReceipts::new([receipt]);
        assert_eq!(receipts.take_next().expect("prepared receipt"), receipt);
        assert!(matches!(
            receipts.take_next(),
            Err(StoreError::Unavailable(_))
        ));
    }
}
