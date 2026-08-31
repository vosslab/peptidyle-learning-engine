//! Focused persistence foundations for the clean single-installation baseline.
//!
//! Product adapters are added only with an exact account, course-membership,
//! Student-ownership, workspace, observer-grant, or worker-lease contract.

use domain::run::AssignmentActivityError;

mod authentication_ceremony;
mod authentication_email;
mod pagination;
pub mod postgres;
mod random_uuid;
pub mod session;
#[path = "contracts/store_error.rs"]
mod store_error;

pub use authentication_ceremony::{
    AuthenticatedAccount, AuthenticationCeremonyLifetime, AuthenticationCeremonyStore,
    AuthenticationSecretHash, EmailAuthenticationChallenge, EmailAuthenticationChallengeId,
    EmailAuthenticationPurpose, MAX_AUTHENTICATION_CEREMONY_SECONDS, Passkey, PasskeyId,
    WebauthnCeremonyId,
};
pub use authentication_email::{
    AuthenticationEmail, AuthenticationEmailError, EmailDomain, MAX_AUTHENTICATION_EMAIL_BYTES,
};
pub use pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use session::{
    SessionId, SessionLifetime, SessionRecord, SessionStore, SessionTokenHash,
    SessionTokenHashParseError,
};
pub use store_error::StoreError;
