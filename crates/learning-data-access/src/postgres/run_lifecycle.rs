use super::*;
use crate::LearnerWorkRoutingBinding;

#[cfg(feature = "postgres")]
pub(super) async fn start_or_resume_run(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    binding: LearnerWorkRoutingBinding,
    proposed_run: RunId,
) -> Result<AssignmentRun, StoreError> {
    let tenant = context.tenant_id();
    let command = crate::MaterializeAssignmentEntitlementCommand::for_learner_action(
        actor,
        binding.course,
        binding.assignment,
        question_model::EntitlementPurpose::StartRun,
    )?;
    // 1817 authorizes and locks the exact route binding before this operation
    // reads any assignment, enrollment, run, or timing source.  Concealing a
    // rejected binding prevents course/assignment and membership oracles.
    let prepared = match super::entitlement::prepare_materialization(transaction, tenant, command)
        .await
    {
        Ok(super::entitlement::PreparedEntitlementMaterialization::Granted(prepared)) => prepared,
        Ok(super::entitlement::PreparedEntitlementMaterialization::Denied(_))
        | Err(StoreError::Forbidden)
        | Err(StoreError::NotFound) => return Err(StoreError::NotFound),
        Err(error) => return Err(error),
    };
    let assignment =
        super::entitlement::hydrate_prepared_assignment(transaction, &prepared).await?;
    let course_accessible: bool =
        sqlx::query_scalar("SELECT public.ple_course_records_accessible($1, $2)")
            .bind(tenant.as_uuid())
            .bind(binding.course.as_uuid())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    if !course_accessible {
        return Err(StoreError::NotFound);
    }
    let prior_run_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM assignment_run run JOIN enrollment enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id WHERE run.tenant_id=$1 AND enrollment.assignment_id=$2 AND enrollment.student_id=$3 AND run.completed_at IS NOT NULL",
    ).bind(tenant.as_uuid()).bind(binding.assignment.as_uuid()).bind(prepared.grant().student().as_uuid()).fetch_one(&mut **transaction).await.map_err(map_sqlx_error)?;
    let (decision, _) = super::course_policy::resolve_granted_effective_policy_read_only(
        transaction,
        prepared.grant().clone(),
        domain::effective_assignment_policy::AuthorizationGate::Authorized,
        u32::try_from(prior_run_count)
            .map_err(|_| StoreError::Unavailable("run count exceeds policy range".to_string()))?,
    )
    .await?;
    let domain::effective_assignment_policy::EffectivePolicyDecision::Allowed {
        start: domain::effective_assignment_policy::StartVerdict::MayStart { .. },
        ..
    } = decision
    else {
        return Err(StoreError::NotFound);
    };
    // The nullable broker witness is the sole reuse hint.  Every later
    // receipt read must retain exactly that enrollment before it can resume.
    if let Some(expected_enrollment) = prepared.existing_enrollment() {
        let (existing_enrollment, existing_summary, _, _) =
            super::entitlement::load_existing_receipt(
                transaction,
                tenant,
                binding.assignment,
                prepared.grant().student(),
            )
            .await?
            .ok_or_else(|| {
                StoreError::InvalidRecord("prepared enrollment disappeared".to_string())
            })?;
        if existing_enrollment.id != expected_enrollment {
            return Err(StoreError::InvalidRecord(
                "existing enrollment disagrees with learner-work witness".to_string(),
            ));
        }
        let active_row = sqlx::query(
            "SELECT payload, payload_sha256 FROM assignment_run WHERE tenant_id=$1 AND enrollment_id=$2 AND completed_at IS NULL ORDER BY run_number DESC LIMIT 1",
        )
        .bind(tenant.as_uuid())
        .bind(existing_enrollment.id.as_uuid())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if let Some(row) = active_row {
            return decode_payload_row(&row);
        }
        if !continued_practice_allows_run(&existing_summary, assignment.policies.continued_practice)
        {
            return Err(StoreError::InvalidRecord(
                "continued-practice policy does not permit another run".to_string(),
            ));
        }
    }
    let crate::AssignmentEntitlementMaterialization::Granted(entitlement) =
        super::entitlement::materialize_prepared_entitlement(transaction, *prepared).await?
    else {
        return Err(StoreError::InvalidRecord(
            "prepared entitlement did not materialize a grant".to_string(),
        ));
    };
    let enrollment = entitlement.enrollment;
    let active_row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2 AND completed_at IS NULL \
         ORDER BY run_number DESC LIMIT 1",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if let Some(row) = active_row {
        return decode_payload_row(&row);
    }
    let now = database_timestamp(transaction).await?;
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
        return Err(StoreError::InvalidRecord(
            "continued-practice policy does not permit another run".to_string(),
        ));
    }
    let max_run_number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(run_number), 0) FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let run_number = u32::try_from(max_run_number)
        .map_err(|_| StoreError::InvalidRecord("run number overflow".to_string()))?
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
    let public_number: i64 =
        sqlx::query_scalar("SELECT nextval('public.assignment_run_public_id_seq'::regclass)")
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
    let reference = question_model::RunReference::new(public_number as u64)
        .ok_or_else(|| StoreError::Unavailable("run route number limit reached".to_string()))?;
    let run = AssignmentRun {
        id: proposed_run,
        reference,
        tenant,
        enrollment: enrollment.id,
        run_number,
        started_at: now,
        completed_at: None,
        score: None,
        mode: match enrollment.status() {
            EnrollmentStatus::InProgress => RunMode::Assigned,
            EnrollmentStatus::Completed => RunMode::Practice,
        },
        variation: assignment.policies.variation,
    };
    let next = project_summary(
        &previous,
        domain::scoring::RunTransition::Started { at: now },
        grade_policy(&assignment),
    )?;
    let (payload, checksum) = encode_payload(&run)?;
    sqlx::query(
        "INSERT INTO assignment_run \
         (tenant_id, run_id, public_id, enrollment_id, run_number, started_at, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, transaction_timestamp(), $6, $7)",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .bind(i64::from(run.reference.number()))
    .bind(run.enrollment.as_uuid())
    .bind(i64::from(run.run_number))
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    insert_assignment_run_items(transaction, &assignment, run.id).await?;
    store_summary(transaction, &next).await?;
    Ok(run)
}

