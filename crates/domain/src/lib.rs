//! MOD-DOMAIN: attempt state, runs, timing, generation, and validation.
//!
//! This crate reaches `question_model` and nothing else, so it has no clock
//! and no database. Time and storage arrive as parameters, which is what lets
//! the same code run on the server and in the browser through `wasm_bridge`
//! and makes the seed-parity test meaningful.

/// Attempt state machine (MOD-STATE).
pub mod attempt;
/// Completion derivation within a run (MOD-STATE).
pub mod completion;
/// Seeded question generation (MOD-GEN).
pub mod generator;
/// Assignment configuration validation (MOD-CAP).
pub mod policy;
/// Continued-practice eligibility and shared run-model errors (MOD-RUN).
pub mod run;
/// Completed-run score selection and summary projection (MOD-SCORE).
pub mod scoring;
/// Timer verdict for time-limited attempts (MOD-TIME).
pub mod timing;
/// Browser-safe student-response format validation (MOD-GRD boundary).
pub mod validation;
