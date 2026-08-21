//! PostgreSQL course-grade configuration, compact-total projection, and audit.
//!
//! This module deliberately reads `student_assignment_summary`, never the run
//! or attempt history.  The caller owns the returned PII at the HTTP boundary.

use async_trait::async_trait;
use domain::course_grade::{CourseGradeAssignment, CourseGradeError, calculate_course_grade};
use question_model::{
    CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme, GradeCategoryId,
    GradeCategoryTitle, LetterBand, LetterBandLabel, PointValue, WeightedGradeCategory,
};
use sqlx::Row;

use super::course_roster::require_course_instructor;
use super::*;
use crate::course_gradebook::{
    course_grade_assignment_points, validate_course_grade_scheme_update,
};
use crate::{
    AuthenticationEmail, CourseGradeAssignmentMembership, CourseGradeAssignmentRecord,
    CourseGradeExport, CourseGradeExportAudit, CourseGradeExportId, CourseGradeSchemeRecord,
    CourseGradeSchemeRevision, CourseGradebookStore, CourseGradebookTotalRow,
    CourseGradebookTotals, CourseRosterId, MAX_COURSE_GRADE_EXPORT_ROWS, SessionTokenHash,
    StoreError, TenantContext, UpdateCourseGradeScheme,
};

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
            require_course_instructor(&mut tx, session, command.course).await?;
            let assignment_ids = lock_assignment_ids(&mut tx, tenant, command.course).await?;
            let current = read_scheme_for_update(&mut tx, tenant, command.course).await?;
            if current.revision != command.expected_revision { return Err(StoreError::Conflict); }
            validate_course_grade_scheme_update(&command, &assignment_ids)?;
            // The category trigger requires delete-before-total and mode-before-insert.
            sqlx::query("DELETE FROM course_grade_category_assignment WHERE tenant_id=$1 AND course_id=$2")
                .bind(tenant.as_uuid()).bind(command.course.as_uuid()).execute(&mut *tx).await.map_err(map_sqlx_error)?;
            sqlx::query("DELETE FROM course_grade_letter_band WHERE tenant_id=$1 AND course_id=$2")
                .bind(tenant.as_uuid()).bind(command.course.as_uuid()).execute(&mut *tx).await.map_err(map_sqlx_error)?;
            sqlx::query("DELETE FROM course_grade_category WHERE tenant_id=$1 AND course_id=$2")
                .bind(tenant.as_uuid()).bind(command.course.as_uuid()).execute(&mut *tx).await.map_err(map_sqlx_error)?;
            let mode = mode_name(command.scheme.mode);
            let revision = current.revision.next()?;
            let changed = sqlx::query("UPDATE course_grade_scheme SET mode=$3, rounding=$4, revision=$5, updated_at=transaction_timestamp() WHERE tenant_id=$1 AND course_id=$2 AND revision=$6")
                .bind(tenant.as_uuid()).bind(command.course.as_uuid()).bind(mode).bind(rounding_name(command.scheme.rounding))
                .bind(revision.to_i64()?).bind(command.expected_revision.to_i64()?).execute(&mut *tx).await.map_err(map_sqlx_error)?;
            if changed.rows_affected() != 1 { return Err(StoreError::Conflict); }
            for category in &command.scheme.categories {
                sqlx::query("INSERT INTO course_grade_category (tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest) VALUES ($1,$2,$3,$4,$5,$6,$7)")
                    .bind(tenant.as_uuid()).bind(command.course.as_uuid()).bind(category.id.as_uuid()).bind(i32::try_from(category.position).map_err(|_| StoreError::InvalidRecord("category position exceeds storage range".into()))?).bind(category.title.as_str()).bind(i32::from(category.weight_basis_points)).bind(i32::try_from(category.drop_lowest).map_err(|_| StoreError::InvalidRecord("drop lowest exceeds storage range".into()))?).execute(&mut *tx).await.map_err(map_sqlx_error)?;
            }
            for member in &command.assignments {
                sqlx::query("UPDATE assignment SET gradebook_included=$4 WHERE tenant_id=$1 AND course_id=$2 AND assignment_id=$3")
                    .bind(tenant.as_uuid()).bind(command.course.as_uuid()).bind(member.assignment.as_uuid()).bind(member.included).execute(&mut *tx).await.map_err(map_sqlx_error)?;
                if let (Some(category), Some(position)) = (member.category, member.position) {
                    sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,$5)")
                        .bind(tenant.as_uuid()).bind(command.course.as_uuid()).bind(category.as_uuid()).bind(member.assignment.as_uuid()).bind(i32::try_from(position).map_err(|_| StoreError::InvalidRecord("membership position exceeds storage range".into()))?).execute(&mut *tx).await.map_err(map_sqlx_error)?;
                }
            }
            for band in &command.scheme.letter_bands {
                let id = fresh_band_id()?;
                sqlx::query("INSERT INTO course_grade_letter_band (tenant_id,course_id,letter_band_id,label,minimum_basis_points) VALUES ($1,$2,$3,$4,$5)")
                    .bind(tenant.as_uuid()).bind(command.course.as_uuid()).bind(id).bind(band.label.as_str()).bind(i32::from(band.minimum_basis_points)).execute(&mut *tx).await.map_err(map_sqlx_error)?;
            }
            let record = read_scheme(&mut tx, tenant, command.course).await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            Ok(record)
        }).await
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
        let rows = totals_with_scheme(&mut tx, tenant, course, &scheme).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseGradebookTotals {
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            rows,
        })
    }

    async fn create_course_grade_export(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseGradeExport, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant_writable_snapshot(context).await?;
        let actor = require_course_instructor(&mut tx, session, course).await?;
        let scheme = read_scheme(&mut tx, tenant, course).await?;
        let rows = totals_with_scheme(&mut tx, tenant, course, &scheme).await?;
        let id = CourseGradeExportId::generate()?;
        let audit = CourseGradeExportAudit {
            id,
            tenant,
            course,
            requested_by: actor,
            scheme_revision: scheme.revision,
            mode: scheme.scheme.mode,
            rounding: scheme.scheme.rounding,
            row_count: rows.len(),
        };
        sqlx::query("INSERT INTO course_total_export_audit (tenant_id,course_id,export_id,requested_by,row_count,scheme_revision,mode,rounding) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(id.as_uuid()).bind(actor.as_uuid()).bind(i32::try_from(rows.len()).expect("bounded rows")).bind(scheme.revision.to_i64()?).bind(mode_name(scheme.scheme.mode)).bind(rounding_name(scheme.scheme.rounding)).execute(&mut *tx).await.map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(CourseGradeExport { audit, rows })
    }
}

