//! Encrypted server-only state for contracted iMathAS launches.

use base64::Engine as _;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use learning_data_access::{ExternalToolBinding, StudentWorkRoutingBinding, TenantContext};
use question_model::{QuestionAttempt, UserId};

use crate::run::RunBackendError;

use super::{map_adapter_error, map_store_error};

/// Server configuration for encrypted contracted-launch state. This is a
/// distinct secret from the signed provider launch and broker correlation
/// keys. It is replica-stable and never becomes a browser value.
pub struct LaunchStateAead {
    cipher: XChaCha20Poly1305,
    cookie_cipher: XChaCha20Poly1305,
    pub(super) adapter_codec: adapter_imathas::broker_provider::LaunchSessionCodec,
}

impl std::fmt::Debug for LaunchStateAead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LaunchStateAead(REDACTED)")
    }
}

impl LaunchStateAead {
    pub fn from_server_secret(secret: [u8; 32]) -> Result<Self, RunBackendError> {
        if secret.iter().all(|byte| *byte == 0) {
            return Err(RunBackendError::Invalid(
                "iMathAS launch-state secret is invalid".into(),
            ));
        }
        let mut cookie_key = sha2::Sha256::new();
        use sha2::Digest as _;
        cookie_key.update(b"ple:imathas:launch-cookie:v2");
        cookie_key.update(secret);
        let cookie_key: [u8; 32] = cookie_key.finalize().into();
        Ok(Self {
            cipher: XChaCha20Poly1305::new((&secret).into()),
            cookie_cipher: XChaCha20Poly1305::new((&cookie_key).into()),
            adapter_codec:
                adapter_imathas::broker_provider::LaunchSessionCodec::from_server_secret(secret)
                    .map_err(map_adapter_error)?,
        })
    }

    /// Versioned bounded ciphertext. The Store receives only this value; the
    /// adapter's authenticated launch codec remains entirely inside it.
    pub fn seal(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, RunBackendError> {
        if plaintext.is_empty() || plaintext.len() > 8_192 || aad.is_empty() || aad.len() > 2_048 {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch state".into(),
            ));
        }
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| {
            RunBackendError::Unavailable("iMathAS launch entropy is unavailable".into())
        })?;
        let nonce = XNonce::try_from(nonce.as_slice())
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch state".into()))?;
        let encrypted = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| RunBackendError::Unavailable("iMathAS launch encryption failed".into()))?;
        let mut result = Vec::with_capacity(1 + nonce.len() + encrypted.len());
        result.push(1);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);
        Ok(result)
    }

    pub fn open(&self, ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, RunBackendError> {
        if !(41..=8_256).contains(&ciphertext.len())
            || aad.is_empty()
            || aad.len() > 2_048
            || ciphertext[0] != 1
        {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch state".into(),
            ));
        }
        let nonce = XNonce::try_from(&ciphertext[1..25])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch state".into()))?;
        self.cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext[25..],
                    aad,
                },
            )
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch state".into()))
    }

    pub(super) fn seal_adapter_session(
        &self,
        session: &adapter_imathas::broker_provider::ContractedLaunchSession,
        aad: &[u8],
    ) -> Result<Vec<u8>, RunBackendError> {
        let inner = self
            .adapter_codec
            .seal(session)
            .map_err(map_adapter_error)?;
        self.seal(inner.to_storage_value().as_bytes(), aad)
    }

    /// Fixed-name cookie codec. Its opaque plaintext is exactly the Store
    /// session UUID and opaque token, never an upstream handle or provider
    /// credential. It uses a distinct derived key/domain from provider state.
    pub fn seal_cookie(
        &self,
        id: uuid::Uuid,
        token: &learning_data_access::ExternalToolLaunchToken,
        aad: &[u8],
    ) -> Result<String, RunBackendError> {
        let token = token.encode_cookie_value();
        let mut plain = Vec::with_capacity(59);
        plain.extend_from_slice(id.as_bytes());
        plain.extend_from_slice(token.as_bytes());
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| {
            RunBackendError::Unavailable("iMathAS launch entropy is unavailable".into())
        })?;
        let nonce_value = XNonce::try_from(nonce.as_slice())
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let encrypted = self
            .cookie_cipher
            .encrypt(&nonce_value, Payload { msg: &plain, aad })
            .map_err(|_| RunBackendError::Unavailable("iMathAS launch encryption failed".into()))?;
        let mut wire = Vec::with_capacity(1 + 24 + encrypted.len());
        wire.push(1);
        wire.extend_from_slice(&nonce);
        wire.extend_from_slice(&encrypted);
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wire))
    }

    pub fn open_cookie(
        &self,
        wire: &str,
        aad: &[u8],
    ) -> Result<(uuid::Uuid, learning_data_access::ExternalToolLaunchToken), RunBackendError> {
        if wire.len() > 256
            || !wire
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch cookie".into(),
            ));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(wire)
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        if !(1 + 24 + 16..=256).contains(&bytes.len()) || bytes[0] != 1 {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch cookie".into(),
            ));
        }
        let nonce = XNonce::try_from(&bytes[1..25])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let plain = self
            .cookie_cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &bytes[25..],
                    aad,
                },
            )
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        if plain.len() != 59 {
            return Err(RunBackendError::Invalid(
                "invalid iMathAS launch cookie".into(),
            ));
        }
        let id = uuid::Uuid::from_slice(&plain[..16])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let token = std::str::from_utf8(&plain[16..])
            .map_err(|_| RunBackendError::Invalid("invalid iMathAS launch cookie".into()))?;
        let token = learning_data_access::ExternalToolLaunchToken::parse_cookie_value(token)
            .map_err(map_store_error)?;
        Ok((id, token))
    }
}

