//! PostgreSQL assembly for the roster-first calculated Gradebook page.

use std::collections::BTreeMap;

use super::*;
use crate::gradebook_cursor::CalculatedGradebookCursor;
use question_model::RunReference;

#[derive(Clone)]
struct AssignmentProjection {
    id: AssignmentId,
    reference: AssignmentReference,
    title: String,
    included: bool,
    category: Option<GradeCategoryId>,
    position: u32,
    points_possible: PointValue,
    scoring_generation: ScoringGeneration,
    scoring_status: question_model::ScoringStatus,
    grade_policy: question_model::GradePolicy,
}

#[derive(Clone, Copy)]
struct RosterStudent {
    student: Uuid,
    membership: CourseMembershipReference,
}

struct RosterPageRow {
    student: RosterStudent,
    display_label: String,
}

#[derive(Clone, Copy)]
struct EnrollmentProjection {
    current_score: Option<f64>,
    selected_run: Option<(RunReference, ActivityTimestamp)>,
    completed_run_count: u32,
}

/// Reads one structural roster page and its page-local score witness from one
/// read-only repeatable-read transaction.  Every value entering a query is a
/// typed parameter (ASVS V1.2.4); public references are resolved only inside
/// the authenticated Instructor's tenant and course boundary.
pub(super) async fn page(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
    course: CourseId,
    request: CalculatedGradebookRequest,
) -> Result<CalculatedGradebookResult, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant_snapshot(context).await?;
    require_course_instructor(&mut tx, session, course).await?;
    let scheme = read_scheme(&mut tx, tenant, course).await?;
    let roster_revision = roster_revision(&mut tx, tenant, course).await?;
    let after = request
        .page
        .after
        .as_ref()
        .map(CalculatedGradebookCursor::decode)
        .transpose()?;
    if let Some(cursor) = after {
        if cursor.scheme_revision != scheme.revision {
            return Ok(CalculatedGradebookResult::ReloadRequired {
                reason: GradebookReloadReason::SchemeChanged,
            });
        }
        if cursor.roster_revision != roster_revision {
            return Ok(CalculatedGradebookResult::ReloadRequired {
                reason: GradebookReloadReason::RosterChanged,
            });
        }
        if cursor.filter != request.filter {
            return Ok(CalculatedGradebookResult::ReloadRequired {
                reason: GradebookReloadReason::FilterChanged,
            });
        }
    }

    let assignments =
        assignment_projections(&mut tx, tenant, course, &scheme, request.filter).await?;
    let roster = roster_page(
        &mut tx,
        tenant,
        course,
        request.filter,
        after.map(|cursor| cursor.last_membership),
        request.page.size.get(),
    )
    .await?;
    let has_more = roster.len() > usize::from(request.page.size.get());
    let roster = roster
        .into_iter()
        .take(usize::from(request.page.size.get()))
        .collect::<Vec<_>>();
    let enrollments = enrollment_projections(&mut tx, tenant, &roster, &assignments).await?;
    let observation_time = database_timestamp(&mut tx).await?;
    let scoring_witnesses = assignments
        .iter()
        .filter(|assignment| {
            !matches!(request.filter, GradebookFilter::Assignment(reference) if reference != assignment.reference)
        })
        .map(|assignment| AssignmentScoringWitness {
            assignment: assignment.reference,
            generation: assignment.scoring_generation,
            status: assignment.scoring_status,
        })
        .collect();
    let rows = roster
        .iter()
        .map(|student| calculated_row(student, &scheme, &assignments, &enrollments, request.filter))
        .collect::<Result<Vec<_>, StoreError>>()?;
    let next_cursor = if has_more {
        let last_membership = rows.last().map(|row| row.membership).ok_or_else(|| {
            StoreError::Unavailable("gradebook page lost its final roster row".into())
        })?;
        Some(
            CalculatedGradebookCursor {
                scheme_revision: scheme.revision,
                roster_revision,
                filter: request.filter,
                last_membership,
            }
            .encode(),
        )
    } else {
        None
    };
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(CalculatedGradebookResult::Page(CalculatedGradebookPage {
        scheme_revision: scheme.revision,
        roster_revision,
        mode: scheme.scheme.mode,
        rounding: scheme.scheme.rounding,
        observation_time,
        scoring_witnesses,
        next_cursor,
        rows,
    }))
}