#[cfg(feature = "postgres")]
pub(super) async fn apply_start_run(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    run: AssignmentRun,
) -> Result<StudentAssignmentSummary, StoreError> {
    ensure_tenant(context, run.tenant)?;
    if run.run_number == 0 || run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "new run must be one-based and incomplete".to_string(),
        ));
    }
    let tenant = context.tenant_id();
    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignment_run WHERE tenant_id = $1 AND run_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if duplicate {
        return Err(StoreError::AlreadyExists);
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let expected_mode = match enrollment.status() {
        EnrollmentStatus::InProgress => RunMode::Assigned,
        EnrollmentStatus::Completed => RunMode::Practice,
    };
    if run.mode != expected_mode {
        return Err(StoreError::InvalidRecord(format!(
            "run mode must be {expected_mode:?} for this enrollment"
        )));
    }
    if run.variation != assignment.policies.variation {
        return Err(StoreError::InvalidRecord(
            "run variation must match its assignment policy".to_string(),
        ));
    }
    let active_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2 AND completed_at IS NULL)",
    )
    .bind(tenant.as_uuid())
    .bind(run.enrollment.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if active_exists {
        return Err(StoreError::InvalidRecord(
            "an enrollment cannot have two in-progress runs".to_string(),
        ));
    }
    let max_run_number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(run_number), 0) FROM assignment_run \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run.enrollment.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let expected_run_number = u32::try_from(max_run_number)
        .map_err(|_| StoreError::InvalidRecord("run number overflow".to_string()))?
        .checked_add(1)
        .ok_or_else(|| StoreError::InvalidRecord("run number overflow".to_string()))?;
    if run.run_number != expected_run_number {
        return Err(StoreError::InvalidRecord(format!(
            "run number must be the next one-based value {expected_run_number}"
        )));
    }
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    if !continued_practice_allows_run(&previous, assignment.policies.continued_practice) {
        return Err(StoreError::InvalidRecord(
            "continued-practice policy does not permit another run".to_string(),
        ));
    }
    let transition = ActivityTransition::StartRun { run: run.clone() };
    let next = project_summary(
        &previous,
        summary_transition(&transition),
        grade_policy(&assignment),
    )?;
    let (run_payload, run_checksum) = encode_payload(&run)?;
    sqlx::query(
        "INSERT INTO assignment_run \
         (tenant_id, run_id, public_id, enrollment_id, run_number, started_at, \
          payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, to_timestamp($6::double precision / 1000.0), $7, $8)",
    )
    .bind(tenant.as_uuid())
    .bind(run.id.as_uuid())
    .bind(i64::from(run.reference.number()))
    .bind(run.enrollment.as_uuid())
    .bind(i64::from(run.run_number))
    .bind(run.started_at.as_unix_millis())
    .bind(run_payload)
    .bind(run_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    insert_assignment_run_items(transaction, &assignment, run.id).await?;
    store_summary(transaction, &next).await?;
    Ok(next)
}