async fn read_scheme(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseGradeSchemeRecord, StoreError> {
    read_scheme_inner(tx, tenant, course, false).await
}
async fn read_scheme_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<CourseGradeSchemeRecord, StoreError> {
    read_scheme_inner(tx, tenant, course, true).await
}
async fn read_scheme_inner(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    for_update: bool,
) -> Result<CourseGradeSchemeRecord, StoreError> {
    let row = if for_update {
        sqlx::query("SELECT mode,rounding,revision FROM course_grade_scheme WHERE tenant_id=$1 AND course_id=$2 FOR UPDATE")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_optional(&mut **tx).await
    } else {
        sqlx::query("SELECT mode,rounding,revision FROM course_grade_scheme WHERE tenant_id=$1 AND course_id=$2")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_optional(&mut **tx).await
    }
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

async fn lock_assignment_ids(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<std::collections::BTreeSet<AssignmentId>, StoreError> {
    let rows=sqlx::query_scalar::<_,Uuid>("SELECT assignment_id FROM assignment WHERE tenant_id=$1 AND course_id=$2 ORDER BY assignment_id FOR UPDATE").bind(tenant.as_uuid()).bind(course.as_uuid()).fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(AssignmentId::from_uuid).collect())
}

/// Advances the strong scheme token after a title-bearing assignment read
/// projection changes through the assignment Store.
pub(super) async fn advance_course_grade_scheme_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
) -> Result<(), StoreError> {
    let row = sqlx::query_scalar::<_, i64>(
        "UPDATE course_grade_scheme SET revision=revision+1, \
         updated_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND course_id=$2 AND revision < 9223372036854775807 \
         RETURNING revision",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::Unavailable("course grade scheme revision could not advance".to_string())
    })?;
    CourseGradeSchemeRevision::from_i64(row)?;
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
fn fresh_band_id() -> Result<Uuid, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::Unavailable(format!("course letter-band ID unavailable: {error}"))
    })?;
    Ok(Uuid::from_bytes(bytes))
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

/// Builds one learner's evaluator input from the preloaded course snapshot.
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

async fn totals_with_scheme(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    course: CourseId,
    scheme: &CourseGradeSchemeRecord,
) -> Result<Vec<CourseGradebookTotalRow>, StoreError> {
    let roster = sqlx::query(
        "SELECT m.student_id,m.roster_id,p.roster_email_normalized,p.roster_email_delivery,p.display_name \
         FROM course_member m JOIN course_roster_profile p \
           ON p.tenant_id=m.tenant_id AND p.course_id=m.course_id \
          AND p.course_membership_id=m.course_membership_id \
         WHERE m.tenant_id=$1 AND m.course_id=$2 AND m.role='student' AND m.status='active' \
         ORDER BY m.roster_id LIMIT $3",
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
        let learner_inputs = course_grade_inputs_for_student(&inputs, &scores, student);
        let outcome =
            calculate_course_grade(&scheme.scheme, &learner_inputs).map_err(|e| match e {
                CourseGradeError::MissingCategory { .. }
                | CourseGradeError::UnknownCategory { .. } => StoreError::Unavailable(
                    "weighted course grade scheme requires a mapping for each included assignment"
                        .into(),
                ),
                other => StoreError::InvalidRecord(other.to_string()),
            })?;
        let delivery: String = person
            .try_get("roster_email_delivery")
            .map_err(map_sqlx_error)?;
        let email = AuthenticationEmail::parse(&delivery)
            .map_err(|_| StoreError::Unavailable("stored roster email is invalid".into()))?;
        let normalized: String = person
            .try_get("roster_email_normalized")
            .map_err(map_sqlx_error)?;
        if email.normalized() != normalized {
            return Err(StoreError::Unavailable(
                "stored roster email normalization is invalid".into(),
            ));
        };
        rows.push(CourseGradebookTotalRow {
            roster_id: CourseRosterId::parse(
                &person
                    .try_get::<String, _>("roster_id")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| StoreError::Unavailable("stored roster ID is invalid".into()))?,
            roster_email: email,
            display_name: person.try_get("display_name").map_err(map_sqlx_error)?,
            outcome,
        });
    }
    Ok(rows)
}
