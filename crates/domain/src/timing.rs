//! Timer verdicts for time-limited attempts (MOD-TIME).
//!
//! Implemented in M2. `timer_verdict(...)` is a pure function of timestamps
//! passed in by the caller -- this crate never reads the clock. Server time is
//! authoritative; the browser's clock is display only. That split is what
//! makes the verdict invariant under client clock skew, which M5 proves.
