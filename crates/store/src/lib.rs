//! Backend-neutral persistence contract (WP-C4, MOD-STO).
//!
//! Globally public content needs no tenant context. Institution-visible
//! catalog content goes through [`CatalogStore`], whose reads require
//! [`TenantContext`]. Every educational-record operation also requires that
//! non-defaultable context. Lists require a bounded [`PageRequest`]; the trait
//! has no unbounded or positional paging method. No SQL type appears in this
//! contract.

use async_trait::async_trait;
use domain::completion::{
    RequiredQuestionState, WithinRunCompletion, derive_within_run_completion,
};
use domain::run::RunModelError;
use domain::scoring::RunTransition;
use objects::{Bucket, ObjectCategory, ObjectKey, ObjectRecord};
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, AssetId, AssignmentEnrollment, AssignmentId, AssignmentRun,
    AssignmentSummary, AttemptProvenance, AttemptResult, BackendCapabilities, CatalogLifecycle,
    CatalogProblemSummary, CourseId, CourseMembership, CourseMembershipRole, CourseRole,
    CourseSummary, EnrollmentId, GradePolicy, ObjectId, ProblemId, PublicationScope,
    QuestionAttempt, QuestionAttemptId, QuestionBackend, QuestionDefinition, RunId, RunPolicies,
    StudentAssignmentSummary, StudentResponse, TenantId, UserId, VersionId, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// In-memory backend used by tests and lanes waiting for PostgreSQL.
pub mod memory;
/// Cursor and bounded-page types shared by every list method.
pub mod pagination;
/// PostgreSQL health and future backend implementation.
pub mod postgres;
/// Explicit tenant context used by every educational-record operation.
pub mod rls;
/// Provider-neutral, replica-safe authentication session contract.
pub mod session;

pub use crate::pagination::{Cursor, Page, PageRequest, PageSize, PaginationError};
pub use crate::rls::TenantContext;
pub use crate::session::{
    SessionLifetime, SessionRecord, SessionStore, SessionSubject, SessionSubjectError,
    SessionTokenHash, SessionTokenHashParseError,
};

/// Shared immutable problem/version reference used by catalog lineage.
pub use question_model::ProblemVersionRef;
/// Compatibility name used by the existing assignment contract.
pub type PublishedVersionRef = ProblemVersionRef;

/// Opaque route identifier for either a logical catalog asset or a tenant object.
///
/// The identifier is never minted independently: public content reuses its
/// [`AssetId`], and a student-record artifact reuses its [`ObjectId`]. That
/// lets one stable `/api/assets/{id}` route serve both classes without
/// collapsing their distinct model identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetDeliveryId(Uuid);

impl AssetDeliveryId {
    /// Builds the route identifier for a logical catalog asset.
    pub fn from_asset(asset: AssetId) -> Self {
        Self(asset.as_uuid())
    }

    /// Builds the route identifier for a tenant-owned physical artifact.
    pub fn from_object(object: ObjectId) -> Self {
        Self(object.as_uuid())
    }

    /// Returns the storage UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for AssetDeliveryId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::str::FromStr for AssetDeliveryId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value).map(Self)
    }
}

/// Authorization linkage stored beside one immutable object record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AssetDeliveryScope {
    /// Content asset whose current visibility comes from its published version.
    Catalog {
        /// Logical asset embedded in browser-safe question markup.
        asset: AssetId,
        /// Exact immutable version owning the asset.
        reference: ProblemVersionRef,
    },
    /// Educational-record artifact visible only to explicitly named users.
    StudentRecord {
        /// Direct RLS boundary owning the artifact.
        tenant: TenantId,
        /// Authenticated users allowed to request a short-lived URL.
        authorized_users: Vec<UserId>,
    },
}

/// Database-authoritative mapping from a route ID to exact stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDeliveryRecord {
    /// Stable identifier accepted by `/api/assets/{id}`.
    pub id: AssetDeliveryId,
    /// Immutable metadata returned after object bytes were written.
    pub object: ObjectRecord,
    /// Visibility and ownership linkage checked on every protected request.
    pub scope: AssetDeliveryScope,
}

