//! QTI XML parsing and the import pipeline (MOD-ADP-QTI).
//!
//! Implemented in M4. Parsing runs against untrusted uploads, so entity
//! expansion, absolute and traversing archive paths, and declared-versus-actual
//! sizes are all rejected rather than trusted.
