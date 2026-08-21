//! Shared host-tool construction of the configured production PostgreSQL Store.

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::postgres::{Pool, PostgresStore};

pub(crate) fn configured_postgres_store(pool: Pool) -> Result<PostgresStore> {
    // ASVS 2.10.3 and 16.5.1: read the secret from the approved environment/file
    // boundary, validate its canonical encoding, and never include its value in errors.
    let encoded = match std::env::var("PLE_QUESTION_ID_SECRET_FILE") {
        Ok(path) => std::fs::read_to_string(path)
            .context("reading the configured Question ID secret file")?,
        Err(std::env::VarError::NotPresent) => std::env::var("PLE_QUESTION_ID_SECRET")
            .context("PLE_QUESTION_ID_SECRET_FILE or PLE_QUESTION_ID_SECRET is required")?,
        Err(error) => return Err(error).context("PLE_QUESTION_ID_SECRET_FILE is not Unicode"),
    };
    let encoded = encoded.trim_end_matches(['\r', '\n']);
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Question ID secret must be canonical base64url")?;
    if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(&decoded) != encoded {
        bail!("Question ID secret must be canonical 32-byte base64url");
    }
    Ok(PostgresStore::with_question_id_secret(
        pool,
        decoded
            .try_into()
            .expect("checked Question ID secret length"),
    ))
}
