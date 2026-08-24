//! Immutable, server-only question source/execution evidence for issued work.
//!
//! The closed V1 shape is deliberately separate from browser DTOs.  It is
//! created by trusted issuance, checked before a first effect, and stored with
//! an attempt or its durable prefetch reservation.  ASVS 1.5.2 and 2.2.1-2.2.3:
//! decoding accepts exactly this known schema and validates cross-field
//! identity before it can influence a grading workflow.

use objects::Sha256Digest;
use question_model::{AssetId, ObjectId, QuestionDefinition, QuestionSource, SourceArtifact};
use serde::{Deserialize, Serialize};

use super::{
    FlatGradingCapability, ReceiptPresentationSnapshot, StoreError, WebworkGradingCapability,
};

/// The only accepted issued-question snapshot schema revision.
pub const ISSUED_QUESTION_SNAPSHOT_SCHEMA_VERSION_V1: u8 = 1;

/// Immutable source/execution witness for an issued question family.
///
/// A variant names the one family authority retained with the full immutable
/// definition.  It intentionally carries no answer key, browser response
/// mapping, renderer secret, provider endpoint, or object-store locator.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "camelCase", deny_unknown_fields)]
pub enum IssuedQuestionFamilyWitnessV1 {
    /// First-party non-flat content and the exact selected asset renditions.
    Native {
        /// Complete server-only physical authority for every non-flat native
        /// asset, including presentation-bearing attempts.
        physical_asset_bindings: Vec<IssuedNativeAssetBindingV1>,
    },
    /// Imported QTI source bytes retained for server-side resolution.
    Qti { source_artifact: SourceArtifact },
    /// Server-brokered external source bytes and configured profile identity.
    External {
        source_artifact: SourceArtifact,
        integration_profile_identity: String,
    },
    /// Native flat questions retain their private grading authority elsewhere.
    Flat {},
    /// WeBWorK retains source, renderer, and response mapping elsewhere.
    Webwork {},
}

/// Exact logical-to-physical native asset authority captured at issuance.
///
/// This remains server-only evidence: the answer-free browser presentation
/// intentionally retains no object locator. Native grading needs the object
/// identity to verify the generated attempt without consulting the catalog.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssuedNativeAssetBindingV1 {
    pub asset: AssetId,
    pub object: ObjectId,
    pub authored_checksum: String,
    pub rendition_checksum: String,
    pub intrinsic_width: Option<u32>,
    pub intrinsic_height: Option<u32>,
}

impl std::fmt::Debug for IssuedQuestionFamilyWitnessV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native {
                physical_asset_bindings,
            } => formatter
                .debug_struct("IssuedQuestionFamilyWitnessV1::Native")
                .field("physical_asset_bindings", &physical_asset_bindings.len())
                .finish(),
            Self::Qti { .. } => {
                formatter.write_str("IssuedQuestionFamilyWitnessV1::Qti([REDACTED])")
            }
            Self::External { .. } => {
                formatter.write_str("IssuedQuestionFamilyWitnessV1::External([REDACTED])")
            }
            Self::Flat {} => formatter.write_str("IssuedQuestionFamilyWitnessV1::Flat"),
            Self::Webwork {} => formatter.write_str("IssuedQuestionFamilyWitnessV1::Webwork"),
        }
    }
}

/// Closed versioned immutable evidence for one issued question.
///
/// This type is server-only by construction: it has no browser serialization
/// boundary and its `Debug` implementation never emits the definition or
/// source witness.  PostgreSQL stores its canonical JSON bytes with the
/// digest returned by [`Self::canonical_payload_sha256`].
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssuedQuestionSnapshotV1 {
    schema_version: u8,
    question: QuestionDefinition,
    family_witness: IssuedQuestionFamilyWitnessV1,
}

impl IssuedQuestionSnapshotV1 {
    /// Constructs and validates immutable V1 evidence at the trusted issue boundary.
    pub fn new(
        question: QuestionDefinition,
        family_witness: IssuedQuestionFamilyWitnessV1,
    ) -> Result<Self, StoreError> {
        let value = Self {
            schema_version: ISSUED_QUESTION_SNAPSHOT_SCHEMA_VERSION_V1,
            question,
            family_witness,
        };
        value.validate_shape()?;
        Ok(value)
    }

    /// Returns the complete immutable definition for trusted server work.
    pub fn question(&self) -> &QuestionDefinition {
        &self.question
    }

