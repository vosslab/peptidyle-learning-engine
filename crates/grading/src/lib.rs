//! MOD-GRD: answer keys, checkers, and correctness decisions.
//!
//! Server-only. This crate sits outside the `wasm_bridge` dependency closure
//! so an answer key cannot reach the browser even by mistake. Nothing here may
//! be re-exported from `wasm_bridge`.

/// Answer checking implementations.
pub mod checker;
/// Integrity contract and private material for the built-in flat Question Format.
pub mod flat_question;
/// Gated access to Answer Keys and other private Question grading records.
pub mod key;

pub use crate::checker::{
    GradingError, QuestionGradingOutcome, grade, question_statistics_observation,
};
pub use crate::key::AnswerKey;
