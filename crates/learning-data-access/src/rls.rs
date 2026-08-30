//! Transaction-local account installation for protected PostgreSQL operations.
//!
//! This module deliberately owns no identity type. The server resolves the
//! opaque session credential and passes the resolved account and session facts to the
//! database adapter, which evaluates exact course, workspace, or capability
//! relationships inside the protected transaction.
