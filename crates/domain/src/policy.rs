//! Assignment configuration validation (MOD-CAP).
//!
//! Implemented in M2. `validate_assignment_config` returns every violation
//! rather than the first one, and each violation names the question and the
//! capability it needs, because an instructor fixing an assignment wants the
//! whole list, not one error per save.
