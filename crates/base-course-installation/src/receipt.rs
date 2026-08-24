//! Canonical generation-bound storage receipt and v1 command output.

use std::fmt::Write as _;

use question_model::{AssignmentId, EnrollmentId, ProblemId, QuestionId, VersionId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::BaseCourseInstallError;
use crate::records::BASELINE_VERSION;

pub(crate) const STORAGE_RECEIPT_BUCKET: &str = "private-content";
pub(crate) const STORAGE_RECEIPT_KEY: &str = "ple/live-demo/base-course-install-receipt.json";
const MAX_STORAGE_RECEIPT_BYTES: usize = 512;

/// Host-only identifiers emitted after a newly completed installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseCourseManifest {
    assignment_id: AssignmentId,
    enrollment_id: EnrollmentId,
    question_id: QuestionId,
    problem_id: ProblemId,
    version_id: VersionId,
}

impl BaseCourseManifest {
    pub(crate) fn new(
        assignment_id: AssignmentId,
        enrollment_id: EnrollmentId,
        question_id: QuestionId,
        problem_id: ProblemId,
        version_id: VersionId,
    ) -> Self {
        Self {
            assignment_id,
            enrollment_id,
            question_id,
            problem_id,
            version_id,
        }
    }

    pub(crate) fn question_id(&self) -> &QuestionId {
        &self.question_id
    }
}

/// Observable lifecycle action performed by one installer call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BaseCourseAction {
    /// A new installing marker and generation were created.
    Prepared,
    /// A fresh generation was seeded and completed.
    Installed,
    /// Existing installing work was resumed.
    Resumed,
    /// A completed installation was returned without seed or storage inspection.
    Retained,
}

/// Durable lifecycle state returned to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseCourseInstallStateOutput {
    /// The host must persist the returned canonical storage receipt.
    Installing,
    /// The deterministic baseline has completed.
    Complete,
}

/// Exact v1 serializable output consumed by the Python lifecycle owner.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseCourseInstallOutput {
    schema_version: u8,
    action: BaseCourseAction,
    install_state: BaseCourseInstallStateOutput,
    baseline_version: &'static str,
    object_manifest: serde_json::Value,
    installation_generation: Uuid,
    storage_receipt_bucket: &'static str,
    storage_receipt_key: &'static str,
    storage_receipt_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_receipt_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completion_receipt_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<BaseCourseManifest>,
}

impl BaseCourseInstallOutput {
    /// Returns the closed action performed by this call.
    pub fn action(&self) -> BaseCourseAction {
        self.action
    }

    /// Returns the durable state observed after this call.
    pub fn install_state(&self) -> BaseCourseInstallStateOutput {
        self.install_state
    }

    /// Returns the stable generation that binds PostgreSQL and object storage.
    pub fn installation_generation(&self) -> Uuid {
        self.installation_generation
    }

