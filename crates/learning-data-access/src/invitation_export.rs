//! Protected pending Student Course Invitation export boundary.
//!
//! The Store returns recipient facts only to the server-side download route.
//! It never grants browser access to private invitation or roster tables.

use async_trait::async_trait;
use question_model::CourseInstanceReference;
use serde::Serialize;

use crate::{SessionTokenHash, StoreError};

/// Server-only recipient facts for one pending Student Course Invitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingInvitationRecipient {
    pub email: String,
    pub roster_id: String,
}

/// Server-only pending invitation export facts for one Course Instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingInvitationExport {
    pub course_name: String,
    pub recipients: Vec<PendingInvitationRecipient>,
}

/// JSON document consumed by the existing attended invitation mailer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvitationMailerExport {
    pub course_name: String,
    pub students: Vec<InvitationMailerRecipient>,
}

/// One attended-mailer recipient. The signup URL is common, never individualized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvitationMailerRecipient {
    pub email: String,
    pub signup_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub roster_id: String,
}

impl PendingInvitationExport {
    /// Converts private Store facts into the established download schema.
    pub fn into_mailer_export(self, signup_url: String) -> InvitationMailerExport {
        InvitationMailerExport {
            course_name: self.course_name,
            students: self
                .recipients
                .into_iter()
                .map(|recipient| InvitationMailerRecipient {
                    email: recipient.email,
                    signup_url: signup_url.clone(),
                    display_name: None,
                    roster_id: recipient.roster_id,
                })
                .collect(),
        }
    }
}

/// Session-authorized boundary for downloading pending Student invitations.
#[async_trait]
pub trait InvitationExportStore: Send + Sync {
    /// Returns pending Student Course Invitations for one current direct Instructor.
    async fn export_pending_course_invitations(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<PendingInvitationExport, StoreError>;
}
