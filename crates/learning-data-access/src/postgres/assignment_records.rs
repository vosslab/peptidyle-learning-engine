use super::*;

mod assignment_support;
pub(crate) use assignment_support::*;

mod learner_disclosure;

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
        // The broker performs the exact visible-publication check and retains
        // a share lock through this transaction. Each Question ID has one
        // immutable publication, so this never selects a successor.
        let lifecycle: Option<String> =
            sqlx::query_scalar("SELECT public.ple_lock_assignable_problem_version($1, $2)")
                .bind(reference.problem.as_uuid())
                .bind(reference.version.as_uuid())
                .fetch_one(&mut **transaction)
                .await
                .map_err(map_sqlx_error)?;
        if !matches!(lifecycle.as_deref(), Some("published" | "deprecated")) {
            return Err(StoreError::InvalidRecord(format!(
                "assignment references a missing, hidden, or inactive publication {}/{}",
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

/// Stores the explicit assignment audience in its normalized relation.  The
/// database's deferred constraint trigger owns same-course and purpose
/// invariants; this writer deliberately has no JSON representation or
/// compatibility default.
#[cfg(feature = "postgres")]
pub(super) async fn replace_postgres_assignment_audience(
    transaction: &mut Transaction<'_, Postgres>,
    assignment: &AssignmentRecord,
) -> Result<(), StoreError> {
    let groups = match &assignment.audience {
        question_model::AssignmentAudience::CourseWide => Vec::new(),
        question_model::AssignmentAudience::AnyOfGroups(groups) => groups
            .iter()
            .map(|group| group.as_uuid())
            .collect::<Vec<_>>(),
    };
    sqlx::query(
        "DELETE FROM assignment_audience_group WHERE tenant_id = $1 AND assignment_id = $2 \
         AND NOT (course_group_id = ANY($3::uuid[]))",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(&groups)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for group in groups {
        sqlx::query(
            "INSERT INTO assignment_audience_group \
             (tenant_id, assignment_id, course_id, course_group_id) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (tenant_id, assignment_id, course_group_id) DO NOTHING",
        )
        .bind(assignment.tenant.as_uuid())
        .bind(assignment.id.as_uuid())
        .bind(assignment.course_id.as_uuid())
        .bind(group)
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
pub(super) fn assignment_audience_kind(
    audience: &question_model::AssignmentAudience,
) -> &'static str {
    match audience {
        question_model::AssignmentAudience::CourseWide => "course_wide",
        question_model::AssignmentAudience::AnyOfGroups(_) => "any_of_groups",
    }
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
        "SELECT assignment_id, course_id, title, lifecycle, instructions, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy, audience_kind, \
                score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                solution_disclosure, class_statistics_disclosure \
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
        "SELECT assignment_id, course_id, title, lifecycle, instructions, completion_policy, \
                completion_threshold::text AS completion_threshold, \
                attempt_selection_policy, continued_practice_policy, \
                practice_max_additional_runs, variation_policy, audience_kind, \
                score_disclosure, per_item_correctness_disclosure, feedback_text_disclosure, \
                solution_disclosure, class_statistics_disclosure \
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
    let audience_kind: String = row.try_get("audience_kind").map_err(map_sqlx_error)?;
    if !matches!(audience_kind.as_str(), "course_wide" | "any_of_groups") {
        return Err(StoreError::Unavailable(
            "stored assignment audience kind is invalid".to_string(),
        ));
    }
    Ok(AssignmentRecord {
        id: AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
        tenant,
        course_id: CourseId::from_uuid(row.try_get("course_id").map_err(map_sqlx_error)?),
        title: row.try_get("title").map_err(map_sqlx_error)?,
        lifecycle: super::course_policy::parse_assignment_lifecycle(
            &row.try_get::<String, _>("lifecycle")
                .map_err(map_sqlx_error)?,
        )?,
        instructions: question_model::AssignmentInstructions::try_new(
            row.try_get("instructions").map_err(map_sqlx_error)?,
        )
        .map_err(|_| {
            StoreError::Unavailable("stored assignment instructions are invalid".to_string())
        })?,
        // `load_assignment_relations` reads the normalized relation and
        // replaces this temporary value before returning an assignment.
        audience: question_model::AssignmentAudience::CourseWide,
        items: Vec::new(),
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: parse_completion_policy(&completion_policy, completion_threshold)?,
            grade: parse_grade_policy(&grade_policy)?,
            continued_practice: parse_continued_practice(&practice_policy, practice_limit)?,
            variation: parse_variation_policy(&variation_policy)?,
        },
        disclosure_policy: learner_disclosure::decode_learner_disclosure_policy(row)?,
    })
}

#[cfg(feature = "postgres")]
pub(super) async fn load_assignment_relations(
    transaction: &mut Transaction<'_, Postgres>,
    mut assignment: AssignmentRecord,
) -> Result<AssignmentRecord, StoreError> {
    let audience_kind: String = sqlx::query_scalar(
        "SELECT audience_kind FROM assignment WHERE tenant_id = $1 AND assignment_id = $2",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_one(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    let audience_rows = sqlx::query_scalar::<_, Uuid>(
        "SELECT course_group_id FROM assignment_audience_group \
         WHERE tenant_id = $1 AND assignment_id = $2 ORDER BY course_group_id",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    assignment.audience = match audience_kind.as_str() {
        "course_wide" if audience_rows.is_empty() => question_model::AssignmentAudience::CourseWide,
        "course_wide" => {
            return Err(StoreError::Unavailable(
                "stored course-wide assignment has audience groups".to_string(),
            ));
        }
        "any_of_groups" => question_model::AssignmentAudience::any_of_groups(
            audience_rows
                .into_iter()
                .map(CourseGroupId::from_uuid)
                .collect(),
        )
        .map_err(|_| {
            StoreError::Unavailable("stored group assignment audience is invalid".to_string())
        })?,
        _ => {
            return Err(StoreError::Unavailable(
                "stored assignment audience kind is invalid".to_string(),
            ));
        }
    };
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
