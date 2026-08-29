//! Typed decoding for the learner-work source-preparation broker.
//!
//! The broker locks and authorizes the source facts under its protected role.
//! This module treats its result as an untrusted database boundary: callers
//! receive a value only after every identity, enum, and ordered collection is
//! checked against the command that caused the capability call.

use domain::entitlement::EntitlementDenial;
use question_model::{
    AssignmentId, AttemptStatus, CourseGroupId, CourseId, CourseMembershipId, EnrollmentId,
    QuestionAttemptId, RunId, TenantId, UserId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use crate::{MaterializeAssignmentEntitlementCommand, StoreError, StudentWorkRoutingBinding};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntitlementPreparationAuthority {
    StudentSelfService,
    StudentSelf,
    DirectInstructor,
}

impl EntitlementPreparationAuthority {
    fn database_value(self) -> &'static str {
        match self {
            Self::StudentSelfService => "student_self_service",
            Self::StudentSelf => "student_self",
            Self::DirectInstructor => "direct_instructor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WitnessAssignmentLifecycle {
    Draft,
    Published,
    Closed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WitnessAudienceKind {
    CourseWide,
    AnyOfGroups,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EntitlementPreparationWitness {
    pub(super) tenant: TenantId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) authority: EntitlementPreparationAuthority,
    pub(super) actor: UserId,
    pub(super) authority_membership: CourseMembershipId,
    pub(super) learner: UserId,
    pub(super) student_membership: CourseMembershipId,
    pub(super) assignment_revision: u64,
    pub(super) lifecycle: WitnessAssignmentLifecycle,
    pub(super) audience_kind: WitnessAudienceKind,
    pub(super) audience_groups: Vec<CourseGroupId>,
    pub(super) current_groups: Vec<CourseGroupId>,
    pub(super) existing_enrollment: Option<EnrollmentId>,
}

pub(super) enum EntitlementPreparationDecision {
    Granted(EntitlementPreparationWitness),
    Denied(EntitlementDenial),
}

/// Strict structural witness for one Student-owned run preparation.
///
/// The SQL wrapper cannot represent an attempt target, so this closed value
/// proves that the broker prepared only the exact run named by the command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StudentRunPreparationWitness {
    pub(super) source: EntitlementPreparationWitness,
    pub(super) run: RunId,
    pub(super) locked_summary_enrollments: Vec<EnrollmentId>,
}

/// Strict answer-free witness for one exact Student-owned attempt aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StudentAttemptPreparationWitness {
    pub(super) source: EntitlementPreparationWitness,
    pub(super) run: RunId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) attempt_status: AttemptStatus,
    pub(super) locked_summary_enrollments: Vec<EnrollmentId>,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedEntitlementWitness {
    tenant: TenantId,
    course: CourseId,
    assignment: AssignmentId,
    learner: UserId,
    actor: UserId,
    authority: EntitlementPreparationAuthority,
}

/// Prepares a materialization command through the closed app authority map.
/// Rule authority is deliberately unavailable to `ple_app`; its future
/// grader-owned route must call the distinct rule capability.
pub(super) async fn prepare_entitlement_materialization(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    command: MaterializeAssignmentEntitlementCommand,
) -> Result<EntitlementPreparationDecision, StoreError> {
    let (actor, authority) = match command.authority() {
        question_model::MaterializationAuthority::Rule(_) => return Err(StoreError::Forbidden),
        question_model::MaterializationAuthority::Actor(actor) => match command.purpose() {
            question_model::EntitlementPurpose::StartRun if actor == command.learner() => {
                (actor, EntitlementPreparationAuthority::StudentSelfService)
            }
            question_model::EntitlementPurpose::GradeBearingAction
                if actor == command.learner() =>
            {
                (actor, EntitlementPreparationAuthority::StudentSelfService)
            }
            question_model::EntitlementPurpose::InstructorIssue => {
                (actor, EntitlementPreparationAuthority::DirectInstructor)
            }
            question_model::EntitlementPurpose::GradeBearingAction => {
                (actor, EntitlementPreparationAuthority::DirectInstructor)
            }
            _ => return Err(StoreError::Forbidden),
        },
    };
    let expected = ExpectedEntitlementWitness {
        tenant,
        course: command.course(),
        assignment: command.assignment(),
        learner: command.learner(),
        actor,
        authority,
    };
    let rows = sqlx::query(
        "SELECT * FROM public.ple_prepare_entitlement_materialization($1, $2, $3, $4, $5, $6)",
    )
    .bind(tenant.as_uuid())
    .bind(command.course().as_uuid())
    .bind(command.assignment().as_uuid())
    .bind(command.learner().as_uuid())
    .bind(authority.database_value())
    .bind(actor.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(super::map_sqlx_error)?;
    let [row] = rows.as_slice() else {
        return Err(StoreError::Unavailable(
            "learner-work preparation returned an unexpected row count".to_string(),
        ));
    };
    let decision_kind: String = row
        .try_get("decision_kind")
        .map_err(|_| invalid_witness("has a missing decision kind"))?;
    match decision_kind.as_str() {
        "granted" => {
            decode_entitlement_witness(row, expected).map(EntitlementPreparationDecision::Granted)
        }
        "learner_not_active_course_student" => {
            validate_denied_entitlement_witness(raw_entitlement_witness(row))?;
            Ok(EntitlementPreparationDecision::Denied(
                EntitlementDenial::StudentNotActiveCourse,
            ))
        }
        _ => Err(invalid_witness("has an unknown decision kind")),
    }
}

/// Calls the run-specific 1817 wrapper before any protected source read and
/// strictly decodes its answer-free structural witness.
pub(super) async fn prepare_student_run_work(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: StudentWorkRoutingBinding,
    actor: UserId,
    run: RunId,
) -> Result<StudentRunPreparationWitness, StoreError> {
    let expected = ExpectedEntitlementWitness {
        tenant,
        course: binding.course,
        assignment: binding.assignment,
        learner: actor,
        actor,
        authority: EntitlementPreparationAuthority::StudentSelf,
    };
    let rows = sqlx::query(
        "SELECT tenant_id, course_id, assignment_id, authority_kind, actor_id, \
                authority_membership_id, learner_id, student_membership_id, \
                assignment_revision, assignment_lifecycle, audience_kind, \
                locked_audience_count, locked_audience_group_ids, \
                locked_current_group_count, locked_current_group_ids, \
                existing_enrollment_id, run_id, locked_summary_count, \
                locked_summary_enrollment_ids \
           FROM public.ple_prepare_student_run_work($1, $2, $3, $4, $5)",
    )
    .bind(tenant.as_uuid())
    .bind(binding.course.as_uuid())
    .bind(binding.assignment.as_uuid())
    .bind(actor.as_uuid())
    .bind(run.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(super::map_sqlx_error)?;
    let [row] = rows.as_slice() else {
        return Err(StoreError::Unavailable(
            "learner-work preparation returned an unexpected row count".to_string(),
        ));
    };
    decode_student_run_witness(row, expected, run)
}

/// Calls the exact-attempt 1817 capability before any protected source read.
/// SQL values are bound parameters (ASVS 1.2.4), and the returned structural
/// witness is positively validated before it becomes authority (ASVS 2.2.1,
/// 8.2.2).
pub(super) async fn prepare_student_attempt_work(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    binding: StudentWorkRoutingBinding,
    actor: UserId,
    attempt: QuestionAttemptId,
) -> Result<StudentAttemptPreparationWitness, StoreError> {
    let expected = ExpectedEntitlementWitness {
        tenant,
        course: binding.course,
        assignment: binding.assignment,
        learner: actor,
        actor,
        authority: EntitlementPreparationAuthority::StudentSelf,
    };
    let rows = sqlx::query(
        "SELECT tenant_id, course_id, assignment_id, authority_kind, actor_id, \
                authority_membership_id, learner_id, student_membership_id, \
                assignment_revision, assignment_lifecycle, audience_kind, \
                locked_audience_count, locked_audience_group_ids, \
                locked_current_group_count, locked_current_group_ids, \
                existing_enrollment_id, run_id, attempt_id, attempt_status, \
                locked_summary_count, locked_summary_enrollment_ids \
           FROM public.ple_prepare_attempt_work($1, $2, $3, $4, $5, 'student_self')",
    )
    .bind(tenant.as_uuid())
    .bind(binding.course.as_uuid())
    .bind(binding.assignment.as_uuid())
    .bind(actor.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_student_attempt_preparation_error)?;
    let [row] = rows.as_slice() else {
        return Err(StoreError::Unavailable(
            "learner-work preparation returned an unexpected row count".to_string(),
        ));
    };
    validate_student_attempt_witness(
        raw_entitlement_witness(row),
        cell(row, "run_id"),
        cell(row, "attempt_id"),
        cell(row, "attempt_status"),
        cell(row, "locked_summary_count"),
        cell(row, "locked_summary_enrollment_ids"),
        expected,
        attempt,
    )
}

/// 1817 deliberately uses this exact SQLSTATE/message pair for a concealed
/// route or current-entitlement denial. Ordinary `42501` failures remain
/// infrastructure/privilege defects rather than becoming a false 404.
fn map_student_attempt_preparation_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database_error) = &error
        && map_student_attempt_preparation_database_error(
            database_error.code().as_deref(),
            database_error.message(),
        )
        .is_some()
    {
        return StoreError::NotFound;
    }
    super::map_sqlx_error(error)
}

fn map_student_attempt_preparation_database_error(code: Option<&str>, message: &str) -> Option<()> {
    (code == Some("42501") && message == "learner work is unavailable").then_some(())
}

#[cfg(test)]
mod preparation_error_tests {
    use super::*;

    #[test]
    fn exact_1817_policy_denial_is_the_only_concealed_permission_error() {
        assert_eq!(
            map_student_attempt_preparation_database_error(
                Some("42501"),
                "learner work is unavailable"
            ),
            Some(())
        );
        assert_eq!(
            map_student_attempt_preparation_database_error(Some("42501"), "permission denied"),
            None
        );
        assert_eq!(
            map_student_attempt_preparation_database_error(
                Some("XX000"),
                "learner work is unavailable"
            ),
            None
        );
    }
}

fn invalid_witness(message: &'static str) -> StoreError {
    StoreError::InvalidRecord(format!("learner-work preparation witness {message}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WitnessCell<T> {
    Missing,
    Null,
    Value(T),
}

#[derive(Debug, Clone)]
struct RawEntitlementWitness {
    tenant: WitnessCell<Uuid>,
    course: WitnessCell<Uuid>,
    assignment: WitnessCell<Uuid>,
    actor: WitnessCell<Uuid>,
    learner: WitnessCell<Uuid>,
    authority: WitnessCell<String>,
    authority_membership: WitnessCell<Uuid>,
    student_membership: WitnessCell<Uuid>,
    assignment_revision: WitnessCell<i64>,
    lifecycle: WitnessCell<String>,
    audience_kind: WitnessCell<String>,
    locked_audience_count: WitnessCell<i64>,
    locked_audience_group_ids: WitnessCell<Vec<Uuid>>,
    locked_current_group_count: WitnessCell<i64>,
    locked_current_group_ids: WitnessCell<Vec<Uuid>>,
    existing_enrollment: WitnessCell<Uuid>,
}

fn cell<T>(row: &sqlx::postgres::PgRow, name: &str) -> WitnessCell<T>
where
    T: for<'r> sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    match row.try_get::<Option<T>, _>(name) {
        Ok(Some(value)) => WitnessCell::Value(value),
        Ok(None) => WitnessCell::Null,
        Err(_) => WitnessCell::Missing,
    }
}

fn required<T>(value: WitnessCell<T>) -> Result<T, StoreError> {
    match value {
        WitnessCell::Value(value) => Ok(value),
        WitnessCell::Missing | WitnessCell::Null => {
            Err(invalid_witness("has a missing or malformed required cell"))
        }
    }
}

fn require_null<T>(value: WitnessCell<T>) -> Result<(), StoreError> {
    match value {
        WitnessCell::Null => Ok(()),
        WitnessCell::Missing | WitnessCell::Value(_) => {
            Err(invalid_witness("discloses fields for a denied decision"))
        }
    }
}

fn validate_denied_entitlement_witness(raw: RawEntitlementWitness) -> Result<(), StoreError> {
    require_null(raw.tenant)?;
    require_null(raw.course)?;
    require_null(raw.assignment)?;
    require_null(raw.actor)?;
    require_null(raw.learner)?;
    require_null(raw.authority)?;
    require_null(raw.authority_membership)?;
    require_null(raw.student_membership)?;
    require_null(raw.assignment_revision)?;
    require_null(raw.lifecycle)?;
    require_null(raw.audience_kind)?;
    require_null(raw.locked_audience_count)?;
    require_null(raw.locked_audience_group_ids)?;
    require_null(raw.locked_current_group_count)?;
    require_null(raw.locked_current_group_ids)?;
    require_null(raw.existing_enrollment)?;
    Ok(())
}

fn decode_entitlement_witness(
    row: &sqlx::postgres::PgRow,
    expected: ExpectedEntitlementWitness,
) -> Result<EntitlementPreparationWitness, StoreError> {
    validate_entitlement_witness(raw_entitlement_witness(row), expected)
}

fn raw_entitlement_witness(row: &sqlx::postgres::PgRow) -> RawEntitlementWitness {
    RawEntitlementWitness {
        tenant: cell(row, "tenant_id"),
        course: cell(row, "course_id"),
        assignment: cell(row, "assignment_id"),
        actor: cell(row, "actor_id"),
        learner: cell(row, "learner_id"),
        authority: cell(row, "authority_kind"),
        authority_membership: cell(row, "authority_membership_id"),
        student_membership: cell(row, "student_membership_id"),
        assignment_revision: cell(row, "assignment_revision"),
        lifecycle: cell(row, "assignment_lifecycle"),
        audience_kind: cell(row, "audience_kind"),
        locked_audience_count: cell(row, "locked_audience_count"),
        locked_audience_group_ids: cell(row, "locked_audience_group_ids"),
        locked_current_group_count: cell(row, "locked_current_group_count"),
        locked_current_group_ids: cell(row, "locked_current_group_ids"),
        existing_enrollment: cell(row, "existing_enrollment_id"),
    }
}

fn decode_student_run_witness(
    row: &sqlx::postgres::PgRow,
    expected: ExpectedEntitlementWitness,
    expected_run: RunId,
) -> Result<StudentRunPreparationWitness, StoreError> {
    validate_student_run_witness(
        raw_entitlement_witness(row),
        cell(row, "run_id"),
        cell(row, "locked_summary_count"),
        cell(row, "locked_summary_enrollment_ids"),
        expected,
        expected_run,
    )
}

fn validate_student_run_witness(
    raw_source: RawEntitlementWitness,
    raw_run: WitnessCell<Uuid>,
    raw_summary_count: WitnessCell<i64>,
    raw_summary_enrollments: WitnessCell<Vec<Uuid>>,
    expected: ExpectedEntitlementWitness,
    expected_run: RunId,
) -> Result<StudentRunPreparationWitness, StoreError> {
    let source = validate_entitlement_witness(raw_source, expected)?;
    let run = required(raw_run)?;
    if run != expected_run.as_uuid() {
        return Err(invalid_witness("does not bind the requested run"));
    }
    if source.authority_membership != source.student_membership
        || source.actor != source.learner
        || source.existing_enrollment.is_none()
    {
        return Err(invalid_witness(
            "does not bind active Student self authority and enrollment",
        ));
    }
    let summaries = decode_sorted_ids(raw_summary_count, raw_summary_enrollments)?;
    let enrollment = source
        .existing_enrollment
        .expect("the preceding closed witness check requires an enrollment");
    if summaries.as_slice() != [enrollment.as_uuid()] {
        return Err(invalid_witness(
            "does not bind the exact enrollment summary",
        ));
    }
    Ok(StudentRunPreparationWitness {
        source,
        run: expected_run,
        locked_summary_enrollments: summaries.into_iter().map(EnrollmentId::from_uuid).collect(),
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_student_attempt_witness(
    raw_source: RawEntitlementWitness,
    raw_run: WitnessCell<Uuid>,
    raw_attempt: WitnessCell<Uuid>,
    raw_status: WitnessCell<String>,
    raw_summary_count: WitnessCell<i64>,
    raw_summary_enrollments: WitnessCell<Vec<Uuid>>,
    expected: ExpectedEntitlementWitness,
    expected_attempt: QuestionAttemptId,
) -> Result<StudentAttemptPreparationWitness, StoreError> {
    let source = validate_entitlement_witness(raw_source, expected)?;
    let run = RunId::from_uuid(required(raw_run)?);
    if required(raw_attempt)? != expected_attempt.as_uuid()
        || source.actor != source.learner
        || source.authority_membership != source.student_membership
    {
        return Err(invalid_witness(
            "does not bind the requested Student-self attempt",
        ));
    }
    let attempt_status = match required(raw_status)?.as_str() {
        "in_progress" => AttemptStatus::InProgress,
        "submitted" => AttemptStatus::Submitted,
        "auto_submitted" => AttemptStatus::AutoSubmitted,
        "cleared" => AttemptStatus::Cleared,
        "exempt" => AttemptStatus::Exempt,
        _ => return Err(invalid_witness("has an unknown attempt status")),
    };
    let summaries = decode_sorted_ids(raw_summary_count, raw_summary_enrollments)?;
    let enrollment = source
        .existing_enrollment
        .ok_or_else(|| invalid_witness("does not bind an existing enrollment for its attempt"))?;
    if summaries.as_slice() != [enrollment.as_uuid()] {
        return Err(invalid_witness(
            "does not bind the exact enrollment summary",
        ));
    }
    Ok(StudentAttemptPreparationWitness {
        source,
        run,
        attempt: expected_attempt,
        attempt_status,
        locked_summary_enrollments: vec![enrollment],
    })
}

fn validate_entitlement_witness(
    raw: RawEntitlementWitness,
    expected: ExpectedEntitlementWitness,
) -> Result<EntitlementPreparationWitness, StoreError> {
    let tenant = required(raw.tenant)?;
    let course = required(raw.course)?;
    let assignment = required(raw.assignment)?;
    let actor = required(raw.actor)?;
    let learner = required(raw.learner)?;
    let authority = required(raw.authority)?;
    if tenant != expected.tenant.as_uuid()
        || course != expected.course.as_uuid()
        || assignment != expected.assignment.as_uuid()
        || actor != expected.actor.as_uuid()
        || learner != expected.learner.as_uuid()
        || authority != expected.authority.database_value()
    {
        return Err(invalid_witness("does not bind the entitlement command"));
    }
    let authority_membership = required(raw.authority_membership)?;
    let student_membership = required(raw.student_membership)?;
    let revision = required(raw.assignment_revision)?;
    let assignment_revision = u64::try_from(revision)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid_witness("has a non-positive assignment revision"))?;
    let lifecycle = match required(raw.lifecycle)?.as_str() {
        "draft" => WitnessAssignmentLifecycle::Draft,
        "published" => WitnessAssignmentLifecycle::Published,
        "closed" => WitnessAssignmentLifecycle::Closed,
        "archived" => WitnessAssignmentLifecycle::Archived,
        _ => return Err(invalid_witness("has an unknown assignment lifecycle")),
    };
    let audience_kind = match required(raw.audience_kind)?.as_str() {
        "course_wide" => WitnessAudienceKind::CourseWide,
        "any_of_groups" => WitnessAudienceKind::AnyOfGroups,
        _ => return Err(invalid_witness("has an unknown audience kind")),
    };
    let audience_groups =
        decode_sorted_ids(raw.locked_audience_count, raw.locked_audience_group_ids)?;
    let current_groups =
        decode_sorted_ids(raw.locked_current_group_count, raw.locked_current_group_ids)?;
    if matches!(audience_kind, WitnessAudienceKind::CourseWide) && !audience_groups.is_empty() {
        return Err(invalid_witness("has course-wide audience groups"));
    }
    let existing_enrollment = match raw.existing_enrollment {
        WitnessCell::Value(value) => Some(value),
        WitnessCell::Null => None,
        WitnessCell::Missing => {
            return Err(invalid_witness("has a malformed existing enrollment cell"));
        }
    };
    Ok(EntitlementPreparationWitness {
        tenant: expected.tenant,
        course: expected.course,
        assignment: expected.assignment,
        authority: expected.authority,
        actor: expected.actor,
        authority_membership: CourseMembershipId::from_uuid(authority_membership),
        learner: expected.learner,
        student_membership: CourseMembershipId::from_uuid(student_membership),
        assignment_revision,
        lifecycle,
        audience_kind,
        audience_groups: audience_groups
            .into_iter()
            .map(CourseGroupId::from_uuid)
            .collect(),
        current_groups: current_groups
            .into_iter()
            .map(CourseGroupId::from_uuid)
            .collect(),
        existing_enrollment: existing_enrollment.map(EnrollmentId::from_uuid),
    })
}

fn decode_sorted_ids(
    count: WitnessCell<i64>,
    ids: WitnessCell<Vec<Uuid>>,
) -> Result<Vec<Uuid>, StoreError> {
    let count = required(count)?;
    let ids = required(ids)?;
    if count < 0 || usize::try_from(count).ok() != Some(ids.len()) {
        return Err(invalid_witness("has an invalid locked identifier count"));
    }
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(invalid_witness(
            "has unordered or duplicate locked identifiers",
        ));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_witness() -> ExpectedEntitlementWitness {
        ExpectedEntitlementWitness {
            tenant: TenantId::from_uuid(Uuid::from_u128(1)),
            course: CourseId::from_uuid(Uuid::from_u128(2)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            learner: UserId::from_uuid(Uuid::from_u128(4)),
            actor: UserId::from_uuid(Uuid::from_u128(5)),
            authority: EntitlementPreparationAuthority::DirectInstructor,
        }
    }

    fn valid_raw_witness() -> RawEntitlementWitness {
        RawEntitlementWitness {
            tenant: WitnessCell::Value(Uuid::from_u128(1)),
            course: WitnessCell::Value(Uuid::from_u128(2)),
            assignment: WitnessCell::Value(Uuid::from_u128(3)),
            actor: WitnessCell::Value(Uuid::from_u128(5)),
            learner: WitnessCell::Value(Uuid::from_u128(4)),
            authority: WitnessCell::Value("direct_instructor".to_string()),
            authority_membership: WitnessCell::Value(Uuid::from_u128(6)),
            student_membership: WitnessCell::Value(Uuid::from_u128(7)),
            assignment_revision: WitnessCell::Value(2),
            lifecycle: WitnessCell::Value("published".to_string()),
            audience_kind: WitnessCell::Value("any_of_groups".to_string()),
            locked_audience_count: WitnessCell::Value(1),
            locked_audience_group_ids: WitnessCell::Value(vec![Uuid::from_u128(8)]),
            locked_current_group_count: WitnessCell::Value(1),
            locked_current_group_ids: WitnessCell::Value(vec![Uuid::from_u128(9)]),
            existing_enrollment: WitnessCell::Value(Uuid::from_u128(10)),
        }
    }

    fn assert_rejected(raw: RawEntitlementWitness) {
        assert!(validate_entitlement_witness(raw, expected_witness()).is_err());
    }

    fn expected_student_run() -> (ExpectedEntitlementWitness, RunId) {
        (
            ExpectedEntitlementWitness {
                tenant: TenantId::from_uuid(Uuid::from_u128(1)),
                course: CourseId::from_uuid(Uuid::from_u128(2)),
                assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
                learner: UserId::from_uuid(Uuid::from_u128(4)),
                actor: UserId::from_uuid(Uuid::from_u128(4)),
                authority: EntitlementPreparationAuthority::StudentSelf,
            },
            RunId::from_uuid(Uuid::from_u128(11)),
        )
    }

    fn valid_student_run_source() -> RawEntitlementWitness {
        let mut raw = valid_raw_witness();
        raw.actor = WitnessCell::Value(Uuid::from_u128(4));
        raw.authority = WitnessCell::Value("student_self".to_string());
        raw.authority_membership = WitnessCell::Value(Uuid::from_u128(7));
        raw
    }

    fn validate_student_run(
        source: RawEntitlementWitness,
        run: WitnessCell<Uuid>,
        summary_count: WitnessCell<i64>,
        summaries: WitnessCell<Vec<Uuid>>,
    ) -> Result<StudentRunPreparationWitness, StoreError> {
        let (expected, expected_run) = expected_student_run();
        validate_student_run_witness(
            source,
            run,
            summary_count,
            summaries,
            expected,
            expected_run,
        )
    }

    #[test]
    fn authority_mapping_is_closed_to_app_owned_provenance() {
        let id = UserId::from_uuid(Uuid::nil());
        let command = MaterializeAssignmentEntitlementCommand::for_learner_action(
            id,
            CourseId::from_uuid(Uuid::nil()),
            AssignmentId::from_uuid(Uuid::nil()),
            question_model::EntitlementPurpose::StartRun,
        )
        .expect("learner start command");
        assert!(
            matches!(command.authority(), question_model::MaterializationAuthority::Actor(actor) if actor == id)
        );
    }

    #[test]
    fn valid_witness_preserves_existing_enrollment() {
        let witness = validate_entitlement_witness(valid_raw_witness(), expected_witness())
            .expect("valid witness");
        assert_eq!(
            witness.existing_enrollment,
            Some(EnrollmentId::from_uuid(Uuid::from_u128(10)))
        );
    }

    #[test]
    fn witness_accepts_missing_existing_enrollment_as_empty_history() {
        let mut raw = valid_raw_witness();
        raw.existing_enrollment = WitnessCell::Null;
        let witness = validate_entitlement_witness(raw, expected_witness())
            .expect("valid witness without prior enrollment");
        assert_eq!(witness.existing_enrollment, None);
    }

    #[test]
    fn witness_rejects_foreign_binding_and_actor_identity() {
        let mut raw = valid_raw_witness();
        raw.tenant = WitnessCell::Value(Uuid::from_u128(11));
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.course = WitnessCell::Value(Uuid::from_u128(11));
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.assignment = WitnessCell::Value(Uuid::from_u128(11));
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.actor = WitnessCell::Value(Uuid::from_u128(11));
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.learner = WitnessCell::Value(Uuid::from_u128(11));
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.authority = WitnessCell::Value("student_self_service".to_string());
        assert_rejected(raw);
    }

    #[test]
    fn witness_rejects_null_or_missing_required_cells() {
        let mut raw = valid_raw_witness();
        raw.tenant = WitnessCell::Null;
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.course = WitnessCell::Missing;
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.authority_membership = WitnessCell::Null;
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.assignment_revision = WitnessCell::Missing;
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.locked_current_group_ids = WitnessCell::Null;
        assert_rejected(raw);
    }

    #[test]
    fn witness_rejects_unknown_lifecycle_or_audience() {
        let mut raw = valid_raw_witness();
        raw.lifecycle = WitnessCell::Value("retired".to_string());
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.audience_kind = WitnessCell::Value("everyone".to_string());
        assert_rejected(raw);
    }

    #[test]
    fn witness_rejects_zero_or_negative_revision() {
        let mut raw = valid_raw_witness();
        raw.assignment_revision = WitnessCell::Value(0);
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.assignment_revision = WitnessCell::Value(-1);
        assert_rejected(raw);
    }

    #[test]
    fn witness_rejects_negative_or_mismatched_counts() {
        let mut raw = valid_raw_witness();
        raw.locked_audience_count = WitnessCell::Value(-1);
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.locked_current_group_count = WitnessCell::Value(2);
        assert_rejected(raw);
    }

    #[test]
    fn witness_rejects_unsorted_or_duplicate_arrays() {
        let mut raw = valid_raw_witness();
        raw.locked_audience_count = WitnessCell::Value(2);
        raw.locked_audience_group_ids =
            WitnessCell::Value(vec![Uuid::from_u128(9), Uuid::from_u128(8)]);
        assert_rejected(raw);

        let mut raw = valid_raw_witness();
        raw.locked_current_group_count = WitnessCell::Value(2);
        raw.locked_current_group_ids =
            WitnessCell::Value(vec![Uuid::from_u128(9), Uuid::from_u128(9)]);
        assert_rejected(raw);
    }

    #[test]
    fn witness_rejects_nonempty_course_wide_audience() {
        let mut raw = valid_raw_witness();
        raw.audience_kind = WitnessCell::Value("course_wide".to_string());
        assert_rejected(raw);
    }

    #[test]
    fn student_run_witness_requires_exact_self_binding_and_summary() {
        let witness = validate_student_run(
            valid_student_run_source(),
            WitnessCell::Value(Uuid::from_u128(11)),
            WitnessCell::Value(1),
            WitnessCell::Value(vec![Uuid::from_u128(10)]),
        )
        .expect("exact Student run witness");
        assert_eq!(witness.run.as_uuid(), Uuid::from_u128(11));
        assert_eq!(
            witness.locked_summary_enrollments,
            vec![EnrollmentId::from_uuid(Uuid::from_u128(10))]
        );

        let mut wrong_actor = valid_student_run_source();
        wrong_actor.actor = WitnessCell::Value(Uuid::from_u128(5));
        assert!(
            validate_student_run(
                wrong_actor,
                WitnessCell::Value(Uuid::from_u128(11)),
                WitnessCell::Value(1),
                WitnessCell::Value(vec![Uuid::from_u128(10)]),
            )
            .is_err()
        );

        let mut wrong_membership = valid_student_run_source();
        wrong_membership.authority_membership = WitnessCell::Value(Uuid::from_u128(6));
        assert!(
            validate_student_run(
                wrong_membership,
                WitnessCell::Value(Uuid::from_u128(11)),
                WitnessCell::Value(1),
                WitnessCell::Value(vec![Uuid::from_u128(10)]),
            )
            .is_err()
        );
    }

    #[test]
    fn student_run_witness_rejects_target_and_summary_shape_drift() {
        let cases = [
            (
                WitnessCell::Missing,
                WitnessCell::Value(1),
                WitnessCell::Value(vec![Uuid::from_u128(10)]),
            ),
            (
                WitnessCell::Value(Uuid::from_u128(12)),
                WitnessCell::Value(1),
                WitnessCell::Value(vec![Uuid::from_u128(10)]),
            ),
            (
                WitnessCell::Value(Uuid::from_u128(11)),
                WitnessCell::Value(0),
                WitnessCell::Value(Vec::new()),
            ),
            (
                WitnessCell::Value(Uuid::from_u128(11)),
                WitnessCell::Value(1),
                WitnessCell::Value(vec![Uuid::from_u128(12)]),
            ),
        ];
        for (run, count, summaries) in cases {
            assert!(
                validate_student_run(valid_student_run_source(), run, count, summaries).is_err()
            );
        }
    }

    #[test]
    fn broker_prelocked_current_facts_are_plain_reads() {
        let source = include_str!("entitlement.rs");
        let start = source
            .find("async fn evaluate_current_broker_prelocked_current_facts")
            .expect("broker-prelocked current-facts resolver remains present");
        let end = source[start..]
            .find("async fn evaluate_current_with_locks")
            .expect("resolver helper follows the broker-prelocked variant")
            + start;
        let resolver = &source[start..end];
        assert!(
            resolver
                .split_whitespace()
                .collect::<String>()
                .contains("false,false,")
        );
        assert!(!resolver.contains("FOR "));

        let student_start = source
            .find("async fn evaluate_current_student_broker_prelocked_current_facts")
            .expect("broker-prelocked student resolver remains present");
        let student_end = source[student_start..]
            .find("async fn load_audience")
            .expect("student resolver ends before audience loader")
            + student_start;
        assert!(!source[student_start..student_end].contains("FOR "));
    }
}
