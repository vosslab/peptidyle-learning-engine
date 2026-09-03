//! Server-only conversion fields for one private mapped QTI item.

use super::{
    QtiChoiceIdMap, QtiChoiceMapPayload, QtiMappingVersion, QtiPrivateMappingChecksumInput,
    QtiProfileId, QtiProfileVersion, QtiPublicMappingChecksumInput,
};
use adapter_ple::question_json::imported::{
    ImportedChoice, ImportedPleQuestionJson, ImportedPleQuestionJsonError,
    ImportedSingleChoiceInput,
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
    pub(super) public_mapping: QtiPublicMappingChecksumInput,
    pub(super) private_mapping: QtiPrivateMappingChecksumInput,
    pub(super) normalized_qti_item_fingerprint: super::NormalizedQtiItemFingerprint,
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
    pub fn public_mapping(&self) -> &QtiPublicMappingChecksumInput {
        &self.public_mapping
    }
    pub fn private_mapping(&self) -> &QtiPrivateMappingChecksumInput {
        &self.private_mapping
    }
    /// Opaque normalized source-item fingerprint for later QTI import binding.
    pub fn normalized_qti_item_fingerprint(&self) -> super::NormalizedQtiItemFingerprint {
        self.normalized_qti_item_fingerprint
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

    /// Converts one supported QTI item into the sole-current PLE Question JSON
    /// source. QTI vendor points remain only in Workspace Import evidence and
    /// never become Question or Assignment policy.
    pub fn into_ple_question_json(
        self,
    ) -> Result<ImportedPleQuestionJson, ImportedPleQuestionJsonError> {
        let input = ImportedSingleChoiceInput::new(
            self.public_mapping.title,
            "Imported from a supported QTI package.".to_string(),
            self.public_mapping.prompt_markdown,
            self.public_mapping
                .choices
                .into_iter()
                .map(|choice| ImportedChoice::new(choice.ple_choice_id, choice.text_markdown))
                .collect(),
            self.correct_ple_choice_id,
        );
        ImportedPleQuestionJson::from_imported(input)
    }
}
