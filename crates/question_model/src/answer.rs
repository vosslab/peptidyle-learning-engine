//! Answer *shapes* -- what a valid response looks like (WP-C1).
//!
//! Read this before adding a type here: the answer format (numeric with
//! tolerance, multiple-choice cardinality, string matching mode) is shared
//! content and belongs in this crate. The answer *value* -- the thing a
//! response is checked against -- is an answer key and belongs in `grading`,
//! which the browser bundle cannot reach. If a type you are about to add would
//! let a caller learn the correct response, it is in the wrong crate.
