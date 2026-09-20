//! Real per-Student Assessment time configuration access boundary.
use crate::{SessionTokenHash, StoreError};
use async_trait::async_trait;
use question_model::{
    AssessmentId, AssessmentStudentTimeAccommodation, CourseInstanceId,
    SaveAssessmentStudentTimeAccommodationInput,
};

/// Direct-Instructor Course authorization is repeated for every read/save.
#[async_trait]
pub trait AssessmentStudentTimeAccommodationStore: Send + Sync {
    /// Loads one active Student by the existing Course roster identifier.
    async fn student_time_configuration(
        &self,
        token: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
        roster_id: String,
        save: Option<SaveAssessmentStudentTimeAccommodationInput>,
    ) -> Result<AssessmentStudentTimeAccommodation, StoreError>;
}
