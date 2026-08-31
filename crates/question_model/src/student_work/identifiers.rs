//! Opaque Student Work identities and deterministic Issued Question identity.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A course-owned assignment offered to Students.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentId(Uuid);

/// One stable current-state item within an Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentEntryId(Uuid);

/// One stable candidate inside its owning Question Pool Assignment Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionPoolCandidateId(Uuid);

/// One immutable Question Pool result for one Assignment Attempt and one Assignment Entry.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionPoolSelectionId(Uuid);

/// A Course Instance containing Assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseId(Uuid);

/// One durable Course Membership record.
///
/// This historical identity is also the current-membership lock target.
/// Revocation and later re-enrollment retain earlier Student Work evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseMembershipId(Uuid);

/// One durable Student Record in a Course Instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StudentRecordId(Uuid);

/// One direct Student Accommodation attached to an Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccommodationId(Uuid);

/// One pass through an Assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssignmentAttemptId(Uuid);

/// One Question issued inside an Assignment Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IssuedQuestionId(Uuid);

/// One server-issued try for one Issued Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionAttemptId(Uuid);

/// One immutable accepted Student Response for one Question Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionSubmissionId(Uuid);

/// Fixed UUIDv5 namespace for durable Issued Question identity derivation.
const ISSUED_QUESTION_NAMESPACE: Uuid = Uuid::from_u128(0xf3d3_b213_5c59_4e39_a76c_196f_82b0_620a);

/// Gives a Student Work identity its shared storage and display behavior.
macro_rules! impl_student_work_identifier {
    ($name:ident) => {
        impl $name {
            /// Wraps a UUID read from storage or an authenticated boundary.
            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the UUID used by storage and logging.
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Mints a fresh server-owned identifier.
            #[cfg(feature = "generate")]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

impl_student_work_identifier!(AssignmentId);
impl_student_work_identifier!(AssignmentEntryId);
impl_student_work_identifier!(QuestionPoolCandidateId);
impl_student_work_identifier!(QuestionPoolSelectionId);
impl_student_work_identifier!(CourseId);
impl_student_work_identifier!(CourseMembershipId);
impl_student_work_identifier!(StudentRecordId);
impl_student_work_identifier!(AccommodationId);
impl_student_work_identifier!(AssignmentAttemptId);
impl_student_work_identifier!(IssuedQuestionId);
impl_student_work_identifier!(QuestionAttemptId);
impl_student_work_identifier!(QuestionSubmissionId);

impl IssuedQuestionId {
    /// Derives the stable identity for one frozen Assignment Attempt entry.
    ///
    /// A Question Pool candidate distinguishes pooled issued questions. A fixed
    /// Question has no candidate, so its explicit discriminator prevents a
    /// collision with a pooled value containing all-zero UUID bytes.
    pub fn for_frozen_content(
        assignment_attempt: AssignmentAttemptId,
        assignment_entry: AssignmentEntryId,
        question_pool_candidate: Option<QuestionPoolCandidateId>,
    ) -> Self {
        let mut name = [0_u8; 49];
        name[..16].copy_from_slice(assignment_attempt.as_uuid().as_bytes());
        name[16..32].copy_from_slice(assignment_entry.as_uuid().as_bytes());
        if let Some(candidate) = question_pool_candidate {
            name[32] = 1;
            name[33..].copy_from_slice(candidate.as_uuid().as_bytes());
        }
        Self(Uuid::new_v5(&ISSUED_QUESTION_NAMESPACE, &name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_question_identity_is_stable_and_distinguishes_frozen_content() {
        let attempt = AssignmentAttemptId::from_uuid(Uuid::from_u128(1));
        let entry = AssignmentEntryId::from_uuid(Uuid::from_u128(2));
        let candidate = QuestionPoolCandidateId::from_uuid(Uuid::from_u128(3));
        let fixed = IssuedQuestionId::for_frozen_content(attempt, entry, None);
        let pooled = IssuedQuestionId::for_frozen_content(attempt, entry, Some(candidate));

        assert_eq!(
            fixed,
            IssuedQuestionId::for_frozen_content(attempt, entry, None)
        );
        assert_ne!(fixed, pooled);
        assert_eq!(pooled.as_uuid().get_version_num(), 5);
    }
}
