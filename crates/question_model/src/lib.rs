//! MOD-QM: the root contract of the Peptidyle Learning Engine.
//!
//! Question types, backend capabilities, identity, and taxonomy live here.
//! No answer-bearing type is ever defined in this crate: answer keys and
//! correctness decisions belong to `grading`, which is server-only.
//!
//! Module bodies are frozen in M1 (WP-C1 through WP-C3).

pub mod answer;
pub mod capability;
pub mod envelope;
pub mod generation;
pub mod identity;
pub mod response;
pub mod run_policy;
pub mod taxonomy;
