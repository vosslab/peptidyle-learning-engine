//! Session-authorized Course Instance creation and teaching-team projections.
//!
//! A Course Instance always records one exact Blueprint Revision and one
//! assigned Instructor before it becomes visible.  This deliberately stops
//! before roster, Assignment, or Student delivery concerns.

use async_trait::async_trait;
use question_model::{
    AccountReference, BlueprintCourseReference, BlueprintRevision, CourseId,
    CourseInstanceReference, CourseSummary, CourseTerm,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// Browser request for one new live Course Instance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCourseInstanceInput {
    /// Exact reusable Blueprint Course source.
    pub blueprint_course: BlueprintCourseReference,
    /// Exact immutable Blueprint Revision source.
    pub blueprint_revision: BlueprintRevision,
    /// Durable human-facing Course Instance title.
    pub title: String,
    /// Initial immutable Course Term.
    pub term: CourseTerm,
    /// Required when a Sysadmin creates for an Instructor; omitted by an Instructor creating for self.
    #[serde(default)]
    pub assigned_instructor: Option<AccountReference>,
}

impl CreateCourseInstanceInput {
    /// Keeps the durable Course title within the same bound enforced at persistence.
    pub fn validate(&self) -> Result<(), StoreError> {
        if self.title != self.title.trim()
            || self.title.is_empty()
            || self.title.chars().count() > 200
        {
            return Err(StoreError::InvalidRecord(
                "Course Instance title is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Browser-safe Course Instance landing-page identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceSummary {
    /// Public C-reference only; internal Course IDs never enter this route.
    pub reference: CourseInstanceReference,
    /// Course Instance title.
    pub title: String,
    /// Current initial Course Term snapshot.
    pub term: CourseTerm,
}

/// Minimal Course Instance workspace projection for the current Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceView {
    /// Course identity and Course Term.
    pub course: CourseInstanceSummary,
    /// Whether the current Instructor is the required Assigned Instructor.
    pub is_assigned_instructor: bool,
    /// Current Teaching Team size; creation starts with exactly one Instructor membership.
    pub active_instructor_count: u32,
}

/// Safe active-Instructor selection identity for a Sysadmin creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseCreationInstructor {
    /// Public Account reference carries neither email nor authority.
    pub reference: AccountReference,
}

/// Creation receipt that does not imply the creator has Course access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedCourseInstance {
    /// Newly allocated Course Instance identity.
    pub course: CourseInstanceSummary,
    /// True only when the creator is the Course Instance's Assigned Instructor.
    pub creator_is_assigned_instructor: bool,
}

/// Persistence contract for the Course Instance and initial Teaching Team boundary.
#[async_trait]
pub trait CourseInstanceStore: Send + Sync {
    /// Resolves one public Course reference only for the current active Course Member.
    async fn resolve_course_navigation(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceReference,
    ) -> Result<CourseId, StoreError>;

    /// Reads one browser-safe Course Summary only for the current active Course Member.
    async fn read_course_summary(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseSummary, StoreError>;

    /// Lists only Course Instances where the current Account has an active Instructor Course Membership.
    async fn list_course_instances(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseInstanceSummary>, StoreError>;

    /// Creates one Course Instance atomically from an exact published Blueprint Revision.
    async fn create_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateCourseInstanceInput,
    ) -> Result<CreatedCourseInstance, StoreError>;

    /// Loads the current Instructor's minimal teaching-team workspace projection.
    async fn load_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceReference,
    ) -> Result<CourseInstanceView, StoreError>;

    /// Lists active Instructor Accounts only for the current Sysadmin's explicit assignment choice.
    async fn list_course_creation_instructors(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseCreationInstructor>, StoreError>;
}
