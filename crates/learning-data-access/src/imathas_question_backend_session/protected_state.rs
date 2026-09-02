use std::collections::BTreeMap;

use super::*;
use question_model::QuestionGradingRule;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

pub const MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES: usize = 64 * 1024;
pub const MAX_IMATHAS_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES: usize =
    MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES - 16;
pub(super) const IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES: usize = 24;
const IMATHAS_QUESTION_BACKEND_STATE_AAD_VERSION: u8 = 1;

#[derive(Clone, PartialEq, Eq)]
pub struct ImathasQuestionBackendStatePlaintext(Vec<u8>);

impl ImathasQuestionBackendStatePlaintext {
    pub fn from_versioned_adapter_bytes(bytes: Vec<u8>) -> Result<Self, StoreError> {
        if bytes.is_empty()
            || bytes[0] == 0
            || bytes.len() > MAX_IMATHAS_QUESTION_BACKEND_STATE_PLAINTEXT_BYTES
        {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend imathas_question_backend state exceeds its bounded encoding".into(),
            ));
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for ImathasQuestionBackendStatePlaintext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasQuestionBackendStatePlaintext")
            .field("bytes", &"[redacted]")
            .field("len", &self.0.len())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImathasQuestionBackendStateKeyId(String);

impl ImathasQuestionBackendStateKeyId {
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        question_model::ImathasDeploymentReference::new(value)
            .map(String::from)
            .map(Self)
            .map_err(|_| {
                StoreError::InvalidRecord(
                    "iMathAS Question Backend imathas_question_backend-state key ID is invalid"
                        .into(),
                )
            })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ImathasQuestionBackendStateKeyId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendStateKeyId([redacted])")
    }
}

pub struct ImathasQuestionBackendStateKeyRing {
    active: ImathasQuestionBackendStateKeyId,
    keys: BTreeMap<ImathasQuestionBackendStateKeyId, [u8; 32]>,
}

impl ImathasQuestionBackendStateKeyRing {
    pub fn new(
        active: ImathasQuestionBackendStateKeyId,
        active_key: [u8; 32],
        retiring: impl IntoIterator<Item = (ImathasQuestionBackendStateKeyId, [u8; 32])>,
    ) -> Result<Self, StoreError> {
        let mut keys = BTreeMap::new();
        keys.insert(active.clone(), active_key);
        for (id, key) in retiring {
            if keys.insert(id, key).is_some() {
                return Err(StoreError::InvalidRecord(
                    "iMathAS Question Backend imathas_question_backend-state key IDs must be unique"
                        .into(),
                ));
            }
        }
        Ok(Self { active, keys })
    }

    fn active_key(&self) -> (&ImathasQuestionBackendStateKeyId, &[u8; 32]) {
        (
            self.active.as_ref(),
            self.keys.get(&self.active).expect("active key exists"),
        )
    }

    fn key(&self, id: &ImathasQuestionBackendStateKeyId) -> Result<&[u8; 32], StoreError> {
        self.keys.get(id).ok_or_else(|| {
            StoreError::InvalidRecord(
                "iMathAS Question Backend imathas_question_backend-state key is unavailable".into(),
            )
        })
    }
}

impl AsRef<ImathasQuestionBackendStateKeyId> for ImathasQuestionBackendStateKeyId {
    fn as_ref(&self) -> &ImathasQuestionBackendStateKeyId {
        self
    }
}

