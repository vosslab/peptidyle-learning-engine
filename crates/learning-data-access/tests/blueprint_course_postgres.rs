#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for immutable Blueprint Revision persistence.

use learning_data_access::postgres::{
    PostgresBlueprintCourseStore, PostgresBlueprintLineageStore, PostgresCourseInstanceStore,
    PostgresCourseThemeStore, PostgresLiveAssessmentStore, PostgresQuestionLibraryStore, lazy_pool,
};
use learning_data_access::{
    ApplyAssessmentBlueprintUpdateInput, BlueprintCourseStore, BlueprintForkSource,
    BlueprintLineageStore, CourseInstanceCreationSource, CourseInstanceStore, CourseThemeStore,
    CreateCourseInstanceInput, LiveAssessmentStore, QuestionLibraryBackendRestriction,
    QuestionLibrarySearchCursorPosition, QuestionLibrarySearchRequest, QuestionLibrarySearchSort,
    QuestionLibraryStore, QuestionLibraryTextField, QuestionLibraryTextTerm, SessionTokenHash,
    StoreError, StoredBlueprintCourseContent,
};
use question_model::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, BlueprintAssessmentContentInput, BlueprintAssessmentDefaults,
    BlueprintAssessmentEditChoice, BlueprintAssessmentEntryInput,
    BlueprintAssessmentReplacementInput, BlueprintAvailability, BlueprintCourseId,
    BlueprintModuleEditChoice, BlueprintModuleReplacementInput, BlueprintRevisionNumber,
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, LateWorkRule, PublishedQuestionId,
    PublishedQuestionRevisionTuple, QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionPoolId,
    QuestionRevisionNumber, RenameBlueprintCourseInput, ReplaceBlueprintCourseContentInput,
    RequestChecksum, ReusableFixedQuestionInput, StudentFeedbackReleaseRule, Theme,
};
use sqlx::{Connection, PgConnection, Row};
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

#[path = "blueprint_course_postgres/support.rs"]
mod blueprint_course_postgres_support;
use blueprint_course_postgres_support::*;
#[path = "blueprint_course_postgres/adoption.rs"]
mod blueprint_course_postgres_adoption;
#[path = "blueprint_course_postgres/append.rs"]
mod blueprint_course_postgres_append;

#[path = "blueprint_course_postgres/exchange.rs"]
mod blueprint_course_postgres_exchange;

#[path = "blueprint_course_postgres/promotion.rs"]
mod blueprint_course_postgres_promotion;
use blueprint_course_postgres_promotion::{discovery, promotion_boundary};

#[path = "blueprint_course_postgres/lifecycle.rs"]
mod blueprint_course_postgres_lifecycle;

#[path = "blueprint_course_postgres/question_library.rs"]
mod blueprint_course_postgres_question_library;

#[path = "blueprint_course_postgres/question_library_import.rs"]
mod blueprint_course_postgres_question_library_import;

#[path = "blueprint_course_postgres/question_library_stewardship.rs"]
mod blueprint_course_postgres_question_library_stewardship;

#[path = "blueprint_course_postgres/question_revision_metadata.rs"]
mod blueprint_course_postgres_question_revision_metadata;

#[path = "blueprint_course_postgres/sysadmin_correction_publication.rs"]
mod blueprint_course_postgres_sysadmin_correction_publication;

#[path = "blueprint_course_postgres/question_fork_parents.rs"]
mod blueprint_course_postgres_question_fork_parents;

#[path = "blueprint_course_postgres/blueprint_pool_members.rs"]
mod blueprint_course_postgres_pool_members;

#[path = "blueprint_course_postgres/lineage_fork.rs"]
mod blueprint_course_postgres_lineage_fork;

#[path = "blueprint_course_postgres/course_publication_pool_reference.rs"]
mod blueprint_course_postgres_course_publication_pool_reference;
