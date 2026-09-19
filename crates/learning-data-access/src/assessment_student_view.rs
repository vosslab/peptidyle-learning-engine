//! Read-only Instructor Student View snapshot boundary.
//!
//! The initial read returns current Assessment policy, exact fixed pins, and
//! exact Pool candidates. A second read reauthorizes one selected Question and
//! returns its immutable render inputs. Rendering and the browser projection
//! remain server responsibilities and cannot create Student Work or Assessment
//! Attempts.

use async_trait::async_trait;
use question_model::{
    AccountTimeZone, AssessmentEditNumber, AssessmentEntryAvailability, AssessmentId,
    AssessmentInstructions, AssessmentQuestionOrderRule, AssessmentStatus, AssessmentTitle,
    CourseInstanceId, DraftImathasQuestionBackendBinding, InstructorStudentViewDelivery, ObjectId,
    QuestionPoolAssessmentEntry, QuestionPoolSelectedItem, QuestionRevisionTuple,
    SourceObjectChecksum,
};

use crate::{ReadyQuestionAssetRendition, SessionTokenHash, StoreError};

/// One authorized current Assessment snapshot used only for Instructor Student View rendering.
///
/// ASVS 8.2.1-8.2.3 and 8.3.1: the Store resolves both opaque route
/// references under the calling session's current direct Instructor Course
/// relationship and returns only the exact fields needed by the render owner.
#[derive(Debug, Clone, PartialEq)]
pub struct InstructorStudentViewSnapshot {
    pub edit_number: AssessmentEditNumber,
    pub status: AssessmentStatus,
    pub title: AssessmentTitle,
    pub instructions: AssessmentInstructions,
    pub display_time_zone: AccountTimeZone,
    pub delivery: InstructorStudentViewDelivery,
    pub assessment_question_order_rule: AssessmentQuestionOrderRule,
    pub entries: Vec<InstructorStudentViewSnapshotEntry>,
}

/// One current authored Assessment Entry and its exact immutable render sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructorStudentViewSnapshotEntry {
    /// One exact fixed Question Revision.
    Fixed {
        /// Zero-based position in the current authored Assessment Entry order.
        authored_position: u32,
        availability: AssessmentEntryAvailability,
        question_revision: QuestionRevisionTuple,
    },
    /// One Assessment-owned Pool with members in current Pool order.
    Pool {
        /// Zero-based position in the current authored Assessment Entry order.
        authored_position: u32,
        availability: AssessmentEntryAvailability,
        assessment_entry: QuestionPoolAssessmentEntry,
        members: Vec<QuestionPoolSelectedItem>,
    },
}

/// Exact backend-specific immutable source needed to render one answer-free preview Question.
///
/// This server-only closed enum prevents source fields for different Question
/// Backends from being combined. It deliberately contains no Attempt issuance,
/// seed, nonce, retained presentation, Student, response, grade, or feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructorStudentViewSource {
    /// Native PLE Question JSON source and ready public asset renditions.
    Ple {
        question_revision: QuestionRevisionTuple,
        source_object_id: ObjectId,
        source_object_checksum: SourceObjectChecksum,
        source_media_type: String,
        question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
    },
    /// WeBWorK PG/PGML source and its canonical registered path.
    Webwork {
        question_revision: QuestionRevisionTuple,
        source_object_id: ObjectId,
        source_object_checksum: SourceObjectChecksum,
        source_media_type: String,
        webwork_pg_path: String,
        question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
    },
    /// iMathAS immutable launch binding; no backend source bytes cross this seam.
    Imathas {
        question_revision: QuestionRevisionTuple,
        source_object_id: ObjectId,
        source_object_checksum: SourceObjectChecksum,
        source_media_type: String,
        /// Immutable source-bound deployment and item. The adapter resolves
        /// its configured render profile without adding Attempt persistence.
        imathas_question_backend_binding: DraftImathasQuestionBackendBinding,
        question_asset_renditions: Vec<ReadyQuestionAssetRendition>,
    },
}

/// Session-authorized read-only persistence boundary for Instructor Student View.
#[async_trait]
pub trait InstructorStudentViewStore: Send + Sync {
    /// Loads one coherent current snapshot through a direct Course Instructor relationship.
    ///
    /// Implementations start a read-only repeatable-read transaction before
    /// authorization so all Assessment pins, Pool members, policies, and
    /// availability facts belong to the same database snapshot.
    async fn load_instructor_student_view_snapshot(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<InstructorStudentViewSnapshot, StoreError>;

    /// Reauthorizes one exact Question against the current Assessment snapshot.
    ///
    /// The expected Edit Number is the manifest precondition. Implementations
    /// return a source only when the Question Revision is the exact fixed entry
    /// or a member of the Assessment-owned Pool at `authored_position`.
    async fn load_instructor_student_view_question_source(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
        expected_edit_number: AssessmentEditNumber,
        authored_position: u32,
        question_revision: QuestionRevisionTuple,
    ) -> Result<InstructorStudentViewSource, StoreError>;
}
