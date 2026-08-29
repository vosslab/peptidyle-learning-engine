//! PostgreSQL Gradebook student and submitted-run selection.

use super::{
    PostgresStore, gradebook_roster_revision, map_sqlx_error, parse_grade_policy,
    public_membership_reference, read_scheme,
};
use crate::gradebook_cursor::{GradebookSelectionCursor, SubmittedRunChoicesCursor};
use crate::postgres::course_roster::require_course_instructor;
use crate::{
    AssignmentInspectionChoice, CourseGradebookStore, GradebookFilterRequest,
    GradebookOperationSelection, GradebookSelectionRequest, GradebookSelectionResult,
    SessionTokenHash, StoreError, StudentSelectionRow, SubmittedRunChoice, SubmittedRunChoicesPage,
    SubmittedRunChoicesRequest, TenantContext,
};
use question_model::{
    ActivityTimestamp, AssignmentReference, CourseId, CourseMembershipReference, GradePolicy,
    RunReference, TenantId,
};
use sqlx::types::Uuid;
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn gradebook_selection(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
    course: CourseId,
    request: GradebookSelectionRequest,
) -> Result<GradebookSelectionResult, StoreError> {
    let resolved = match request.filter {
        GradebookFilterRequest::Operation(operation) => Some((
            operation,
            store
                .resolve_gradebook_operation(context, session, course, operation)
                .await?,
        )),
        GradebookFilterRequest::All
        | GradebookFilterRequest::Assignment(_)
        | GradebookFilterRequest::Student(_) => None,
    };
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant_snapshot(context).await?;
    require_course_instructor(&mut tx, session, course).await?;
    let (assignment, operation) = match (request.filter, resolved) {
        (GradebookFilterRequest::Assignment(assignment), None) => (assignment, None),
        (
            GradebookFilterRequest::Operation(_),
            Some((operation, GradebookOperationSelection::Assignment { assignment })),
        ) => (assignment, Some(operation)),
        (
            GradebookFilterRequest::Operation(_),
            Some((
                _,
                GradebookOperationSelection::SingleStudent {
                    membership,
                    assignment,
                },
            )),
        ) => {
            let inspection_choice =
                selection_choice(&mut tx, tenant, course, membership, assignment).await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(GradebookSelectionResult::SingleStudent {
                membership,
                assignment,
                inspection_choice,
            });
        }
        _ => return Err(StoreError::NotFound),
    };
    ensure_course_assignment(&mut tx, tenant, course, assignment).await?;
    let scheme = read_scheme(&mut tx, tenant, course).await?;
    let roster_revision = gradebook_roster_revision(&mut tx, tenant, course).await?;
    let after = request
        .page
        .after
        .as_ref()
        .map(GradebookSelectionCursor::decode)
        .transpose()?;
    if let Some(cursor) = after
        && (cursor.scheme_revision != scheme.revision
            || cursor.roster_revision != roster_revision
            || cursor.assignment != assignment
            || cursor.operation != operation)
    {
        return Err(StoreError::NotFound);
    }
    let last = after
        .map(|cursor| {
            i32::try_from(cursor.last_membership.number()).map_err(|_| StoreError::NotFound)
        })
        .transpose()?;
    let rows = sqlx::query(
        "SELECT member.public_id,profile.display_name FROM course_member AS member \
         JOIN course_roster_profile AS profile ON profile.tenant_id=member.tenant_id AND profile.course_id=member.course_id AND profile.course_membership_id=member.course_membership_id \
         WHERE member.tenant_id=$1 AND member.course_id=$2 AND member.role='student' AND member.status='active' \
           AND ($3::integer IS NULL OR member.public_id>$3) ORDER BY member.public_id LIMIT $4",
    ).bind(tenant.as_uuid()).bind(course.as_uuid()).bind(last).bind(i64::from(request.page.size.get()) + 1)
        .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
    let has_more = rows.len() > usize::from(request.page.size.get());
    let mut selections = Vec::new();
    for row in rows.into_iter().take(usize::from(request.page.size.get())) {
        let membership = public_membership_reference(&row, "public_id")?;
        let display_label: String = row.try_get("display_name").map_err(map_sqlx_error)?;
        let inspection_choice =
            selection_choice(&mut tx, tenant, course, membership, assignment).await?;
        selections.push(StudentSelectionRow {
            membership,
            display_label,
            assignment,
            inspection_choice,
        });
    }
    let next_cursor = if has_more {
        Some(
            GradebookSelectionCursor {
                scheme_revision: scheme.revision,
                roster_revision,
                assignment,
                operation,
                last_membership: selections
                    .last()
                    .expect("following selection page has final row")
                    .membership,
            }
            .encode(),
        )
    } else {
        None
    };
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(GradebookSelectionResult::StudentSelection {
        rows: selections,
        next_cursor,
    })
}

