#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for immutable Blueprint Revision persistence.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use learning_data_access::postgres::{
    PostgresBlueprintCourseStore, PostgresCourseInstanceStore, lazy_pool,
};
use learning_data_access::{
    BlueprintCourseStore, CourseInstanceCreationSource, CourseInstancePoolIdIssuer,
    CourseInstanceStore, CreateCourseInstanceInput, SessionTokenHash, StoreError,
    StoredBlueprintCourseContent,
};
use question_model::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, BlueprintAssessmentContentInput, BlueprintAssessmentDefaults,
    BlueprintAssessmentEditChoice, BlueprintAssessmentEntryInput,
    BlueprintAssessmentReplacementInput, BlueprintAvailability, BlueprintCourseId,
    BlueprintModuleEditChoice, BlueprintModuleReplacementInput, BlueprintRevisionNumber,
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, LateWorkRule, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionId, QuestionRevisionNumber, QuestionRevisionTuple,
    RenameBlueprintCourseInput, ReplaceBlueprintCourseContentInput, RequestChecksum,
    ReusableFixedQuestionInput, StudentFeedbackReleaseRule,
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
