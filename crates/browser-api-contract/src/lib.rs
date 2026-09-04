#![forbid(unsafe_code)]

//! Pure route-only browser API contract values.
//!
//! The admission front door is intentionally empty until a Server Route owns
//! its first DTO. Values introduced here remain owned, runtime-
//! free, and fallible-behavior-free; Axum, persistence, application state, and
//! project tooling stay outside this product boundary.

pub mod grading_operations;
