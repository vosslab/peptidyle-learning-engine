//! Transaction-local actor installation for protected PostgreSQL operations.
//!
//! This module deliberately owns no identity type. The server resolves the
//! opaque session credential and passes its [`crate::ActorContext`] to the
//! database adapter, which evaluates exact course, workspace, or capability
//! relationships inside the protected transaction.
