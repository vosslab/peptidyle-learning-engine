//! Operator-owned Bloom provider composition.

use std::sync::Arc;

use anyhow::{Result, bail};
use learning_data_access::postgres::{Pool, PostgresBloomClassificationPreparationStore};

use crate::bloom_classification::{
    BloomClassifier, BloomPublicationPreparation, NotConfiguredBloomClassifier,
    ollama::{OllamaBloomClassifier, OllamaBloomClassifierConfig, OllamaTransportSecurity},
};

const BASE_URL_ENV: &str = "PLE_BLOOM_OLLAMA_BASE_URL";
const MODEL_ENV: &str = "PLE_BLOOM_MODEL";
const CONTEXT_TOKENS_ENV: &str = "PLE_BLOOM_CONTEXT_TOKENS";

/// Builds the private provider plus the existing one-use PostgreSQL receipt boundary.
///
/// All three provider values absent selects a fail-closed unavailable classifier.
/// Any partial or malformed selection is an operator-visible composition error.
pub fn bloom_publication_preparation_from_env(
    pool: Pool,
) -> Result<Arc<BloomPublicationPreparation>> {
    bloom_publication_preparation_from_values(pool, |name| std::env::var(name).ok())
}

fn bloom_publication_preparation_from_values(
    pool: Pool,
    mut value_for: impl FnMut(&str) -> Option<String>,
) -> Result<Arc<BloomPublicationPreparation>> {
    let base_url = nonempty(value_for(BASE_URL_ENV));
    let model = nonempty(value_for(MODEL_ENV));
    let context_tokens = nonempty(value_for(CONTEXT_TOKENS_ENV));
    let classifier: Arc<dyn BloomClassifier> = match (base_url, model, context_tokens) {
        (None, None, None) => Arc::new(NotConfiguredBloomClassifier),
        (Some(base_url), Some(model), Some(context_tokens)) => {
            let context_tokens = context_tokens
                .parse::<u32>()
                .map_err(|_| anyhow::anyhow!("{CONTEXT_TOKENS_ENV} is invalid"))?;
            let transport_security = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref()
                == Some("disposable-local")
            {
                OllamaTransportSecurity::AllowContainedPlaintext
            } else {
                OllamaTransportSecurity::RequireTls
            };
            let config = OllamaBloomClassifierConfig::new(
                &base_url,
                model,
                context_tokens,
                transport_security,
            )
            .map_err(anyhow::Error::new)?;
            Arc::new(OllamaBloomClassifier::new(config).map_err(anyhow::Error::new)?)
        }
        _ => {
            bail!(
                "{BASE_URL_ENV}, {MODEL_ENV}, and {CONTEXT_TOKENS_ENV} must be configured together"
            )
        }
    };
    let receipts = Arc::new(PostgresBloomClassificationPreparationStore::new(pool));
    Ok(Arc::new(BloomPublicationPreparation::new(
        classifier, receipts,
    )))
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}
