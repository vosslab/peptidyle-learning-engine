//! PostgreSQL course-grade configuration, compact-total projection, and audit.
//!
//! This module deliberately reads `student_assignment_summary`, never the run
//! or attempt history.  The caller owns the returned PII at the HTTP boundary.

use async_trait::async_trait;
use domain::course_grade::{CourseGradeAssignment, CourseGradeError, calculate_course_grade};
use question_model::{
    AssignmentReference, CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme,
    CourseMembershipReference, GradeCategoryId, GradeCategoryTitle, LetterBand, LetterBandLabel,
    PointValue, ScoringGeneration, WeightedGradeCategory,
};
use serde_json::{Value, json};
use sqlx::Row;

use super::course_roster::require_course_instructor;
use super::*;
use crate::course_gradebook::{
    course_grade_assignment_points, validate_course_grade_scheme_update_shape,
};
use crate::{
    AssignmentInspectionChoice, AssignmentScoringWitness, AuthenticationEmail,
    CalculatedAssignmentCell, CalculatedAssignmentCellAvailability, CalculatedGradebookPage,
    CalculatedGradebookRequest, CalculatedGradebookResult, CalculatedGradebookRow,
    CourseGradeAssignmentMembership, CourseGradeAssignmentRecord, CourseGradeExport,
    CourseGradeExportAudit, CourseGradeExportId, CourseGradeExportRow, CourseGradeSchemeRecord,
    CourseGradeSchemeRevision, CourseGradebookStore, CourseGradebookTotalRow,
    CourseGradebookTotals, CourseRosterId, GradebookFilter, GradebookOperationSelection,
    GradebookReloadReason, GradebookSelectionRequest, GradebookSelectionResult,
    MAX_COURSE_GRADE_EXPORT_ROWS, RosterRevision, SessionTokenHash, StoreError,
    SubmittedRunChoicesPage, SubmittedRunChoicesRequest, TenantContext, UpdateCourseGradeScheme,
};

#[path = "course_gradebook/calculated.rs"]
mod calculated;

#[path = "course_gradebook/selection.rs"]
mod selection;

#[async_trait]
impl CourseGradebookStore for PostgresStore {
    async fn course_grade_scheme(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeSchemeRecord, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        require_course_instructor(&mut tx, session, course).await?;
        let result = read_scheme(&mut tx, tenant, course).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn update_course_grade_scheme(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: UpdateCourseGradeScheme,
    ) -> Result<CourseGradeSchemeRecord, StoreError> {
        retry_transaction(|| async {
            let tenant = context.tenant_id();
            let mut tx = self.begin_tenant(context).await?;
            validate_course_grade_scheme_update_shape(&command)?;
            let payload = grade_scheme_replacement_payload(&command)?;
            let row = sqlx::query(
                "SELECT tenant_id,actor_id,course_id,scheme_revision,mode,rounding \
                 FROM public.ple_replace_course_grade_scheme_v1($1,$2,$3,$4,$5)",
            )
            .bind(tenant.as_uuid())
            .bind(session.to_string())
            .bind(command.course.as_uuid())
            .bind(command.expected_revision.to_i64()?)
            .bind(payload)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::Unavailable(
                "course grade-control capability returned no witness".to_string(),
            ))?;
            validate_scheme_replacement_witness(&row, tenant, &command)?;
            let record = read_scheme(&mut tx, tenant, command.course).await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(record)
        })
        .await
    }

