//! H5P Package Import for ungraded practice.
//!
//! H5P evaluates in the browser and never reaches the Question Backend or
//! grading lifecycles. The import result remains ungraded practice and cannot
//! be used where a graded assignment is required.

/// Import path from an H5P package into the internal question model.
pub mod import;

pub use import::{
    ArchivedH5pPackage, H5pArchiveError, H5pArchiveResolver, H5pChoice, H5pImportError,
    H5pImportRequest, H5pImporter, H5pPackageImportFingerprint, H5pPackageImportReference,
    H5pUnsupportedFeature, IMPORT_SCHEMA_VERSION, ImportedH5pQuestion,
};
