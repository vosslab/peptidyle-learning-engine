//! PostgreSQL persistence for the Sysadmin-only TOTP second factor.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use question_model::{AccountId, ProductRole, Timestamp};
use sqlx::postgres::PgRow;
use sqlx::{Postgres, Row, Transaction};
use zeroize::Zeroize;

use super::Pool;
use super::connection::{map_sqlx_error, parse_account_id};
use crate::{
    AuthenticationCeremonyLifetime, AuthenticationSecretHash, PendingSysadminTotpAttestation,
    SessionId, SessionLifetime, SessionRecord, SessionTokenHash, StoreError,
    SysadminTotpAttestationId, SysadminTotpCounter, SysadminTotpSeed, SysadminTotpStore,
    SysadminTotpVerificationReservation,
};

const TOTP_SEED_NONCE_BYTES: usize = 24;
const TOTP_SEED_CIPHERTEXT_MINIMUM_BYTES: usize = 36;
const TOTP_SEED_CIPHERTEXT_MAXIMUM_BYTES: usize = 80;
const TOTP_SEED_AAD_VERSION: u8 = 2;

/// Redacted identifier for a rotating Sysadmin TOTP seed-encryption key.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SysadminTotpSeedKeyId(String);

impl SysadminTotpSeedKeyId {
    /// Parses a bounded, operator-supplied key identifier.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(StoreError::InvalidRecord(
                "Sysadmin TOTP seed key ID is invalid".into(),
            ));
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SysadminTotpSeedKeyId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SysadminTotpSeedKeyId([redacted])")
    }
}

/// Active and retiring keys for AEAD-wrapped Sysadmin TOTP seeds.
pub struct SysadminTotpSeedKeyRing {
    active: SysadminTotpSeedKeyId,
    keys: BTreeMap<SysadminTotpSeedKeyId, [u8; 32]>,
}

impl Drop for SysadminTotpSeedKeyRing {
    fn drop(&mut self) {
        for key in self.keys.values_mut() {
            key.zeroize();
        }
    }
}

impl SysadminTotpSeedKeyRing {
    /// Builds a key ring with exactly one active key and optional retiring keys.
    pub fn new(
        active: SysadminTotpSeedKeyId,
        active_key: [u8; 32],
        retiring: impl IntoIterator<Item = (SysadminTotpSeedKeyId, [u8; 32])>,
    ) -> Result<Self, StoreError> {
        let mut keys = BTreeMap::new();
        keys.insert(active.clone(), active_key);
        for (id, key) in retiring {
            if keys.insert(id, key).is_some() {
                return Err(StoreError::InvalidRecord(
                    "Sysadmin TOTP seed key IDs must be unique".into(),
                ));
            }
        }
        Ok(Self { active, keys })
    }

    fn active_key(&self) -> (&SysadminTotpSeedKeyId, &[u8; 32]) {
        (
            &self.active,
            self.keys.get(&self.active).expect("active key is present"),
        )
    }

    fn key(&self, id: &SysadminTotpSeedKeyId) -> Result<&[u8; 32], StoreError> {
        self.keys.get(id).ok_or_else(|| {
            StoreError::Unavailable("Sysadmin TOTP seed decryption key is unavailable".into())
        })
    }
}

impl std::fmt::Debug for SysadminTotpSeedKeyRing {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SysadminTotpSeedKeyRing([redacted])")
    }
}

struct EncryptedSysadminTotpSeed {
    key_id: SysadminTotpSeedKeyId,
    nonce: [u8; TOTP_SEED_NONCE_BYTES],
    ciphertext: Vec<u8>,
}

impl EncryptedSysadminTotpSeed {
    fn seal(
        key_ring: &SysadminTotpSeedKeyRing,
        account: AccountId,
        seed: &SysadminTotpSeed,
    ) -> Result<Self, StoreError> {
        let (key_id, key) = key_ring.active_key();
        let mut nonce = [0_u8; TOTP_SEED_NONCE_BYTES];
        // ASVS 6.5.3 and 11.2.1: each wrapping nonce comes from the OS CSPRNG.
        getrandom::fill(&mut nonce).map_err(|_| {
            StoreError::Unavailable("Sysadmin TOTP seed nonce randomness unavailable".into())
        })?;
        let cipher_nonce = XNonce::try_from(nonce.as_slice())
            .map_err(|_| StoreError::InvalidRecord("Sysadmin TOTP seed nonce is invalid".into()))?;
        let ciphertext = XChaCha20Poly1305::new(key.into())
            .encrypt(
                &cipher_nonce,
                Payload {
                    msg: seed.as_bytes(),
                    aad: &seed_aad(account),
                },
            )
            .map_err(|_| {
                StoreError::Unavailable("Sysadmin TOTP seed cannot be encrypted".into())
            })?;
        if !(TOTP_SEED_CIPHERTEXT_MINIMUM_BYTES..=TOTP_SEED_CIPHERTEXT_MAXIMUM_BYTES)
            .contains(&ciphertext.len())
        {
            return Err(StoreError::InvalidRecord(
                "Sysadmin TOTP seed ciphertext is outside its bounded encoding".into(),
            ));
        }
        Ok(Self {
            key_id: key_id.clone(),
            nonce,
            ciphertext,
        })
    }