async fn roster_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<RosterRevision, StoreError> {
    let revision: Option<i64> = sqlx::query_scalar(
        "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    revision
        .map(RosterRevision::from_stored)
        .transpose()?
        .ok_or(StoreError::NotFound)
}

async fn assignment_projections(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    scheme: &CourseGradeSchemeRecord,
    filter: GradebookFilter,
) -> Result<Vec<AssignmentProjection>, StoreError> {
    let requested = match filter {
        GradebookFilter::Assignment(reference) => Some(reference),
        GradebookFilter::All | GradebookFilter::Student(_) => None,
    };
    let assignment_ids = scheme
        .assignments
        .iter()
        .map(|assignment| assignment.assignment.as_uuid())
        .collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT assignment_id,public_id,title,scoring_generation,scoring_status \
         FROM assignment WHERE tenant_id=$1 AND course_id=$2 AND assignment_id=ANY($3::uuid[])",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(&assignment_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let mut values = BTreeMap::new();
    for row in rows {
        let id = AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?);
        let public_id: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
        let public_id = u32::try_from(public_id)
            .ok()
            .and_then(|value| AssignmentReference::new(u64::from(value)))
            .ok_or_else(|| {
                StoreError::Unavailable("stored assignment reference is invalid".into())
            })?;
        let generation: i64 = row.try_get("scoring_generation").map_err(map_sqlx_error)?;
        let generation = u64::try_from(generation)
            .ok()
            .and_then(ScoringGeneration::new)
            .ok_or_else(|| {
                StoreError::Unavailable("stored scoring generation is invalid".into())
            })?;
        values.insert(
            id,
            (
                public_id,
                row.try_get("title").map_err(map_sqlx_error)?,
                generation,
                decode_scoring_status(&row)?,
            ),
        );
    }
    if values.len() != scheme.assignments.len() {
        return Err(StoreError::Unavailable(
            "course grade scheme and current assignments disagree".into(),
        ));
    }
    let mut result = Vec::new();
    for configured in &scheme.assignments {
        let (reference, title, scoring_generation, scoring_status) =
            values.remove(&configured.assignment).ok_or_else(|| {
                StoreError::Unavailable("course grade assignment is unavailable".into())
            })?;
        let assignment = load_assignment(tx, tenant, configured.assignment).await?;
        result.push(AssignmentProjection {
            id: configured.assignment,
            reference,
            title,
            included: configured.included,
            category: configured.category,
            position: configured.position.unwrap_or_default(),
            points_possible: configured
                .included
                .then(|| course_grade_assignment_points(&assignment))
                .transpose()?
                .unwrap_or(PointValue::ZERO),
            scoring_generation,
            scoring_status,
            grade_policy: assignment.policies.grade,
        });
    }
    if requested.is_some_and(|selected| {
        !result
            .iter()
            .any(|assignment| assignment.reference == selected)
    }) {
        return Err(StoreError::NotFound);
    }
    Ok(result)
}

