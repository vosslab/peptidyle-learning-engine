//! Opaque per-attempt QTI grading authority.

use serde::{Deserialize, Serialize};

use super::{QtiGradingCapability, StoreError};

/// Exact private QTI payload copied at trusted issue time. This record is
/// serializable only for server-owned attempt/prefetch persistence; it is not
/// a browser DTO and diagnostics never reveal answer material.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssuedQtiGradingContractV1 {
    item_id: String,
    payload: Vec<u8>,
    payload_sha256: String,
}

impl IssuedQtiGradingContractV1 {
    pub fn new(
        question: &question_model::QuestionDefinition,
        item_id: String,
        payload: crate::QtiImportGradingPayload,
    ) -> Result<Self, StoreError> {
        let question_model::QuestionSource::Qti {
            item_id: source_item_id,
            ..
        } = &question.source
        else {
            return Err(StoreError::InvalidRecord(
                "QTI grading contract requires a QTI question".to_string(),
            ));
        };
        if source_item_id != &item_id {
            return Err(StoreError::InvalidRecord(
                "QTI grading contract item disagrees with question source".to_string(),
            ));
        }
        Ok(Self {
            item_id,
            payload: payload.bytes().to_vec(),
            payload_sha256: payload.sha256().to_string(),
        })
    }

    pub fn item_id(&self) -> &str {
        &self.item_id
    }

    pub fn payload(&self) -> Result<crate::QtiImportGradingPayload, StoreError> {
        if self.payload_sha256.len() != 64
            || !self
                .payload_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            || objects::Sha256Digest::compute(&self.payload).to_string() != self.payload_sha256
        {
            return Err(StoreError::Unavailable(
                "stored issued QTI grading payload checksum mismatch".to_string(),
            ));
        }
        crate::QtiImportGradingPayload::new(self.payload.clone()).map_err(|_| {
            StoreError::Unavailable("stored issued QTI grading payload is invalid".to_string())
        })
    }

    pub(crate) fn validate_for_question(
        &self,
        question: &question_model::QuestionDefinition,
    ) -> Result<(), StoreError> {
        let question_model::QuestionSource::Qti { item_id, .. } = &question.source else {
            return Err(StoreError::Unavailable(
                "stored QTI grading contract has a non-QTI snapshot".to_string(),
            ));
        };
        if item_id != &self.item_id {
            return Err(StoreError::Unavailable(
                "stored QTI grading contract item disagrees with snapshot".to_string(),
            ));
        }
        self.payload().map(|_| ())
    }
}

impl std::fmt::Debug for IssuedQtiGradingContractV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedQtiGradingContractV1")
            .field("item_id", &self.item_id)
            .field("payload", &"[REDACTED]")
            .field("payload_sha256", &self.payload_sha256)
            .finish()
    }
}

pub(crate) fn validate_issued_qti_grading(
    question: &question_model::QuestionDefinition,
    capability: QtiGradingCapability,
    contract: Option<&IssuedQtiGradingContractV1>,
) -> Result<(), StoreError> {
    let is_qti = matches!(question.source, question_model::QuestionSource::Qti { .. });
    match (is_qti, capability, contract) {
        (true, QtiGradingCapability::Required, Some(contract)) => {
            contract.validate_for_question(question)
        }
        (true, _, _) => Err(StoreError::InvalidRecord(
            "QTI issuance lacks its immutable private grading contract".to_string(),
        )),
        (false, QtiGradingCapability::NotApplicable, None) => Ok(()),
        (false, _, _) => Err(StoreError::InvalidRecord(
            "non-QTI issuance carries private QTI grading authority".to_string(),
        )),
    }
}