    async fn course_gradebook_totals(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradebookTotals, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_snapshot(context).await?;
        require_course_instructor(&mut tx, session, course).await?;
        let scheme = read_scheme(&mut tx, tenant, course).await?;
        let rows = export_rows_with_scheme(&mut tx, tenant, course, &scheme)
            .await?
            .into_iter()
            .map(|row| CourseGradebookTotalRow {
                display_name: row.display_name,
                outcome: row.outcome,
            })
            .collect();
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseGradebookTotals {
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            rows,
        })
    }

    async fn calculated_gradebook_page(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        request: CalculatedGradebookRequest,
    ) -> Result<CalculatedGradebookResult, StoreError> {
        calculated::page(self, context, session, course, request).await
    }

    async fn resolve_gradebook_operation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: question_model::CourseId,
        operation: question_model::GradingOperationReference,
    ) -> Result<GradebookOperationSelection, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query(
            "SELECT target_kind, assignment_reference, membership_reference \
             FROM public.ple_resolve_instructor_grading_operation_v1($1,$2,$3,$4)",
        )
        .bind(tenant.as_uuid())
        .bind(session.to_string())
        .bind(course.as_uuid())
        .bind(i32::try_from(operation.number()).map_err(|_| StoreError::NotFound)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let assignment = public_assignment_reference(&row, "assignment_reference")?;
        let target: String = row.try_get("target_kind").map_err(map_sqlx_error)?;
        let result = match target.as_str() {
            "assignment_scoring_generation" => {
                GradebookOperationSelection::Assignment { assignment }
            }
            "submission" => GradebookOperationSelection::SingleStudent {
                membership: public_membership_reference(&row, "membership_reference")?,
                assignment,
            },
            _ => return Err(StoreError::NotFound),
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn gradebook_selection(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        request: GradebookSelectionRequest,
    ) -> Result<GradebookSelectionResult, StoreError> {
        selection::gradebook_selection(self, context, session, course, request).await
    }

    async fn submitted_run_choices(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        request: SubmittedRunChoicesRequest,
    ) -> Result<SubmittedRunChoicesPage, StoreError> {
        selection::submitted_run_choices(self, context, session, course, request).await
    }

    async fn create_course_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeExport, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_writable_snapshot(context).await?;
        require_course_instructor(&mut tx, session, course).await?;
        let scheme = read_scheme(&mut tx, tenant, course).await?;
        let rows = export_rows_with_scheme(&mut tx, tenant, course, &scheme).await?;
        let id = CourseGradeExportId::generate()?;
        let row = sqlx::query(
            "SELECT tenant_id,actor_id,course_id,export_id,row_count,scheme_revision,mode,rounding \
             FROM public.ple_record_course_grade_export_audit_v1($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(tenant.as_uuid())
        .bind(session.to_string())
        .bind(course.as_uuid())
        .bind(id.as_uuid())
        .bind(i32::try_from(rows.len()).expect("bounded export rows"))
        .bind(scheme.revision.to_i64()?)
        .bind(mode_name(scheme.scheme.mode))
        .bind(rounding_name(scheme.scheme.rounding))
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Unavailable(
            "course grade export capability returned no witness".to_string(),
        ))?;
        let actor = validate_export_audit_witness(&row, tenant, course, id, &scheme, rows.len())?;
        let audit = CourseGradeExportAudit {
            id,
            course,
            requested_by: actor,
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            row_count: rows.len(),
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseGradeExport { audit, rows })
    }
}

