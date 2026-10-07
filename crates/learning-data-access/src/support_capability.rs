//! Confirmed, audited Sysadmin access to one named Student roster record.

use async_trait::async_trait;
use question_model::{AccountId, CourseInstanceId, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{SessionTokenHash, StoreError};

/// Explicit confirmation required before a protected Student-data read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmedStudentDataAccess {
    pub administrative_access_confirmed: bool,
}

impl ConfirmedStudentDataAccess {
    pub fn validate(self) -> Result<(), StoreError> {
        if self.administrative_access_confirmed {
            Ok(())
        } else {
            Err(StoreError::InvalidRecord(
                "Administrative Student-data access must be confirmed".to_string(),
            ))
        }
    }
}

/// Receipt for the immutable access event written with the protected read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentDataAccessAuditReceipt {
    pub event_id: Uuid,
    pub occurred_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SysadminStudentRosterState {
    ActiveStudent,
    InvitationPending,
    Removed,
}

/// The smallest named roster projection needed for administrative support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysadminStudentRosterRecord {
    pub course_instance_id: CourseInstanceId,
    pub student_account_id: AccountId,
    pub roster_id: String,
    pub roster_name: String,
    pub state: SysadminStudentRosterState,
    pub audit: StudentDataAccessAuditReceipt,
}

#[async_trait]
pub trait SysadminStudentDataStore: Send + Sync {
    /// Reads and audits one named roster record in one transaction. SQL derives
    /// the acting account from the authenticated session.
    async fn read_sysadmin_student_roster_record(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        roster_id: String,
        confirmation: ConfirmedStudentDataAccess,
    ) -> Result<Option<SysadminStudentRosterRecord>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_student_read_requires_explicit_positive_confirmation() {
        let confirmed: ConfirmedStudentDataAccess =
            serde_json::from_str(r#"{"administrativeAccessConfirmed":true}"#)
                .expect("confirmation payload");
        assert!(confirmed.validate().is_ok());

        let unconfirmed: ConfirmedStudentDataAccess =
            serde_json::from_str(r#"{"administrativeAccessConfirmed":false}"#)
                .expect("false confirmation payload");
        assert!(unconfirmed.validate().is_err());
        assert!(serde_json::from_str::<ConfirmedStudentDataAccess>("{} ").is_err());
        assert!(
            serde_json::from_str::<ConfirmedStudentDataAccess>(
                r#"{"administrativeAccessConfirmed":true,"accountId":"attacker"}"#
            )
            .is_err()
        );
    }
}