/// Audit payload appended before a protected signed URL is requested.
///
/// It deliberately contains neither the signed URL nor session credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetAccessEvent {
    /// Tenant in whose security context the request was authorized.
    pub tenant: TenantId,
    /// Authenticated person requesting the protected object.
    pub actor: UserId,
    /// Stable route identifier requested by the actor.
    pub delivery: AssetDeliveryId,
    /// Exact physical object whose URL may be issued.
    pub object: ObjectId,
    /// Bucket whose fixed delivery lifetime applies.
    pub bucket: Bucket,
    /// Database-authoritative authorization time.
    pub occurred_at: ActivityTimestamp,
}

/// Protected object record and the timestamp used to bound its signed URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedAssetDelivery {
    /// Exact immutable object record selected by the database registry.
    pub record: AssetDeliveryRecord,
    /// Database-authoritative time already captured in the access audit.
    pub authorized_at: ActivityTimestamp,
}

/// Tenant-owned editable question draft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftRecord {
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Browser-safe question definition with no `ProblemId`.
    pub question: QuestionDefinition,
    /// Earlier version in the same owned linear chain, for a new revision.
    pub revises: Option<ProblemVersionRef>,
    /// Source version when creating a new attributed fork.
    pub derived_from: Option<ProblemVersionRef>,
}

/// Shared immutable published content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedProblemRecord {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Exact immutable version.
    pub version: VersionId,
    /// Browser-safe definition whose IDs match this record.
    pub question: QuestionDefinition,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Institution-only or cross-tenant catalog visibility.
    pub scope: PublicationScope,
    /// Discoverability and new-assignment state.
    pub lifecycle: CatalogLifecycle,
    /// Ordered, nonempty owners of this problem's linear version chain.
    pub authors: Vec<UserId>,
    /// Earlier version in the same problem chain.
    pub previous_version: Option<VersionId>,
    /// Original source when this problem is a fork.
    pub derived_from: Option<ProblemVersionRef>,
    /// Backend-authoritative time at which this version became immutable.
    pub published_at: ActivityTimestamp,
}

impl PublishedProblemRecord {
    /// Builds the hot browse projection without loading another representation.
    pub fn summary(&self) -> CatalogProblemSummary {
        CatalogProblemSummary {
            problem: self.problem,
            version: self.version,
            backend: QuestionBackend::from(&self.question.source),
            capabilities: self.capabilities.clone(),
            metadata: self.question.metadata.clone(),
            scope: self.scope,
            lifecycle: self.lifecycle.clone(),
            authors: self.authors.clone(),
            previous_version: self.previous_version,
            derived_from: self.derived_from,
            published_at: self.published_at,
        }
    }
}

/// Atomic publication of the exact draft that passed API validation.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishDraftCommand {
    /// Exact draft value validated before entering storage.
    pub expected_draft: DraftRecord,
    /// Fresh problem for a new work/fork, or existing problem for a revision.
    pub problem: ProblemId,
    /// Authenticated author performing the transition.
    pub publisher: UserId,
    /// Institution-only or public target.
    pub scope: PublicationScope,
    /// Trusted capabilities resolved from the server adapter registry.
    pub capabilities: BackendCapabilities,
}

/// Allowed post-publication lifecycle changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogTransition {
    /// Hide a version from browsing and new assignments.
    Deprecate {
        /// Required author explanation.
        reason: String,
    },
    /// Move an already deprecated version to historical status.
    Archive,
}

/// Tenant-owned course, including its explicit course-local access list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRecord {
    /// Durable course identity.
    pub id: CourseId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Human-facing course or section title.
    pub title: String,
    /// Explicit course-local permissions keyed by authenticated user identity.
    pub members: Vec<CourseMembership>,
}

impl CourseRecord {
    /// Returns the browser projection for one direct member or tenant administrator.
    pub fn summary(&self, role: CourseRole) -> CourseSummary {
        CourseSummary {
            id: self.id,
            tenant: self.tenant,
            title: self.title.clone(),
            role,
        }
    }

    /// Resolves one authenticated user's explicit course role.
    pub fn role_for(&self, user: UserId) -> Option<CourseRole> {
        self.members
            .iter()
            .find(|membership| membership.user == user)
            .map(|membership| membership.role.into())
    }
}

/// Explicit scope for course-list authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseListScope {
    /// Return only courses carrying a direct membership for this user.
    Member(UserId),
    /// Return every course in the tenant under coarse administrator authority.
    TenantAdministrator,
}

