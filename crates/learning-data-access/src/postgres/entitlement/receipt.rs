//! PostgreSQL persistence and decoding for entitlement receipts.

use question_model::{
    ActivityTimestamp, AssignmentEnrollment, CourseGroupId, CourseGroupPurpose, CourseMembershipId,
    EnrollmentId, EntitlementMaterialization, EvaluatorVersion, MaterializationAuthority,
    MaterializationBasis, MaterializationDisposition, StudentAssignmentSummary,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::super::{load_postgres_enrollment, map_sqlx_error};
use crate::{MaterializeAssignmentEntitlementCommand, StoreError};

pub(crate) fn decode_group_purpose(value: String) -> Result<CourseGroupPurpose, StoreError> {
    match value.as_str() {
        "section" => Ok(CourseGroupPurpose::Section),
        "lab" => Ok(CourseGroupPurpose::Lab),
        "cohort" => Ok(CourseGroupPurpose::Cohort),
        "accommodation" => Ok(CourseGroupPurpose::Accommodation),
        "work" => Ok(CourseGroupPurpose::Work),
        _ => Err(StoreError::Unavailable(
            "stored group purpose is invalid".to_string(),
        )),
    }
}

fn decode_materialization_authority(
    row: &sqlx::postgres::PgRow,
) -> Result<MaterializationAuthority, StoreError> {
    let actor: Option<Uuid> = row
        .try_get("materialized_by_user_id")
        .map_err(map_sqlx_error)?;
    let rule: Option<String> = row
        .try_get("materialization_rule")
        .map_err(map_sqlx_error)?;
    match (actor, rule.as_deref()) {
        (Some(actor), None) => Ok(MaterializationAuthority::Actor(
            question_model::UserId::from_uuid(actor),
        )),
        (None, Some("imported_grade")) => Ok(MaterializationAuthority::Rule(
            question_model::MaterializationRule::ImportedGrade,
        )),
        (None, Some("automated_grader")) => Ok(MaterializationAuthority::Rule(
            question_model::MaterializationRule::AutomatedGrader,
        )),
        _ => Err(StoreError::Unavailable(
            "stored materialization authority is invalid".to_string(),
        )),
    }
}

pub(crate) async fn load_existing_receipt(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    assignment: question_model::AssignmentId,
    student: question_model::StudentId,
) -> Result<
    Option<(
        AssignmentEnrollment,
        StudentAssignmentSummary,
        EntitlementMaterialization,
        MaterializationDisposition,
    )>,
    StoreError,
> {
    let row = sqlx::query("SELECT enrollment_id, course_membership_id, floor(extract(epoch FROM materialized_at)*1000)::bigint AS occurred, materialization_purpose, materialized_by_user_id, materialization_rule, evaluator_version FROM enrollment WHERE tenant_id=$1 AND assignment_id=$2 AND student_id=$3 FOR UPDATE")
		.bind(tenant.as_uuid())
		.bind(assignment.as_uuid())
		.bind(student.as_uuid())
		.fetch_optional(&mut **transaction)
		.await
		.map_err(map_sqlx_error)?;
    let Some(row) = row else { return Ok(None) };
    let id = EnrollmentId::from_uuid(row.try_get("enrollment_id").map_err(map_sqlx_error)?);
    let enrollment = load_postgres_enrollment(transaction, tenant, id).await?;
    let enrolled_user = enrollment.user;
    let summary_row = sqlx::query("SELECT tenant_id, enrollment_id, current_score, best_score, latest_score, completed_run_count, total_question_attempts, floor(extract(epoch FROM last_activity_at) * 1000)::bigint AS last_activity_at_millis FROM student_assignment_summary WHERE tenant_id=$1 AND enrollment_id=$2")
		.bind(tenant.as_uuid())
		.bind(id.as_uuid())
		.fetch_optional(&mut **transaction)
		.await
		.map_err(map_sqlx_error)?
		.ok_or_else(|| StoreError::Unavailable("entitlement receipt has no summary".to_string()))?;
    let summary = super::super::decode_summary_row(&summary_row)?;
    let basis_row = sqlx::query("SELECT scope_kind, course_group_id, course_group_purpose FROM enrollment_entitlement_basis_receipt WHERE tenant_id=$1 AND enrollment_id=$2")
		.bind(tenant.as_uuid())
		.bind(id.as_uuid())
		.fetch_optional(&mut **transaction)
		.await
		.map_err(map_sqlx_error)?
		.ok_or_else(|| StoreError::Unavailable("entitlement receipt has no basis".to_string()))?;
    let basis = match basis_row
        .try_get::<String, _>("scope_kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "course_wide" => MaterializationBasis::CourseWide,
        "group_audience" => MaterializationBasis::GroupAudience {
            group: CourseGroupId::from_uuid(
                basis_row
                    .try_get("course_group_id")
                    .map_err(map_sqlx_error)?,
            ),
            purpose: decode_group_purpose(
                basis_row
                    .try_get("course_group_purpose")
                    .map_err(map_sqlx_error)?,
            )?,
        },
        _ => {
            return Err(StoreError::Unavailable(
                "stored receipt basis is invalid".to_string(),
            ));
        }
    };
    let purpose = match row
        .try_get::<String, _>("materialization_purpose")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "start_run" => question_model::EntitlementPurpose::StartRun,
        "grade_bearing_action" => question_model::EntitlementPurpose::GradeBearingAction,
        "instructor_issue" => question_model::EntitlementPurpose::InstructorIssue,
        _ => {
            return Err(StoreError::Unavailable(
                "stored materialization purpose is invalid".to_string(),
            ));
        }
    };
    let materialization = EntitlementMaterialization {
        enrollment: id,
        membership: CourseMembershipId::from_uuid(
            row.try_get("course_membership_id")
                .map_err(map_sqlx_error)?,
        ),
        user: enrolled_user,
        occurred_at: ActivityTimestamp::from_unix_millis(
            row.try_get("occurred").map_err(map_sqlx_error)?,
        ),
        purpose,
        authority: decode_materialization_authority(&row)?,
        basis,
        evaluator_version: EvaluatorVersion(
            u16::try_from(
                row.try_get::<i32, _>("evaluator_version")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::Unavailable("stored evaluator version is invalid".to_string())
            })?,
        ),
    };
    Ok(Some((
        enrollment,
        summary,
        materialization,
        MaterializationDisposition::Existing,
    )))
}