pub(super) async fn submitted_run_choices(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
    course: CourseId,
    request: SubmittedRunChoicesRequest,
) -> Result<SubmittedRunChoicesPage, StoreError> {
    if let Some(operation) = request.operation {
        match store
            .resolve_gradebook_operation(context, session, course, operation)
            .await?
        {
            GradebookOperationSelection::Assignment { assignment }
                if assignment == request.assignment => {}
            GradebookOperationSelection::SingleStudent {
                membership,
                assignment,
            } if membership == request.membership && assignment == request.assignment => {}
            _ => return Err(StoreError::NotFound),
        }
    }
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant_snapshot(context).await?;
    require_course_instructor(&mut tx, session, course).await?;
    let roster_revision = gradebook_roster_revision(&mut tx, tenant, course).await?;
    let after = request
        .page
        .after
        .as_ref()
        .map(SubmittedRunChoicesCursor::decode)
        .transpose()?;
    if let Some(cursor) = after
        && (cursor.roster_revision != roster_revision
            || cursor.membership != request.membership
            || cursor.assignment != request.assignment
            || cursor.operation != request.operation)
    {
        return Err(StoreError::NotFound);
    }
    let enrollment = active_enrollment_for_choice(
        &mut tx,
        tenant,
        course,
        request.membership,
        request.assignment,
    )
    .await?;
    let after_time = after.map(|cursor| cursor.submitted_at_millis);
    let after_run = after
        .map(|cursor| i32::try_from(cursor.last_run.number()).map_err(|_| StoreError::NotFound))
        .transpose()?;
    let rows = sqlx::query(
        "SELECT run.public_id,floor(extract(epoch FROM run.completed_at)*1000)::bigint AS completed_at, \
                (run.run_id=enrollment.current_grade_run_id) AS score_selected \
         FROM assignment_run AS run JOIN enrollment ON enrollment.tenant_id=run.tenant_id AND enrollment.enrollment_id=run.enrollment_id \
         WHERE run.tenant_id=$1 AND run.enrollment_id=$2 AND run.completed_at IS NOT NULL \
           AND ($3::bigint IS NULL OR (floor(extract(epoch FROM run.completed_at)*1000)::bigint,run.public_id)<($3,$4)) \
         ORDER BY run.completed_at DESC,run.public_id DESC LIMIT $5",
    ).bind(tenant.as_uuid()).bind(enrollment).bind(after_time).bind(after_run).bind(i64::from(request.page.size.get()) + 1)
        .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
    let has_more = rows.len() > usize::from(request.page.size.get());
    let mut choices = Vec::new();
    for row in rows.into_iter().take(usize::from(request.page.size.get())) {
        let run: i32 = row.try_get("public_id").map_err(map_sqlx_error)?;
        let run = RunReference::new(u64::from(
            u32::try_from(run).map_err(|_| StoreError::NotFound)?,
        ))
        .ok_or(StoreError::NotFound)?;
        let millis: i64 = row.try_get("completed_at").map_err(map_sqlx_error)?;
        choices.push(SubmittedRunChoice {
            run,
            submitted_at: ActivityTimestamp::from_unix_millis(millis),
            score_selected: row.try_get("score_selected").map_err(map_sqlx_error)?,
        });
    }
    let next_cursor = if has_more {
        choices.last().map(|last| {
            SubmittedRunChoicesCursor {
                roster_revision,
                membership: request.membership,
                assignment: request.assignment,
                operation: request.operation,
                submitted_at_millis: last.submitted_at.as_unix_millis(),
                last_run: last.run,
            }
            .encode()
        })
    } else {
        None
    };
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(SubmittedRunChoicesPage {
        roster_revision,
        next_cursor,
        rows: choices,
    })
}

