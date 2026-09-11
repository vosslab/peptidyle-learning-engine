//! Authenticated Instructor self-profile preference boundary.

use async_trait::async_trait;
use question_model::AccountTimeZone;
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Self-profile preferences returned for the authenticated Instructor.
pub struct InstructorProfile {
    pub time_zone: AccountTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// Requested self-profile preference changes for the authenticated Instructor.
pub struct UpdateInstructorProfileInput {
    pub time_zone: AccountTimeZone,
}

#[async_trait]
/// Reads and updates self-profile preferences for the authenticated Instructor only.
pub trait InstructorProfileStore: Send + Sync {
    async fn read_instructor_profile(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<InstructorProfile, StoreError>;

    async fn update_instructor_profile(
        &self,
        session_token_hash: SessionTokenHash,
        input: UpdateInstructorProfileInput,
    ) -> Result<InstructorProfile, StoreError>;
}
