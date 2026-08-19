//! PostgreSQL ownership of sealed effective-policy receipt persistence.

use std::num::NonZeroU32;

use domain::effective_assignment_policy::{EffectiveAssignmentPolicy, PolicySource, ResolvedField};
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentId, CourseGroupId, CourseId,
    LateSubmissionPolicy, QuestionAttemptId, StudentId, TenantId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

use super::*;
use crate::*;

/// Exact immutable values appended to one sealed effective-policy generation.
/// The caller owns policy resolution and timing calculation; this module owns
/// the normalized persistence protocol and its sealed-read reconstruction.
pub(super) struct EffectivePolicyReceiptWrite<'a> {
    pub(super) tenant: TenantId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) generation: i64,
    pub(super) policy: &'a EffectiveAssignmentPolicy,
    pub(super) effective_deadline: Option<ActivityTimestamp>,
    pub(super) effective_grace_seconds: u32,
    pub(super) auto_submit_at: Option<ActivityTimestamp>,
    pub(super) revision: AssignmentRevision,
}

/// Appends all seven policy fields and their normalized source rows, then seals
/// the parent.  Callers may repoint the mutable current row only after this
/// function succeeds, preserving the receipt/current foreign-key chain.
pub(super) async fn append_sealed_effective_policy_receipt(
    tx: &mut Transaction<'_, Postgres>,
    write: EffectivePolicyReceiptWrite<'_>,
) -> Result<(), StoreError> {
    let EffectivePolicyReceiptWrite {
        tenant,
        course,
        assignment,
        attempt,
        generation,
        policy,
        effective_deadline,
        effective_grace_seconds,
        auto_submit_at,
        revision,
    } = write;
    let inserted = sqlx::query(
        "INSERT INTO attempt_effective_policy_receipt \
         (tenant_id,attempt_id,receipt_generation,attempt_occurred_at,assignment_id,course_id,\
          resolved_available_at,resolved_due_at,resolved_closes_at,resolved_late_submission_policy,\
          resolved_deadline_behavior,resolved_time_limit_seconds,resolved_attempt_limit,\
          effective_deadline,effective_grace_seconds,auto_submit_at,assignment_revision) \
         SELECT $1,$2,$3,attempt.occurred_at,$4,$5,\
          to_timestamp($6::double precision/1000),to_timestamp($7::double precision/1000),\
          to_timestamp($8::double precision/1000),$9,'auto_submit',$10,$11,\
          to_timestamp($12::double precision/1000),$13,to_timestamp($14::double precision/1000),$15 \
         FROM question_attempt attempt WHERE attempt.tenant_id=$1 AND attempt.attempt_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(generation)
    .bind(assignment.as_uuid())
    .bind(course.as_uuid())
    .bind(policy.available_at.value.map(|value| value.as_unix_millis()))
    .bind(policy.due_at.value.map(|value| value.as_unix_millis()))
    .bind(policy.closes_at.value.map(|value| value.as_unix_millis()))
    .bind(super::assignment_timing::late_submission_policy_name(
        policy.late_submission.value,
    ))
    .bind(
        policy
            .time_limit_seconds
            .value
            .map(|value| i32::try_from(value.get()))
            .transpose()
            .map_err(|_| StoreError::Conflict)?,
    )
    .bind(
        policy
            .attempt_limit
            .value
            .map(|value| i32::try_from(value.get()))
            .transpose()
            .map_err(|_| StoreError::Conflict)?,
    )
    .bind(effective_deadline.map(|value| value.as_unix_millis()))
    .bind(i32::try_from(effective_grace_seconds).map_err(|_| StoreError::Conflict)?)
    .bind(auto_submit_at.map(|value| value.as_unix_millis()))
    .bind(i64::try_from(revision.value()).map_err(|_| StoreError::Conflict)?)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if inserted.rows_affected() != 1 {
        return Err(StoreError::Conflict);
    }

    for (field, source) in policy_fields(policy) {
        for (source_order, (source_layer, source_id)) in
            source_rows(source)?.into_iter().enumerate()
        {
            sqlx::query(
                "INSERT INTO attempt_effective_policy_receipt_field_source \
                 (tenant_id,attempt_id,receipt_generation,field_name,source_order,source_layer,source_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(tenant.as_uuid())
            .bind(attempt.as_uuid())
            .bind(generation)
            .bind(field)
            .bind(i32::try_from(source_order).map_err(|_| StoreError::Conflict)?)
            .bind(source_layer)
            .bind(source_id)
            .execute(&mut **tx)
            .await
            .map_err(map_sqlx_error)?;
        }
    }
    sqlx::query(
        "UPDATE attempt_effective_policy_receipt SET sealed_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND attempt_id=$2 AND receipt_generation=$3",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(generation)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

fn policy_fields(policy: &EffectiveAssignmentPolicy) -> [(&'static str, &PolicySource); 7] {
    [
        ("available_at", &policy.available_at.source),
        ("due_at", &policy.due_at.source),
        ("closes_at", &policy.closes_at.source),
        ("time_limit_seconds", &policy.time_limit_seconds.source),
        ("attempt_limit", &policy.attempt_limit.source),
        ("late_submission_policy", &policy.late_submission.source),
        ("deadline_behavior", &policy.deadline_behavior.source),
    ]
}

fn source_rows(source: &PolicySource) -> Result<Vec<(&'static str, Option<Uuid>)>, StoreError> {
    Ok(match source {
        PolicySource::Base => vec![("base", None)],
        PolicySource::GroupScheduleOffsets(ids) => ids
            .iter()
            .map(|id| ("group_schedule_offset", Some(id.as_uuid())))
            .collect(),
        PolicySource::GroupAccommodations(ids) => ids
            .iter()
            .map(|id| ("group_accommodation", Some(id.as_uuid())))
            .collect(),
        PolicySource::IndividualException(id) => {
            vec![("individual_exception", Some(id.as_uuid()))]
        }
    })
}

/// Reads only the generation selected by the current pointer. Missing or
/// malformed sealed evidence is unavailable rather than reconstructed from
/// mutable policy inputs.
pub(super) async fn read_current_effective_policy_receipt(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<Option<IssuedEffectivePolicyReceipt>, StoreError> {
    let row = sqlx::query(
        "SELECT receipt.receipt_generation, \
         floor(extract(epoch FROM receipt.resolved_available_at)*1000)::bigint AS available, \
         floor(extract(epoch FROM receipt.resolved_due_at)*1000)::bigint AS due, \
         floor(extract(epoch FROM receipt.resolved_closes_at)*1000)::bigint AS closes, \
         receipt.resolved_late_submission_policy, receipt.resolved_deadline_behavior, \
         receipt.resolved_time_limit_seconds, receipt.resolved_attempt_limit \
         FROM attempt_effective_policy_current current_effect \
         JOIN attempt_effective_policy_receipt receipt \
           ON receipt.tenant_id=current_effect.tenant_id \
          AND receipt.attempt_id=current_effect.attempt_id \
          AND receipt.receipt_generation=current_effect.receipt_generation \
         WHERE current_effect.tenant_id=$1 AND current_effect.attempt_id=$2 \
           AND receipt.sealed_at IS NOT NULL",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let generation = u64::try_from(
        row.try_get::<i64, _>("receipt_generation")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| {
        StoreError::Unavailable("stored effective-policy receipt generation is invalid".to_string())
    })?;
    let sources = sqlx::query(
        "SELECT field_name, source_order, source_layer, source_id \
         FROM attempt_effective_policy_receipt_field_source \
         WHERE tenant_id=$1 AND attempt_id=$2 AND receipt_generation=$3 \
         ORDER BY field_name, source_order",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .bind(i64::try_from(generation).map_err(|_| {
        StoreError::Unavailable("effective-policy receipt generation overflows storage".to_string())
    })?)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let source_for = |field| decode_source(&sources, field);
    let timestamp = |column| -> Result<Option<ActivityTimestamp>, StoreError> {
        Ok(row
            .try_get::<Option<i64>, _>(column)
            .map_err(map_sqlx_error)?
            .map(ActivityTimestamp::from_unix_millis))
    };
    let nonzero = |column| -> Result<Option<NonZeroU32>, StoreError> {
        row.try_get::<Option<i32>, _>(column)
            .map_err(map_sqlx_error)?
            .map(|value| {
                NonZeroU32::new(u32::try_from(value).map_err(|_| {
                    StoreError::Unavailable("stored effective-policy limit is invalid".to_string())
                })?)
                .ok_or_else(|| {
                    StoreError::Unavailable("stored effective-policy limit is zero".to_string())
                })
            })
            .transpose()
    };
    let late = match row
        .try_get::<String, _>("resolved_late_submission_policy")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "accept" => LateSubmissionPolicy::Accept,
        "reject" => LateSubmissionPolicy::Reject,
        "mark_late" => LateSubmissionPolicy::MarkLate,
        _ => {
            return Err(StoreError::Unavailable(
                "stored late policy is invalid".to_string(),
            ));
        }
    };
    let deadline = match row
        .try_get::<String, _>("resolved_deadline_behavior")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "auto_submit" => AssignmentDeadlineBehavior::AutoSubmit,
        _ => {
            return Err(StoreError::Unavailable(
                "stored deadline behavior is invalid".to_string(),
            ));
        }
    };
    Ok(Some(IssuedEffectivePolicyReceipt {
        attempt,
        generation,
        policy: EffectiveAssignmentPolicy {
            available_at: ResolvedField {
                value: timestamp("available")?,
                source: source_for("available_at")?,
            },
            due_at: ResolvedField {
                value: timestamp("due")?,
                source: source_for("due_at")?,
            },
            closes_at: ResolvedField {
                value: timestamp("closes")?,
                source: source_for("closes_at")?,
            },
            time_limit_seconds: ResolvedField {
                value: nonzero("resolved_time_limit_seconds")?,
                source: source_for("time_limit_seconds")?,
            },
            attempt_limit: ResolvedField {
                value: nonzero("resolved_attempt_limit")?,
                source: source_for("attempt_limit")?,
            },
            late_submission: ResolvedField {
                value: late,
                source: source_for("late_submission_policy")?,
            },
            deadline_behavior: ResolvedField {
                value: deadline,
                source: source_for("deadline_behavior")?,
            },
        },
    }))
}

fn decode_source(rows: &[PgRow], field: &str) -> Result<PolicySource, StoreError> {
    let rows: Vec<_> = rows
        .iter()
        .filter(|row| row.try_get::<String, _>("field_name").ok().as_deref() == Some(field))
        .collect();
    if rows.is_empty() {
        return Err(StoreError::Unavailable(format!(
            "sealed effective-policy receipt is missing {field} provenance"
        )));
    }
    let layer: String = rows[0].try_get("source_layer").map_err(map_sqlx_error)?;
    match layer.as_str() {
        "base"
            if rows.len() == 1
                && rows[0]
                    .try_get::<Option<Uuid>, _>("source_id")
                    .map_err(map_sqlx_error)?
                    .is_none() =>
        {
            Ok(PolicySource::Base)
        }
        "group_schedule_offset" | "group_accommodation" => {
            let mut ids = Vec::with_capacity(rows.len());
            for (expected, row) in rows.iter().enumerate() {
                if row
                    .try_get::<String, _>("source_layer")
                    .map_err(map_sqlx_error)?
                    != layer
                    || row
                        .try_get::<i32, _>("source_order")
                        .map_err(map_sqlx_error)?
                        != i32::try_from(expected).map_err(|_| {
                            StoreError::Unavailable(
                                "effective-policy provenance is too large".to_string(),
                            )
                        })?
                {
                    return Err(StoreError::Unavailable(
                        "sealed effective-policy provenance order is malformed".to_string(),
                    ));
                }
                ids.push(CourseGroupId::from_uuid(
                    row.try_get::<Option<Uuid>, _>("source_id")
                        .map_err(map_sqlx_error)?
                        .ok_or_else(|| {
                            StoreError::Unavailable("sealed group provenance has no id".to_string())
                        })?,
                ));
            }
            Ok(if layer == "group_schedule_offset" {
                PolicySource::GroupScheduleOffsets(ids)
            } else {
                PolicySource::GroupAccommodations(ids)
            })
        }
        "individual_exception" if rows.len() == 1 => {
            Ok(PolicySource::IndividualException(StudentId::from_uuid(
                rows[0]
                    .try_get::<Option<Uuid>, _>("source_id")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "sealed individual provenance has no id".to_string(),
                        )
                    })?,
            )))
        }
        _ => Err(StoreError::Unavailable(
            "sealed effective-policy provenance is malformed".to_string(),
        )),
    }
}
