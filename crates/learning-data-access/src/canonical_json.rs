//! Versioned source-text integrity encoding for immutable grading evidence.

use objects::Sha256Digest;
use serde::Serialize;

use crate::StoreError;

/// Current `ple-canonical-json-v1` protocol version.
pub(crate) const CANONICAL_JSON_V1_VERSION: u16 = 1;

/// Maximum UTF-8 source-text size for one `ple-canonical-json-v1` value.
///
/// This matches the established PostgreSQL broker JSON ceiling. Smaller domain
/// budgets, such as private-feedback's 64 KiB limit, remain in force before
/// values reach this general evidence boundary.
pub(crate) const MAX_CANONICAL_JSON_V1_BYTES: usize = 512 * 1024;

/// One server-private `ple-canonical-json-v1` evidence representation.
///
/// `source` is the sole byte authority. `projection` is parsed once from that
/// source for structural database queries; it is never re-serialized to
/// establish the digest.
#[derive(Clone, PartialEq)]
pub(crate) struct CanonicalJsonV1 {
    pub(crate) version: u16,
    pub(crate) source: String,
    pub(crate) projection: serde_json::Value,
    pub(crate) sha256: Sha256Digest,
}

impl std::fmt::Debug for CanonicalJsonV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CanonicalJsonV1")
            .field("version", &self.version)
            .field("source", &"[SERVER-ONLY]")
            .field("projection", &"[SERVER-ONLY]")
            .field("sha256", &self.sha256)
            .finish()
    }
}

/// Serializes one typed value into the exact `ple-canonical-json-v1` source.
///
/// ASVS 1.1.1 and 1.5.3: serialize the typed value once, then parse that exact
/// UTF-8 source once into the projection used by every later structural check.
/// ASVS 2.2.1-2.2.3 and 15.3.5: enforce a positive byte range and strict
/// source-to-projection coherence at this trusted service boundary.
/// ASVS 11.4.3: use a 256-bit SHA-256 digest over the exact UTF-8 source.
pub(crate) fn canonical_json_bytes_v1<T: Serialize>(
    artifact: &'static str,
    value: &T,
) -> Result<CanonicalJsonV1, StoreError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        StoreError::InvalidRecord(format!(
            "{artifact} canonical JSON serialization failed: {error}"
        ))
    })?;
    validate_source_size(artifact, bytes.len())?;
    let source = String::from_utf8(bytes).map_err(|error| {
        StoreError::InvalidRecord(format!("{artifact} canonical JSON was not UTF-8: {error}"))
    })?;
    let projection = serde_json::from_str(&source).map_err(|error| {
        StoreError::InvalidRecord(format!(
            "{artifact} canonical JSON parse-back failed: {error}"
        ))
    })?;
    Ok(CanonicalJsonV1 {
        version: CANONICAL_JSON_V1_VERSION,
        sha256: Sha256Digest::compute(source.as_bytes()),
        source,
        projection,
    })
}

/// Verifies persisted `ple-canonical-json-v1` source text before typed decode.
///
/// This accepts only the exact source bytes and parsed projection already
/// carried by storage; callers deserialize `source` into their closed typed
/// value only after this function succeeds.
pub(crate) fn verify_canonical_json_v1(
    artifact: &'static str,
    source: &str,
    projection: &serde_json::Value,
    expected_sha256: Sha256Digest,
) -> Result<CanonicalJsonV1, StoreError> {
    validate_source_size(artifact, source.len())?;
    let sha256 = Sha256Digest::compute(source.as_bytes());
    if sha256 != expected_sha256 {
        return Err(StoreError::InvalidRecord(format!(
            "{artifact} canonical JSON digest does not match its source"
        )));
    }
    let parsed_projection = serde_json::from_str(source).map_err(|error| {
        StoreError::InvalidRecord(format!("{artifact} canonical JSON parse failed: {error}"))
    })?;
    if parsed_projection != *projection {
        return Err(StoreError::InvalidRecord(format!(
            "{artifact} canonical JSON projection does not match its source"
        )));
    }
    Ok(CanonicalJsonV1 {
        version: CANONICAL_JSON_V1_VERSION,
        source: source.to_owned(),
        projection: parsed_projection,
        sha256,
    })
}

fn validate_source_size(artifact: &str, size: usize) -> Result<(), StoreError> {
    if size == 0 || size > MAX_CANONICAL_JSON_V1_BYTES {
        return Err(StoreError::InvalidRecord(format!(
            "{artifact} canonical JSON must be between 1 and {MAX_CANONICAL_JSON_V1_BYTES} bytes"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CANONICAL_JSON_V1_VERSION, canonical_json_bytes_v1, verify_canonical_json_v1};
    use serde::Serialize;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct StableEvidence<'a> {
        attempt_id: &'a str,
        earned_points: f64,
        detail: StableDetail<'a>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct StableDetail<'a> {
        correct: bool,
        rationale: &'a str,
    }

    fn stable_evidence() -> StableEvidence<'static> {
        StableEvidence {
            attempt_id: "attempt-7",
            earned_points: 8.5,
            detail: StableDetail {
                correct: true,
                rationale: "The amino terminus is protonated.",
            },
        }
    }

    #[test]
    fn v1_retains_one_source_digest_and_projection() {
        let encoded = canonical_json_bytes_v1("test evidence", &stable_evidence())
            .expect("stable evidence encodes");

        assert_eq!(encoded.version, CANONICAL_JSON_V1_VERSION);
        assert_eq!(
            encoded.source,
            r#"{"attemptId":"attempt-7","earnedPoints":8.5,"detail":{"correct":true,"rationale":"The amino terminus is protonated."}}"#
        );
        assert_eq!(
            encoded.projection,
            serde_json::json!({
                "attemptId": "attempt-7",
                "earnedPoints": 8.5,
                "detail": {
                    "correct": true,
                    "rationale": "The amino terminus is protonated.",
                },
            })
        );
        assert_eq!(
            encoded.sha256.to_string(),
            "4246068d50e8b07cf95644f13526dbed7f112f2c5c4f520b168e5a018df44d5c"
        );
    }

    #[test]
    fn v1_rejects_altered_source_or_digest() {
        let encoded = canonical_json_bytes_v1("test evidence", &stable_evidence())
            .expect("stable evidence encodes");
        let altered_source = encoded.source.replace("attempt-7", "attempt-8");

        assert!(
            verify_canonical_json_v1(
                "test evidence",
                &altered_source,
                &encoded.projection,
                encoded.sha256,
            )
            .is_err()
        );
        assert!(
            verify_canonical_json_v1(
                "test evidence",
                &encoded.source,
                &encoded.projection,
                objects::Sha256Digest::compute(b"altered digest"),
            )
            .is_err()
        );
        assert!(
            verify_canonical_json_v1(
                "test evidence",
                &encoded.source,
                &serde_json::json!({"attemptId": "attempt-7"}),
                encoded.sha256,
            )
            .is_err()
        );
    }
}