fn public_assignment_reference(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<AssignmentReference, StoreError> {
    let value: i32 = row.try_get(column).map_err(map_sqlx_error)?;
    u32::try_from(value)
        .ok()
        .and_then(|value| AssignmentReference::new(u64::from(value)))
        .ok_or(StoreError::NotFound)
}

fn public_membership_reference(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<CourseMembershipReference, StoreError> {
    let value: Option<i32> = row.try_get(column).map_err(map_sqlx_error)?;
    value
        .and_then(|value| u32::try_from(value).ok())
        .and_then(|value| CourseMembershipReference::new(u64::from(value)))
        .ok_or(StoreError::NotFound)
}

async fn gradebook_roster_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<RosterRevision, StoreError> {
    sqlx::query_scalar(
        "SELECT revision FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .map(RosterRevision::from_stored)
    .transpose()?
    .ok_or(StoreError::NotFound)
}

fn grade_scheme_replacement_payload(
    command: &UpdateCourseGradeScheme,
) -> Result<Value, StoreError> {
    let categories = command
        .scheme
        .categories
        .iter()
        .map(|category| {
            Ok(json!({
                "id": category.id.as_uuid().to_string(),
                "position": i32::try_from(category.position).map_err(|_| StoreError::InvalidRecord("category position exceeds storage range".into()))?,
                "title": category.title.as_str(),
                "weightBasisPoints": i32::from(category.weight_basis_points),
                "dropLowest": i32::try_from(category.drop_lowest).map_err(|_| StoreError::InvalidRecord("drop lowest exceeds storage range".into()))?,
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let assignments = command
        .assignments
        .iter()
        .map(|member| {
            Ok(json!({
                "assignmentId": member.assignment.as_uuid().to_string(),
                "included": member.included,
                "categoryId": member.category.map(|category| category.as_uuid().to_string()),
                "position": member.position.map(i32::try_from).transpose().map_err(|_| StoreError::InvalidRecord("membership position exceeds storage range".into()))?,
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let bands = command
        .scheme
        .letter_bands
        .iter()
        .map(|band| {
            json!({
                "label": band.label.as_str(),
                "minimumBasisPoints": i32::from(band.minimum_basis_points),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "mode": mode_name(command.scheme.mode),
        "rounding": rounding_name(command.scheme.rounding),
        "categories": categories,
        "assignments": assignments,
        "letterBands": bands,
    }))
}

fn invalid_grade_control_witness() -> StoreError {
    StoreError::Unavailable(
        "course grade-control capability returned an invalid witness".to_string(),
    )
}

fn validate_scheme_replacement_witness(
    row: &PgRow,
    tenant: TenantId,
    command: &UpdateCourseGradeScheme,
) -> Result<(), StoreError> {
    let expected_revision = command.expected_revision.next()?;
    let returned_tenant: Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let _: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
    let returned_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
    let revision = CourseGradeSchemeRevision::from_i64(
        row.try_get("scheme_revision").map_err(map_sqlx_error)?,
    )?;
    let mode: String = row.try_get("mode").map_err(map_sqlx_error)?;
    let rounding: String = row.try_get("rounding").map_err(map_sqlx_error)?;
    if returned_tenant != tenant.as_uuid()
        || returned_course != command.course.as_uuid()
        || revision != expected_revision
        || mode != mode_name(command.scheme.mode)
        || rounding != rounding_name(command.scheme.rounding)
    {
        return Err(invalid_grade_control_witness());
    }
    Ok(())
}

fn validate_export_audit_witness(
    row: &PgRow,
    tenant: TenantId,
    course: CourseId,
    export: CourseGradeExportId,
    scheme: &CourseGradeSchemeRecord,
    row_count: usize,
) -> Result<question_model::UserId, StoreError> {
    let returned_tenant: Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let actor: Uuid = row.try_get("actor_id").map_err(map_sqlx_error)?;
    let returned_course: Uuid = row.try_get("course_id").map_err(map_sqlx_error)?;
    let returned_export: Uuid = row.try_get("export_id").map_err(map_sqlx_error)?;
    let returned_count: i32 = row.try_get("row_count").map_err(map_sqlx_error)?;
    let revision = CourseGradeSchemeRevision::from_i64(
        row.try_get("scheme_revision").map_err(map_sqlx_error)?,
    )?;
    let mode: String = row.try_get("mode").map_err(map_sqlx_error)?;
    let rounding: String = row.try_get("rounding").map_err(map_sqlx_error)?;
    if returned_tenant != tenant.as_uuid()
        || returned_course != course.as_uuid()
        || returned_export != export.as_uuid()
        || usize::try_from(returned_count).ok() != Some(row_count)
        || revision != scheme.revision
        || mode != mode_name(scheme.scheme.mode)
        || rounding != rounding_name(scheme.scheme.rounding)
    {
        return Err(invalid_grade_control_witness());
    }
    Ok(question_model::UserId::from_uuid(actor))
}

async fn read_scheme(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseGradeSchemeRecord, StoreError> {
    read_scheme_inner(tx, tenant, course).await
}
async fn read_scheme_inner(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseGradeSchemeRecord, StoreError> {
    let row = sqlx::query(
        "SELECT mode,rounding,revision FROM course_grade_scheme WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let mode = decode_mode(&row)?;
    let rounding = decode_rounding(&row)?;
    let revision =
        CourseGradeSchemeRevision::from_i64(row.try_get("revision").map_err(map_sqlx_error)?)?;
    let category_rows = sqlx::query("SELECT category_id,position,title,weight_basis_points,drop_lowest FROM course_grade_category WHERE tenant_id=$1 AND course_id=$2 ORDER BY position,category_id").bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut categories = Vec::new();
    for (position, row) in category_rows.iter().enumerate() {
        categories.push(WeightedGradeCategory {
            id: GradeCategoryId::from_uuid(row.try_get("category_id").map_err(map_sqlx_error)?),
            title: GradeCategoryTitle::new(
                row.try_get::<String, _>("title").map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::Unavailable("stored category title is invalid".into()))?,
            position: u32::try_from(row.try_get::<i32, _>("position").map_err(map_sqlx_error)?)
                .map_err(|_| {
                    StoreError::Unavailable("stored category position is invalid".into())
                })?,
            weight_basis_points: u16::try_from(
                row.try_get::<i32, _>("weight_basis_points")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::Unavailable("stored category weight is invalid".into()))?,
            drop_lowest: u32::try_from(
                row.try_get::<i32, _>("drop_lowest")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::Unavailable("stored category drop value is invalid".into()))?,
        });
        if categories.last().expect("pushed").position != position as u32 {
            return Err(StoreError::Unavailable(
                "stored category positions are not canonical".into(),
            ));
        }
    }
    let band_rows = sqlx::query("SELECT label,minimum_basis_points FROM course_grade_letter_band WHERE tenant_id=$1 AND course_id=$2 ORDER BY minimum_basis_points DESC,label").bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut letter_bands = Vec::new();
    for row in &band_rows {
        letter_bands.push(LetterBand {
            label: LetterBandLabel::new(row.try_get::<String, _>("label").map_err(map_sqlx_error)?)
                .map_err(|_| StoreError::Unavailable("stored letter label is invalid".into()))?,
            minimum_basis_points: u16::try_from(
                row.try_get::<i32, _>("minimum_basis_points")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::Unavailable("stored letter threshold is invalid".into()))?,
        });
    }
    let scheme = CourseGradeScheme {
        mode,
        rounding,
        categories,
        letter_bands,
    };
    scheme
        .validate()
        .map_err(|_| StoreError::Unavailable("stored course grade scheme is invalid".into()))?;
    let rows = sqlx::query("SELECT a.assignment_id,a.title,a.gradebook_included,m.category_id,m.position FROM assignment a LEFT JOIN course_grade_category_assignment m ON m.tenant_id=a.tenant_id AND m.assignment_id=a.assignment_id WHERE a.tenant_id=$1 AND a.course_id=$2 ORDER BY a.assignment_id").bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    let mut assignments = Vec::new();
    for row in rows {
        assignments.push(CourseGradeAssignmentRecord {
            assignment: AssignmentId::from_uuid(
                row.try_get("assignment_id").map_err(map_sqlx_error)?,
            ),
            title: row.try_get("title").map_err(map_sqlx_error)?,
            included: row.try_get("gradebook_included").map_err(map_sqlx_error)?,
            category: row
                .try_get::<Option<Uuid>, _>("category_id")
                .map_err(map_sqlx_error)?
                .map(GradeCategoryId::from_uuid),
            position: row
                .try_get::<Option<i32>, _>("position")
                .map_err(map_sqlx_error)?
                .map(|v| {
                    u32::try_from(v).map_err(|_| {
                        StoreError::Unavailable("stored membership position is invalid".into())
                    })
                })
                .transpose()?,
        });
    }
    let memberships = assignments
        .iter()
        .map(|member| CourseGradeAssignmentMembership {
            assignment: member.assignment,
            included: member.included,
            category: member.category,
            position: member.position,
        })
        .collect::<Vec<_>>();
    validate_stored_memberships(&scheme, &memberships)?;
    Ok(CourseGradeSchemeRecord {
        course,
        revision,
        scheme,
        assignments,
    })
}

/// Rejects corrupted persisted mappings, while permitting the explicit
/// weighted-mode transition where a newly included assignment is not mapped.
fn validate_stored_memberships(
    scheme: &CourseGradeScheme,
    assignments: &[CourseGradeAssignmentMembership],
) -> Result<(), StoreError> {
    match scheme.mode {
        CourseGradeMode::TotalPoints => {
            if assignments
                .iter()
                .any(|member| member.category.is_some() || member.position.is_some())
            {
                return Err(StoreError::Unavailable(
                    "stored total-points membership has a category mapping".into(),
                ));
            }
        }
        CourseGradeMode::WeightedCategories => {
            let known: std::collections::BTreeSet<_> = scheme
                .categories
                .iter()
                .map(|category| category.id)
                .collect();
            let mut positions: std::collections::BTreeMap<
                GradeCategoryId,
                std::collections::BTreeSet<u32>,
            > = std::collections::BTreeMap::new();
            for member in assignments {
                if member.category.is_some() != member.position.is_some()
                    || member
                        .category
                        .is_some_and(|category| !known.contains(&category))
                {
                    return Err(StoreError::Unavailable(
                        "stored weighted membership is invalid".into(),
                    ));
                }
                if let (Some(category), Some(position)) = (member.category, member.position)
                    && !positions.entry(category).or_default().insert(position)
                {
                    return Err(StoreError::Unavailable(
                        "stored weighted membership position is duplicated".into(),
                    ));
                }
            }
            for category in &scheme.categories {
                let positions = positions.get(&category.id).cloned().unwrap_or_default();
                if !positions
                    .iter()
                    .copied()
                    .eq(0..u32::try_from(positions.len()).expect("position count fits"))
                {
                    return Err(StoreError::Unavailable(
                        "stored weighted membership positions are not canonical".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn mode_name(mode: CourseGradeMode) -> &'static str {
    match mode {
        CourseGradeMode::TotalPoints => "total_points",
        CourseGradeMode::WeightedCategories => "weighted_categories",
    }
}
fn rounding_name(_: CourseGradeRoundingRule) -> &'static str {
    "four_decimal_places_half_away_from_zero"
}
fn decode_mode(row: &PgRow) -> Result<CourseGradeMode, StoreError> {
    match row
        .try_get::<String, _>("mode")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "total_points" => Ok(CourseGradeMode::TotalPoints),
        "weighted_categories" => Ok(CourseGradeMode::WeightedCategories),
        _ => Err(StoreError::Unavailable(
            "stored course grade mode is invalid".into(),
        )),
    }
}
fn decode_rounding(row: &PgRow) -> Result<CourseGradeRoundingRule, StoreError> {
    let value: String = row.try_get("rounding").map_err(map_sqlx_error)?;
    if value == "four_decimal_places_half_away_from_zero" {
        Ok(CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero)
    } else {
        Err(StoreError::Unavailable(
            "stored course rounding is invalid".into(),
        ))
    }
}
#[derive(Clone)]
struct TotalAssignmentInput {
    assignment: AssignmentId,
    position: u32,
    included: bool,
    category: Option<GradeCategoryId>,
    points_possible: PointValue,
    scoring_status: question_model::ScoringStatus,
}

/// Builds one Student's evaluator input from the preloaded course snapshot.
/// The absence of database arguments makes the one-load-per-assignment
/// boundary explicit and testable.
fn course_grade_inputs_for_student(
    assignments: &[TotalAssignmentInput],
    scores: &std::collections::BTreeMap<(Uuid, AssignmentId), Option<f64>>,
    student: Uuid,
) -> Vec<CourseGradeAssignment> {
    assignments
        .iter()
        .map(|assignment| CourseGradeAssignment {
            assignment: assignment.assignment,
            position: assignment.position,
            included: assignment.included,
            category: assignment.category,
            selected_current_score: assignment
                .included
                .then(|| {
                    scores
                        .get(&(student, assignment.assignment))
                        .copied()
                        .flatten()
                })
                .flatten(),
            points_possible: assignment.points_possible,
            scoring_status: assignment.scoring_status,
        })
        .collect()
}

async fn export_rows_with_scheme(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    scheme: &CourseGradeSchemeRecord,
) -> Result<Vec<CourseGradeExportRow>, StoreError> {
    let roster = sqlx::query(
        "SELECT m.student_id,m.roster_id,p.roster_email_normalized,p.roster_email_delivery,p.display_name \
         FROM course_member m JOIN course_roster_profile p \
           ON p.tenant_id=m.tenant_id AND p.course_id=m.course_id \
          AND p.course_membership_id=m.course_membership_id \
         WHERE m.tenant_id=$1 AND m.course_id=$2 AND m.role='student' AND m.status='active' \
         ORDER BY m.roster_id NULLS LAST,m.public_id LIMIT $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(i64::try_from(MAX_COURSE_GRADE_EXPORT_ROWS + 1).expect("bounded"))
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if roster.len() > MAX_COURSE_GRADE_EXPORT_ROWS {
        return Err(StoreError::InvalidRecord(
            "course grade export exceeds the row limit".into(),
        ));
    };
    let assignment_rows = sqlx::query(
        "SELECT assignment_id,scoring_status FROM assignment \
         WHERE tenant_id=$1 AND course_id=$2 ORDER BY assignment_id",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let mut inputs = Vec::with_capacity(assignment_rows.len());
    for assignment_row in &assignment_rows {
        let assignment = AssignmentId::from_uuid(
            assignment_row
                .try_get("assignment_id")
                .map_err(map_sqlx_error)?,
        );
        let membership = scheme
            .assignments
            .iter()
            .find(|member| member.assignment == assignment)
            .ok_or_else(|| {
                StoreError::Unavailable("course grade scheme omits a current assignment".into())
            })?;
        let (points_possible, scoring_status) = if membership.included {
            let record = load_assignment(tx, tenant, assignment).await?;
            (
                course_grade_assignment_points(&record)?,
                decode_scoring_status(assignment_row)?,
            )
        } else {
            (PointValue::ZERO, question_model::ScoringStatus::Current)
        };
        inputs.push(TotalAssignmentInput {
            assignment,
            position: membership.position.unwrap_or_default(),
            included: membership.included,
            category: membership.category,
            points_possible,
            scoring_status,
        });
    }
    let students: Vec<Uuid> = roster
        .iter()
        .map(|row| row.try_get("student_id").map_err(map_sqlx_error))
        .collect::<Result<_, _>>()?;
    let assignments: Vec<Uuid> = inputs
        .iter()
        .filter(|input| input.included)
        .map(|input| input.assignment.as_uuid())
        .collect();
    let summary_rows = sqlx::query(
        "SELECT e.student_id,e.assignment_id,sas.current_score \
         FROM enrollment e LEFT JOIN student_assignment_summary sas \
           ON sas.tenant_id=e.tenant_id AND sas.enrollment_id=e.enrollment_id \
         WHERE e.tenant_id=$1 AND e.student_id=ANY($2::uuid[]) AND e.assignment_id=ANY($3::uuid[])",
    )
    .bind(tenant.as_uuid())
    .bind(&students)
    .bind(&assignments)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let mut scores = std::collections::BTreeMap::new();
    for row in summary_rows {
        scores.insert(
            (
                row.try_get("student_id").map_err(map_sqlx_error)?,
                AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?),
            ),
            row.try_get("current_score").map_err(map_sqlx_error)?,
        );
    }
    let mut rows = Vec::new();
    for person in roster {
        let student: Uuid = person.try_get("student_id").map_err(map_sqlx_error)?;
        let student_inputs = course_grade_inputs_for_student(&inputs, &scores, student);
        let outcome =
            calculate_course_grade(&scheme.scheme, &student_inputs).map_err(|e| match e {
                CourseGradeError::MissingCategory { .. }
                | CourseGradeError::UnknownCategory { .. } => StoreError::Unavailable(
                    "weighted course grade scheme requires a mapping for each included assignment"
                        .into(),
                ),
                other => StoreError::InvalidRecord(other.to_string()),
            })?;
        let delivery: Option<String> = person
            .try_get("roster_email_delivery")
            .map_err(map_sqlx_error)?;
        let normalized: Option<String> = person
            .try_get("roster_email_normalized")
            .map_err(map_sqlx_error)?;
        let email = match (normalized, delivery) {
            (Some(normalized), Some(delivery)) => {
                let email = AuthenticationEmail::parse(&delivery).map_err(|_| {
                    StoreError::Unavailable("stored roster email is invalid".into())
                })?;
                if email.normalized() != normalized {
                    return Err(StoreError::Unavailable(
                        "stored roster email normalization is invalid".into(),
                    ));
                }
                Some(email)
            }
            (None, None) => None,
            _ => {
                return Err(StoreError::Unavailable(
                    "stored roster email is incomplete".into(),
                ));
            }
        };
        let roster_id = person
            .try_get::<Option<String>, _>("roster_id")
            .map_err(map_sqlx_error)?
            .map(|value| {
                CourseRosterId::parse(&value)
                    .map_err(|_| StoreError::Unavailable("stored roster ID is invalid".into()))
            })
            .transpose()?;
        rows.push(CourseGradeExportRow {
            roster_id,
            roster_email: email,
            display_name: person.try_get("display_name").map_err(map_sqlx_error)?,
            outcome,
        });
    }
    Ok(rows)
}
