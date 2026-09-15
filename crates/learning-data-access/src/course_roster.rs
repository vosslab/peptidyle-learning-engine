//! Course Roster Import, invitation claim, and access-revocation boundary.
//!
//! Import resolves a Student Authentication Email to one global Student
//! Account (creating it only when absent) and records a pending Course
//! Invitation. Claim is the separate transaction that creates the stable
//! Student Record and active Student Course Membership for the exact Course
//! Instance. No Assessment or Student-work record is created here.

use async_trait::async_trait;
use question_model::CourseInstanceReference;
use serde::{Deserialize, Serialize};

use crate::{AuthenticationEmail, SessionTokenHash, StoreError};

/// A single normalized roster row supplied by a current Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRosterImportEntry {
    /// Student Authentication Email used only for private account resolution.
    pub email: String,
    /// Course-scoped institutional roster identifier; never an Account identity.
    pub roster_id: String,
}

/// One bounded Course Roster Import request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRosterImportInput {
    /// The reviewed import rows to commit atomically.
    pub entries: Vec<CourseRosterImportEntry>,
}

/// Private, validated import row retained only through the Store transaction.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ValidatedCourseRosterImportEntry {
    pub normalized_email: String,
    pub delivery_email: String,
    pub roster_id: String,
}

impl CourseRosterImportInput {
    /// Validates the small reviewed import shape before account resolution begins.
    pub(crate) fn validated_entries(
        &self,
    ) -> Result<Vec<ValidatedCourseRosterImportEntry>, StoreError> {
        if self.entries.is_empty() || self.entries.len() > 50 {
            return Err(StoreError::InvalidRecord(
                "Course Roster Import must contain between one and fifty rows".to_string(),
            ));
        }
        let mut emails = std::collections::BTreeSet::new();
        let mut roster_ids = std::collections::BTreeSet::new();
        self.entries
            .iter()
            .map(|entry| {
                let email = AuthenticationEmail::parse(&entry.email).map_err(|_| {
                    StoreError::InvalidRecord("Course Roster Import email is invalid".to_string())
                })?;
                // ASVS 2.2.1, 2.2.2: enforce the Student institutional-email
                // business rule at the trusted roster-import boundary before an
                // Account lookup or creation can occur.
                if !is_institutional_student_email(&email) {
                    return Err(StoreError::InvalidRecord(
                        "Course Roster Import requires a Student institutional email".to_string(),
                    ));
                }
                if !emails.insert(email.normalized().to_string()) {
                    return Err(StoreError::InvalidRecord(
                        "Course Roster Import repeats a Student Authentication Email".to_string(),
                    ));
                }
                let roster_id = entry.roster_id.trim();
                if roster_id.is_empty()
                    || roster_id.len() > 64
                    || !roster_id.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
                    || !roster_ids.insert(roster_id.to_string())
                {
                    return Err(StoreError::InvalidRecord(
                        "Course Roster Import roster identifier is invalid or repeated".to_string(),
                    ));
                }
                Ok(ValidatedCourseRosterImportEntry {
                    normalized_email: email.normalized().to_string(),
                    delivery_email: email.delivery().to_string(),
                    roster_id: roster_id.to_string(),
                })
            })
            .collect()
    }
}

/// PLE currently recognizes United States institutional Student addresses by
/// their exact `.edu` domain suffix. The normalized `EmailDomain` prevents
/// a lookalike suffix (for example, `university.edu.example`) from passing.
fn is_institutional_student_email(email: &AuthenticationEmail) -> bool {
    email.domain().as_str().ends_with(".edu")
}

/// Browser-safe state of one course-scoped roster entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRosterEntry {
    /// Course-local roster identifier used for the roster and later export only.
    pub roster_id: String,
    /// Pending invitation or current Student Course Membership state.
    pub state: CourseRosterEntryState,
}

/// The only roster states visible to the direct Teaching Team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseRosterEntryState {
    /// The student has not yet claimed the Course Invitation.
    InvitationPending,
    /// The Student Record and active Student Course Membership exist.
    ActiveStudent,
}

/// Claim result intentionally omits every private Account and Student Record identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimedCourseInvitation {
    /// A current Student Course Membership now exists for the authenticated Student.
    pub active_student_membership: bool,
}

/// Session-authorized persistence boundary for the roster lifecycle.
#[async_trait]
pub trait CourseRosterStore: Send + Sync {
    /// Lists the direct Instructor's authorized current roster projection.
    async fn list_course_roster(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<Vec<CourseRosterEntry>, StoreError>;

    /// Resolves or creates Student Accounts and records pending Course Invitations atomically.
    async fn import_course_roster(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        input: CourseRosterImportInput,
    ) -> Result<Vec<CourseRosterEntry>, StoreError>;

    /// Lets the authenticated invitation target create or reuse its Student Record and membership.
    async fn claim_course_invitation(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
    ) -> Result<ClaimedCourseInvitation, StoreError>;

    /// Ends a pending invitation or active Student Course Membership without deleting records.
    async fn revoke_course_roster_entry(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceReference,
        roster_id: String,
    ) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use super::{CourseRosterImportEntry, CourseRosterImportInput};

    #[test]
    fn roster_import_accepts_institutional_student_identity() {
        let input = CourseRosterImportInput {
            entries: vec![CourseRosterImportEntry {
                email: "Student@biology.roosevelt.edu".to_string(),
                roster_id: "bio301-student".to_string(),
            }],
        };

        let entries = input
            .validated_entries()
            .expect("institutional Student identity is valid for roster import");
        assert_eq!(entries[0].normalized_email, "student@biology.roosevelt.edu");
    }

    #[test]
    fn roster_import_rejects_noninstitutional_address_before_account_resolution() {
        for email in ["student@example.com", "student@university.edu.example"] {
            let input = CourseRosterImportInput {
                entries: vec![CourseRosterImportEntry {
                    email: email.to_string(),
                    roster_id: "bio301-student".to_string(),
                }],
            };

            match input.validated_entries() {
                Err(crate::StoreError::InvalidRecord(message)) => assert_eq!(
                    message,
                    "Course Roster Import requires a Student institutional email"
                ),
                _ => panic!("noninstitutional email must be rejected before account resolution"),
            }
        }
    }
}
