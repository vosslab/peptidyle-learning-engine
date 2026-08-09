//! Server-only conversion fields for one private mapped QTI item.

use objects::Sha256Digest;

use super::{
    QtiChoiceIdMap, QtiChoiceMapPayload, QtiMappingVersion, QtiPrivateMappingDigestInput,
    QtiProfileId, QtiProfileVersion, QtiPublicMappingDigestInput,
};

/// Private fields passed only to the server's native conversion bridge.
///
/// ```compile_fail
/// use adapter_qti::profiles::QtiMappedItemServerParts;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<QtiMappedItemServerParts>();
/// ```
///
/// ```compile_fail
/// use adapter_qti::profiles::QtiMappedItemServerParts;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<QtiMappedItemServerParts>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct QtiMappedItemServerParts {
    pub(super) profile: QtiProfileId,
    pub(super) profile_version: QtiProfileVersion,
    pub(super) mapping_version: QtiMappingVersion,
    pub(super) public_mapping: QtiPublicMappingDigestInput,
    pub(super) private_mapping: QtiPrivateMappingDigestInput,
    pub(super) normalized_profile_item_sha256: Sha256Digest,
    pub(super) correct_ple_choice_id: String,
    pub(super) choice_map: Vec<QtiChoiceIdMap>,
    pub(super) choice_map_payload: QtiChoiceMapPayload,
}

impl QtiMappedItemServerParts {
    pub fn profile(&self) -> QtiProfileId {
        self.profile
    }
    pub fn profile_version(&self) -> QtiProfileVersion {
        self.profile_version
    }
    pub fn mapping_version(&self) -> QtiMappingVersion {
        self.mapping_version
    }
    pub fn public_mapping(&self) -> &QtiPublicMappingDigestInput {
        &self.public_mapping
    }
    pub fn private_mapping(&self) -> &QtiPrivateMappingDigestInput {
        &self.private_mapping
    }
    /// Opaque normalized source-item fingerprint for later provenance binding.
    pub fn normalized_profile_item_sha256(&self) -> Sha256Digest {
        self.normalized_profile_item_sha256
    }
    pub fn server_correct_ple_choice_id(&self) -> &str {
        &self.correct_ple_choice_id
    }
    pub fn server_ordered_choice_map(&self) -> &[QtiChoiceIdMap] {
        &self.choice_map
    }
    pub fn server_choice_map_payload(&self) -> &QtiChoiceMapPayload {
        &self.choice_map_payload
    }
}
