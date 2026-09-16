//! Session-authorized Course Instance creation and teaching-team projections.
//!
//! A Course Instance begins empty or adopts one exact Blueprint Revision, and
//! records one initial accountable Instructor before it becomes visible. This
//! deliberately stops before roster, Assessment, or Student delivery concerns.

use async_trait::async_trait;
use question_model::{
    AccountReference, BlueprintCourseReference, BlueprintRevision, CourseId,
    CourseInstanceReference, CourseSummary, CourseTerm, CourseTheme, QuestionId,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

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
        /// Exact reusable Blueprint Course source.
        blueprint_course: BlueprintCourseReference,
        /// Exact immutable Blueprint Revision source.
        blueprint_revision: BlueprintRevision,
    },
}

/// Browser request for one new live Course Instance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCourseInstanceInput {
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
    pub assigned_instructor: Option<AccountReference>,
}

impl CreateCourseInstanceInput {
    /// Keeps both durable Course Instance names within the persistence bound.
    pub fn validate(&self) -> Result<(), StoreError> {
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
    /// Public C-reference only; internal Course IDs never enter this route.
    pub reference: CourseInstanceReference,
    /// Compact Course Instance name for constrained navigation.
    pub short_name: String,
    /// Descriptive Course Instance name for headings and breadcrumbs.
    pub long_name: String,
    /// Current initial Course Term snapshot.
    pub term: CourseTerm,
    /// Course-owned identity for a list row; this does not establish route scope.
    pub theme: CourseTheme,
}

fn valid_name(value: &str) -> bool {
    !value.is_empty() && value == value.trim() && value.chars().count() <= 200
}

/// Minimal Course Instance workspace projection for the current Teaching Team Member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceView {
    /// Course identity and Course Term.
    pub course: CourseInstanceSummary,
    /// Current Teaching Team size; creation starts with exactly one Instructor membership.
    pub active_instructor_count: u32,
    /// Original adoption and current head, only while the actor can read the source.
    pub blueprint_origin: Option<CourseInstanceBlueprintOrigin>,
}

/// Read-only adoption provenance, not a claim about current Assessment content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseInstanceBlueprintOrigin {
    /// Readable parent Blueprint public identity.
    pub reference: BlueprintCourseReference,
    /// Immutable Revision originally adopted when the Course was created.
    pub adopted_revision: BlueprintRevision,
    /// Current readable source Revision, without applying any changes.
    pub current_revision: BlueprintRevision,
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
}

/// Server-only issuer for the fresh public identity of each Assessment-owned
/// Question Pool fork created while adopting a Blueprint.  The browser never
/// supplies this value and persistence never receives the HMAC capability.
pub trait CourseInstancePoolIdIssuer: Send + Sync {
    /// Issues one candidate canonical Question Pool public ID.
    fn issue_question_pool_id(&self) -> Result<QuestionId, StoreError>;
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
        course: CourseInstanceReference,
        instructor: AccountReference,
    ) -> Result<(), StoreError>;

    /// Loads the current Instructor's minimal teaching-team workspace projection.
    async fn load_course_instance(
        &self,
        session_token_hash: SessionTokenHash,
        reference: CourseInstanceReference,
    ) -> Result<CourseInstanceView, StoreError>;

    /// Lists active Instructor Accounts only for the current Sysadmin's explicit assessment choice.
    async fn list_course_creation_instructors(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<CourseCreationInstructor>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::CourseInstanceCreationSource;

    #[test]
    fn creation_source_accepts_only_the_closed_browser_wire() {
        let empty = serde_json::json!({"kind": "empty"});
        let adopted = serde_json::json!({
            "kind": "adopted",
            "blueprintCourse": "BP7K3M2Q",
            "blueprintRevision": "1"
        });
        assert!(serde_json::from_value::<CourseInstanceCreationSource>(empty).is_ok());
        assert!(serde_json::from_value::<CourseInstanceCreationSource>(adopted).is_ok());
        assert!(
            serde_json::from_value::<CourseInstanceCreationSource>(serde_json::json!({
                "kind": "adopted",
                "blueprint_course": "BP7K3M2Q",
                "blueprint_revision": "1"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CourseInstanceCreationSource>(serde_json::json!({
                "kind": "adopted",
                "blueprintCourse": "BP7K3M2Q",
                "blueprintRevision": "1",
                "blueprint_course": "BP7K3M2Q"
            }))
            .is_err()
        );
    }
}
