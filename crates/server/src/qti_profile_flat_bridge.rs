//! Pure server-side conversion from a validated QTI profile item to flat v2.
//!
//! The QTI adapter owns hostile archive parsing and the native adapter owns
//! flat-question validation, canonicalization, and compilation.  This module
//! deliberately only translates the already owner-bound mapping between them.

use std::fmt;

use adapter_native::flat_question::FlatQuestionError;
use adapter_native::flat_question::imported::{
    ImportedChoice, ImportedFlatQuestion, ImportedFlatQuestionError, ImportedSingleChoiceInput,
};
use adapter_qti::profiles::{
    QtiMappedItem, QtiMappedItemServerParts, QtiMappingVersion, QtiProfileId, QtiProfileVersion,
};
use grading::flat_question::FlatQuestionPrivate;
use learning_data_access::{
    FlatImportChoiceMapPayload, FlatImportConversionVersion, PersistedFlatImportProfile,
};
use question_model::{DraftQuestionDefinition, WorkspaceId};

/// Persisted identity of the composed QTI-profile to native-flat conversion.
pub(crate) const QTI_PROFILE_FLAT_CONVERSION_VERSION: &str = "ple-qti-profile-flat-conversion/v1";

/// Answer-free failure classes for the QTI-to-flat boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QtiProfileFlatBridgeError {
    UnsupportedProfile,
    UnsupportedContractVersion,
    NativeImport,
    NativeCompilation,
    PersistenceContract,
}

impl fmt::Display for QtiProfileFlatBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedProfile => "QTI profile is not supported for flat conversion",
            Self::UnsupportedContractVersion => {
                "QTI profile contract version is not supported for flat conversion"
            }
            Self::NativeImport => "QTI mapping cannot form a valid imported flat question",
            Self::NativeCompilation => "imported flat question could not be compiled",
            Self::PersistenceContract => "QTI mapping cannot form durable flat provenance",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for QtiProfileFlatBridgeError {}

/// One pure native conversion and the private evidence needed by provenance.
///
/// This remains crate-private and deliberately implements neither `Debug` nor
/// serialization: it owns the grader-only compiled material and the QTI
/// vendor-to-PLE map retained for the later provenance command.
pub(crate) struct QtiProfileFlatBridgeResult {
    canonical_source: Vec<u8>,
    draft: DraftQuestionDefinition,
    private: FlatQuestionPrivate,
    mapping_parts: QtiMappedItemServerParts,
}

impl QtiProfileFlatBridgeResult {
    pub(crate) fn canonical_source(&self) -> &[u8] {
        &self.canonical_source
    }

    pub(crate) fn draft(&self) -> &DraftQuestionDefinition {
        &self.draft
    }

    pub(crate) fn private(&self) -> &FlatQuestionPrivate {
        &self.private
    }

    /// Retains all mapping evidence for the future atomic provenance command.
    pub(crate) fn mapping_parts(&self) -> &QtiMappedItemServerParts {
        &self.mapping_parts
    }

    pub(crate) fn persisted_profile(&self) -> PersistedFlatImportProfile {
        match self.mapping_parts.profile() {
            QtiProfileId::CANVAS => PersistedFlatImportProfile::CanvasQti12V1,
            QtiProfileId::BLACKBOARD => PersistedFlatImportProfile::BlackboardQti21V1,
            QtiProfileId::GENERIC => {
                unreachable!("the bridge rejects generic mappings before constructing a result")
            }
        }
    }

    pub(crate) fn persisted_conversion_version(
        &self,
    ) -> Result<FlatImportConversionVersion, QtiProfileFlatBridgeError> {
        FlatImportConversionVersion::new(QTI_PROFILE_FLAT_CONVERSION_VERSION)
            .map_err(|_| QtiProfileFlatBridgeError::PersistenceContract)
    }

    pub(crate) fn persisted_choice_map(
        &self,
    ) -> Result<FlatImportChoiceMapPayload, QtiProfileFlatBridgeError> {
        FlatImportChoiceMapPayload::from_canonical_bytes(
            self.mapping_parts
                .server_choice_map_payload()
                .server_bytes()
                .to_vec(),
        )
        .map_err(|_| QtiProfileFlatBridgeError::PersistenceContract)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Vec<u8>,
        DraftQuestionDefinition,
        FlatQuestionPrivate,
        QtiMappedItemServerParts,
    ) {
        (
            self.canonical_source,
            self.draft,
            self.private,
            self.mapping_parts,
        )
    }
}

