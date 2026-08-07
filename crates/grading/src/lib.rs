//! MOD-GRD: answer keys, checkers, and correctness decisions.
//!
//! Server-only. This crate sits outside the `wasm_bridge` dependency closure
//! so an answer key cannot reach the browser even by mistake. Nothing here may
//! be re-exported from `wasm_bridge`.

/// Answer checking implementations.
pub mod checker;
/// Gated access to grading materials and answer keys.
pub mod key;

pub use crate::key::AnswerKey;