    fn open(
        key_ring: &SysadminTotpSeedKeyRing,
        account: AccountId,
        key_id: SysadminTotpSeedKeyId,
        nonce: Vec<u8>,
        ciphertext: Vec<u8>,
    ) -> Result<SysadminTotpSeed, StoreError> {
        let nonce: [u8; TOTP_SEED_NONCE_BYTES] = nonce
            .try_into()
            .map_err(|_| StoreError::InvalidRecord("Sysadmin TOTP seed nonce is invalid".into()))?;
        if !(TOTP_SEED_CIPHERTEXT_MINIMUM_BYTES..=TOTP_SEED_CIPHERTEXT_MAXIMUM_BYTES)
            .contains(&ciphertext.len())
        {
            return Err(StoreError::InvalidRecord(
                "Sysadmin TOTP seed ciphertext is invalid".into(),
            ));
        }
        let cipher_nonce = XNonce::try_from(nonce.as_slice())
            .map_err(|_| StoreError::InvalidRecord("Sysadmin TOTP seed nonce is invalid".into()))?;
        let plaintext = XChaCha20Poly1305::new(key_ring.key(&key_id)?.into())
            .decrypt(
                &cipher_nonce,
                Payload {
                    msg: &ciphertext,
                    aad: &seed_aad(account),
                },
            )
            .map_err(|_| {
                StoreError::InvalidRecord("Sysadmin TOTP seed cannot be authenticated".into())
            })?;
        SysadminTotpSeed::from_csprng_bytes(plaintext)
    }
}

fn seed_aad(account: AccountId) -> Vec<u8> {
    let mut aad = vec![TOTP_SEED_AAD_VERSION];
    aad.extend_from_slice(account.as_str().as_bytes());
    aad
}

/// PostgreSQL adapter for pending Sysadmin MFA and encrypted seed storage.
#[derive(Clone)]
pub struct PostgresSysadminTotpStore {
    pool: Pool,
    key_ring: Arc<SysadminTotpSeedKeyRing>,
}