/// Converts one validated private QTI mapping without touching Store, objects,
/// schema, or HTTP state.
pub(crate) fn bridge_qti_mapped_item(
    item: QtiMappedItem,
    workspace: WorkspaceId,
) -> Result<QtiProfileFlatBridgeResult, QtiProfileFlatBridgeError> {
    let mapping_parts = item.into_server_parts();
    validate_contract(&mapping_parts)?;

    let public = mapping_parts.public_mapping();
    let imported = ImportedFlatQuestion::from_imported(ImportedSingleChoiceInput::new(
        public.title.clone(),
        public.prompt_markdown.clone(),
        public
            .choices
            .iter()
            .map(|choice| {
                ImportedChoice::new(choice.ple_choice_id.clone(), choice.text_markdown.clone())
            })
            .collect(),
        mapping_parts.server_correct_ple_choice_id().to_string(),
        public.points.clone(),
    ))
    .map_err(map_import_error)?;
    let canonical_source = imported
        .canonical_bytes()
        .map_err(|_| QtiProfileFlatBridgeError::NativeImport)?;
    let (draft, private) = imported
        .compile_parts(workspace)
        .map_err(map_compile_error)?;

    Ok(QtiProfileFlatBridgeResult {
        canonical_source,
        draft,
        private,
        mapping_parts,
    })
}

fn validate_contract(parts: &QtiMappedItemServerParts) -> Result<(), QtiProfileFlatBridgeError> {
    if !matches!(
        parts.profile(),
        QtiProfileId::CANVAS | QtiProfileId::BLACKBOARD
    ) {
        return Err(QtiProfileFlatBridgeError::UnsupportedProfile);
    }
    if parts.profile_version() != QtiProfileVersion::V1
        || parts.mapping_version() != QtiMappingVersion::V1
    {
        return Err(QtiProfileFlatBridgeError::UnsupportedContractVersion);
    }
    Ok(())
}

fn map_import_error(_: ImportedFlatQuestionError) -> QtiProfileFlatBridgeError {
    QtiProfileFlatBridgeError::NativeImport
}

