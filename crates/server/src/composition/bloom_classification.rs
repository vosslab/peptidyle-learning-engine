//! Provider-neutral Bloom publication composition.

use std::sync::Arc;

use learning_data_access::postgres::{Pool, PostgresBloomClassificationPreparationStore};

use crate::bloom_classification::{BloomPublicationPreparation, NotConfiguredBloomClassifier};

/// Builds the existing receipt boundary without selecting an AI provider.
///
/// Ordinary runtime composition remains available. Publication fails closed
/// until an approved provider is supplied through the provider-neutral trait.
pub fn bloom_publication_preparation(pool: Pool) -> Arc<BloomPublicationPreparation> {
    let receipts = Arc::new(PostgresBloomClassificationPreparationStore::new(pool));
    Arc::new(BloomPublicationPreparation::new(
        Arc::new(NotConfiguredBloomClassifier),
        receipts,
    ))
}
