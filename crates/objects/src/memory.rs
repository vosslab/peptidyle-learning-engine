//! `MemoryObjectStore`: the in-memory backend (MOD-OBJ stub).
//!
//! Implemented in M1 (WP-C4). This backend exists so adapter and API lanes can
//! start before MinIO or S3 is wired. It runs the same conformance suite the
//! MinIO and S3 backends later run unchanged -- that shared suite is the
//! contract, not this implementation.
