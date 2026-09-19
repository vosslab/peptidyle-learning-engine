//! Opaque Student Work identities and deterministic Issued Question identity.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A course-owned assessment offered to Students.
///
/// This is the canonical public ID (`AXXXXXXXZ`), not a second identifier.
pub use crate::public_route::AssessmentId;

/// One stable Assessment Entry within an Assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssessmentEntryId(Uuid);

/// One immutable Question Pool result for one Assessment Attempt and one Assessment Entry.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionPoolSelectionId(Uuid);

/// A Course Instance containing Assessments.
///
/// This is the canonical public ID (`CIXXXXXXXZ`), not a second identifier.
pub use crate::public_route::CourseInstanceId;

/// One durable Course Membership record.
///
/// This historical identity is also the current-membership lock target.
/// Revocation and later re-enrollment retain earlier Student Work evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CourseMembershipId(Uuid);

/// One durable Student Record in a Course Instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StudentRecordId(Uuid);

/// One direct Student Accommodation attached to an Assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccommodationId(Uuid);

/// One pass through an Assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssessmentAttemptId(Uuid);

/// One Question issued inside an Assessment Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IssuedQuestionId(Uuid);

/// One server-issued try for one Issued Question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionAttemptId(Uuid);

/// One immutable accepted Student Response for one Question Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionResponseId(Uuid);

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

        impl std::str::FromStr for $name {
            type Err = &'static str;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value)
                    .map(Self)
                    .map_err(|_| "must be a UUID")
            }
        }
    };
}

impl_student_work_identifier!(AssessmentEntryId);
impl_student_work_identifier!(QuestionPoolSelectionId);
impl_student_work_identifier!(CourseMembershipId);
impl_student_work_identifier!(StudentRecordId);
impl_student_work_identifier!(AccommodationId);
impl_student_work_identifier!(AssessmentAttemptId);
impl_student_work_identifier!(IssuedQuestionId);
impl_student_work_identifier!(QuestionAttemptId);
impl_student_work_identifier!(QuestionResponseId);

impl IssuedQuestionId {
    /// Derives the stable identity for one frozen Assessment Attempt entry.
    ///
    /// A Pool member distinguishes pooled Issued Questions. A fixed
    /// Question has no Pool member, so its explicit discriminator prevents a collision.
    pub fn for_frozen_content(
        assessment_attempt: AssessmentAttemptId,
        assessment_entry: AssessmentEntryId,
        pool: Option<(&crate::QuestionId, crate::QuestionPoolEditNumber, u32)>,
    ) -> Self {
        let mut name = Vec::with_capacity(96);
        name.extend_from_slice(assessment_attempt.as_uuid().as_bytes());
        name.extend_from_slice(assessment_entry.as_uuid().as_bytes());
        if let Some((question_pool_id, question_pool_edit_number, member_position)) = pool {
            name.push(1);
            name.extend_from_slice(question_pool_id.as_str().as_bytes());
            name.extend_from_slice(&question_pool_edit_number.get().to_be_bytes());
            name.extend_from_slice(&member_position.to_be_bytes());
        }
        Self(Uuid::new_v5(&ISSUED_QUESTION_NAMESPACE, &name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_question_identity_is_stable_and_distinguishes_frozen_content() {
        let attempt = AssessmentAttemptId::from_uuid(Uuid::from_u128(1));
        let entry = AssessmentEntryId::from_uuid(Uuid::from_u128(2));
        let question_pool_id: crate::QuestionId = "7654-Z321".parse().expect("valid Pool ID");
        let question_pool_edit_number =
            crate::QuestionPoolEditNumber::new(1).expect("positive Pool Edit Number");
        let fixed = IssuedQuestionId::for_frozen_content(attempt, entry, None);
        let pooled = IssuedQuestionId::for_frozen_content(
            attempt,
            entry,
            Some((&question_pool_id, question_pool_edit_number, 0)),
        );

        assert_eq!(
            fixed,
            IssuedQuestionId::for_frozen_content(attempt, entry, None)
        );
        assert_ne!(fixed, pooled);
        assert_eq!(pooled.as_uuid().get_version_num(), 5);
    }

    #[test]
    fn assessment_attempt_id_parses_canonical_uuid_text() {
        let id = AssessmentAttemptId::from_uuid(Uuid::from_u128(1));
        assert_eq!(id.to_string().parse::<AssessmentAttemptId>(), Ok(id));
        assert_eq!(
            "not-a-uuid".parse::<AssessmentAttemptId>(),
            Err("must be a UUID")
        );
    }
}
