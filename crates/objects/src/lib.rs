//! MOD-OBJ: the `ObjectStore` trait and its backends.
//!
//! Keys are built only from identity types and versions, never from a
//! caller-supplied string, so one tenant cannot address another tenant's
//! artifact. Checksums are computed on write and verified on read.

/// Bucket lifecycle for the `content`, `student-records`, and
/// `temp-processing` buckets.
pub mod bucket;
/// In-memory backend used by tests and the M1 conformance suite.
pub mod memory;
/// MinIO backend for the development and test containers.
pub mod minio;
/// AWS S3 backend for production deployments.
pub mod s3;