pub(crate) async fn insert_receipt(
    transaction: &mut Transaction<'_, Postgres>,
    grant: &domain::entitlement::EntitlementGrant,
    command: MaterializeAssignmentEntitlementCommand,
    now: ActivityTimestamp,
) -> Result<
    (
        AssignmentEnrollment,
        StudentAssignmentSummary,
        EntitlementMaterialization,
        MaterializationDisposition,
    ),
    StoreError,
> {
    let id = EnrollmentId::from_uuid(random_uuid()?);
    let purpose = match command.purpose() {
        question_model::EntitlementPurpose::StartRun => "start_run",
        question_model::EntitlementPurpose::GradeBearingAction => "grade_bearing_action",
        question_model::EntitlementPurpose::InstructorIssue => "instructor_issue",
    };
    let (actor, rule) = match command.authority() {
        MaterializationAuthority::Actor(actor) => (Some(actor.as_uuid()), None),
        MaterializationAuthority::Rule(question_model::MaterializationRule::ImportedGrade) => {
            (None, Some("imported_grade"))
        }
        MaterializationAuthority::Rule(question_model::MaterializationRule::AutomatedGrader) => {
            (None, Some("automated_grader"))
        }
    };
    let (kind, group, group_purpose) = match grant.basis() {
        MaterializationBasis::CourseWide => ("course_wide", None, None),
        MaterializationBasis::GroupAudience { group, purpose } => (
            "group_audience",
            Some(group.as_uuid()),
            Some(match purpose {
                CourseGroupPurpose::Section => "section",
                CourseGroupPurpose::Lab => "lab",
                CourseGroupPurpose::Cohort => "cohort",
                _ => return Err(StoreError::Conflict),
            }),
        ),
    };
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO enrollment \
		 (tenant_id, enrollment_id, assignment_id, course_id, course_membership_id, \
		  user_id, student_id, materialized_at, materialization_purpose, \
		  materialized_by_user_id, materialization_rule, evaluator_version) \
		 VALUES ($1, $2, $3, $4, $5, $6, $7, \
		         to_timestamp($8::double precision / 1000), $9, $10, $11, $12) \
		 ON CONFLICT (tenant_id, assignment_id, student_id) DO NOTHING \
		 RETURNING enrollment_id",
    )
    .bind(grant.tenant().as_uuid())
    .bind(id.as_uuid())
    .bind(grant.assignment().as_uuid())
    .bind(grant.course().as_uuid())
    .bind(grant.membership().as_uuid())
    .bind(grant.learner().as_uuid())
    .bind(grant.student().as_uuid())
    .bind(now.as_unix_millis())
    .bind(purpose)
    .bind(actor)
    .bind(rule)
    .bind(i32::from(EvaluatorVersion::INITIAL.0))
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let Some(materialized_id) = inserted.map(EnrollmentId::from_uuid) else {
        return load_existing_receipt(
            transaction,
            grant.tenant(),
            grant.assignment(),
            grant.student(),
        )
        .await?
        .ok_or(StoreError::Conflict);
    };
    sqlx::query(
        "INSERT INTO enrollment_entitlement_basis_receipt \
		 (tenant_id, enrollment_id, scope_receipt_id, scope_kind, course_id, \
		  course_group_id, course_group_purpose) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(grant.tenant().as_uuid())
    .bind(materialized_id.as_uuid())
    .bind(random_uuid()?)
    .bind(kind)
    .bind(grant.course().as_uuid())
    .bind(group)
    .bind(group_purpose)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for (scope_group, scope_purpose) in grant.applicable_policy_scopes().iter() {
        let scope_purpose = match scope_purpose {
            CourseGroupPurpose::Section => "section",
            CourseGroupPurpose::Lab => "lab",
            CourseGroupPurpose::Cohort => "cohort",
            CourseGroupPurpose::Accommodation => "accommodation",
            CourseGroupPurpose::Work => continue,
        };
        sqlx::query(
            "INSERT INTO enrollment_applicable_policy_scope_receipt \
			 (tenant_id, enrollment_id, course_id, course_group_id, course_group_purpose) \
			 VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(grant.tenant().as_uuid())
        .bind(materialized_id.as_uuid())
        .bind(grant.course().as_uuid())
        .bind(scope_group.as_uuid())
        .bind(scope_purpose)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    sqlx::query(
        "UPDATE enrollment SET entitlement_receipts_sealed_at = transaction_timestamp() \
		 WHERE tenant_id = $1 AND enrollment_id = $2 \
		   AND entitlement_receipts_sealed_at IS NULL",
    )
    .bind(grant.tenant().as_uuid())
    .bind(materialized_id.as_uuid())
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let summary = StudentAssignmentSummary::empty(grant.tenant(), materialized_id);
    sqlx::query("INSERT INTO student_assignment_summary (tenant_id, enrollment_id, current_score, best_score, latest_score, completed_run_count, total_question_attempts, last_activity_at) VALUES ($1, $2, NULL, NULL, NULL, 0, 0, NULL)")
		.bind(grant.tenant().as_uuid())
		.bind(materialized_id.as_uuid())
		.execute(&mut **transaction)
		.await
		.map_err(map_sqlx_error)?;
    let enrollment = load_postgres_enrollment(transaction, grant.tenant(), materialized_id).await?;
    Ok((
        enrollment,
        summary,
        EntitlementMaterialization {
            enrollment: materialized_id,
            membership: grant.membership(),
            user: grant.learner(),
            occurred_at: now,
            purpose: command.purpose(),
            authority: command.authority(),
            basis: grant.basis(),
            evaluator_version: EvaluatorVersion::INITIAL,
        },
        MaterializationDisposition::Created,
    ))
}

fn random_uuid() -> Result<Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("entitlement ID randomness unavailable: {error}"))
    })
}