#[cfg(feature = "postgres")]
pub(super) async fn insert_assignment_run_items(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    run: RunId,
) -> Result<(), StoreError> {
    for item in select_assignment_run_items(assignment, run)? {
        sqlx::query(
            "INSERT INTO assignment_run_item \
             (tenant_id, run_id, assignment_item_id, source_position, issued_position, \
              problem_id, version_id, selection_group_id, selection_seed) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(run.as_uuid())
        .bind(item.assignment_item.as_uuid())
        .bind(i32::try_from(item.source_position).map_err(|_| {
            StoreError::InvalidRecord("run source position is too large".to_string())
        })?)
        .bind(i32::try_from(item.issued_position).map_err(|_| {
            StoreError::InvalidRecord("run issued position is too large".to_string())
        })?)
        .bind(item.reference.problem.as_uuid())
        .bind(item.reference.version.as_uuid())
        .bind(item.selection_group.map(|group| group.as_uuid()))
        .bind(
            item.selection_seed
                .map(i64::try_from)
                .transpose()
                .map_err(|_| {
                    StoreError::InvalidRecord("selection seed is too large".to_string())
                })?,
        )
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn load_assignment_run_items(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
) -> Result<Vec<AssignmentRunItem>, StoreError> {
    let rows = sqlx::query(
        "SELECT assignment_item_id, source_position, issued_position, problem_id, \
                version_id, selection_group_id, selection_seed \
         FROM assignment_run_item WHERE tenant_id = $1 AND run_id = $2 \
         ORDER BY issued_position",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    rows.iter()
        .map(|row| {
            let selection_seed: Option<i64> =
                row.try_get("selection_seed").map_err(map_sqlx_error)?;
            Ok(AssignmentRunItem {
                run,
                assignment_item: AssignmentItemId::from_uuid(
                    row.try_get("assignment_item_id").map_err(map_sqlx_error)?,
                ),
                source_position: stored_u32(row, "source_position", "run source position")?,
                issued_position: stored_u32(row, "issued_position", "run issued position")?,
                reference: ProblemVersionRef {
                    problem: ProblemId::from_uuid(
                        row.try_get("problem_id").map_err(map_sqlx_error)?,
                    ),
                    version: VersionId::from_uuid(
                        row.try_get("version_id").map_err(map_sqlx_error)?,
                    ),
                },
                selection_group: row
                    .try_get::<Option<Uuid>, _>("selection_group_id")
                    .map_err(map_sqlx_error)?
                    .map(AssignmentSelectionGroupId::from_uuid),
                selection_seed: selection_seed.map(|seed| seed as u64),
            })
        })
        .collect()
}

#[cfg(feature = "postgres")]
pub(super) async fn apply_question_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    attempt: QuestionAttempt,
) -> Result<StudentAssignmentSummary, StoreError> {
    ensure_tenant(context, attempt.tenant)?;
    let tenant = context.tenant_id();
    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM question_attempt \
         WHERE tenant_id = $1 AND attempt_id = $2)",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if duplicate {
        return Err(StoreError::AlreadyExists);
    }
    let run = load_run_for_update(transaction, tenant, attempt.run).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "question attempts cannot be added to a completed run".to_string(),
        ));
    }
    let enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let matches_run_item: bool =
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignment_run_item \
         WHERE tenant_id = $1 AND run_id = $2 AND issued_position = $3 \
           AND problem_id = $4 AND version_id = $5)",
        )
        .bind(tenant.as_uuid())
        .bind(attempt.run.as_uuid())
        .bind(i32::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("assignment position is too large".to_string())
        })?)
        .bind(attempt.problem.as_uuid())
        .bind(attempt.question_version.as_uuid())
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    if !matches_run_item {
        return Err(StoreError::InvalidRecord(
            "question attempt must match an immutable run item".to_string(),
        ));
    }
    // The direct activity transition owns only the checksummed QuestionAttempt
    // record.  It has no presentation snapshot, grading envelope, or
    // family-specific first-grade contract to freeze alongside an envelope
    // capability.  Its complete issuance shape is therefore the explicit
    // no-presentation shape.  Normal learner issuance owns the richer
    // protected payloads in runs::attempt_issuance.
    let issuance_shape = match attempt.issued_capability {
        question_model::IssuedAttemptCapabilityV1::NotApplicable => {
            ("not_applicable", false, false)
        }
        _ => {
            return Err(StoreError::InvalidRecord(
                "an attempt with an issued presentation must be created through question issuance"
                    .to_string(),
            ));
        }
    };
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let transition = ActivityTransition::RecordQuestionAttempt {
        attempt: Box::new(attempt.clone()),
    };
    let next = project_summary(
        &previous,
        summary_transition(&transition),
        grade_policy(&assignment),
    )?;
    let occurred_at = attempt
        .timer
        .submitted_at
        .unwrap_or(attempt.timer.issued_at)
        .as_unix_millis();
    let submitted_at = attempt
        .timer
        .submitted_at
        .map(|value| value.as_unix_millis());
    let (payload, checksum) = encode_payload(&attempt)?;
    sqlx::query(
        "INSERT INTO question_attempt \
         (tenant_id, attempt_id, run_id, problem_id, version_id, assignment_position, \
          occurred_at, payload, payload_sha256, presentation_capability, \
          flat_grading_required, webwork_grading_required, attempt_status, submitted_at) \
         VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7::double precision / 1000.0), \
          $8, $9, $10, $11, $12, $13, \
          to_timestamp($14::double precision / 1000.0))",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.id.as_uuid())
    .bind(attempt.run.as_uuid())
    .bind(attempt.problem.as_uuid())
    .bind(attempt.question_version.as_uuid())
    .bind(i64::from(attempt.assignment_position))
    .bind(occurred_at)
    .bind(payload)
    .bind(checksum)
    .bind(issuance_shape.0)
    .bind(issuance_shape.1)
    .bind(issuance_shape.2)
    .bind(attempt_status_name(attempt.status))
    .bind(submitted_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    store_summary(transaction, &next).await?;
    Ok(next)
}