impl PostgresSysadminTotpStore {
    /// Binds the already-attested authentication pool and server-held key ring.
    pub fn new(pool: Pool, key_ring: Arc<SysadminTotpSeedKeyRing>) -> Self {
        Self { pool, key_ring }
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl SysadminTotpStore for PostgresSysadminTotpStore {
    async fn provision_sysadmin_totp_seed(
        &self,
        account: AccountId,
        seed: SysadminTotpSeed,
    ) -> Result<(), StoreError> {
        let encrypted = EncryptedSysadminTotpSeed::seal(&self.key_ring, account.clone(), &seed)?;
        let mut transaction = self.begin().await?;
        sqlx::query("SELECT ple_api.provision_sysadmin_totp_credential($1, $2, $3, $4)")
            .bind(account.as_str())
            .bind(encrypted.key_id.as_str())
            .bind(encrypted.nonce.to_vec())
            .bind(encrypted.ciphertext)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    async fn create_pending_sysadmin_totp_attestation(
        &self,
        account: AccountId,
        browser_binding_hash: AuthenticationSecretHash,
        lifetime: AuthenticationCeremonyLifetime,
    ) -> Result<Option<SysadminTotpAttestationId>, StoreError> {
        let attestation = SysadminTotpAttestationId::generate()?;
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT attestation_id FROM ple_api.create_pending_sysadmin_totp_attestation($1, $2, $3, $4)",
        )
        .bind(attestation.as_uuid())
        .bind(account.as_str())
        .bind(browser_binding_hash.as_bytes().to_vec())
        .bind(i64::from(lifetime.as_seconds()))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(row.map(|row| SysadminTotpAttestationId::from_uuid(row.get("attestation_id"))))
    }

    async fn load_pending_sysadmin_totp_attestation(
        &self,
        attestation: SysadminTotpAttestationId,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<PendingSysadminTotpAttestation>, StoreError> {
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT account_id, encryption_key_id, seed_nonce, encrypted_seed \
             FROM ple_api.load_pending_sysadmin_totp_attestation($1, $2)",
        )
        .bind(attestation.as_uuid())
        .bind(browser_binding_hash.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(|row| decode_pending_row(&self.key_ring, attestation, &row))
            .transpose()
    }

    async fn reserve_sysadmin_totp_verification_attempt(
        &self,
        attestation: SysadminTotpAttestationId,
        browser_binding_hash: AuthenticationSecretHash,
    ) -> Result<Option<SysadminTotpVerificationReservation>, StoreError> {
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT account_id FROM ple_api.reserve_sysadmin_totp_verification_attempt($1, $2)",
        )
        .bind(attestation.as_uuid())
        .bind(browser_binding_hash.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(row
            .map(|row| {
                parse_account_id(row.get::<String, _>("account_id"))
                    .map(|account| SysadminTotpVerificationReservation { account })
            })
            .transpose()?)
    }

    async fn consume_sysadmin_totp_attestation_into_session(
        &self,
        attestation: SysadminTotpAttestationId,
        browser_binding_hash: AuthenticationSecretHash,
        counter: SysadminTotpCounter,
        token_hash: SessionTokenHash,
        lifetime: SessionLifetime,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let session = SessionId::generate()?;
        let mut transaction = self.begin().await?;
        let row = sqlx::query(
            "SELECT session_id, encode(token_hash, 'hex') AS session_hash, account_id, product_role, \
                    floor(extract(epoch FROM created_at) * 1000)::bigint AS created_at_millis, \
                    floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
             FROM ple_api.consume_sysadmin_totp_attestation_into_session($1, $2, $3, $4, decode($5, 'hex'), $6)",
        )
        .bind(attestation.as_uuid())
        .bind(browser_binding_hash.as_bytes().to_vec())
        .bind(counter.as_i64())
        .bind(session.as_uuid())
        .bind(token_hash.to_string())
        .bind(i64::from(lifetime.as_seconds()))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.as_ref().map(decode_session_row).transpose()
    }
}

fn decode_pending_row(
    key_ring: &SysadminTotpSeedKeyRing,
    attestation: SysadminTotpAttestationId,
    row: &PgRow,
) -> Result<PendingSysadminTotpAttestation, StoreError> {
    let account = parse_account_id(row.try_get("account_id").map_err(map_sqlx_error)?)?;
    let key_id = SysadminTotpSeedKeyId::parse(
        row.try_get::<String, _>("encryption_key_id")
            .map_err(map_sqlx_error)?,
    )?;
    let nonce = row.try_get("seed_nonce").map_err(map_sqlx_error)?;
    let ciphertext = row.try_get("encrypted_seed").map_err(map_sqlx_error)?;
    let seed =
        EncryptedSysadminTotpSeed::open(key_ring, account.clone(), key_id, nonce, ciphertext)?;
    Ok(PendingSysadminTotpAttestation {
        id: attestation,
        account,
        seed,
    })
}

fn decode_session_row(row: &PgRow) -> Result<SessionRecord, StoreError> {
    let token_hash: String = row.try_get("session_hash").map_err(map_sqlx_error)?;
    let product_role: String = row.try_get("product_role").map_err(map_sqlx_error)?;
    if product_role != "sysadmin" {
        return Err(StoreError::Unavailable(
            "Sysadmin TOTP ceremony returned a non-Sysadmin role".into(),
        ));
    }
    Ok(SessionRecord {
        id: SessionId::from_uuid(row.try_get("session_id").map_err(map_sqlx_error)?),
        token_hash: SessionTokenHash::from_hex(token_hash.trim_end()).map_err(|error| {
            StoreError::Unavailable(format!("stored Sysadmin session hash is invalid: {error}"))
        })?,
        account: parse_account_id(row.try_get("account_id").map_err(map_sqlx_error)?)?,
        product_role: ProductRole::Sysadmin,
        created_at: Timestamp::from_unix_millis(
            row.try_get("created_at_millis").map_err(map_sqlx_error)?,
        ),
        expires_at: Timestamp::from_unix_millis(
            row.try_get("expires_at_millis").map_err(map_sqlx_error)?,
        ),
    })
}
