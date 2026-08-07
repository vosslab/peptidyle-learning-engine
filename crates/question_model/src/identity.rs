//! Identity and lifecycle (WP-C2, MOD-ID).
//!
//! `WorkspaceId`, `ProblemId`, and `VersionId` are branded types that cannot
//! substitute for one another, so a draft ID can never be passed where a
//! published problem is required.
//!
//! The lifecycle rule a maintainer can apply in one sentence: a draft lives in
//! an instructor workspace and has no `ProblemId`; publishing is the only
//! transition that constructs one, and a published version is immutable
//! thereafter. External IDs are UUIDv7 or random, never sequential, so a
//! catalog number leaks no volume information.