/// Tenant-owned assignment that references shared immutable content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRecord {
    /// Durable assignment identity.
    pub id: AssignmentId,
    /// Direct RLS boundary.
    pub tenant: TenantId,
    /// Tenant-owned course containing the assignment.
    pub course_id: CourseId,
    /// Human-facing assignment title.
    pub title: String,
    /// Shared problem versions selected for the assignment.
    pub problems: Vec<PublishedVersionRef>,
    /// Four independent run policies.
    pub policies: RunPolicies,
}

/// Bounded client-generated key for replaying one submission safely.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubmissionIdempotencyKey(String);

impl SubmissionIdempotencyKey {
    /// Parses one visible ASCII key without accepting whitespace or controls.
    pub fn parse(value: impl Into<String>) -> Result<Self, StoreError> {
        const MAX_KEY_BYTES: usize = 200;
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_KEY_BYTES
            || !value.is_ascii()
            || value
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err(StoreError::InvalidRecord(
                "idempotency key must contain 1 to 200 visible ASCII characters".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the validated header value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Server-owned data needed to issue or resume one question instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueQuestionAttemptCommand {
    /// Authenticated enrollment owner.
    pub actor: UserId,
    /// Fresh proposed identity; ignored when an active instance already exists.
    pub attempt: QuestionAttemptId,
    /// Active run receiving the question.
    pub run: RunId,
    /// Zero-based logical assignment position.
    pub assignment_position: u32,
    /// Exact immutable content identity at that position.
    pub problem: ProblemId,
    /// Exact immutable version at that position.
    pub question_version: VersionId,
    /// Fresh operating-system-random seed for the proposed instance.
    pub seed: u64,
    /// Hash of the generated parameters.
    pub parameter_hash: String,
    /// Adapter, generator, renderer, source, asset, and grading provenance.
    pub provenance: AttemptProvenance,
}

/// Trusted server result to persist for one student response.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitQuestionAttemptCommand {
    /// Authenticated enrollment owner.
    pub actor: UserId,
    /// Issued question being answered.
    pub attempt: QuestionAttemptId,
    /// Student-controlled response already validated and server-graded.
    pub response: StudentResponse,
    /// Key-free grading result produced inside the server boundary.
    pub result: AttemptResult,
    /// Stable key reused by browser retries of this exact response.
    pub idempotency_key: SubmissionIdempotencyKey,
}

/// First committed submission result or an exact idempotent replay of it.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmissionRecord {
    /// Browser-safe attempt projection with response and disclosed result data.
    pub attempt: QuestionAttempt,
    /// Run after any completion derived by this submission.
    pub run: AssignmentRun,
    /// Compact projection updated in the same transaction as the submission.
    pub summary: StudentAssignmentSummary,
}

impl AssignmentRecord {
    /// Builds the browser-safe assignment projection.
    pub fn summary(&self) -> AssignmentSummary {
        AssignmentSummary {
            id: self.id,
            tenant: self.tenant,
            course_id: self.course_id,
            title: self.title.clone(),
            problems: self.problems.clone(),
            policies: self.policies,
        }
    }
}

/// One atomic activity write applied with its compact summary projection.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityTransition {
    /// Creates a new incomplete run and records its start activity.
    StartRun {
        /// New run with server-supplied start time.
        run: AssignmentRun,
    },
    /// Appends one immutable question-attempt record.
    RecordQuestionAttempt {
        /// Attempt carrying response, result, timing, and reproducibility data.
        attempt: Box<QuestionAttempt>,
    },
    /// Completes an existing run and projects its score.
    CompleteRun {
        /// Existing run to complete.
        run: RunId,
        /// Final score fraction.
        score: f64,
        /// Authoritative PostgreSQL timestamp.
        at: ActivityTimestamp,
    },
}

/// Portable persistence failure with no SQL type in its variants.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    /// Requested record is absent in the active tenant or shared catalog.
    NotFound,
    /// Immutable identity already exists.
    AlreadyExists,
    /// A tenant-owned record disagrees with authenticated context.
    TenantMismatch,
    /// Stored state changed after a caller validated its expected value.
    Conflict,
    /// Authenticated identity lacks ownership or role for the operation.
    Forbidden,
    /// Record shape violates a model invariant.
    InvalidRecord(String),
    /// Pure activity projection rejected the transition.
    RunModel(RunModelError),
    /// The database-authoritative timer no longer accepts this response.
    TimedOut,
    /// Backend state is temporarily unavailable.
    Unavailable(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "record not found"),
            Self::AlreadyExists => write!(formatter, "immutable record already exists"),
            Self::TenantMismatch => write!(formatter, "record tenant does not match context"),
            Self::Conflict => write!(formatter, "record changed before the operation committed"),
            Self::Forbidden => write!(formatter, "operation is not authorized"),
            Self::InvalidRecord(message) => write!(formatter, "invalid record: {message}"),
            Self::RunModel(error) => write!(formatter, "activity transition rejected: {error}"),
            Self::TimedOut => write!(formatter, "question attempt timed out"),
            Self::Unavailable(message) => write!(formatter, "store unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<RunModelError> for StoreError {
    fn from(error: RunModelError) -> Self {
        Self::RunModel(error)
    }
}

/// Immutable asset registry and protected-delivery authorization boundary.
#[async_trait]
pub trait AssetStore: Send + Sync {
    /// Records metadata only after the owning workflow has stored object bytes.
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError>;