impl std::fmt::Debug for ImathasQuestionBackendStateKeyRing {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ImathasQuestionBackendStateKeyRing([redacted])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImathasQuestionBackendStateCipher {
    key_id: ImathasQuestionBackendStateKeyId,
    nonce: [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
    pub(super) ciphertext: Vec<u8>,
}

/// Exact encrypted imathas_question_backend-state row facts for Store reconstruction.
#[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
pub(crate) struct ImathasQuestionBackendStateCipherStorageParts {
    pub(crate) key_id: ImathasQuestionBackendStateKeyId,
    pub(crate) nonce: Vec<u8>,
    pub(crate) ciphertext: Vec<u8>,
}

pub(super) trait ImathasQuestionBackendStateNonceSource {
    fn fill_nonce(
        &self,
        nonce: &mut [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
    ) -> Result<(), StoreError>;
}

struct OperatingSystemNonceSource;

impl ImathasQuestionBackendStateNonceSource for OperatingSystemNonceSource {
    fn fill_nonce(
        &self,
        nonce: &mut [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
    ) -> Result<(), StoreError> {
        getrandom::fill(nonce).map_err(|_| {
            StoreError::Unavailable(
                "iMathAS Question Backend imathas_question_backend-state nonce randomness unavailable".into(),
            )
        })
    }
}

impl ImathasQuestionBackendStateCipher {
    pub fn seal(
        key_ring: &ImathasQuestionBackendStateKeyRing,
        session: &ImathasQuestionBackendSession,
        plaintext: &ImathasQuestionBackendStatePlaintext,
    ) -> Result<Self, StoreError> {
        Self::seal_with_nonce_source(key_ring, session, plaintext, &OperatingSystemNonceSource)
    }

    pub(super) fn seal_with_nonce_source(
        key_ring: &ImathasQuestionBackendStateKeyRing,
        session: &ImathasQuestionBackendSession,
        plaintext: &ImathasQuestionBackendStatePlaintext,
        nonce_source: &impl ImathasQuestionBackendStateNonceSource,
    ) -> Result<Self, StoreError> {
        let (key_id, key) = key_ring.active_key();
        let mut nonce = [0_u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES];
        // ASVS 11.2.1 and 11.5.1: a fresh OS-CSPRNG nonce for each encryption.
        nonce_source.fill_nonce(&mut nonce)?;
        let ciphertext = XChaCha20Poly1305::new(key.into())
            .encrypt(
                &XNonce::try_from(nonce.as_slice()).map_err(|_| {
                    StoreError::InvalidRecord(
                        "iMathAS Question Backend imathas_question_backend-state nonce is invalid"
                            .into(),
                    )
                })?,
                Payload {
                    msg: plaintext.as_bytes(),
                    aad: &imathas_question_backend_state_aad(session),
                },
            )
            .map_err(|_| {
                StoreError::InvalidRecord(
                    "iMathAS Question Backend imathas_question_backend state cannot be encrypted"
                        .into(),
                )
            })?;
        if ciphertext.len() > MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend imathas_question_backend-state ciphertext exceeds its bound".into(),
            ));
        }
        Ok(Self {
            key_id: key_id.clone(),
            nonce,
            ciphertext,
        })
    }

    pub fn open(
        &self,
        key_ring: &ImathasQuestionBackendStateKeyRing,
        session: &ImathasQuestionBackendSession,
    ) -> Result<ImathasQuestionBackendStatePlaintext, StoreError> {
        if self.ciphertext.is_empty()
            || self.ciphertext.len() > MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES
        {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend imathas_question_backend-state ciphertext is invalid"
                    .into(),
            ));
        }
        let plaintext = XChaCha20Poly1305::new(key_ring.key(&self.key_id)?.into())
            .decrypt(
                &XNonce::try_from(self.nonce.as_slice()).map_err(|_| {
                    StoreError::InvalidRecord(
                        "iMathAS Question Backend imathas_question_backend-state nonce is invalid"
                            .into(),
                    )
                })?,
                Payload {
                    msg: &self.ciphertext,
                    aad: &imathas_question_backend_state_aad(session),
                },
            )
            .map_err(|_| {
                StoreError::InvalidRecord(
                    "iMathAS Question Backend imathas_question_backend state cannot be authenticated"
                        .into(),
                )
            })?;
        ImathasQuestionBackendStatePlaintext::from_versioned_adapter_bytes(plaintext)
    }

    #[allow(dead_code)] // Used by the feature-gated PostgreSQL Store.
    pub(crate) fn from_storage_parts(
        parts: ImathasQuestionBackendStateCipherStorageParts,
    ) -> Result<Self, StoreError> {
        let nonce = parts.nonce.try_into().map_err(|_| {
            StoreError::InvalidRecord(
                "iMathAS Question Backend imathas_question_backend-state nonce is invalid".into(),
            )
        })?;
        Self::from_row_parts(parts.key_id, nonce, parts.ciphertext)
    }

    #[allow(dead_code)] // Retained while the B3 Store migrates to typed storage parts.
    pub(crate) fn from_row_parts(
        key_id: ImathasQuestionBackendStateKeyId,
        nonce: [u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES],
        ciphertext: Vec<u8>,
    ) -> Result<Self, StoreError> {
        if ciphertext.len() < 17
            || ciphertext.len() > MAX_IMATHAS_QUESTION_BACKEND_STATE_CIPHERTEXT_BYTES
        {
            return Err(StoreError::InvalidRecord(
                "iMathAS Question Backend imathas_question_backend-state ciphertext is invalid"
                    .into(),
            ));
        }
        Ok(Self {
            key_id,
            nonce,
            ciphertext,
        })
    }

    pub fn key_id(&self) -> &ImathasQuestionBackendStateKeyId {
        &self.key_id
    }
    pub fn nonce(&self) -> &[u8; IMATHAS_QUESTION_BACKEND_STATE_NONCE_BYTES] {
        &self.nonce
    }
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

