//! MOD-EXPORT: the print model and its writers.
//!
//! A question whose capabilities mark it unexportable is refused before the
//! build starts, so a printable exam never contains a placeholder.

/// Microsoft Word output.
pub mod docx;
/// PDF output, the standard printed-exam artifact.
pub mod pdf;