    /// Resolves only globally public catalog content for direct CDN delivery.
    ///
    /// Institution content and every educational record deliberately look
    /// absent here so callers cannot bypass the authenticated path.
    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError>;

    /// Authorizes one protected request and appends its audit event atomically.
    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError>;
}

/// Catalog operations that require visibility, ownership, and atomic publish.
#[async_trait]
pub trait CatalogStore: Send + Sync {
    /// Validates the stored draft expectation and atomically publishes it.
    async fn publish_draft(
        &self,
        context: TenantContext,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError>;

    /// Resolves an exact visible version, including deprecated or archived ones.
    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists discoverable hot metadata in stable cursor order.
    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError>;

    /// Lists distinct controlled taxonomy terms in stable cursor order.
    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError>;

    /// Applies an author-owned, one-way post-publication transition.
    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError>;
}

/// Persistence operations consumed by catalog, course, run, and worker lanes.
#[async_trait]
pub trait Store: Send + Sync {
    /// Creates or replaces a tenant-owned editable draft.
    async fn upsert_draft(
        &self,
        context: TenantContext,
        draft: DraftRecord,
    ) -> Result<(), StoreError>;

    /// Reads a draft only inside the active tenant.
    async fn get_draft(
        &self,
        context: TenantContext,
        workspace: WorkspaceId,
    ) -> Result<Option<DraftRecord>, StoreError>;

    /// Resolves one exact globally public version.
    ///
    /// Tenant-visible institution content is intentionally absent; use
    /// [`CatalogStore::get_catalog_problem`] when a session tenant is known.
    async fn get_published_problem(
        &self,
        problem: ProblemId,
        version: VersionId,
    ) -> Result<Option<PublishedProblemRecord>, StoreError>;

    /// Lists globally public, discoverable versions in stable cursor order.
    ///
    /// Tenant-visible institution content is intentionally absent; use
    /// [`CatalogStore::list_catalog`] when a session tenant is known.
    async fn list_published_problems(
        &self,
        page: PageRequest,
    ) -> Result<Page<PublishedProblemRecord>, StoreError>;

    /// Creates or replaces a tenant-owned course and its explicit memberships.
    async fn upsert_course(
        &self,
        context: TenantContext,
        course: CourseRecord,
    ) -> Result<(), StoreError>;

    /// Reads one course inside the active tenant for authorization checks.
    async fn get_course(
        &self,
        context: TenantContext,
        course: CourseId,
    ) -> Result<Option<CourseRecord>, StoreError>;

    /// Lists courses visible to a member or tenant administrator.
    async fn list_courses(
        &self,
        context: TenantContext,
        scope: CourseListScope,
        page: PageRequest,
    ) -> Result<Page<CourseSummary>, StoreError>;

    /// Creates or replaces a tenant-owned assignment definition.
    async fn upsert_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentRecord,
    ) -> Result<(), StoreError>;

    /// Reads one assignment inside the active tenant.
    async fn get_assignment(
        &self,
        context: TenantContext,
        assignment: AssignmentId,
    ) -> Result<Option<AssignmentRecord>, StoreError>;

