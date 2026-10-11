//! Browser-safe global account identity and account roles.

use serde::{Deserialize, Serialize};

/// One global login account, distinct from a course enrollment's student record.
///
/// This is the canonical public ID (`UXXXXXXXZ`), not a second identifier.
pub use crate::public_route::AccountId;

/// The one immutable global User Role assigned to an Account.
///
/// Course-specific permissions remain exact course relationships and do not
/// become global session User Roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserRole {
    /// Completes assigned Questions and views personal results.
    Student,
    /// Authors content and manages courses and assignments.
    Instructor,
    /// Manages the platform and approves real-person instructor access.
    Sysadmin,
}