/// Canonical associated data prevents a ciphertext copied across tenant,
/// learner, attempt, immutable source, or integration-profile boundaries from
/// being restored. It deliberately contains no secret; its only job is exact
/// cryptographic binding.
#[allow(dead_code)]
pub(super) fn launch_state_aad(
    context: TenantContext,
    actor: UserId,
    student_work_binding: StudentWorkRoutingBinding,
    attempt: &QuestionAttempt,
    binding: &ExternalToolBinding,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(512);
    result.extend_from_slice(b"ple:imathas:launch-state:v2\0");
    for value in [
        context.tenant_id().as_uuid().to_string(),
        actor.as_uuid().to_string(),
        student_work_binding.course.as_uuid().to_string(),
        student_work_binding.assignment.as_uuid().to_string(),
        attempt.id.as_uuid().to_string(),
        attempt.problem.as_uuid().to_string(),
        attempt.question_version.as_uuid().to_string(),
        attempt.seed.to_string(),
        binding.provider.clone(),
        binding.source_object.as_uuid().to_string(),
        binding.source_sha256.clone(),
        binding.integration_profile.clone(),
    ] {
        result.extend_from_slice(value.as_bytes());
        result.push(0);
    }
    result
}

#[allow(dead_code)]
pub(crate) fn launch_cookie_aad(
    context: TenantContext,
    actor: UserId,
    student_work_binding: StudentWorkRoutingBinding,
    attempt: question_model::QuestionAttemptId,
) -> Vec<u8> {
    format!(
        "ple:imathas:launch-cookie:v2\\0{}\\0{}\\0{}\\0{}\\0{}\\0",
        context.tenant_id().as_uuid(),
        actor.as_uuid(),
        student_work_binding.course.as_uuid(),
        student_work_binding.assignment.as_uuid(),
        attempt.as_uuid(),
    )
    .into_bytes()
}

/// Encodes the one fixed-name HttpOnly cookie after Store creation. The value
/// is intentionally not a DTO and should be written directly to `Set-Cookie`.
#[allow(dead_code)]
pub(crate) fn launch_cookie_value(
    aead: &LaunchStateAead,
    context: TenantContext,
    actor: UserId,
    student_work_binding: StudentWorkRoutingBinding,
    attempt: question_model::QuestionAttemptId,
    created: &learning_data_access::CreatedExternalToolLaunchSession,
) -> Result<String, RunBackendError> {
    aead.seal_cookie(
        created.id,
        &created.token,
        &launch_cookie_aad(context, actor, student_work_binding, attempt),
    )
}
