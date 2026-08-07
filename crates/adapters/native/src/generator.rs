//! Seeded parameter generation for the native question family (MOD-GEN).
//!
//! Implemented in M2. The contract that matters: generation is a pure function
//! of `(seed, spec)` and must produce byte-identical output on the server and
//! in the browser, which is what `crates/domain/tests/seed_vectors.json`
//! locks down in WP-C5.
//!
//! The question family itself (peptide sequence and molecular weight is the
//! leading candidate) is confirmed with the owner at MOD-ADP-NAT entry.
