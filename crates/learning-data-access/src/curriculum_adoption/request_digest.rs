//! Canonical typed request-envelope binding for curriculum-adoption receipts.

use objects::Sha256Digest;
use question_model::UserId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::StoreError;

const REQUEST_DIGEST_DOMAIN: &[u8] = b"ple:curriculum-adoption-request:v1\0";
const REQUEST_DIGEST_VERSION: u8 = 1;

/// Closed semantic operation bound into an idempotent adoption receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CurriculumAdoptionOperation {
    ForkAlpha,
    InstantiateBlueprint,
    InstantiateAlpha,
    RolloverCourse,
    ShiftCourseTerm,
    FastForwardAssignment,
    CreateSourceDerivedAssignment,
}

/// Fixed-width receipt digest shared by every persistence adapter.
pub(crate) type CurriculumAdoptionRequestDigest = Sha256Digest;

#[derive(Serialize)]
struct RequestDigestEnvelope<'a, T> {
    version: u8,
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    request: &'a T,
}

/// Hashes one strictly decoded typed request under its server-derived actor.
pub(crate) fn request_digest<T: Serialize>(
    operation: CurriculumAdoptionOperation,
    actor: UserId,
    request: &T,
) -> Result<CurriculumAdoptionRequestDigest, StoreError> {
    let wire = serde_json::to_vec(&RequestDigestEnvelope {
        version: REQUEST_DIGEST_VERSION,
        operation,
        actor,
        request,
    })
    .map_err(|error| {
        StoreError::InvalidRecord(format!("request digest encoding failed: {error}"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(REQUEST_DIGEST_DOMAIN);
    hasher.update(wire);
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use uuid::Uuid;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct TypedMeaning {
        source: String,
        revision: u8,
    }

    fn actor(value: u128) -> UserId {
        UserId::from_uuid(Uuid::from_u128(value))
    }

    #[test]
    fn typed_decode_normalizes_key_order_before_request_hashing() {
        let left: TypedMeaning =
            serde_json::from_str(r#"{"source":"BP-1","revision":2}"#).expect("typed request");
        let right: TypedMeaning =
            serde_json::from_str(r#"{"revision":2,"source":"BP-1"}"#).expect("typed request");

        assert_eq!(
            request_digest(
                CurriculumAdoptionOperation::InstantiateBlueprint,
                actor(1),
                &left
            )
            .expect("digest"),
            request_digest(
                CurriculumAdoptionOperation::InstantiateBlueprint,
                actor(1),
                &right,
            )
            .expect("digest")
        );
    }

    #[test]
    fn actor_operation_and_typed_meaning_are_independent_receipt_bindings() {
        let meaning = TypedMeaning {
            source: "BP-1".into(),
            revision: 2,
        };
        let baseline = request_digest(
            CurriculumAdoptionOperation::InstantiateBlueprint,
            actor(1),
            &meaning,
        )
        .expect("digest");
        assert_ne!(
            baseline,
            request_digest(
                CurriculumAdoptionOperation::InstantiateBlueprint,
                actor(2),
                &meaning,
            )
            .expect("actor digest")
        );
        assert_ne!(
            baseline,
            request_digest(
                CurriculumAdoptionOperation::FastForwardAssignment,
                actor(1),
                &meaning,
            )
            .expect("operation digest")
        );
        assert_ne!(
            baseline,
            request_digest(
                CurriculumAdoptionOperation::InstantiateBlueprint,
                actor(1),
                &TypedMeaning {
                    revision: 3,
                    ..meaning
                },
            )
            .expect("meaning digest")
        );
    }
}