async fn roster_page(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    filter: GradebookFilter,
    after: Option<CourseMembershipReference>,
    page_size: u16,
) -> Result<Vec<RosterPageRow>, StoreError> {
    let selected = match filter {
        GradebookFilter::Student(reference) => Some(reference),
        GradebookFilter::All | GradebookFilter::Assignment(_) => None,
    };
    if let Some(reference) = selected {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM course_member WHERE tenant_id=$1 AND course_id=$2 \
             AND public_id=$3 AND role='student' AND status='active')",
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .bind(i32::try_from(reference.number()).map_err(|_| StoreError::NotFound)?)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
        if !active {
            return Err(StoreError::NotFound);
        }
    }
    let after = after
        .map(|reference| {
            i32::try_from(reference.number()).map_err(|_| {
                StoreError::InvalidRecord("invalid calculated gradebook cursor".into())
            })
        })
        .transpose()?;
    let selected = selected
        .map(|reference| i32::try_from(reference.number()).map_err(|_| StoreError::NotFound))
        .transpose()?;
    let rows = sqlx::query(
        "SELECT member.student_id,member.public_id,profile.display_name \
         FROM course_member AS member LEFT JOIN course_roster_profile AS profile \
           ON profile.tenant_id=member.tenant_id AND profile.course_id=member.course_id \
          AND profile.course_membership_id=member.course_membership_id \
         WHERE member.tenant_id=$1 AND member.course_id=$2 AND member.role='student' \
           AND member.status='active' AND ($3::integer IS NULL OR member.public_id=$3) \
           AND ($4::integer IS NULL OR member.public_id>$4) \
         ORDER BY member.public_id LIMIT $5",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(selected)
    .bind(after)
    .bind(i64::from(page_size) + 1)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    rows.into_iter()
        .map(|row| {
            let public_id: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
            let membership = u32::try_from(public_id)
                .ok()
                .and_then(|value| CourseMembershipReference::new(u64::from(value)))
                .ok_or_else(|| {
                    StoreError::Unavailable("stored course-membership reference is invalid".into())
                })?;
            Ok(RosterPageRow {
                student: RosterStudent {
                    student: row.try_get("student_id").map_err(map_sqlx_error)?,
                    membership,
                },
                display_label: row
                    .try_get::<Option<String>, _>("display_name")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| {
                        StoreError::Unavailable(
                            "active student membership lacks a roster profile".into(),
                        )
                    })?,
            })
        })
        .collect()
}

async fn enrollment_projections(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    roster: &[RosterPageRow],
    assignments: &[AssignmentProjection],
) -> Result<BTreeMap<(Uuid, AssignmentId), EnrollmentProjection>, StoreError> {
    if roster.is_empty() || assignments.is_empty() {
        return Ok(BTreeMap::new());
    }
    let students = roster
        .iter()
        .map(|row| row.student.student)
        .collect::<Vec<_>>();
    let assignments = assignments
        .iter()
        .map(|assignment| assignment.id.as_uuid())
        .collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT enrollment.student_id,enrollment.assignment_id,summary.current_score, \
                selected.public_id AS selected_run_public_id, \
                floor(extract(epoch FROM selected.completed_at)*1000)::bigint AS selected_run_completed_at, \
                summary.completed_run_count \
         FROM enrollment \
         LEFT JOIN student_assignment_summary AS summary \
           ON summary.tenant_id=enrollment.tenant_id AND summary.enrollment_id=enrollment.enrollment_id \
         LEFT JOIN assignment_run AS selected \
           ON selected.tenant_id=enrollment.tenant_id AND selected.run_id=enrollment.current_grade_run_id \
         WHERE enrollment.tenant_id=$1 AND enrollment.student_id=ANY($2::uuid[]) \
           AND enrollment.assignment_id=ANY($3::uuid[])",
    )
    .bind(tenant.as_uuid())
    .bind(&students)
    .bind(&assignments)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let mut result = BTreeMap::new();
    for row in rows {
        let selected_public: Option<i32> = row
            .try_get("selected_run_public_id")
            .map_err(map_sqlx_error)?;
        let completed_at: Option<i64> = row
            .try_get("selected_run_completed_at")
            .map_err(map_sqlx_error)?;
        let selected_run = match (selected_public, completed_at) {
            (None, None) => None,
            (Some(public_id), Some(completed_at)) => {
                let reference = u32::try_from(public_id)
                    .ok()
                    .and_then(|value| RunReference::new(u64::from(value)))
                    .ok_or_else(|| {
                        StoreError::Unavailable("stored run reference is invalid".into())
                    })?;
                Some((reference, ActivityTimestamp::from_unix_millis(completed_at)))
            }
            _ => {
                return Err(StoreError::Unavailable(
                    "selected course-grade run is incomplete or malformed".into(),
                ));
            }
        };
        let completed_run_count: i64 = row
            .try_get::<Option<i64>, _>("completed_run_count")
            .map_err(map_sqlx_error)?
            .ok_or_else(|| {
                StoreError::Unavailable("materialized enrollment lacks its Student summary".into())
            })?;
        let completed_run_count = u32::try_from(completed_run_count).map_err(|_| {
            StoreError::Unavailable("completed run count exceeds supported range".into())
        })?;
        result.insert(
            (
                row.try_get("student_id").map_err(map_sqlx_error)?,
                AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
            ),
            EnrollmentProjection {
                current_score: row.try_get("current_score").map_err(map_sqlx_error)?,
                selected_run,
                completed_run_count,
            },
        );
    }
    Ok(result)
}