    /// Returns the family witness for trusted server work.
    pub fn family_witness(&self) -> &IssuedQuestionFamilyWitnessV1 {
        &self.family_witness
    }

    /// Returns the canonical JSON payload and SHA-256 used by both stores.
    pub fn canonical_payload(&self) -> Result<(serde_json::Value, String), StoreError> {
        self.validate_shape()?;
        let value = serde_json::to_value(self).map_err(|error| {
            StoreError::InvalidRecord(format!("issued snapshot encode failed: {error}"))
        })?;
        let bytes = serde_json::to_vec(&value).map_err(|error| {
            StoreError::InvalidRecord(format!("issued snapshot encode failed: {error}"))
        })?;
        Ok((value, Sha256Digest::compute(&bytes).to_string()))
    }

    /// Returns the checksum of the exact canonical V1 payload.
    pub fn canonical_payload_sha256(&self) -> Result<String, StoreError> {
        self.canonical_payload().map(|(_, checksum)| checksum)
    }

    /// Decodes a stored payload only after recomputing its canonical digest.
    pub fn decode_checked(
        payload: serde_json::Value,
        expected_sha256: &str,
    ) -> Result<Self, StoreError> {
        if !is_lower_sha256(expected_sha256) {
            return Err(StoreError::Unavailable(
                "stored issued snapshot checksum is invalid".to_string(),
            ));
        }
        let bytes = serde_json::to_vec(&payload).map_err(|error| {
            StoreError::Unavailable(format!("stored issued snapshot encode failed: {error}"))
        })?;
        if Sha256Digest::compute(&bytes).to_string() != expected_sha256 {
            return Err(StoreError::Unavailable(
                "stored issued snapshot checksum mismatch".to_string(),
            ));
        }
        let value: Self = serde_json::from_value(payload.clone()).map_err(|error| {
            StoreError::Unavailable(format!("stored issued snapshot decode failed: {error}"))
        })?;
        value.validate_shape().map_err(|_| {
            StoreError::Unavailable("stored issued snapshot is invalid".to_string())
        })?;
        // `QuestionDefinition` predates this closed persistence boundary and
        // accepts its own additive fields. Re-encoding and requiring exact
        // value equality makes V1 closed recursively as well: an unknown
        // nested field is stripped by serde and therefore fails this check.
        let (canonical, _) = value.canonical_payload().map_err(|_| {
            StoreError::Unavailable("stored issued snapshot is invalid".to_string())
        })?;
        if canonical != payload {
            return Err(StoreError::Unavailable(
                "stored issued snapshot contains unknown fields".to_string(),
            ));
        }
        Ok(value)
    }

    /// Validates that the snapshot is bound to the exact attempt identity.
    pub fn validate_for_attempt(
        &self,
        problem: question_model::ProblemId,
        version: question_model::VersionId,
    ) -> Result<(), StoreError> {
        self.validate_shape()?;
        if self.question.problem != problem || self.question.version != version {
            return Err(StoreError::Unavailable(
                "issued question snapshot disagrees with its attempt".to_string(),
            ));
        }
        Ok(())
    }

