//! AWS S3 backend (MOD-OBJ).
//!
//! Implemented in M2, behind the `s3` feature so the memory backend and the
//! test suite never pull the AWS SDK. No AWS type may leak through the
//! `ObjectStore` trait: the trait is what adapters code against, and it has to
//! stay portable to MinIO.
