//! Answer-key storage and access (MOD-GRD).
//!
//! Implemented in M2. Two gates protect this module: the student-facing
//! database role has no grant on any answer-key table, and `grading` is absent
//! from the WASM dependency closure. Both are tested, not assumed.