    /// Verifies that the only retained native object authority is exactly the
    /// provenance recorded on the owning attempt.  A mismatch is unavailable
    /// evidence, never an occasion to resolve current catalog bindings.
    pub fn validate_native_provenance(&self, asset_objects: &[ObjectId]) -> Result<(), StoreError> {
        if let IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings,
        } = &self.family_witness
        {
            let objects: Vec<_> = physical_asset_bindings
                .iter()
                .map(|binding| binding.object)
                .collect();
            if objects != asset_objects {
                return Err(StoreError::Unavailable(
                    "issued native asset authority disagrees with attempt provenance".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Validates family and specialized-capability agreement at issuance.
    pub fn validate_for_issuance(
        &self,
        flat_grading: FlatGradingCapability,
        webwork_grading: WebworkGradingCapability,
        qti_grading: crate::QtiGradingCapability,
    ) -> Result<(), StoreError> {
        self.validate_shape()?;
        let expected = match &self.family_witness {
            IssuedQuestionFamilyWitnessV1::Flat {} => (
                FlatGradingCapability::Required,
                WebworkGradingCapability::NotApplicable,
                crate::QtiGradingCapability::NotApplicable,
            ),
            IssuedQuestionFamilyWitnessV1::Webwork {} => (
                FlatGradingCapability::NotApplicable,
                WebworkGradingCapability::Required,
                crate::QtiGradingCapability::NotApplicable,
            ),
            IssuedQuestionFamilyWitnessV1::Qti { .. } => (
                FlatGradingCapability::NotApplicable,
                WebworkGradingCapability::NotApplicable,
                crate::QtiGradingCapability::Required,
            ),
            IssuedQuestionFamilyWitnessV1::Native { .. }
            | IssuedQuestionFamilyWitnessV1::External { .. } => (
                FlatGradingCapability::NotApplicable,
                WebworkGradingCapability::NotApplicable,
                crate::QtiGradingCapability::NotApplicable,
            ),
        };
        if (flat_grading, webwork_grading, qti_grading) != expected {
            return Err(StoreError::InvalidRecord(
                "issued snapshot family and grading capabilities disagree".to_string(),
            ));
        }
        Ok(())
    }

    /// Refuses duplicate physical-asset authority when an existing immutable
    /// issued presentation already retains the selected bindings.
    pub fn validate_for_issuance_context(
        &self,
        flat_grading: FlatGradingCapability,
        webwork_grading: WebworkGradingCapability,
        qti_grading: crate::QtiGradingCapability,
        presentation: Option<&ReceiptPresentationSnapshot>,
    ) -> Result<(), StoreError> {
        self.validate_for_issuance(flat_grading, webwork_grading, qti_grading)?;
        if let (
            IssuedQuestionFamilyWitnessV1::Native {
                physical_asset_bindings,
            },
            Some(presentation),
        ) = (&self.family_witness, presentation)
        {
            self.validate_native_presentation_derivative(physical_asset_bindings, presentation)?;
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), StoreError> {
        if self.schema_version != ISSUED_QUESTION_SNAPSHOT_SCHEMA_VERSION_V1 {
            return Err(StoreError::InvalidRecord(
                "issued question snapshot schema version is unsupported".to_string(),
            ));
        }
        self.question.source.validate().map_err(|error| {
            StoreError::InvalidRecord(format!("issued snapshot source is invalid: {error}"))
        })?;
        match (&self.question.source, &self.family_witness) {
            (
                QuestionSource::Native { .. },
                IssuedQuestionFamilyWitnessV1::Native {
                    physical_asset_bindings,
                },
            ) => {
                if physical_asset_bindings.iter().any(|binding| {
                    !is_lower_sha256(&binding.authored_checksum)
                        || !is_lower_sha256(&binding.rendition_checksum)
                        || (binding.intrinsic_width.is_some() != binding.intrinsic_height.is_some())
                        || binding.intrinsic_width.is_some_and(|width| width == 0)
                        || binding.intrinsic_height.is_some_and(|height| height == 0)
                }) || physical_asset_bindings
                    .windows(2)
                    .any(|pair| pair[0].asset >= pair[1].asset)
                {
                    return Err(StoreError::InvalidRecord(
                        "native issued asset bindings must be canonical and valid".to_string(),
                    ));
                }
            }
            (QuestionSource::Native { .. }, IssuedQuestionFamilyWitnessV1::Flat {}) => {}
            (QuestionSource::Webwork { .. }, IssuedQuestionFamilyWitnessV1::Webwork {}) => {}
            (
                QuestionSource::Qti {
                    package_object,
                    package_sha256,
                    ..
                },
                IssuedQuestionFamilyWitnessV1::Qti { source_artifact },
            ) if source_artifact.object == *package_object
                && source_artifact.sha256 == *package_sha256 => {}
            (
                QuestionSource::Imathas {
                    snapshot,
                    snapshot_sha256,
                    integration_profile,
                    ..
                },
                IssuedQuestionFamilyWitnessV1::External {
                    source_artifact,
                    integration_profile_identity,
                },
            ) if source_artifact.object == *snapshot
                && source_artifact.sha256 == *snapshot_sha256
                && integration_profile_identity == integration_profile
                && !integration_profile_identity.trim().is_empty() => {}
            _ => {
                return Err(StoreError::InvalidRecord(
                    "issued question snapshot family witness disagrees with its definition"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_native_presentation_derivative(
        &self,
        bindings: &[IssuedNativeAssetBindingV1],
        presentation: &ReceiptPresentationSnapshot,
    ) -> Result<(), StoreError> {
        if presentation.asset_bindings.len() != bindings.len() {
            return Err(StoreError::InvalidRecord(
                "native presentation asset bindings disagree with issued authority".to_string(),
            ));
        }
        for (issued, disclosed) in bindings.iter().zip(&presentation.asset_bindings) {
            if issued.asset != disclosed.asset
                || issued.authored_checksum != disclosed.authored_checksum
                || issued.rendition_checksum != disclosed.rendition_checksum
                || issued.intrinsic_width != disclosed.intrinsic_width
                || issued.intrinsic_height != disclosed.intrinsic_height
            {
                return Err(StoreError::InvalidRecord(
                    "native presentation is not a projection of issued authority".to_string(),
                ));
            }
        }
        // `ReceiptPresentationSnapshot` is reconstructed and validated from
        // its envelope by `validate_issued_presentation`; that contract
        // rejects an unreferenced binding.  Equality above therefore proves
        // the envelope reaches precisely this issued logical asset set.
        Ok(())
    }
}

impl std::fmt::Debug for IssuedQuestionSnapshotV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedQuestionSnapshotV1")
            .field("schema_version", &self.schema_version)
            .field("question", &"[SERVER-ONLY]")
            .field("family_witness", &self.family_witness)
            .finish()
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use question_model::{
        DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionDefinition,
        QuestionMetadata, QuestionSource, ResponseDefinition, WorkspaceId,
        answer::TextMatchMode,
        generation::RandomizationDefinition,
        run_policy::{AttemptPolicy, TimingPolicy},
        taxonomy::License,
    };
    use uuid::Uuid;

    use super::*;

    fn question() -> QuestionDefinition {
        QuestionDefinition::from_draft(
            DraftQuestionDefinition {
                workspace: WorkspaceId::from_uuid(Uuid::from_u128(1)),
                source: DraftQuestionSource::Native {
                    family: "native".to_string(),
                },
                prompt: Vec::new(),
                response: ResponseDefinition::ShortText {
                    match_mode: TextMatchMode::Exact,
                    max_length: 10,
                },
                attempt_policy: AttemptPolicy { max_attempts: None },
                timing_policy: TimingPolicy::Untimed,
                randomization: RandomizationDefinition::Static,
                grading: GradingDefinition::Ungraded,
                metadata: QuestionMetadata {
                    title: "Question".to_string(),
                    tags: Vec::new(),
                    taxonomy: Vec::new(),
                    license: License::CcBySa,
                    language: "en-US".to_string(),
                },
            },
            question_model::ProblemId::from_uuid(Uuid::from_u128(2)),
            question_model::VersionId::from_uuid(Uuid::from_u128(3)),
            QuestionSource::Native {
                family: "native".to_string(),
            },
        )
    }

    #[test]
    fn rejects_family_mismatch_and_redacts_debug() {
        let err =
            IssuedQuestionSnapshotV1::new(question(), IssuedQuestionFamilyWitnessV1::Webwork {})
                .expect_err("native question cannot carry a WebWork witness");
        assert!(matches!(err, StoreError::InvalidRecord(_)));
        let snapshot = IssuedQuestionSnapshotV1::new(
            question(),
            IssuedQuestionFamilyWitnessV1::Native {
                physical_asset_bindings: Vec::new(),
            },
        )
        .expect("valid native snapshot");
        let debug = format!("{snapshot:?}");
        assert!(debug.contains("[SERVER-ONLY]"));
        assert!(!debug.contains("Question\""));
    }

    #[test]
    fn checksum_and_cross_attempt_identity_fail_closed() {
        let snapshot = IssuedQuestionSnapshotV1::new(
            question(),
            IssuedQuestionFamilyWitnessV1::Native {
                physical_asset_bindings: Vec::new(),
            },
        )
        .expect("valid native snapshot");
        let (payload, checksum) = snapshot.canonical_payload().expect("canonical payload");
        assert_eq!(
            IssuedQuestionSnapshotV1::decode_checked(payload.clone(), &checksum)
                .expect("checked decode"),
            snapshot
        );
        assert!(matches!(
            IssuedQuestionSnapshotV1::decode_checked(payload, &"0".repeat(64)),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            snapshot.validate_for_attempt(
                question_model::ProblemId::from_uuid(Uuid::from_u128(4)),
                snapshot.question().version
            ),
            Err(StoreError::Unavailable(_))
        ));
    }
}
