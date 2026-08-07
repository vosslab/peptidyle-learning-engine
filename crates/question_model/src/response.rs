//! Response types: what a student submits (WP-C1).
//!
//! Response and grading shapes are enums whose invalid combinations do not
//! compile -- a numeric response paired with a multiple-choice grading rule
//! should be unrepresentable rather than validated at run time.
