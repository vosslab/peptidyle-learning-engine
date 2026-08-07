//! MOD-ADP-H5P: the H5P adapter for ungraded practice.
//!
//! H5P evaluates in the browser, so this adapter declares
//! `serverGrading: false` and never reaches `grading`. Declaring the
//! limitation honestly is the M4 acceptance criterion: an H5P activity must
//! not be usable where a graded assignment is required.

/// Import path from an H5P package into the internal question model.
pub mod import_stub;