    /// Lists assignments inside the active tenant in stable cursor order.
    async fn list_assignments(
        &self,
        context: TenantContext,
        course: CourseId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRecord>, StoreError>;

    /// Creates one enrollment and its empty compact summary.
    async fn create_enrollment(
        &self,
        context: TenantContext,
        enrollment: AssignmentEnrollment,
    ) -> Result<(), StoreError>;

    /// Reads one enrollment inside the active tenant.
    async fn get_enrollment(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<AssignmentEnrollment>, StoreError>;

    /// Starts the next run or returns the enrollment's existing active run.
    ///
    /// The backend owns the timestamp, one-based run number, mode, policy,
    /// and compact-summary transition. The proposed ID is used only when a new
    /// run is actually inserted.
    async fn start_or_resume_run(
        &self,
        context: TenantContext,
        actor: UserId,
        assignment: AssignmentId,
        proposed_run: RunId,
    ) -> Result<AssignmentRun, StoreError>;

    /// Issues a fresh question or returns the run's unresolved instance.
    ///
    /// Storage supplies the authoritative issue time and deadline and permits
    /// at most one unresolved question in a run.
    async fn issue_or_resume_question_attempt(
        &self,
        context: TenantContext,
        command: IssueQuestionAttemptCommand,
    ) -> Result<QuestionAttempt, StoreError>;

    /// Lists browser-safe attempt projections for one run in stable cursor order.
    async fn list_question_attempts(
        &self,
        context: TenantContext,
        run: RunId,
        page: PageRequest,
    ) -> Result<Page<QuestionAttempt>, StoreError>;

    /// Returns a prior exact submission before invoking a grading backend again.
    ///
    /// A changed response or key for an already submitted attempt is a conflict.
    async fn replay_submission(
        &self,
        context: TenantContext,
        actor: UserId,
        attempt: QuestionAttemptId,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<Option<SubmissionRecord>, StoreError>;

    /// Atomically records the first response, grade event, run completion, and summary.
    ///
    /// The backend supplies the submission timestamp and applies the authored
    /// timing policy. Exact retries return the first committed record.
    async fn submit_question_attempt(
        &self,
        context: TenantContext,
        command: SubmitQuestionAttemptCommand,
    ) -> Result<SubmissionRecord, StoreError>;

    /// Atomically writes activity and its compact summary projection.
    async fn apply_activity_transition(
        &self,
        context: TenantContext,
        transition: ActivityTransition,
    ) -> Result<StudentAssignmentSummary, StoreError>;

    /// Reads one run inside the active tenant.
    async fn get_run(
        &self,
        context: TenantContext,
        run: RunId,
    ) -> Result<Option<AssignmentRun>, StoreError>;

    /// Lists runs for one enrollment in stable cursor order.
    async fn list_runs(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
        page: PageRequest,
    ) -> Result<Page<AssignmentRun>, StoreError>;

    /// Reads one question attempt inside the active tenant.
    async fn get_question_attempt(
        &self,
        context: TenantContext,
        attempt: QuestionAttemptId,
    ) -> Result<Option<QuestionAttempt>, StoreError>;

    /// Reads the transactionally maintained summary for one enrollment.
    async fn get_summary(
        &self,
        context: TenantContext,
        enrollment: EnrollmentId,
    ) -> Result<Option<StudentAssignmentSummary>, StoreError>;
}

/// Extracts the grade policy used by an activity projection.
pub(crate) fn grade_policy(assignment: &AssignmentRecord) -> GradePolicy {
    assignment.policies.grade
}

/// Maps a storage activity write to the pure domain transition.
pub(crate) fn summary_transition(transition: &ActivityTransition) -> RunTransition {
    match transition {
        ActivityTransition::StartRun { run } => RunTransition::Started { at: run.started_at },
        ActivityTransition::RecordQuestionAttempt { attempt } => {
            RunTransition::QuestionAttemptRecorded {
                at: attempt
                    .timer
                    .submitted_at
                    .unwrap_or(attempt.timer.issued_at),
            }
        }
        ActivityTransition::CompleteRun { score, at, .. } => RunTransition::Completed {
            score: *score,
            at: *at,
        },
    }
}

/// Refuses a tenant-owned record outside the authenticated context.
pub(crate) fn ensure_tenant(
    context: TenantContext,
    record_tenant: TenantId,
) -> Result<(), StoreError> {
    if context.tenant_id() == record_tenant {
        Ok(())
    } else {
        Err(StoreError::TenantMismatch)
    }
}

/// Validates a course record before either backend persists it.
pub(crate) fn validate_course(course: &CourseRecord) -> Result<(), StoreError> {
    validate_title("course", &course.title)?;
    if course.members.is_empty() {
        return Err(StoreError::InvalidRecord(
            "course must have at least one member".to_string(),
        ));
    }
    if !course
        .members
        .iter()
        .any(|membership| membership.role == CourseMembershipRole::Instructor)
    {
        return Err(StoreError::InvalidRecord(
            "course must have at least one instructor".to_string(),
        ));
    }
    let unique_members: std::collections::BTreeSet<_> = course
        .members
        .iter()
        .map(|membership| membership.user)
        .collect();
    if unique_members.len() != course.members.len() {
        return Err(StoreError::InvalidRecord(
            "course memberships must have unique users".to_string(),
        ));
    }
    Ok(())
}

/// Validates assignment fields independent of catalog visibility.
pub(crate) fn validate_assignment(assignment: &AssignmentRecord) -> Result<(), StoreError> {
    validate_title("assignment", &assignment.title)?;
    if assignment.problems.is_empty() {
        return Err(StoreError::InvalidRecord(
            "assignment must reference at least one published problem version".to_string(),
        ));
    }
    Ok(())
}

/// Validates that delivery metadata agrees with the typed immutable object key.
pub(crate) fn validate_asset_delivery(record: &AssetDeliveryRecord) -> Result<(), StoreError> {
    if record.object.id != record.object.key.object_id()
        || record.object.bucket != record.object.key.bucket()
        || record.object.category != record.object.key.category()
        || record.object.version != record.object.key.version_id()
    {
        return Err(StoreError::InvalidRecord(
            "object metadata must agree with its typed key".to_string(),
        ));
    }
    if record.object.media_type.trim().is_empty()
        || record.object.license.trim().is_empty()
        || record.object.provenance.trim().is_empty()
    {
        return Err(StoreError::InvalidRecord(
            "object media type, license, and provenance must not be empty".to_string(),
        ));
    }
    match (&record.scope, &record.object.key) {
        (
            AssetDeliveryScope::Catalog { asset, reference },
            ObjectKey::ProblemAsset {
                problem,
                version,
                asset: key_asset,
                object: _,
            },
        ) if record.id == AssetDeliveryId::from_asset(*asset)
            && *asset == *key_asset
            && reference.problem == *problem
            && reference.version == *version
            && record.object.bucket == Bucket::Content
            && record.object.category == ObjectCategory::Asset => {}
        (
            AssetDeliveryScope::StudentRecord {
                tenant,
                authorized_users,
            },
            ObjectKey::StudentRecord {
                tenant: key_tenant,
                object,
            },
        ) if record.id == AssetDeliveryId::from_object(*object)
            && *tenant == *key_tenant
            && record.object.bucket == Bucket::StudentRecords
            && record.object.category == ObjectCategory::Export =>
        {
            if authorized_users.is_empty() {
                return Err(StoreError::InvalidRecord(
                    "student-record delivery must authorize at least one user".to_string(),
                ));
            }
            let unique: std::collections::BTreeSet<_> = authorized_users.iter().copied().collect();
            if unique.len() != authorized_users.len() {
                return Err(StoreError::InvalidRecord(
                    "student-record authorized users must be unique".to_string(),
                ));
            }
        }
        _ => {
            return Err(StoreError::InvalidRecord(
                "only matching catalog assets and student-record exports may be delivered"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// Derives a completed run score from one current result per assignment position.
pub(crate) fn completed_run_score(
    results: &[Option<AttemptResult>],
    requirement: question_model::CompletionRequirement,
) -> Result<Option<f64>, StoreError> {
    if results.iter().any(Option::is_none) {
        return Ok(None);
    }
    let questions: Vec<_> = results
        .iter()
        .map(|result| {
            let result = result.expect("missing results returned before projection");
            RequiredQuestionState {
                answered: true,
                correct: result.correct,
                points_earned: result.points_earned,
                points_possible: result.points_possible,
            }
        })
        .collect();
    if derive_within_run_completion(&questions, requirement)? == WithinRunCompletion::InProgress {
        return Ok(None);
    }
    let earned: f64 = questions
        .iter()
        .map(|question| question.points_earned)
        .sum();
    let possible: f64 = questions
        .iter()
        .map(|question| question.points_possible)
        .sum();
    if !earned.is_finite() || !possible.is_finite() || possible <= 0.0 {
        return Err(StoreError::RunModel(RunModelError::InvalidQuestionPoints));
    }
    let score = earned / possible;
    if !score.is_finite() || !(0.0..=1.0).contains(&score) {
        return Err(StoreError::RunModel(RunModelError::InvalidQuestionPoints));
    }
    Ok(Some(score))
}

/// Refuses malformed backend grades before they can enter attempt history.
pub(crate) fn validate_attempt_result(result: AttemptResult) -> Result<(), StoreError> {
    if !result.points_earned.is_finite()
        || !result.points_possible.is_finite()
        || result.points_earned < 0.0
        || result.points_possible <= 0.0
        || result.points_earned > result.points_possible
    {
        return Err(StoreError::InvalidRecord(
            "attempt result points must be finite with 0 <= earned <= possible and possible > 0"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_title(kind: &str, title: &str) -> Result<(), StoreError> {
    const MAX_TITLE_CHARS: usize = 200;
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must not be empty"
        )));
    }
    if trimmed.chars().count() > MAX_TITLE_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must contain at most {MAX_TITLE_CHARS} characters"
        )));
    }
    if trimmed != title {
        return Err(StoreError::InvalidRecord(format!(
            "{kind} title must not have leading or trailing whitespace"
        )));
    }
    Ok(())
}

/// Enforces the draft identity invariant before a backend writes bytes.
pub(crate) fn validate_draft(draft: &DraftRecord) -> Result<(), StoreError> {
    if !draft.question.is_draft() {
        return Err(StoreError::InvalidRecord(
            "draft must not carry a ProblemId".to_string(),
        ));
    }
    validate_question_policies(&draft.question)?;
    if draft.revises.is_some() && draft.derived_from.is_some() {
        return Err(StoreError::InvalidRecord(
            "draft cannot be both a revision and a new fork".to_string(),
        ));
    }
    if draft
        .revises
        .or(draft.derived_from)
        .is_some_and(|source| source.version == draft.question.version)
    {
        return Err(StoreError::InvalidRecord(
            "draft version must differ from its lineage source".to_string(),
        ));
    }
    Ok(())
}

/// Enforces published identity agreement before immutable insertion.
pub(crate) fn validate_published(record: &PublishedProblemRecord) -> Result<(), StoreError> {
    if record.question.problem != Some(record.problem) || record.question.version != record.version
    {
        return Err(StoreError::InvalidRecord(
            "published record IDs must match its question definition".to_string(),
        ));
    }
    if record.authors.is_empty() {
        return Err(StoreError::InvalidRecord(
            "published problem must have at least one author".to_string(),
        ));
    }
    validate_question_policies(&record.question)?;
    let mut authors = record.authors.clone();
    authors.sort_unstable();
    authors.dedup();
    if authors.len() != record.authors.len() {
        return Err(StoreError::InvalidRecord(
            "published problem authors must be unique".to_string(),
        ));
    }
    if record
        .previous_version
        .is_some_and(|previous| previous == record.version)
    {
        return Err(StoreError::InvalidRecord(
            "published version cannot revise itself".to_string(),
        ));
    }
    Ok(())
}

fn validate_question_policies(question: &QuestionDefinition) -> Result<(), StoreError> {
    if question.attempt_policy.max_attempts == Some(0) {
        return Err(StoreError::InvalidRecord(
            "question max attempts must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// Maintains enrollment completion and grade-run pointers with a summary.
pub(crate) fn project_enrollment_completion(
    enrollment: &mut AssignmentEnrollment,
    previous: &StudentAssignmentSummary,
    grade: GradePolicy,
    run: RunId,
    score: f64,
    at: ActivityTimestamp,
) {
    let is_first_completion = previous.completed_run_count == 0;
    let is_new_best = previous.best_score.is_none_or(|best| score > best);

    if enrollment.first_completed_at.is_none() {
        enrollment.first_completed_at = Some(at);
    }
    if is_new_best || enrollment.best_grade_run.is_none() {
        enrollment.best_grade_run = Some(run);
    }
    enrollment.current_grade_run = match grade {
        GradePolicy::First if is_first_completion => Some(run),
        GradePolicy::First => enrollment.current_grade_run,
        GradePolicy::Latest => Some(run),
        GradePolicy::Highest if is_new_best => Some(run),
        GradePolicy::Highest => enrollment.current_grade_run,
        GradePolicy::InstructorSelected => enrollment.current_grade_run,
    };
}
