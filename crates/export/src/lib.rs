//! MOD-EXPORT: answer-key-free print models and deterministic artifact writers.
//!
//! The worker resolves immutable, published assets before it calls this crate.
//! This crate receives verified bytes only: it never receives an object key,
//! a URL, account-ownership information, or an answer key.

/// Microsoft Word output.
pub mod docx;
/// PDF output.
pub mod pdf;

mod print_exam;

pub use crate::print_exam::{
    ExportArtifact, ExportBundle, ExportCandidate, ExportabilityError, PrintExam, PrintLayout,
    PrintQuestion, PrintableAsset, TrustedAssetResolver, UnexportableQuestion,
};

pub(crate) use crate::print_exam::{FlowBlock, exam_flow};
