//! Local-development identity composition.
//!
//! This capability keeps the intentionally local bearer scheme paired with
//! its plain-HTTP cookie transport and prevents production callers from
//! assembling either piece independently.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use learning_data_access::{SessionLifetime, SessionSubject};
use question_model::{TenantId, UserId, UserRole};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::auth::{CookieTransport, IdentityProvider, IdentityProviderError, SessionConfig};
use crate::catalog::{PublicReviewGate, ReviewGateError};
use crate::course::{LocalDevelopmentRosterDirectory, LocalDevelopmentRosterIdentity};

use super::settings::required_env;

const LOCAL_CREDENTIAL_BYTES: usize = 32;
const LOCAL_CREDENTIAL_ENCODED_LEN: usize = 43;
const MAX_LOCAL_LEARNER_ALIAS_BYTES: usize = 128;

/// The private request shape for the sole local-only authentication path.
/// Operator-owned configuration maps its bearer credential to identity; the
/// browser cannot choose tenant, user, display name, or roles.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LocalLoginPresentation {
    pub(super) credential: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalIdentityFile {
    credentials: Vec<LocalIdentityRecord>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalIdentityRecord {
    credential_sha256: String,
    learner_alias: String,
    tenant_id: Uuid,
    user_id: Uuid,
    display_name: String,
    roles: Vec<UserRole>,
}

/// File-backed identity is deliberately binary-private. The only constructor
/// pairs it with [`CookieTransport::LocalHttp`] below, so library consumers
/// cannot accidentally deploy this local bearer scheme over another transport.
pub(super) struct LocalFileIdentityProvider {
    identities: Vec<LocalFileIdentity>,
}

struct LocalFileIdentity {
    credential_hash: [u8; 32],
    learner_alias: String,
    subject: SessionSubject,
}

impl LocalFileIdentityProvider {
    pub(super) fn from_path(path: impl AsRef<Path>) -> Result<Self, IdentityProviderError> {
        let bytes = std::fs::read(path).map_err(|_| {
            IdentityProviderError::Unavailable(
                "local development identity configuration is unreadable".to_string(),
            )
        })?;
        Self::from_json_bytes(&bytes)
    }

    pub(super) fn from_json_bytes(bytes: &[u8]) -> Result<Self, IdentityProviderError> {
        let file: LocalIdentityFile = serde_json::from_slice(bytes).map_err(|_| {
            IdentityProviderError::Unavailable(
                "local development identity configuration is invalid".to_string(),
            )
        })?;
        if file.credentials.is_empty() {
            return Err(IdentityProviderError::Unavailable(
                "local development identity configuration is invalid".to_string(),
            ));
        }

        let mut hashes = HashSet::with_capacity(file.credentials.len());
        let mut aliases = HashSet::with_capacity(file.credentials.len());
        let mut identities = Vec::with_capacity(file.credentials.len());
        for record in file.credentials {
            if record.tenant_id.is_nil() || record.user_id.is_nil() {
                return Err(IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                ));
            }
            let credential_hash =
                decode_lowercase_sha256(&record.credential_sha256).ok_or_else(|| {
                    IdentityProviderError::Unavailable(
                        "local development identity configuration is invalid".to_string(),
                    )
                })?;
            if !hashes.insert(credential_hash) {
                return Err(IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                ));
            }
            let learner_alias =
                validated_local_learner_alias(&record.learner_alias).ok_or_else(|| {
                    IdentityProviderError::Unavailable(
                        "local development identity configuration is invalid".to_string(),
                    )
                })?;
            if !aliases.insert(learner_alias.clone()) {
                return Err(IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                ));
            }
            let subject = SessionSubject::new(
                TenantId::from_uuid(record.tenant_id),
                UserId::from_uuid(record.user_id),
                record.display_name,
                record.roles,
            )
            .map_err(|_| {
                IdentityProviderError::Unavailable(
                    "local development identity configuration is invalid".to_string(),
                )
            })?;
            identities.push(LocalFileIdentity {
                credential_hash,
                learner_alias,
                subject,
            });
        }
        Ok(Self { identities })
    }

    pub(super) fn roster_directory(&self) -> LocalDevelopmentRosterDirectory {
        LocalDevelopmentRosterDirectory::new(self.identities.iter().map(|identity| {
            (
                identity.learner_alias.clone(),
                LocalDevelopmentRosterIdentity {
                    tenant: identity.subject.tenant(),
                    user: identity.subject.user(),
                    display_name: identity.subject.display_name().to_string(),
                    roles: identity.subject.roles().to_vec(),
                },
            )
        }))
        .expect("validated local identity configuration has unique aliases")
    }
}

