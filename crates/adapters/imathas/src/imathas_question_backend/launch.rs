//! iMathAS private launch-state encoding and grading-context HMAC.

use hmac::{Hmac, KeyInit, Mac};
use learning_data_access::ImathasQuestionBackendStatePlaintext;
use sha2::Sha256;

use super::{ImathasAdapterError, ImathasLaunchReference};
use crate::cache::hex;

const IMATHAS_LAUNCH_STATE_VERSION: u8 = 1;
const MAX_IMATHAS_LAUNCH_HANDLE_BYTES: usize = 128;

/// Bounded, versioned, server-only iMathAS state stored inside LDA AEAD
/// plaintext. It deliberately contains only the iMathAS proxy handle.
pub struct ImathasLaunchState {
    handle: ImathasLaunchReference,
}

impl ImathasLaunchState {
    pub fn from_launch_handle(handle: ImathasLaunchReference) -> Self {
        Self { handle }
    }

    pub fn encode(&self) -> Result<ImathasQuestionBackendStatePlaintext, ImathasAdapterError> {
        let handle = self.handle.protected_value().as_bytes();
        if handle.is_empty() || handle.len() > MAX_IMATHAS_LAUNCH_HANDLE_BYTES {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        }
        let mut bytes = Vec::with_capacity(handle.len() + 2);
        bytes.push(IMATHAS_LAUNCH_STATE_VERSION);
        bytes.push(handle.len() as u8);
        bytes.extend_from_slice(handle);
        ImathasQuestionBackendStatePlaintext::from_versioned_adapter_bytes(bytes)
            .map_err(|_| ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)
    }

    pub fn decode(
        value: &ImathasQuestionBackendStatePlaintext,
    ) -> Result<Self, ImathasAdapterError> {
        let bytes = value.as_bytes();
        let Some((&version, rest)) = bytes.split_first() else {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        };
        if version != IMATHAS_LAUNCH_STATE_VERSION || rest.is_empty() {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        }
        let length = usize::from(rest[0]);
        let Some(handle) = rest.get(1..) else {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        };
        if length == 0 || length > MAX_IMATHAS_LAUNCH_HANDLE_BYTES || handle.len() != length {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        }
        let handle = std::str::from_utf8(handle)
            .map_err(|_| ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)?;
        Ok(Self {
            handle: ImathasLaunchReference::from_server_handle(handle).map_err(|_| {
                ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication
            })?,
        })
    }

    pub(crate) fn handle(&self) -> &ImathasLaunchReference {
        &self.handle
    }
}

impl std::fmt::Debug for ImathasLaunchState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasLaunchState(REDACTED)")
    }
}

/// HMAC-SHA-256 authentication for the exact grading context and LDA challenge.
/// This is authentication only; LDA owns protected-state encryption and storage.
pub struct ImathasSessionAuthenticationCodec {
    secret: [u8; 32],
}

impl std::fmt::Debug for ImathasSessionAuthenticationCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImathasSessionAuthenticationCodec(REDACTED)")
    }
}

impl ImathasSessionAuthenticationCodec {
    pub fn from_server_secret(secret: [u8; 32]) -> Result<Self, ImathasAdapterError> {
        if secret.iter().all(|byte| *byte == 0) {
            return Err(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication);
        }
        Ok(Self { secret })
    }

    pub fn authenticate_for_lda(
        &self,
        grading_context: &learning_data_access::ImathasGradingContext,
        challenge: &learning_data_access::ImathasQuestionBackendSessionChallenge,
    ) -> learning_data_access::ImathasQuestionBackendSessionAuthentication {
        let payload = grading_context.authentication_payload_v1();
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.secret).expect("fixed key");
        mac.update(b"ple:imathas:imathas-question-backend-launch-session-authentication:v1");
        mac.update(&payload);
        mac.update(challenge.as_bytes());
        learning_data_access::ImathasQuestionBackendSessionAuthentication::from_server_value(
            format!("{}.{}", hex(&payload), hex(&mac.finalize().into_bytes())),
        )
        .expect("fixed HMAC encoding is valid")
    }

    /// Recomputes the row-530 authentication before the adapter performs
    /// iMathAS I/O. LDA persists the authenticated value; this adapter owns
    /// only verification of the exact iMathAS Grading Context and iMathAS Session Challenge
    /// binding at its iMathAS boundary.
    pub fn verifies_for_lda(
        &self,
        grading_context: &learning_data_access::ImathasGradingContext,
        challenge: &learning_data_access::ImathasQuestionBackendSessionChallenge,
        authentication: &learning_data_access::ImathasQuestionBackendSessionAuthentication,
    ) -> bool {
        crate::constant_time_eq(
            self.authenticate_for_lda(grading_context, challenge)
                .as_str()
                .as_bytes(),
            authentication.as_str().as_bytes(),
        )
    }
}