async fn active_enrollment_for_choice(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
) -> Result<Uuid, StoreError> {
    ensure_course_assignment(tx, tenant, course, assignment).await?;
    sqlx::query_scalar(
        "SELECT enrollment.enrollment_id FROM course_member AS member JOIN assignment AS assignment_row \
         ON assignment_row.tenant_id=member.tenant_id AND assignment_row.course_id=member.course_id \
         JOIN enrollment ON enrollment.tenant_id=member.tenant_id AND enrollment.student_id=member.student_id AND enrollment.assignment_id=assignment_row.assignment_id \
         WHERE member.tenant_id=$1 AND member.course_id=$2 AND member.public_id=$3 \
           AND member.role='student' AND member.status='active' AND assignment_row.public_id=$4",
    ).bind(tenant.as_uuid()).bind(course.as_uuid())
        .bind(i32::try_from(membership.number()).map_err(|_| StoreError::NotFound)?)
        .bind(i32::try_from(assignment.number()).map_err(|_| StoreError::NotFound)?)
        .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)
}

async fn ensure_course_assignment(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    reference: AssignmentReference,
) -> Result<(), StoreError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignment WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3)",
    ).bind(tenant.as_uuid()).bind(course.as_uuid())
        .bind(i32::try_from(reference.number()).map_err(|_| StoreError::NotFound)?)
        .fetch_one(&mut **tx).await.map_err(map_sqlx_error)?;
    exists.then_some(()).ok_or(StoreError::NotFound)
}

async fn selection_choice(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
) -> Result<AssignmentInspectionChoice, StoreError> {
    ensure_course_assignment(tx, tenant, course, assignment).await?;
    let row = sqlx::query(
        "SELECT assignment_row.attempt_selection_policy, selected.public_id AS selected_run, \
                floor(extract(epoch FROM selected.completed_at)*1000)::bigint AS selected_at, \
                COALESCE(summary.completed_run_count,0) AS completed_run_count \
         FROM course_member AS member JOIN assignment AS assignment_row \
           ON assignment_row.tenant_id=member.tenant_id AND assignment_row.course_id=member.course_id \
         LEFT JOIN enrollment ON enrollment.tenant_id=member.tenant_id AND enrollment.student_id=member.student_id AND enrollment.assignment_id=assignment_row.assignment_id \
         LEFT JOIN student_assignment_summary AS summary ON summary.tenant_id=enrollment.tenant_id AND summary.enrollment_id=enrollment.enrollment_id \
         LEFT JOIN assignment_run AS selected ON selected.tenant_id=enrollment.tenant_id AND selected.run_id=enrollment.current_grade_run_id \
         WHERE member.tenant_id=$1 AND member.course_id=$2 AND member.public_id=$3 \
           AND member.role='student' AND member.status='active' AND assignment_row.public_id=$4",
    ).bind(tenant.as_uuid()).bind(course.as_uuid())
        .bind(i32::try_from(membership.number()).map_err(|_| StoreError::NotFound)?)
        .bind(i32::try_from(assignment.number()).map_err(|_| StoreError::NotFound)?)
        .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::NotFound)?;
    let selected_run: Option<i32> = row.try_get("selected_run").map_err(map_sqlx_error)?;
    let selected_at: Option<i64> = row.try_get("selected_at").map_err(map_sqlx_error)?;
    if let (Some(run), Some(submitted_at)) = (selected_run, selected_at) {
        let policy: String = row
            .try_get("attempt_selection_policy")
            .map_err(map_sqlx_error)?;
        let basis: GradePolicy = parse_grade_policy(&policy)?;
        return Ok(AssignmentInspectionChoice::SelectedRun {
            basis: basis.into(),
            run: RunReference::new(u64::from(
                u32::try_from(run).map_err(|_| StoreError::NotFound)?,
            ))
            .ok_or(StoreError::NotFound)?,
            submitted_at: ActivityTimestamp::from_unix_millis(submitted_at),
        });
    }
    let count: i64 = row.try_get("completed_run_count").map_err(map_sqlx_error)?;
    let count = u32::try_from(count).map_err(|_| StoreError::NotFound)?;
    Ok(if count == 0 {
        AssignmentInspectionChoice::NoSubmittedRun
    } else {
        AssignmentInspectionChoice::ChooseRun {
            completed_run_count: count,
        }
    })
}