fn validated_local_learner_alias(value: &str) -> Option<String> {
    (1..=MAX_LOCAL_LEARNER_ALIAS_BYTES)
        .contains(&value.len())
        .then_some(())?;
    value
        .bytes()
        .all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
        .then(|| value.to_string())
}

#[async_trait::async_trait]
impl IdentityProvider for LocalFileIdentityProvider {
    type Presentation = LocalLoginPresentation;

    async fn verify(
        &self,
        presentation: &Self::Presentation,
    ) -> Result<SessionSubject, IdentityProviderError> {
        let credential = canonical_local_credential(&presentation.credential)
            .ok_or(IdentityProviderError::Rejected)?;
        let presented_hash: [u8; 32] = Sha256::digest(credential).into();
        // Compare every configured identity so the configured record cannot
        // affect lookup timing. Only raw, validated 32-byte bearer material is
        // hashed; the base64url transport spelling is never persisted.
        let mut matched: Option<&SessionSubject> = None;
        for identity in &self.identities {
            if bool::from(identity.credential_hash.ct_eq(&presented_hash)) {
                matched = Some(&identity.subject);
            }
        }
        matched.cloned().ok_or(IdentityProviderError::Rejected)
    }
}

fn canonical_local_credential(value: &str) -> Option<[u8; LOCAL_CREDENTIAL_BYTES]> {
    if value.len() != LOCAL_CREDENTIAL_ENCODED_LEN {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    let credential: [u8; LOCAL_CREDENTIAL_BYTES] = decoded.try_into().ok()?;
    (URL_SAFE_NO_PAD.encode(credential) == value).then_some(credential)
}

fn decode_lowercase_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(bytes)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

pub(super) fn local_development_session_config() -> SessionConfig {
    SessionConfig::new(
        SessionLifetime::from_seconds(8 * 60 * 60)
            .expect("eight-hour local development session lifetime is positive"),
        CookieTransport::LocalHttp,
    )
}

/// An unforgeable local-only pairing of the file-backed bearer provider and
/// the deliberately insecure plain-HTTP cookie policy.
pub(super) struct LocalDevelopmentAuthentication {
    pub(super) provider: Arc<LocalFileIdentityProvider>,
    pub(super) roster_directory: Arc<LocalDevelopmentRosterDirectory>,
    pub(super) session_config: SessionConfig,
}

pub(super) fn local_development_authentication_from_env() -> Result<LocalDevelopmentAuthentication>
{
    let provider = required_env("PLE_AUTH_PROVIDER")?;
    let development_flag = required_env("PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH")?;
    let path = required_env("PLE_LOCAL_AUTH_FILE")?;
    local_development_authentication(&provider, &development_flag, &path)
}

pub(super) fn local_development_authentication(
    provider: &str,
    development_flag: &str,
    path: impl AsRef<Path>,
) -> Result<LocalDevelopmentAuthentication> {
    if provider != "local-file" {
        bail!(
            "PLE_AUTH_PROVIDER={provider:?} is not available; deployment requires a configured institution OIDC provider"
        );
    }
    if development_flag != "1" {
        bail!(
            "PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH must be exactly 1 when PLE_AUTH_PROVIDER=local-file"
        );
    }
    let provider = LocalFileIdentityProvider::from_path(path).map_err(|_| {
        anyhow::anyhow!("local development identity configuration could not be loaded")
    })?;
    let provider = Arc::new(provider);
    Ok(LocalDevelopmentAuthentication {
        roster_directory: Arc::new(provider.roster_directory()),
        provider,
        session_config: local_development_session_config(),
    })
}

/// Local work must not accidentally publish shared educational content.
#[derive(Debug, Clone, Copy)]
pub(super) struct LocalDevelopmentReviewGate;

#[async_trait::async_trait]
impl PublicReviewGate for LocalDevelopmentReviewGate {
    async fn allows_publication(
        &self,
        _tenant: learning_data_access::TenantContext,
        _publisher: UserId,
        _draft: &learning_data_access::DraftRecord,
    ) -> Result<bool, ReviewGateError> {
        Ok(false)
    }
}
