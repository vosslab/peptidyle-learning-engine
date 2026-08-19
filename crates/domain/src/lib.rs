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
/// Key-free deterministic workspace-draft prompt preview (MOD-WASM).
pub mod draft_preview;
/// Pure current assignment-policy resolution after S5 entitlement.
pub mod effective_assignment_policy;
/// Pure current-entitlement evaluation. Persistence supplies normalized facts;
/// this module never reads a roster, database, clock, or browser token.
pub mod entitlement;
/// Seeded question generation (MOD-GEN).
pub mod generator;
/// Current tenant-owned course item-analysis projections (MOD-STATS).
pub mod item_analysis;
/// Assignment configuration validation (MOD-CAP).
pub mod policy;
/// Continued-practice eligibility and shared run-model errors (MOD-RUN).
pub mod run;
/// Completed-run score selection and summary projection (MOD-SCORE).
pub mod scoring;
/// Retention-safe anonymous question-statistics aggregation (MOD-STATS).
pub mod statistics;
/// Timer verdict for time-limited attempts (MOD-TIME).
pub mod timing;
/// Browser-safe student-response format validation (MOD-GRD boundary).
pub mod validation;
