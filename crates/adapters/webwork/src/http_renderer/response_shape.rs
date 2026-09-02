//! Fixed public JSON member set of the external webwork-pg-renderer image.

/// The renderer response is closed so unexpected upstream response fields refuse.
pub(super) const RESPONSE_KEYS: &[&str] = &[
    "JWT",
    "debug",
    "flags",
    "problem_result",
    "problem_state",
    "renderedHTML",
    "resources",
];