impl std::fmt::Debug for ImathasQuestionBackendStateCipher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImathasQuestionBackendStateCipher")
            .field("key_id", &"[redacted]")
            .field("nonce", &"[redacted]")
            .field("ciphertext", &"[redacted]")
            .field("ciphertext_len", &self.ciphertext.len())
            .finish()
    }
}

fn imathas_question_backend_state_aad(session: &ImathasQuestionBackendSession) -> Vec<u8> {
    let mut aad = vec![IMATHAS_QUESTION_BACKEND_STATE_AAD_VERSION];
    let question_grading_rule = question_grading_rule_aad(session.question_grading_rule());
    for value in [
        session.reference.as_uuid().as_bytes().as_slice(),
        session.account.as_uuid().as_bytes().as_slice(),
        session.course.as_uuid().as_bytes().as_slice(),
        session.assignment.as_uuid().as_bytes().as_slice(),
        session
            .grading_context
            .question_attempt()
            .as_uuid()
            .as_bytes()
            .as_slice(),
        session
            .grading_context
            .question_revision()
            .question_id
            .to_string()
            .as_bytes(),
        &session
            .grading_context
            .question_revision()
            .revision_number
            .get()
            .to_be_bytes(),
        session
            .imathas_question_backend_binding
            .deployment_reference()
            .as_str()
            .as_bytes(),
        session
            .imathas_question_backend_binding
            .item_reference()
            .as_str()
            .as_bytes(),
        session.source_object.object.as_uuid().as_bytes().as_slice(),
        session.source_object_checksum.as_str().as_bytes(),
        session
            .imathas_question_backend_binding
            .profile()
            .as_str()
            .as_bytes(),
        &session
            .grading_context
            .question_seed()
            .value()
            .to_be_bytes(),
        &question_grading_rule,
        session.response_checksum.as_bytes(),
        session.challenge.as_bytes(),
        session.authentication.as_str().as_bytes(),
        session.imathas_launch_binding_checksum.as_str().as_bytes(),
        &session.issued_at.as_unix_millis().to_be_bytes(),
        &session.expires_at.as_unix_millis().to_be_bytes(),
    ] {
        append_aad_field(&mut aad, value);
    }
    aad
}

fn question_grading_rule_aad(rule: &QuestionGradingRule) -> Vec<u8> {
    match rule {
        QuestionGradingRule::AllOrNothing { points } => {
            let mut bytes = vec![1];
            bytes.extend_from_slice(&points.to_bits().to_be_bytes());
            bytes
        }
        QuestionGradingRule::PartialCredit { points } => {
            let mut bytes = vec![2];
            bytes.extend_from_slice(&points.to_bits().to_be_bytes());
            bytes
        }
        QuestionGradingRule::Ungraded => vec![3],
    }
}

fn append_aad_field(aad: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).expect("bounded AAD field fits u32");
    aad.extend_from_slice(&length.to_be_bytes());
    aad.extend_from_slice(value);
}
