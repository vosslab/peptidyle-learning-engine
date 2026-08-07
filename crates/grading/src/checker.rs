//! Answer checking (MOD-GRD).
//!
//! Implemented in M2. Server-only: this code path decides correctness and
//! therefore sees answer keys. Nothing here may be re-exported through
//! `wasm_bridge`, and no response body may carry a key.
