//! Aggregate verification hooks.
//!
//! Private evidence decoding is intentionally centralized here as the shared
//! verifier becomes available.  Callers currently use the locked head and
//! capability functions; this seam prevents route code from decoding JSON.

use super::super::*;
use super::rows::HydratedRun;

pub(super) fn require_active(run: &HydratedRun) -> Result<(), StoreError> {
    run.receipt
        .lifecycle
        .is_active()
        .then_some(())
        .ok_or(StoreError::Conflict)
}
