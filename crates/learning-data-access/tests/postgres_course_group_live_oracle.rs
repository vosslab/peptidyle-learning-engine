#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle entry point for the T2 group boundary.
//!
//! It intentionally starts by proving that a disposable endpoint presents the
//! fully migrated application schema.  The adjacent S3/S5 ignored live tests
//! remain the source-owned fixtures for publication, issue, receipt-history,
//! and RLS mechanics; this file reserves the behavior-named T2 entry point
//! without creating a divergent private fixture.

#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
use course_creation_support::sysadmin_course_creation_authority;

#[path = "fixtures/published_assignment.rs"]
mod published_assignment;

use domain::effective_assignment_policy::{
    BaseAssignmentPolicy, GroupAccommodation, GroupScheduleOffset, IndividualPolicyException,
    PolicyModificationMode, PolicyPatch, PolicyPatchSet, PolicySource, ScheduleOffsetSeconds,
};
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentRecord, AssignmentUpdate, CatalogStore, CourseGroupManagementStore,
    CourseGroupMembershipWarning, CourseGroupRecord, CourseRecord, CourseRosterStore,
    CreateCourseCommand, DeleteGroupAccommodationCommand, DeleteGroupScheduleOffsetCommand,
    DeleteIndividualPolicyExceptionCommand, DraftRecord, FlatGradingCapability,
    IssueQuestionAttemptCommand, LearnerWorkRoutingBinding, PageRequest, PageSize,
    PresentationCapability, PutCourseGroupCommand, PutGroupAccommodationCommand,
    PutGroupScheduleOffsetCommand, PutIndividualPolicyExceptionCommand, ReplaceAssignmentCommand,
    Store, StoreError, StoredIndividualPolicyException, TenantContext, UpsertCourseMember,
    WebworkGradingCapability,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentPolicyExceptionId, AssignmentScoringMode, AttemptStatus, BackendCapabilities,
    Capability, CourseGroupId, CourseGroupPurpose, CourseId, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ImplementationVersion, PointValue, ProblemId,
    ProblemVersionRef, PublicationScope, QuestionAttemptId, QuestionMetadata, QuestionSource,
    ResponseDefinition, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
use std::num::NonZeroU32;
use uuid::Uuid;

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
#[path = "postgres_course_group_live_oracle/fixture.rs"]
mod fixture;
#[path = "postgres_course_group_live_oracle/policy_history.rs"]
mod policy_history;
#[path = "postgres_course_group_live_oracle/reresolution.rs"]
mod reresolution;
use acceptance_runtime::load as load_acceptance_runtime;
