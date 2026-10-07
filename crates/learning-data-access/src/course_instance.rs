//! Session-authorized Course Instance creation and teaching-team projections.
//!
//! A Course Instance begins empty or adopts one exact Blueprint Revision, and
//! records one initial accountable Instructor before it becomes visible. This
//! deliberately stops before roster, Assessment, or Student delivery concerns.

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintCourseRevisionTuple, CourseInstanceId, CourseSummary, CourseTerm, Theme,
};
use serde::{Deserialize, Serialize};

use crate::{DiscoveryPageRequest, Page, SessionTokenHash, StoreError};

/// Closed source for a new live Course Instance.
///
/// An Empty Course is intentionally independent of every Blueprint read and
/// materialization path. An Adopted Course names one immutable source
/// Revision. The tagged JSON representation rejects old flat source fields.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CourseInstanceCreationSource {
    /// Start a teaching Course without imported Blueprint content.
    Empty,
    /// Materialize content from exactly this reusable Blueprint Revision.
    Adopted {
        /// Exact immutable Blueprint Course plus Revision Number source.
        blueprint_course_revision_tuple: BlueprintCourseRevisionTuple,
    },
}

/// Browser request for one new live Course Instance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCourseInstanceInput {
    /// Explicit Course metadata, independent of adopted content.
    pub classification: question_model::CourseClassification,
    /// Explicit Empty or exact Adopted Course source.
    pub source: CourseInstanceCreationSource,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and breadcrumbs.
    pub long_name: String,
    /// Initial immutable Course Term.
    pub term: CourseTerm,
    /// Required when a Sysadmin creates for an Instructor; omitted by an Instructor creating for self.
    #[serde(default)]
    pub assigned_instructor_account_id: Option<AccountId>,
}

impl CreateCourseInstanceInput {
    /// Keeps both durable Course Instance names within the persistence bound.
    pub fn validate(&self) -> Result<(), StoreError> {
        self.classification
            .validate()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        if !valid_name(&self.short_name) || !valid_name(&self.long_name) {
            return Err(StoreError::InvalidRecord(
                "Course Instance name is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

/// Browser-safe Course Instance landing-page identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceSummary {
    pub classification: question_model::CourseClassification,
    /// Stored Course activity state; it is independent of Student-data retention.
    pub lifecycle_state: CourseInstanceLifecycleState,
    pub course_edit_number: question_model::CourseEditNumber,
    /// Public Course Instance ID; internal UUIDs never enter this route.
    pub id: CourseInstanceId,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and breadcrumbs.
    pub long_name: String,
    /// Current initial Course Term snapshot.
    pub term: CourseTerm,
    /// Course-owned identity for a list row; this does not establish route scope.
    pub theme: Theme,
}

pub use question_model::CourseInstanceLifecycleState;

/// Closed Student-data retention state. The value is Course status, not a Student count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CourseRetentionLifecycleState {
    Active,
    Archived,
    Deleted,
}

/// Sysadmin inspection of one installation Course. Student records stay out of this projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationCourseInspection {
    /// Public Course Instance ID.
    pub id: CourseInstanceId,
    /// Compact Course Instance name.
    pub short_name: String,
    /// Descriptive Course Instance name.
    pub long_name: String,
    /// Inclusive Course Term.
    pub term: CourseTerm,
    /// Course activity, independent of Student-data retention.
    pub lifecycle_state: CourseInstanceLifecycleState,
    /// Student-data retention state, without enrollment counts.
    pub retention_lifecycle_state: CourseRetentionLifecycleState,
    /// Public display names of the active Instructors. Empty when none are recorded.
    pub instructor_display_names: Vec<String>,
}

fn valid_name(value: &str) -> bool {
    !value.is_empty() && value == value.trim() && value.chars().count() <= 200
}

/// Minimal Course Instance workspace projection for the current Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceView {
    /// Course identity and Course Term.
    pub course_instance: CourseInstanceSummary,
    /// Current Teaching Team size; creation starts with exactly one Instructor membership.
    pub active_instructor_count: u32,
    /// Original adoption and current head, only while the actor can read the source.
    pub blueprint_origin: Option<CourseInstanceBlueprintOrigin>,
}

/// Read-only adoption provenance, not a claim about current Assessment content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceBlueprintOrigin {
    /// Immutable Blueprint Revision originally adopted when the Course was created.
    pub adopted_blueprint_course_revision_tuple: BlueprintCourseRevisionTuple,
    /// Current readable source Blueprint Revision, without applying any changes.
    pub current_blueprint_course_revision_tuple: BlueprintCourseRevisionTuple,
}

/// Safe active-Instructor selection identity for a Sysadmin creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseCreationInstructor {
    /// Public Account ID carries neither email nor authority.
    pub account_id: AccountId,
}

/// Creation receipt that does not imply the creator has Course access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedCourseInstance {
    /// Newly allocated Course Instance identity.
    pub course_instance: CourseInstanceSummary,
}

