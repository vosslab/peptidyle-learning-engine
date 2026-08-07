//! Bucket lifecycle (MOD-OBJ).
//!
//! Implemented in M2. Three buckets with different rules, which is why they
//! are three buckets and not three prefixes in one:
//!
//! - `content`: source packages, shared assets, cached renders. Immutable CDN
//!   URLs for public content, authorized 60-minute URLs for secure content,
//!   retained indefinitely and versioned.
//! - `student-records`: exports, uploaded responses, annotated exams.
//!   Authorized 5-minute URLs, always logged, explicitly deletable.
//! - `temp-processing`: extraction and conversion workspaces. Never served,
//!   lifecycle-expired in days.
//!
//! A course deletion must remove `student-records` artifacts while leaving
//! `content` untouched; separate buckets make that a policy, not a filter.
