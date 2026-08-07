//! Within-run completion (MOD-STATE).
//!
//! Implemented in M2. Completion is a *derivation* over the run's attempts,
//! never a stored boolean. A stored flag is what drifts out of sync with the
//! attempts that produced it, and reconciling that drift later is the bug this
//! design avoids.
