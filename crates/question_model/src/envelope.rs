//! The question envelope: the rendered payload a client receives (WP-C1).
//!
//! The envelope is what MOD-UI-RENDER maps to components, so every block kind
//! it can contain is enumerated here rather than left open. It carries no
//! answer key and no grading material: a browser network trace containing
//! either is an M3 gate failure.
