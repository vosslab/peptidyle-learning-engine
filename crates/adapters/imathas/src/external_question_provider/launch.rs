//! Authenticated protected server launch-state persistence.

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use question_model::generation::QuestionSeed;
use question_model::{QuestionAttemptId, Timestamp};
use sha2::Sha256;
use uuid::Uuid;

use super::{
    ContractedLaunchSession, ExternalToolGradingContext, ExternalToolLaunchReference,
    ImathasAdapterError, LaunchLedgerStorageParts, ScoredEmbedFailure, ScoredEmbedLaunchLedger,
    map_transport,
};
use crate::cache::{external_tool_grading_context_payload, hex};
use crate::{ExternalToolLaunchChallenge, ExternalToolLaunchSessionAuthentication};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractedLaunchExpectation {
    pub(crate) binding: ExternalToolGradingContext,
    provider_key: String,
    source_digest: String,
}

impl ContractedLaunchExpectation {
    pub fn new(
        binding: ExternalToolGradingContext,
        provider_key: impl Into<String>,
        source_digest: impl Into<String>,
    ) -> Result<Self, ImathasAdapterError> {
        let provider_key = provider_key.into();
        let source_digest = source_digest.into();
        if !valid_provider(&provider_key) || !valid_digest(&source_digest) {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        Ok(Self {
            binding,
            provider_key,
            source_digest,
        })
    }
}

/// Opaque non-serde launch state for protected server storage.
#[derive(Clone, PartialEq, Eq)]
pub struct PersistedContractedLaunchSession(String);

impl std::fmt::Debug for PersistedContractedLaunchSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PersistedContractedLaunchSession(REDACTED)")
    }
}

impl PersistedContractedLaunchSession {
    pub fn to_storage_value(&self) -> String {
        self.0.clone()
    }

    pub fn from_storage_value(value: &str) -> Result<Self, ImathasAdapterError> {
        if value.is_empty() || value.len() > 8192 || !value.is_ascii() {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
        if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes) != value {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        Ok(Self(value.to_owned()))
    }
}

/// HMAC-SHA-256 codec for one complete protected External Tool Launch Session.
///
/// This server-only codec creates the authentication value from the exact
/// grading context and fresh challenge, then authenticates and restores every
/// persisted launch fact before any provider result is accepted. ASVS 2.2.1,
/// 2.3.1, and 7.2.1 require this trusted-server boundary.
pub struct ExternalToolLaunchSessionAuthenticationCodec {
    secret: [u8; 32],
}

impl std::fmt::Debug for ExternalToolLaunchSessionAuthenticationCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExternalToolLaunchSessionAuthenticationCodec(REDACTED)")
    }
}

impl ExternalToolLaunchSessionAuthenticationCodec {
    pub fn from_server_secret(secret: [u8; 32]) -> Result<Self, ImathasAdapterError> {
        if secret.iter().all(|byte| *byte == 0) {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        Ok(Self { secret })
    }

    /// Mints a canonical server-held authentication value for one exact
    /// grading context and challenge. Its opaque value remains inside the
    /// protected launch record and is never a browser/provider DTO.
    pub fn authenticate(
        &self,
        binding: &ExternalToolGradingContext,
        challenge: &ExternalToolLaunchChallenge,
    ) -> ExternalToolLaunchSessionAuthentication {
        let payload = external_tool_grading_context_payload(binding);
        let mac = self.authentication_mac(&payload, challenge);
        ExternalToolLaunchSessionAuthentication(format!("{}.{}", hex(&payload), hex(&mac)))
    }

    pub fn seal(
        &self,
        session: &ContractedLaunchSession,
    ) -> Result<PersistedContractedLaunchSession, ImathasAdapterError> {
        let p = session.ledger.storage_parts();
        let mut data = Vec::with_capacity(512);
        data.extend_from_slice(b"PLEIMLS1");
        data.push(1);
        write_binding(&mut data, &p.binding)?;
        for value in [
            &p.provider_key,
            &p.provider_question_id,
            &p.source_digest,
            &p.profile,
        ] {
            write_text(&mut data, value)?;
        }
        data.extend_from_slice(&p.provider_seed.to_be_bytes());
        data.extend_from_slice(&p.expires_at.as_unix_millis().to_be_bytes());
        write_text(&mut data, &p.launch_session_authentication)?;
        data.extend_from_slice(&p.challenge);
        write_text(&mut data, &p.binding_digest)?;
        data.push(u8::from(p.consumed));
        write_text(&mut data, session.handle.protected_value())?;
        data.extend_from_slice(&self.mac(&data));
        Ok(PersistedContractedLaunchSession(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data),
        ))
    }

