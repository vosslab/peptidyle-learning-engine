use super::*;
use question_model::PoolDrawAlgorithm;

mod assignment_support;
pub(crate) use assignment_support::*;

mod student_disclosure;

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
        disclosure_policy: student_disclosure::decode_student_disclosure_policy(row)?,
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
                algorithm: PoolDrawAlgorithm::from_storage_version(stored_u16(
                    row,
                    "algorithm_version",
                    "selection algorithm version",
                )?)
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "stored selection algorithm version is unsupported".to_string(),
                    )
                })?,
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
