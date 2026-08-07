//! MOD-ADP-NAT: the first-party algorithmic question adapter.
//!
//! This adapter is the boundary's test case: it exercises the most capability
//! flags with the least rendering machinery, so the adapter contract is
//! stressed before any UI exists. The M4 acceptance criterion for every other
//! adapter is zero diff inside `crates/domain`, and this crate is where that
//! contract is first proven.

/// Seeded parameter generation for the native question family (MOD-GEN).
pub mod generator;
/// Response-widget rendering hooks, wired to the UI in M3.
pub mod renderer_stub;