    /// Returns the exact canonical receipt bytes for host persistence.
    pub fn storage_receipt_json(&self) -> &str {
        &self.storage_receipt_json
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BaseCourseStorageReceipt {
    schema_version: u8,
    baseline_version: String,
    installation_generation: Uuid,
    storage_receipt_bucket: String,
    storage_receipt_key: String,
    object_manifest: serde_json::Value,
}

pub(crate) fn canonical_storage_receipt(
    generation: Uuid,
) -> Result<String, BaseCourseInstallError> {
    serde_json::to_string(&expected_receipt(generation)).map_err(|source| {
        BaseCourseInstallError::serialization(
            "serializing the canonical Base Course storage receipt",
            source,
        )
    })
}

pub(crate) fn validate_storage_receipt(
    value: &str,
    generation: Uuid,
) -> Result<String, BaseCourseInstallError> {
    // ASVS 1.5.2 and 2.2.1: allowlist one typed, size-bounded receipt before trusting it.
    if value.len() > MAX_STORAGE_RECEIPT_BYTES {
        return Err(BaseCourseInstallError::receipt(
            "receipt exceeds the supported v1 size",
        ));
    }
    let receipt: BaseCourseStorageReceipt = serde_json::from_str(value)
        .map_err(|_| BaseCourseInstallError::receipt("receipt is not canonical v1 JSON"))?;
    let canonical = canonical_storage_receipt(generation)?;
    if receipt != expected_receipt(generation) || value != canonical {
        return Err(BaseCourseInstallError::receipt(
            "receipt does not exactly bind the prepared installation generation",
        ));
    }
    Ok(sha256_hex(value.as_bytes()))
}

pub(crate) fn verify_retained_storage_receipt_sha256(
    generation: Uuid,
    persisted_sha256: &str,
) -> Result<(), BaseCourseInstallError> {
    let canonical = canonical_storage_receipt(generation)?;
    if persisted_sha256 != sha256_hex(canonical.as_bytes()) {
        return Err(BaseCourseInstallError::baseline(
            "the retained receipt hash differs from the current canonical receipt; change the baseline version before installing changed receipt bytes",
        ));
    }
    Ok(())
}

pub(crate) fn output(
    action: BaseCourseAction,
    install_state: BaseCourseInstallStateOutput,
    installation_generation: Uuid,
    storage_receipt_sha256: Option<String>,
    completion_receipt_sha256: Option<String>,
    manifest: Option<BaseCourseManifest>,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    Ok(BaseCourseInstallOutput {
        schema_version: 1,
        action,
        install_state,
        baseline_version: BASELINE_VERSION,
        object_manifest: serde_json::json!([]),
        installation_generation,
        storage_receipt_bucket: STORAGE_RECEIPT_BUCKET,
        storage_receipt_key: STORAGE_RECEIPT_KEY,
        storage_receipt_json: canonical_storage_receipt(installation_generation)?,
        storage_receipt_sha256,
        completion_receipt_sha256,
        manifest,
    })
}

fn expected_receipt(generation: Uuid) -> BaseCourseStorageReceipt {
    BaseCourseStorageReceipt {
        schema_version: 1,
        baseline_version: BASELINE_VERSION.to_string(),
        installation_generation: generation,
        storage_receipt_bucket: STORAGE_RECEIPT_BUCKET.to_string(),
        storage_receipt_key: STORAGE_RECEIPT_KEY.to_string(),
        object_manifest: serde_json::json!([]),
    }
}

fn sha256_hex(value: &[u8]) -> String {
    // ASVS 11.4.3: use collision-resistant 256-bit SHA-256 for receipt integrity.
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut hash, "{byte:02x}").expect("writing a receipt hash to String cannot fail");
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_receipt_is_exact_canonical_and_generation_bound() {
        let generation = Uuid::from_u128(6);
        let receipt = canonical_storage_receipt(generation).unwrap();
        assert_eq!(
            receipt,
            "{\"schemaVersion\":1,\"baselineVersion\":\"base-course-v1\",\"installationGeneration\":\"00000000-0000-0000-0000-000000000006\",\"storageReceiptBucket\":\"private-content\",\"storageReceiptKey\":\"ple/live-demo/base-course-install-receipt.json\",\"objectManifest\":[]}"
        );
        assert!(validate_storage_receipt(&receipt, generation).is_ok());
        assert!(validate_storage_receipt(&receipt, Uuid::from_u128(7)).is_err());
        assert!(validate_storage_receipt(&format!("{receipt} "), generation).is_err());
        assert!(
            validate_storage_receipt(&"x".repeat(MAX_STORAGE_RECEIPT_BYTES + 1), generation)
                .is_err()
        );
    }

    #[test]
    fn retained_hash_must_name_the_current_canonical_receipt() {
        let generation = Uuid::from_u128(8);
        let receipt = canonical_storage_receipt(generation).unwrap();
        let current = validate_storage_receipt(&receipt, generation).unwrap();
        assert!(verify_retained_storage_receipt_sha256(generation, &current).is_ok());
        assert!(verify_retained_storage_receipt_sha256(generation, &"a".repeat(64)).is_err());
    }

    #[test]
    fn v1_output_preserves_exact_field_order_and_optional_fields() {
        let value = output(
            BaseCourseAction::Prepared,
            BaseCourseInstallStateOutput::Installing,
            Uuid::from_u128(6),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            "{\"schemaVersion\":1,\"action\":\"prepared\",\"installState\":\"installing\",\"baselineVersion\":\"base-course-v1\",\"objectManifest\":[],\"installationGeneration\":\"00000000-0000-0000-0000-000000000006\",\"storageReceiptBucket\":\"private-content\",\"storageReceiptKey\":\"ple/live-demo/base-course-install-receipt.json\",\"storageReceiptJson\":\"{\\\"schemaVersion\\\":1,\\\"baselineVersion\\\":\\\"base-course-v1\\\",\\\"installationGeneration\\\":\\\"00000000-0000-0000-0000-000000000006\\\",\\\"storageReceiptBucket\\\":\\\"private-content\\\",\\\"storageReceiptKey\\\":\\\"ple/live-demo/base-course-install-receipt.json\\\",\\\"objectManifest\\\":[]}\"}"
        );

        let manifest = BaseCourseManifest::new(
            AssignmentId::from_uuid(Uuid::from_u128(1)),
            EnrollmentId::from_uuid(Uuid::from_u128(2)),
            QuestionId::from_canonical_parts("000000", '0').unwrap(),
            ProblemId::from_uuid(Uuid::from_u128(3)),
            VersionId::from_uuid(Uuid::from_u128(4)),
        );
        for (action, action_name) in [
            (BaseCourseAction::Installed, "installed"),
            (BaseCourseAction::Resumed, "resumed"),
        ] {
            let complete = output(
                action,
                BaseCourseInstallStateOutput::Complete,
                Uuid::from_u128(6),
                Some("a".repeat(64)),
                Some("b".repeat(64)),
                Some(manifest.clone()),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_string(&complete).unwrap(),
                format!(
                    "{{\"schemaVersion\":1,\"action\":\"{action_name}\",\"installState\":\"complete\",\"baselineVersion\":\"base-course-v1\",\"objectManifest\":[],\"installationGeneration\":\"00000000-0000-0000-0000-000000000006\",\"storageReceiptBucket\":\"private-content\",\"storageReceiptKey\":\"ple/live-demo/base-course-install-receipt.json\",\"storageReceiptJson\":\"{{\\\"schemaVersion\\\":1,\\\"baselineVersion\\\":\\\"base-course-v1\\\",\\\"installationGeneration\\\":\\\"00000000-0000-0000-0000-000000000006\\\",\\\"storageReceiptBucket\\\":\\\"private-content\\\",\\\"storageReceiptKey\\\":\\\"ple/live-demo/base-course-install-receipt.json\\\",\\\"objectManifest\\\":[]}}\",\"storageReceiptSha256\":\"{}\",\"completionReceiptSha256\":\"{}\",\"manifest\":{{\"assignmentId\":\"00000000-0000-0000-0000-000000000001\",\"enrollmentId\":\"00000000-0000-0000-0000-000000000002\",\"questionId\":\"000-0000\",\"problemId\":\"00000000-0000-0000-0000-000000000003\",\"versionId\":\"00000000-0000-0000-0000-000000000004\"}}}}",
                    "a".repeat(64),
                    "b".repeat(64),
                )
            );
        }

        let retained = output(
            BaseCourseAction::Retained,
            BaseCourseInstallStateOutput::Complete,
            Uuid::from_u128(6),
            Some("a".repeat(64)),
            Some("b".repeat(64)),
            None,
        )
        .unwrap();
        let retained = serde_json::to_value(retained).unwrap();
        assert_eq!(retained["action"], "retained");
        assert_eq!(retained["completionReceiptSha256"], "b".repeat(64));
        assert!(retained.get("manifest").is_none());
    }
}
