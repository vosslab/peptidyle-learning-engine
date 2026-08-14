use super::*;

#[cfg(feature = "postgres")]
pub(super) async fn validate_postgres_assignment_references(
    transaction: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    let course_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM course WHERE tenant_id = $1 AND course_id = $2)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if !course_exists {
        return Err(StoreError::InvalidRecord(
            "assignment references a missing course".to_string(),
        ));
    }
    for reference in assignment.references() {
        // The broker performs the exact public-or-granted visibility check and
        // retains a share lock through this transaction. Deprecated immutable
        // versions remain assignable; archived versions are historical-only.
        let lifecycle: Option<String> =
            sqlx::query_scalar("SELECT public.ple_lock_assignable_problem_version($1, $2)")
                .bind(reference.problem.as_uuid())
                .bind(reference.version.as_uuid())
                .fetch_one(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !matches!(lifecycle.as_deref(), Some("published" | "deprecated")) {
            return Err(StoreError::InvalidRecord(format!(
                "assignment references a missing, hidden, or inactive published version {}/{}",
                reference.problem, reference.version
            )));
        }
    }
    let _ = context;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn insert_postgres_assignment_items(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    for item in &assignment.items {
        insert_postgres_assignment_item(transaction, assignment, item).await?;
    }
    for group in &assignment.selection_groups {
        insert_postgres_assignment_group(transaction, assignment, group).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn replace_postgres_assignment_items(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    let existing_item_rows = sqlx::query(
        "SELECT assignment_item_id, problem_id, version_id FROM assignment_item \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut existing_items = BTreeMap::new();
    for row in &existing_item_rows {
        existing_items.insert(
            AssignmentItemId::from_uuid(row.try_get("assignment_item_id").map_err(map_sqlx_error)?),
            ProblemVersionRef {
                problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
                version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
            },
        );
    }
    for item in &assignment.items {
        if existing_items
            .get(&item.id)
            .is_some_and(|reference| *reference != item.reference)
        {
            return Err(StoreError::InvalidRecord(
                "replacing pinned content requires a new assignment item identity".to_string(),
            ));
        }
    }

    let existing_candidate_rows = sqlx::query(
        "SELECT candidate_id, selection_group_id, problem_id, version_id \
         FROM assignment_selection_candidate \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR UPDATE",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut existing_candidates = BTreeMap::new();
    for row in &existing_candidate_rows {
        existing_candidates.insert(
            AssignmentItemId::from_uuid(row.try_get("candidate_id").map_err(map_sqlx_error)?),
            (
                AssignmentSelectionGroupId::from_uuid(
                    row.try_get("selection_group_id").map_err(map_sqlx_error)?,
                ),
                ProblemVersionRef {
                    problem: ProblemId::from_uuid(
                        row.try_get("problem_id").map_err(map_sqlx_error)?,
                    ),
                    version: VersionId::from_uuid(
                        row.try_get("version_id").map_err(map_sqlx_error)?,
                    ),
                },
            ),
        );
    }
    for group in &assignment.selection_groups {
        for candidate in &group.candidates {
            if existing_candidates
                .get(&candidate.id)
                .is_some_and(|stored| *stored != (group.id, candidate.reference))
            {
                return Err(StoreError::InvalidRecord(
                    "moving or replacing a selection candidate requires a new identity".to_string(),
                ));
            }
        }
    }

    let item_ids = assignment
        .items
        .iter()
        .map(|item| item.id.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM assignment_item WHERE tenant_id = $1 AND assignment_id = $2 \
           AND NOT (assignment_item_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&item_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    let group_ids = assignment
        .selection_groups
        .iter()
        .map(|group| group.id.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM assignment_selection_group \
         WHERE tenant_id = $1 AND assignment_id = $2 \
           AND NOT (selection_group_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&group_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    let candidate_ids = assignment
        .selection_groups
        .iter()
        .flat_map(|group| group.candidates.iter())
        .map(|candidate| candidate.id.as_uuid())
        .collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM assignment_selection_candidate \
         WHERE tenant_id = $1 AND assignment_id = $2 \
           AND NOT (candidate_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&candidate_ids)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    const POSITION_STAGING_OFFSET: i32 = 1_000_000;
    sqlx::query(
        "UPDATE assignment_item SET position = position + $3 \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(POSITION_STAGING_OFFSET)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE assignment_selection_group SET position = position + $3 \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(POSITION_STAGING_OFFSET)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    sqlx::query(
        "UPDATE assignment_selection_candidate SET position = position + $3 \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(POSITION_STAGING_OFFSET)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    for item in &assignment.items {
        let position = i32::try_from(item.position).map_err(|_| {
            StoreError::InvalidRecord("assignment item position is too large".to_string())
        })?;
        if existing_items.contains_key(&item.id) {
            sqlx::query(
                "UPDATE assignment_item SET position = $4, points_possible = $5::numeric, \
                        delivery_state = $6, scoring_mode = $7, revision = revision + 1, \
                        updated_at = transaction_timestamp() \
                 WHERE tenant_id = $1 AND assignment_id = $2 AND assignment_item_id = $3",
            )
            .bind(assignment.tenant.as_uuid())
            .bind(assignment.id.as_uuid())
            .bind(item.id.as_uuid())
            .bind(position)
            .bind(item.points_possible.to_string())
            .bind(assignment_delivery_state_name(item.delivery_state))
            .bind(assignment_scoring_mode_name(item.scoring_mode))
            .execute(&mut **transaction)
            .await
            .map_err(map_sqlx_error)?;
        } else {
            insert_postgres_assignment_item(transaction, assignment, item).await?;
        }
    }

    for group in &assignment.selection_groups {
        let position = i32::try_from(group.position).map_err(|_| {
            StoreError::InvalidRecord("selection group position is too large".to_string())
        })?;
        let updated = sqlx::query(
            "UPDATE assignment_selection_group \
             SET position = $4, draw_count = $5, points_per_item = $6::numeric, \
                 ordering_policy = $7, algorithm_version = $8, revision = revision + 1, \
                 updated_at = transaction_timestamp() \
             WHERE tenant_id = $1 AND assignment_id = $2 AND selection_group_id = $3",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(group.id.as_uuid())
        .bind(position)
        .bind(i32::try_from(group.draw_count).map_err(|_| {
            StoreError::InvalidRecord("selection group draw count is too large".to_string())
        })?)
        .bind(group.points_per_item.to_string())
        .bind(selection_ordering_name(group.ordering))
        .bind(i32::from(group.algorithm_version))
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() == 0 {
            insert_postgres_assignment_group(transaction, assignment, group).await?;
        } else {
            for candidate in &group.candidates {
                let updated = sqlx::query(
                    "UPDATE assignment_selection_candidate SET position = $5, delivery_state = $6, \
                            updated_at = transaction_timestamp() \
                     WHERE tenant_id = $1 AND assignment_id = $2 \
                       AND selection_group_id = $3 AND candidate_id = $4",
                )
                .bind(assignment.tenant.as_uuid())
                .bind(assignment.id.as_uuid())
                .bind(group.id.as_uuid())
                .bind(candidate.id.as_uuid())
                .bind(i32::try_from(candidate.position).map_err(|_| {
                    StoreError::InvalidRecord(
                        "selection candidate position is too large".to_string(),
                    )
                })?)
                .bind(assignment_delivery_state_name(candidate.delivery_state))
                .execute(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
                if updated.rows_affected() == 0 {
                    insert_postgres_assignment_candidate(
                        transaction,
                        assignment,
                        group.id,
                        candidate,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn insert_postgres_assignment_item(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    item: &AssignmentItem,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO assignment_item \
         (tenant_id, assignment_id, assignment_item_id, position, problem_id, version_id, \
          points_possible, delivery_state, scoring_mode) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::numeric, $8, $9)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(item.id.as_uuid())
    .bind(i32::try_from(item.position).map_err(|_| {
        StoreError::InvalidRecord("assignment item position is too large".to_string())
    })?)
    .bind(item.reference.problem.as_uuid())
    .bind(item.reference.version.as_uuid())
    .bind(item.points_possible.to_string())
    .bind(assignment_delivery_state_name(item.delivery_state))
    .bind(assignment_scoring_mode_name(item.scoring_mode))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn insert_postgres_assignment_group(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    group: &AssignmentSelectionGroup,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO assignment_selection_group \
         (tenant_id, assignment_id, selection_group_id, position, draw_count, \
          points_per_item, ordering_policy, algorithm_version) \
         VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(group.id.as_uuid())
    .bind(i32::try_from(group.position).map_err(|_| {
        StoreError::InvalidRecord("selection group position is too large".to_string())
    })?)
    .bind(i32::try_from(group.draw_count).map_err(|_| {
        StoreError::InvalidRecord("selection group draw count is too large".to_string())
    })?)
    .bind(group.points_per_item.to_string())
    .bind(selection_ordering_name(group.ordering))
    .bind(i32::from(group.algorithm_version))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for candidate in &group.candidates {
        insert_postgres_assignment_candidate(transaction, assignment, group.id, candidate).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn insert_postgres_assignment_candidate(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
    group: AssignmentSelectionGroupId,
    candidate: &AssignmentSelectionCandidate,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO assignment_selection_candidate \
         (tenant_id, assignment_id, selection_group_id, candidate_id, position, problem_id, \
          version_id, delivery_state) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(group.as_uuid())
    .bind(candidate.id.as_uuid())
    .bind(i32::try_from(candidate.position).map_err(|_| {
        StoreError::InvalidRecord("selection candidate position is too large".to_string())
    })?)
    .bind(candidate.reference.problem.as_uuid())
    .bind(candidate.reference.version.as_uuid())
    .bind(assignment_delivery_state_name(candidate.delivery_state))
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn load_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, title, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy \
         FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let header = decode_assignment_header(&row, tenant)?;
    load_assignment_relations(transaction, header).await
}

#[cfg(feature = "postgres")]
pub(super) async fn load_assignment_for_share(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    assignment: AssignmentId,
) -> Result<AssignmentRecord, StoreError> {
    let row = sqlx::query(
        "SELECT assignment_id, course_id, title, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy \
         FROM assignment \
         WHERE tenant_id = $1 AND assignment_id = $2 FOR SHARE",
    )
    .bind(tenant.as_uuid())
    .bind(assignment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let header = decode_assignment_header(&row, tenant)?;
    load_assignment_relations(transaction, header).await
}

#[cfg(feature = "postgres")]
pub(super) fn decode_assignment_header(
    row: &PgRow,
    tenant: TenantId,
) -> Result<AssignmentRecord, StoreError> {
    let completion_policy: String = row.try_get("completion_policy").map_err(map_sqlx_error)?;
    let completion_threshold: Option<String> = row
        .try_get("completion_threshold")
        .map_err(map_sqlx_error)?;
    let grade_policy: String = row
        .try_get("attempt_selection_policy")
        .map_err(map_sqlx_error)?;
    let practice_policy: String = row
        .try_get("continued_practice_policy")
        .map_err(map_sqlx_error)?;
    let practice_limit: Option<i32> = row
        .try_get("practice_max_additional_runs")
        .map_err(map_sqlx_error)?;
    let variation_policy: String = row.try_get("variation_policy").map_err(map_sqlx_error)?;
    Ok(AssignmentRecord {
        id: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        tenant,
        course_id: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        title: row.try_get("title").map_err(map_sqlx_error)?,
        items: Vec::new(),
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: parse_completion_policy(&completion_policy, completion_threshold)?,
            grade: parse_grade_policy(&grade_policy)?,
            continued_practice: parse_continued_practice(&practice_policy, practice_limit)?,
            variation: parse_variation_policy(&variation_policy)?,
        },
    })
}

#[cfg(feature = "postgres")]
pub(super) async fn load_assignment_relations(
    transaction: &mut Transaction<'_, Postgres>,
    mut assignment: AssignmentRecord,
) -> Result<AssignmentRecord, StoreError> {
    let item_rows = sqlx::query(
        "SELECT assignment_item_id, position, problem_id, version_id, \
                points_possible::text AS points_possible, delivery_state, scoring_mode \
         FROM assignment_item WHERE tenant_id = $1 AND assignment_id = $2 \
         ORDER BY position, assignment_item_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment.items = item_rows
        .iter()
        .map(decode_assignment_item)
        .collect::<Result<Vec<_>, _>>()?;

    let candidate_rows = sqlx::query(
        "SELECT selection_group_id, candidate_id, position, problem_id, version_id, delivery_state \
         FROM assignment_selection_candidate \
         WHERE tenant_id = $1 AND assignment_id = $2 \
         ORDER BY selection_group_id, position, candidate_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let mut candidates: BTreeMap<AssignmentSelectionGroupId, Vec<AssignmentSelectionCandidate>> =
        BTreeMap::new();
    for row in &candidate_rows {
        let group = AssignmentSelectionGroupId::from_uuid(
            row.try_get("selection_group_id").map_err(map_sqlx_error)?,
        );
        candidates
            .entry(group)
            .or_default()
            .push(decode_assignment_candidate(row)?);
    }

    let group_rows = sqlx::query(
        "SELECT selection_group_id, position, draw_count, \
                points_per_item::text AS points_per_item, ordering_policy, algorithm_version \
         FROM assignment_selection_group WHERE tenant_id = $1 AND assignment_id = $2 \
         ORDER BY position, selection_group_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment.selection_groups = group_rows
        .iter()
        .map(|row| {
            let id = AssignmentSelectionGroupId::from_uuid(
                row.try_get("selection_group_id").map_err(map_sqlx_error)?,
            );
            Ok(AssignmentSelectionGroup {
                id,
                position: stored_u32(row, "position", "selection group position")?,
                draw_count: stored_u32(row, "draw_count", "selection group draw count")?,
                points_per_item: stored_points(row, "points_per_item")?,
                ordering: parse_selection_ordering(
                    &row.try_get::<String, _>("ordering_policy")
                        .map_err(map_sqlx_error)?,
                )?,
                algorithm_version: stored_u16(
                    row,
                    "algorithm_version",
                    "selection algorithm version",
                )?,
                candidates: candidates.remove(&id).unwrap_or_default(),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    if !candidates.is_empty() {
        return Err(StoreError::Unavailable(
            "stored selection candidate has no assignment group".to_string(),
        ));
    }
    validate_assignment(&assignment).map_err(|error| {
        StoreError::Unavailable(format!("stored assignment is invalid: {error}"))
    })?;
    Ok(assignment)
}

#[cfg(feature = "postgres")]
pub(super) fn decode_assignment_item(row: &PgRow) -> Result<AssignmentItem, StoreError> {
    Ok(AssignmentItem {
        id: AssignmentItemId::from_uuid(row.try_get("assignment_item_id").map_err(map_sqlx_error)?),
        reference: ProblemVersionRef {
            problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
            version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        },
        position: stored_u32(row, "position", "assignment item position")?,
        points_possible: stored_points(row, "points_possible")?,
        delivery_state: parse_assignment_delivery_state(
            &row.try_get::<String, _>("delivery_state")
                .map_err(map_sqlx_error)?,
        )?,
        scoring_mode: parse_assignment_scoring_mode(
            &row.try_get::<String, _>("scoring_mode")
                .map_err(map_sqlx_error)?,
        )?,
    })
}

#[cfg(feature = "postgres")]
pub(super) fn decode_assignment_candidate(
    row: &PgRow,
) -> Result<AssignmentSelectionCandidate, StoreError> {
    Ok(AssignmentSelectionCandidate {
        id: AssignmentItemId::from_uuid(row.try_get("candidate_id").map_err(map_sqlx_error)?),
        position: stored_u32(row, "position", "selection candidate position")?,
        reference: ProblemVersionRef {
            problem: ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?),
            version: VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?),
        },
        delivery_state: parse_assignment_delivery_state(
            &row.try_get::<String, _>("delivery_state")
                .map_err(map_sqlx_error)?,
        )?,
    })
}

#[cfg(feature = "postgres")]
pub(super) fn stored_points(row: &PgRow, column: &str) -> Result<PointValue, StoreError> {
    row.try_get::<String, _>(column)
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|error| StoreError::Unavailable(format!("stored points are invalid: {error}")))
}

#[cfg(feature = "postgres")]
pub(super) fn stored_u32(row: &PgRow, column: &str, description: &str) -> Result<u32, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u32::try_from(value)
        .map_err(|_| StoreError::Unavailable(format!("stored {description} is invalid")))
}

#[cfg(feature = "postgres")]
pub(super) fn stored_u16(row: &PgRow, column: &str, description: &str) -> Result<u16, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u16::try_from(value)
        .map_err(|_| StoreError::Unavailable(format!("stored {description} is invalid")))
}

#[cfg(feature = "postgres")]
pub(super) async fn load_enrollment_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<AssignmentEnrollment, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM enrollment \
         WHERE tenant_id = $1 AND enrollment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
pub(super) async fn load_run_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: RunId,
) -> Result<AssignmentRun, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM assignment_run \
         WHERE tenant_id = $1 AND run_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(run.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
pub(super) async fn load_summary_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    enrollment: EnrollmentId,
) -> Result<StudentAssignmentSummary, StoreError> {
    let row = sqlx::query(
        "SELECT payload, payload_sha256 FROM student_assignment_summary \
         WHERE tenant_id = $1 AND enrollment_id = $2 FOR UPDATE",
    )
    .bind(tenant.as_uuid())
    .bind(enrollment.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    decode_payload_row(&row)
}

#[cfg(feature = "postgres")]
pub(super) async fn store_summary(
    transaction: &mut Transaction<'_, Postgres>,
    summary: &StudentAssignmentSummary,
) -> Result<(), StoreError> {
    let (payload, checksum) = encode_payload(summary)?;
    sqlx::query(
        "UPDATE student_assignment_summary SET payload = $3, payload_sha256 = $4, \
         updated_at = transaction_timestamp() WHERE tenant_id = $1 AND enrollment_id = $2",
    )
    .bind(summary.tenant.as_uuid())
    .bind(summary.enrollment.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) async fn insert_problem_version(
    transaction: &mut Transaction<'_, Postgres>,
    record: &PublishedProblemRecord,
    content_sha256: &str,
) -> Result<(), StoreError> {
    let backend = question_backend_name(QuestionBackend::from(&record.question.source));
    let (lifecycle, lifecycle_reason) = catalog_lifecycle_parts(&record.lifecycle);
    let derived_from_problem = record.derived_from.map(|source| source.problem.as_uuid());
    let derived_from_version = record.derived_from.map(|source| source.version.as_uuid());
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id, version_id, version_number, content_sha256, workspace_id, title, \
          backend, capabilities, metadata, \
          publication_scope, lifecycle, lifecycle_reason, authors, previous_version_id, \
          derived_from_problem_id, derived_from_version_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
    )
    .bind(record.problem.as_uuid())
    .bind(record.version.as_uuid())
    .bind(i64::from(record.version_number.value()))
    .bind(content_sha256)
    .bind(record.question.workspace.as_uuid())
    .bind(&record.question.metadata.title)
    .bind(backend)
    .bind(Json(record.capabilities.clone()))
    .bind(Json(record.question.metadata.clone()))
    .bind(publication_scope_name(record.scope))
    .bind(lifecycle)
    .bind(lifecycle_reason)
    .bind(Json(record.authors.clone()))
    .bind(record.previous_version.map(|version| version.as_uuid()))
    .bind(derived_from_problem)
    .bind(derived_from_version)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

/// Persists a server-only source binding in the same transaction that makes
/// its immutable version visible.
#[cfg(feature = "postgres")]
pub(super) async fn insert_published_source_artifact(
    transaction: &mut Transaction<'_, Postgres>,
    artifact: &PublishedSourceArtifact,
) -> Result<(), StoreError> {
    let (payload, checksum) = encode_payload(artifact)?;
    sqlx::query(
        "INSERT INTO published_source_artifact \
         (problem_id, version_id, backend, object_id, payload, payload_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(artifact.reference.problem.as_uuid())
    .bind(artifact.reference.version.as_uuid())
    .bind(question_backend_name(artifact.backend))
    .bind(artifact.object.id.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

/// Inserts a QTI candidate asset while the containing publication transaction
/// is still private. The ordinary asset API deliberately cannot do this: it
/// only accepts assets for an already visible version.
#[cfg(feature = "postgres")]
pub(super) async fn insert_catalog_asset_delivery(
    transaction: &mut Transaction<'_, Postgres>,
    record: &AssetDeliveryRecord,
) -> Result<(), StoreError> {
    validate_asset_delivery(record)?;
    let AssetDeliveryScope::Catalog { asset, reference } = record.scope else {
        return Err(StoreError::InvalidRecord(
            "QTI promotion assets must be catalog assets".to_string(),
        ));
    };
    let (payload, checksum) = encode_payload(record)?;
    sqlx::query(
        "INSERT INTO asset_delivery \
         (delivery_id, delivery_kind, tenant_id, object_id, problem_id, version_id, \
          asset_id, payload, payload_sha256) \
         VALUES ($1, 'catalog', NULL, $2, $3, $4, $5, $6, $7)",
    )
    .bind(record.id.as_uuid())
    .bind(record.object.id.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .bind(asset.as_uuid())
    .bind(payload)
    .bind(checksum)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) fn question_backend_name(backend: QuestionBackend) -> &'static str {
    match backend {
        QuestionBackend::Native => "native",
        QuestionBackend::Webwork => "webwork",
        QuestionBackend::Qti => "qti",
        QuestionBackend::H5p => "h5p",
        QuestionBackend::Imathas => "imathas",
    }
}
