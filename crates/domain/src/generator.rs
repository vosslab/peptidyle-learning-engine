//! Seeded generation entry point (MOD-GEN).
//!
//! Implemented in M2. `generate(seed, spec)` must be deterministic across the
//! server and the browser: same seed, same bytes. `crates/domain/tests/`
//! carries the seed-vector table and the parity harness that proves it, and
//! that test is the reason this crate may not reach a clock or a database.
