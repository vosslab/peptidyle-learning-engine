//! `MemoryStore`: the in-memory backend (MOD-STO stub).
//!
//! Implemented in M1 (WP-C4). Every API lane builds against this before
//! PostgreSQL exists. It runs the same conformance suite the PostgreSQL
//! backend later runs unchanged, including the cursor-pagination cases.
