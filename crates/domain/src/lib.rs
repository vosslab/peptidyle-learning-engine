//! MOD-DOMAIN: attempt state, runs, timing, generation, and validation.
//!
//! This crate reaches `question_model` and nothing else, so it has no clock
//! and no database. Time and storage arrive as parameters, which is what lets
//! the same code run on the server and in the browser through `wasm_bridge`
//! and makes the seed-parity test meaningful.

/// Attempt lifecycle and data structures (MOD-RUN).
pub mod attempt;
/// Completion derivation within a run (MOD-STATE).
pub mod completion;
/// Seeded question generation (MOD-GEN).
pub mod generator;
/// Assignment configuration validation (MOD-CAP).
pub mod policy;
/// Score aggregation and grade display (MOD-SCORE).
pub mod scoring;
/// Timer verdict for time-limited attempts (MOD-TIME).
pub mod timing;