fn map_compile_error(_: FlatQuestionError) -> QtiProfileFlatBridgeError {
    QtiProfileFlatBridgeError::NativeCompilation
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use adapter_native::flat_question::FlatQuestionDocument;
    use adapter_qti::QtiImportLimits;
    use adapter_qti::profiles::{import_blackboard_qti21, import_canvas_qti12};
    use question_model::WorkspaceId;
    use uuid::Uuid;

    use super::*;

    const CANVAS_MANIFEST: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_positive_manifest.xml");
    const CANVAS_META: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_assessment_meta.xml");
    const CANVAS_ITEM: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/canvas_positive_item.xml");
    const BLACKBOARD_MANIFEST: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/blackboard_positive_manifest.xml");
    const BLACKBOARD_META: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/blackboard_assessment_meta.xml");
    const BLACKBOARD_ITEM: &str =
        include_str!("../../adapters/qti/tests/fixtures/profiles/blackboard_positive_item.xml");
    const HAND_AUTHORED: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Favorite color","prompt":"What is my favorite color?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"blue"},"feedback":{},"points":1.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},"timingPolicy":{"kind":"untimed"},"tags":[],"taxonomy":[],"license":{"kind":"allRightsReserved"},"language":"en-US"}"#;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_uuid(Uuid::from_u128(0x5154_495f_4252_4944_4745))
    }

    fn archive(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (path, contents) in entries {
            zip.start_file(*path, options).expect("fixture entry");
            zip.write_all(contents.as_bytes()).expect("fixture bytes");
        }
        zip.finish().expect("fixture archive").into_inner()
    }

    fn canvas_item(item: &str) -> QtiMappedItem {
        import_canvas_qti12(
            &archive(&[
                ("imsmanifest.xml", CANVAS_MANIFEST),
                ("canvas_qti12_questions/assessment_meta.xml", CANVAS_META),
                ("canvas_qti12_questions/canvas-1.xml", item),
            ]),
            QtiImportLimits::default(),
        )
        .expect("frozen Canvas package maps")
        .into_mapped_items()
        .pop()
        .expect("one mapped Canvas item")
    }

    fn blackboard_item() -> QtiMappedItem {
        import_blackboard_qti21(
            &archive(&[
                ("imsmanifest.xml", BLACKBOARD_MANIFEST),
                ("qti21_items/assessment_meta.xml", BLACKBOARD_META),
                ("qti21_items/bb-1.xml", BLACKBOARD_ITEM),
            ]),
            QtiImportLimits::default(),
        )
        .expect("frozen Blackboard package maps")
        .into_mapped_items()
        .pop()
        .expect("one mapped Blackboard item")
    }

    fn hand_authored_parts() -> (Vec<u8>, DraftQuestionDefinition, FlatQuestionPrivate) {
        let document = FlatQuestionDocument::parse(HAND_AUTHORED.as_bytes())
            .expect("hand-authored source parses");
        let canonical = document.canonical_bytes().expect("canonical source");
        let compiled = document
            .compile(workspace())
            .expect("hand-authored compile");
        let (draft, private) = compiled.into_parts();
        (canonical, draft, private)
    }

    #[test]
    fn canvas_fixture_matches_equivalent_hand_authored_source_and_private_binding() {
        let bridged = bridge_qti_mapped_item(canvas_item(CANVAS_ITEM), workspace())
            .expect("Canvas bridge succeeds");
        let (canonical, draft, private) = hand_authored_parts();
        assert_eq!(bridged.canonical_source(), canonical);
        assert_eq!(bridged.draft(), &draft);
        assert!(bridged.private() == &private);
        assert_eq!(
            bridged.persisted_profile(),
            PersistedFlatImportProfile::CanvasQti12V1
        );
        assert_eq!(
            bridged
                .persisted_conversion_version()
                .expect("conversion version is storage-valid")
                .as_str(),
            QTI_PROFILE_FLAT_CONVERSION_VERSION
        );
        assert_eq!(
            bridged
                .persisted_choice_map()
                .expect("choice map is storage-valid")
                .sha256(),
            bridged
                .mapping_parts()
                .server_choice_map_payload()
                .server_sha256()
        );
        bridged
            .private()
            .validate_for_draft(bridged.draft())
            .expect("private answer binds exact public draft");
    }

    #[test]
    fn blackboard_defaulted_one_point_fixture_matches_hand_authored_source() {
        let bridged = bridge_qti_mapped_item(blackboard_item(), workspace())
            .expect("Blackboard bridge succeeds");
        let (canonical, draft, private) = hand_authored_parts();
        assert_eq!(bridged.canonical_source(), canonical);
        assert_eq!(bridged.draft(), &draft);
        assert!(bridged.private() == &private);
        assert_eq!(bridged.mapping_parts().public_mapping().points, "1.0");
        assert_eq!(
            bridged.persisted_profile(),
            PersistedFlatImportProfile::BlackboardQti21V1
        );
    }

    #[test]
    fn conversion_is_deterministic_and_public_outputs_do_not_reveal_vendor_identifiers() {
        let item = CANVAS_ITEM
            .replacen("blue,red", "bad blue,bad red", 1)
            .replace("ident=\"blue\"", "ident=\"bad blue\"")
            .replace("ident=\"red\"", "ident=\"bad red\"")
            .replace(">blue</varequal>", ">bad blue</varequal>");
        let first = bridge_qti_mapped_item(canvas_item(&item), workspace()).expect("first bridge");
        let second =
            bridge_qti_mapped_item(canvas_item(&item), workspace()).expect("second bridge");
        assert_eq!(first.canonical_source(), second.canonical_source());
        assert_eq!(first.draft(), second.draft());
        assert!(first.private() == second.private());

        let public = serde_json::to_string(first.draft()).expect("public draft serializes");
        let private = first
            .private()
            .canonical_bytes()
            .expect("private canonical bytes");
        let private = String::from_utf8(private).expect("canonical JSON is UTF-8");
        for vendor_identifier in ["bad blue", "bad red"] {
            assert!(!public.contains(vendor_identifier));
            assert!(!private.contains(vendor_identifier));
        }
        assert!(!public.contains("correctChoice"));
        assert!(first.mapping_parts().server_ordered_choice_map().len() == 2);
        let (_, _, _, retained_parts) = first.into_parts();
        assert_eq!(retained_parts.server_ordered_choice_map().len(), 2);
    }
}
