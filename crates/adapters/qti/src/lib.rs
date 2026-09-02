//! MOD-ADP-QTI: QTI import and export.
//!
//! Import is hostile-input territory: the M4 gate is a hostile-ZIP corpus
//! rejected in full with actionable errors. Unsupported QTI features are
//! recorded rather than silently dropped, and the original package is archived
//! through `objects` so it stays re-importable.

/// QTI XML parsing and the import pipeline.
mod archive;
mod model;
pub mod parser;
pub mod profiles;
mod xml;

pub use crate::parser::{
    ImportedQtiPackage, ImportedQtiQuestion, QtiAssetObject, QtiAssetReferenceError,
    QtiImportError, QtiImportLimits, QtiImporter, QtiItemImportResult, QtiItemImportStatus,
    QtiManifest, QtiResource, UnsupportedFeature, qti_question_asset_checksums,
};
pub use crate::profiles::{
    QTI_PROFILE_MATRIX, QtiImportChecksums, QtiImportResultChecksumInput, QtiMappingVersion,
    QtiPrivateChoiceMapChecksumInput, QtiPrivateFeedbackChecksumInput,
    QtiPrivateMappingChecksumInput, QtiProfileContractError, QtiProfileDetection,
    QtiProfileDetectionEvidence, QtiProfileDiagnostic, QtiProfileDiagnosticCode, QtiProfileId,
    QtiProfileItemEvidence, QtiProfileMatrixDetail, QtiProfileResourceEvidence, QtiProfileVersion,
    QtiPublicChoiceChecksumInput, QtiPublicMappingChecksumInput, QtiWorkspaceImportItemResult,
    detect_qti_profile,
};