    pub fn restore(
        &self,
        value: &PersistedContractedLaunchSession,
        expected: &ContractedLaunchExpectation,
    ) -> Result<ContractedLaunchSession, ImathasAdapterError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&value.0)
            .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
        if bytes.len() < 256 || bytes.len() > 6144 {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        let (data, mac) = bytes.split_at(bytes.len() - 32);
        if !constant_time_equal(&self.mac(data), mac) {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        let mut cursor = Cursor::new(data);
        if cursor.take(8)? != b"PLEIMLS1" || cursor.u8()? != 1 {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        let binding = read_binding(&mut cursor)?;
        let provider_key = cursor.text()?;
        let provider_question_id = cursor.text()?;
        let source_digest = cursor.text()?;
        let profile = cursor.text()?;
        let provider_seed = cursor.u16()?;
        let expires_at = Timestamp::from_unix_millis(cursor.i64()?);
        let launch_session_authentication = cursor.text()?;
        let challenge: [u8; 32] = cursor
            .take(32)?
            .try_into()
            .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
        let binding_digest = cursor.text()?;
        let consumed = match cursor.u8()? {
            0 => false,
            1 => true,
            _ => return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication),
        };
        let handle = ExternalToolLaunchReference::from_server_handle(cursor.text()?)
            .map_err(map_transport)?;
        if !cursor.finished()
            || binding != expected.binding
            || provider_key != expected.provider_key
            || source_digest != expected.source_digest
        {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        let ledger = ScoredEmbedLaunchLedger::from_storage_parts(LaunchLedgerStorageParts {
            binding,
            provider_key,
            provider_question_id,
            source_digest,
            profile,
            provider_seed,
            expires_at,
            launch_session_authentication,
            challenge,
            binding_digest,
            consumed,
        })
        .map_err(ScoredEmbedFailure::into_adapter_error)?;
        Ok(ContractedLaunchSession { ledger, handle })
    }

    fn mac(&self, data: &[u8]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).expect("fixed key");
        mac.update(b"ple:imathas:launch-session-codec:v1");
        mac.update(data);
        mac.finalize().into_bytes().into()
    }

    fn authentication_mac(
        &self,
        payload: &[u8],
        challenge: &ExternalToolLaunchChallenge,
    ) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).expect("fixed key");
        mac.update(b"ple:imathas:external-tool-launch-session-authentication:v1");
        mac.update(payload);
        mac.update(&challenge.0);
        mac.finalize().into_bytes().into()
    }
}

fn write_binding(
    data: &mut Vec<u8>,
    binding: &ExternalToolGradingContext,
) -> Result<(), ImathasAdapterError> {
    data.extend_from_slice(binding.attempt.as_uuid().as_bytes());
    write_text(data, &binding.question_revision.question_id.to_string())?;
    data.extend_from_slice(
        &binding
            .question_revision
            .revision_number
            .get()
            .to_be_bytes(),
    );
    data.extend_from_slice(&binding.seed.value().to_be_bytes());
    Ok(())
}

fn read_binding(
    cursor: &mut Cursor<'_>,
) -> Result<ExternalToolGradingContext, ImathasAdapterError> {
    let attempt = QuestionAttemptId::from_uuid(Uuid::from_bytes(
        cursor
            .take(16)?
            .try_into()
            .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?,
    ));
    let question_id = cursor
        .text()?
        .parse()
        .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
    let revision_number = question_model::QuestionRevisionNumber::new(cursor.u32()?)
        .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
    Ok(ExternalToolGradingContext {
        attempt,
        question_revision: question_model::QuestionRevisionReference {
            question_id,
            revision_number,
        },
        seed: QuestionSeed::new(cursor.u64()?),
    })
}

fn write_text(data: &mut Vec<u8>, value: &str) -> Result<(), ImathasAdapterError> {
    if value.is_empty() || value.len() > 512 || !value.is_ascii() {
        return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
    }
    let length: u16 = value
        .len()
        .try_into()
        .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
    data.extend_from_slice(&length.to_be_bytes());
    data.extend_from_slice(value.as_bytes());
    Ok(())
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], ImathasAdapterError> {
        let end = self
            .at
            .checked_add(count)
            .ok_or(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
        let out = self
            .data
            .get(self.at..end)
            .ok_or(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
        self.at = end;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, ImathasAdapterError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ImathasAdapterError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().map_err(
            |_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication,
        )?))
    }
    fn u32(&mut self) -> Result<u32, ImathasAdapterError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().map_err(
            |_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication,
        )?))
    }
    fn u64(&mut self) -> Result<u64, ImathasAdapterError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().map_err(
            |_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication,
        )?))
    }
    fn i64(&mut self) -> Result<i64, ImathasAdapterError> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().map_err(
            |_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication,
        )?))
    }
    fn text(&mut self) -> Result<String, ImathasAdapterError> {
        let length = usize::from(self.u16()?);
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication)?;
        if value.is_empty() || !value.is_ascii() {
            return Err(ImathasAdapterError::InvalidExternalToolLaunchSessionAuthentication);
        }
        Ok(value.to_owned())
    }
    fn finished(&self) -> bool {
        self.at == self.data.len()
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |value, (a, b)| value | (a ^ b))
            == 0
}

fn valid_provider(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