#[cfg(feature = "postgres")]
pub(super) async fn apply_complete_run(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    run_id: RunId,
    score: f64,
    at: question_model::ActivityTimestamp,
) -> Result<StudentAssignmentSummary, StoreError> {
    let tenant = context.tenant_id();
    let mut run = load_run_for_update(transaction, tenant, run_id).await?;
    if run.completed_at.is_some() || run.score.is_some() {
        return Err(StoreError::InvalidRecord(
            "completed run cannot be completed again".to_string(),
        ));
    }
    let mut enrollment = load_enrollment_for_update(transaction, tenant, run.enrollment).await?;
    let assignment = load_assignment(transaction, tenant, enrollment.assignment).await?;
    let previous = load_summary_for_update(transaction, tenant, enrollment.id).await?;
    let transition = ActivityTransition::CompleteRun {
        run: run_id,
        score,
        at,
    };
    let grade = grade_policy(&assignment);
    let next = project_summary(&previous, summary_transition(&transition), grade)?;
    run.completed_at = Some(at);
    run.score = Some(score);
    project_enrollment_completion(&mut enrollment, &previous, grade, run_id, score, at);
    let (run_payload, run_checksum) = encode_payload(&run)?;
    sqlx::query(
        "UPDATE assignment_run SET completed_at = to_timestamp($3::double precision / 1000.0), \
         payload = $4, payload_sha256 = $5 WHERE tenant_id = $1 AND run_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(run_id.as_uuid())
    .bind(at.as_unix_millis())
    .bind(run_payload)
    .bind(run_checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE enrollment SET first_completed_at = to_timestamp($3::double precision / 1000.0), \
                current_grade_run_id = $4, best_grade_run_id = $5 \
         WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.id.as_uuid())
    .bind(
        enrollment
            .first_completed_at
            .map(|value| value.as_unix_millis()),
    )
    .bind(enrollment.current_grade_run.map(|value| value.as_uuid()))
    .bind(enrollment.best_grade_run.map(|value| value.as_uuid()))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    store_summary(transaction, &next).await?;
    Ok(next)
}