fn calculated_row(
    roster: &RosterPageRow,
    scheme: &CourseGradeSchemeRecord,
    assignments: &[AssignmentProjection],
    enrollments: &BTreeMap<(Uuid, AssignmentId), EnrollmentProjection>,
    filter: GradebookFilter,
) -> Result<CalculatedGradebookRow, StoreError> {
    let course_grade_assignments = assignments
        .iter()
        .map(|assignment| CourseGradeAssignment {
            assignment: assignment.id,
            position: assignment.position,
            included: assignment.included,
            category: assignment.category,
            selected_current_score: assignment
                .included
                .then(|| {
                    enrollments
                        .get(&(roster.student.student, assignment.id))
                        .and_then(|enrollment| enrollment.current_score)
                })
                .flatten(),
            points_possible: assignment.points_possible,
            scoring_status: if assignment.included {
                assignment.scoring_status
            } else {
                question_model::ScoringStatus::Current
            },
        })
        .collect::<Vec<_>>();
    let outcome = calculate_course_grade(&scheme.scheme, &course_grade_assignments)
        .map_err(course_grade_error)?;
    let dropped_assignments = outcome
        .dropped_assignment_ids
        .iter()
        .map(|dropped| {
            assignments
                .iter()
                .find(|assignment| assignment.id == *dropped)
                .map(|assignment| assignment.reference)
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "dropped course assignment lacks public reference".to_string(),
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let assignment_cells = assignments
        .iter()
        .filter(|assignment| {
            !matches!(filter, GradebookFilter::Assignment(reference) if reference != assignment.reference)
        })
        .map(|assignment| {
            let enrollment = enrollments.get(&(roster.student.student, assignment.id));
            let inspection_choice = match enrollment.and_then(|enrollment| enrollment.selected_run)
            {
                Some((run, submitted_at)) => AssignmentInspectionChoice::SelectedRun {
                    basis: assignment.grade_policy.into(),
                    run,
                    submitted_at,
                },
                None => match enrollment
                    .map(|enrollment| enrollment.completed_run_count)
                    .unwrap_or(0)
                {
                    0 => AssignmentInspectionChoice::NoSubmittedRun,
                    completed_run_count => AssignmentInspectionChoice::ChooseRun {
                        completed_run_count,
                    },
                },
            };
            CalculatedAssignmentCell {
                assignment: assignment.reference,
                title: assignment.title.clone(),
                included: assignment.included,
                category: assignment.category,
                availability: if enrollment.is_some() {
                    CalculatedAssignmentCellAvailability::Available
                } else {
                    CalculatedAssignmentCellAvailability::Unavailable
                },
                selected_score: (assignment.scoring_status
                    == question_model::ScoringStatus::Current)
                    .then(|| enrollment.and_then(|enrollment| enrollment.current_score))
                    .flatten(),
                scoring_status: assignment.scoring_status,
                inspection_choice,
            }
        })
        .collect();
    Ok(CalculatedGradebookRow {
        membership: roster.student.membership,
        display_label: roster.display_label.clone(),
        outcome,
        dropped_assignments,
        assignment_cells,
    })
}

fn course_grade_error(error: CourseGradeError) -> StoreError {
    match error {
        CourseGradeError::MissingCategory { .. } | CourseGradeError::UnknownCategory { .. } => {
            StoreError::Unavailable(
                "weighted course grade scheme requires a mapping for each included assignment"
                    .into(),
            )
        }
        other => StoreError::InvalidRecord(other.to_string()),
    }
}