/// Persistence contract for the Course Instance and initial Teaching Team boundary.
#[async_trait]
pub trait CourseInstanceStore: Send + Sync {
    /// Changes only Course metadata under the current co-Instructor authority.
    async fn update_course_classification(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        expected_edit_number: question_model::CourseEditNumber,
        classification: question_model::CourseClassification,
    ) -> Result<CourseClassificationUpdate, StoreError>;
    /// Resolves one public Course Instance ID only for the current active Course Member.
    async fn resolve_course_navigation(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<CourseInstanceId, StoreError>;

    /// Reads one browser-safe Course Summary only for the current active Course Member.
    async fn read_course_summary(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<CourseSummary, StoreError>;

    /// Lists only Course Instances where the current Account has an active Instructor Course Membership.
    async fn list_course_instances(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseInstanceSummary>, StoreError>;

    /// Creates one Course Instance atomically from an exact saved Blueprint Revision.
    async fn create_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateCourseInstanceInput,
    ) -> Result<CreatedCourseInstance, StoreError>;

    /// Adds an active Instructor Course Membership when the current active
    /// Instructor is already a member. Membership, rather than creator or
    /// assigned-Instructor status, is the authority boundary.
    async fn add_course_instructor(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
        instructor: AccountId,
    ) -> Result<(), StoreError>;

    /// Loads the current Instructor's minimal teaching-team workspace projection.
    async fn load_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<CourseInstanceView, StoreError>;

    /// Lists active Instructor Accounts only for the current Sysadmin's explicit assessment choice.
    async fn list_course_creation_instructors(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseCreationInstructor>, StoreError>;

    /// Lists installation Courses for the current Sysadmin. An empty query returns the first page.
    async fn list_installation_courses(
        &self,
        session_token_hash: SessionTokenHash,
        query: &str,
        page: DiscoveryPageRequest,
    ) -> Result<Page<InstallationCourseInspection>, StoreError>;

    /// Reads one installation Course for the current Sysadmin.
    async fn load_installation_course(
        &self,
        session_token_hash: SessionTokenHash,
        course_instance_id: CourseInstanceId,
    ) -> Result<InstallationCourseInspection, StoreError>;
}

/// Accepted metadata state; no-op updates keep their validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseClassificationUpdate {
    pub classification: question_model::CourseClassification,
    pub course_edit_number: question_model::CourseEditNumber,
    pub changed: bool,
}

#[cfg(test)]
mod tests {
    use super::CourseInstanceCreationSource;

    #[test]
    fn creation_requires_explicit_course_classification() {
        let mut input = serde_json::json!({
            "source": {"kind": "empty"},
            "shortName": "Genetics",
            "longName": "Genetics Course",
            "term": {"startDate": "2026-09-01", "endDate": "2026-12-01"}
        });
        assert!(serde_json::from_value::<super::CreateCourseInstanceInput>(input.clone()).is_err());
        input["classification"] = serde_json::json!({
            "disciplineUuid": "00000000-0000-0000-0000-000000000001",
            "subjectUuid": null,
            "topicUuid": null,
            "subtopicUuid": null,
            "tags": []
        });
        let parsed: super::CreateCourseInstanceInput =
            serde_json::from_value(input).expect("explicit classification");
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn creation_source_accepts_only_the_closed_browser_wire() {
        let empty = serde_json::json!({"kind": "empty"});
        let blueprint_course = question_model::BlueprintCourseId::from_random_identity("7K3M2QX")
            .expect("canonical Blueprint Course ID");
        let adopted = serde_json::json!({
            "kind": "adopted",
            "blueprintCourseRevisionTuple": {
                "blueprintCourseId": blueprint_course,
                "revisionNumber": "1"
            }
        });
        assert!(serde_json::from_value::<CourseInstanceCreationSource>(empty).is_ok());
        assert!(serde_json::from_value::<CourseInstanceCreationSource>(adopted).is_ok());
        assert!(
            serde_json::from_value::<CourseInstanceCreationSource>(serde_json::json!({
                "kind": "adopted",
                "blueprint_course": question_model::BlueprintCourseId::from_random_identity("7K3M2QX")
                    .expect("canonical Blueprint Course ID"),
                "blueprint_revision": "1"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CourseInstanceCreationSource>(serde_json::json!({
                "kind": "adopted",
                "blueprintCourseId": question_model::BlueprintCourseId::from_random_identity("7K3M2QX")
                    .expect("canonical Blueprint Course ID"),
                "blueprintRevisionNumber": "1"
            }))
            .is_err(),
            "split Blueprint Course ID plus Revision Number siblings are not a Tuple"
        );
        assert!(
            serde_json::from_value::<CourseInstanceCreationSource>(serde_json::json!({
                "kind": "adopted",
                "blueprintCourseRevisionTuple": {
                    "blueprintCourseId": question_model::BlueprintCourseId::from_random_identity("7K3M2QX")
                        .expect("canonical Blueprint Course ID"),
                    "revisionNumber": "1"
                },
                "blueprintCourseId": question_model::BlueprintCourseId::from_random_identity("7K3M2QX")
                    .expect("canonical Blueprint Course ID")
            }))
            .is_err()
        );
    }
}
