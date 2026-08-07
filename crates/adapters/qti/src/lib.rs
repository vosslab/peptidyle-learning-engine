//! MOD-ADP-QTI: QTI import and export.
//!
//! Import is hostile-input territory: the M4 gate is a hostile-ZIP corpus
//! rejected in full with actionable errors. Unsupported QTI features are
//! recorded rather than silently dropped, and the original package is archived
//! through `objects` so it stays re-importable.

/// QTI XML parsing and the import pipeline.
pub mod parser_stub;
