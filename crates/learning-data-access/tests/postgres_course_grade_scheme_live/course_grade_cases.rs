use super::load_acceptance_runtime;
use super::{
    create_fixture_course, create_published_assignment, id, numeric_assignment,
    publish_fixture_question, session, set_summary_scores,
};

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AssignmentEntitlementMaterialization, CourseGradeAssignmentMembership,
    CourseGradeSchemeRevision, CourseGradebookStore, CourseRosterContact, CourseRosterId,
    CourseRosterStore, MaterializeAssignmentEntitlementCommand, Store, StoreError, TenantContext,
    UpdateCourseGradeScheme, UpsertCourseMember,
};
use question_model::{
    CourseGradeMode, CourseGradeRoundingRule, CourseGradeScheme, CourseId, EntitlementPurpose,
    GradeCategoryId, GradeCategoryTitle, TenantId, UserId, WeightedGradeCategory,
};
use sqlx::Row;
use uuid::Uuid;
async fn scalar_i64(pool: &sqlx::PgPool, sql: &'static str) -> i64 {
    sqlx::query(sql)
        .fetch_one(pool)
        .await
        .expect("live oracle query")
        .try_get(0)
        .expect("integer result")
}

async fn scheme_version_marker(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    course: CourseId,
) -> (f64, i64) {
    let mut tx = pool.begin().await.expect("timestamp transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant context");
    let row = sqlx::query("SELECT extract(epoch FROM updated_at)::float8, xmin::text::bigint FROM course_grade_scheme WHERE tenant_id=$1 AND course_id=$2")
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .expect("scheme timestamp");
    let marker = (
        row.try_get(0).expect("timestamp value"),
        row.try_get(1).expect("tuple xmin"),
    );
    tx.commit().await.expect("timestamp commit");
    marker
}

async fn app_without_tenant_count(pool: &sqlx::PgPool) -> i64 {
    let mut tx = pool.begin().await.expect("no-context transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("app role");
    let count: i64 = sqlx::query("SELECT count(*) FROM course_grade_scheme")
        .fetch_one(&mut *tx)
        .await
        .expect("no-context query")
        .try_get(0)
        .expect("count");
    tx.rollback().await.expect("no-context rollback");
    count
}

async fn app_foreign_tenant_count(pool: &sqlx::PgPool, tenant: TenantId) -> i64 {
    let mut tx = pool.begin().await.expect("foreign-context transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("foreign tenant context");
    let count: i64 = sqlx::query("SELECT count(*) FROM course_grade_scheme")
        .fetch_one(&mut *tx)
        .await
        .expect("foreign tenant query")
        .try_get(0)
        .expect("count");
    tx.rollback().await.expect("foreign rollback");
    count
}

async fn app_cannot_mutate_audit(pool: &sqlx::PgPool, tenant: TenantId, course: CourseId) {
    let mut tx = pool.begin().await.expect("audit privilege transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *tx)
        .await
        .expect("app role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant context");
    sqlx::query("SAVEPOINT audit_update_denied")
        .execute(&mut *tx)
        .await
        .expect("audit update savepoint");
    assert!(
        sqlx::query(
            "UPDATE course_total_export_audit SET row_count=0 WHERE tenant_id=$1 AND course_id=$2"
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .execute(&mut *tx)
        .await
        .is_err()
    );
    sqlx::query("ROLLBACK TO SAVEPOINT audit_update_denied")
        .execute(&mut *tx)
        .await
        .expect("audit update savepoint rollback");
    sqlx::query("SAVEPOINT audit_delete_denied")
        .execute(&mut *tx)
        .await
        .expect("audit delete savepoint");
    sqlx::query("DELETE FROM course_total_export_audit WHERE tenant_id=$1 AND course_id=$2")
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .execute(&mut *tx)
        .await
        .expect_err("ple_app must not delete a durable export audit");
    sqlx::query("ROLLBACK TO SAVEPOINT audit_delete_denied")
        .execute(&mut *tx)
        .await
        .expect("audit delete savepoint rollback");
    tx.rollback().await.expect("audit privilege rollback");
}

/// The graph report explicitly says SQL extraction is incomplete because
/// `tree_sitter_sql` is unavailable.  Keep these physical probes against a
/// migrated database rather than relying on catalog-text assertions.
async fn physical_scheme_and_score_guards(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    course: CourseId,
    assignment: question_model::AssignmentId,
    enrollment: Uuid,
) {
    let other_course = CourseId::from_uuid(id());
    let mut tx = pool.begin().await.expect("physical guard transaction");
    // These rolled-back statements probe physical constraints as the schema
    // owner. Product mutation authority is exercised separately through the
    // Store capability and explicit ple_app denial checks.
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("tenant context");
    // The Store fixture already exercised weighted persistence. Reset only
    // this rolled-back transaction so the first guard proves the mode rule
    // instead of failing for an unrelated duplicate category.
    for statement in [
        "DELETE FROM course_grade_category_assignment WHERE tenant_id=$1 AND course_id=$2",
        "DELETE FROM course_grade_letter_band WHERE tenant_id=$1 AND course_id=$2",
        "DELETE FROM course_grade_category WHERE tenant_id=$1 AND course_id=$2",
    ] {
        sqlx::query(statement)
            .bind(tenant.as_uuid())
            .bind(course.as_uuid())
            .execute(&mut *tx)
            .await
            .expect("reset physical grade-scheme fixture");
    }
    sqlx::query(
        "UPDATE course_grade_scheme SET mode='total_points' WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .execute(&mut *tx)
    .await
    .expect("reset physical total-points mode");
    // This small macro makes every intentionally failing statement recover to
    // a savepoint, so PostgreSQL's aborted-transaction behavior is itself not
    // mistaken for a later constraint result.
    macro_rules! rejected {
        ($statement:expr, $message:literal) => {{
            sqlx::query("SAVEPOINT physical_guard")
                .execute(&mut *tx)
                .await
                .expect("savepoint");
            assert!($statement.execute(&mut *tx).await.is_err(), $message);
            sqlx::query("ROLLBACK TO SAVEPOINT physical_guard")
                .execute(&mut *tx)
                .await
                .expect("savepoint rollback");
        }};
    }
    let category = id();
    rejected!(
        sqlx::query("INSERT INTO course_grade_category (tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest) VALUES ($1,$2,$3,0,'No total',10000,0)")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category),
        "total-points mode rejects categories"
    );
    // Switch through the Store-independent physical mode guard, then install
    // one valid category for assignment and duplicate-position probes.
    sqlx::query("UPDATE course_grade_scheme SET mode='weighted_categories' WHERE tenant_id=$1 AND course_id=$2")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).execute(&mut *tx).await.expect("physical weighted switch");
    sqlx::query("INSERT INTO course_grade_category (tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest) VALUES ($1,$2,$3,0,'Physical',10000,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).execute(&mut *tx).await.expect("valid category");
    rejected!(
        sqlx::query(
            "UPDATE course_grade_scheme SET mode='total_points' WHERE tenant_id=$1 AND course_id=$2"
        )
        .bind(tenant.as_uuid())
        .bind(course.as_uuid()),
        "total mode rejects extant categories"
    );
    for (position, title, weight, drop) in [
        (-1, "Bad", 1, 0),
        (1, "Bad", 0, 0),
        (1, "Bad", 10001, 0),
        (1, "Bad", 1, -1),
    ] {
        rejected!(sqlx::query("INSERT INTO course_grade_category (tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(id()).bind(position).bind(title).bind(weight).bind(drop), "category scalar boundary rejected");
    }
    for title in [" ".to_owned(), "x".repeat(201)] {
        rejected!(sqlx::query("INSERT INTO course_grade_category (tenant_id,course_id,category_id,position,title,weight_basis_points,drop_lowest) VALUES ($1,$2,$3,1,$4,1,0)")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(id()).bind(title), "category title boundary rejected");
    }
    rejected!(sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,0)")
        .bind(tenant.as_uuid()).bind(other_course.as_uuid()).bind(category).bind(assignment.as_uuid()), "cross-course category membership FK rejected");
    sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).bind(assignment.as_uuid()).execute(&mut *tx).await.expect("valid assignment membership");
    let second_assignment = id();
    sqlx::query("INSERT INTO assignment (tenant_id,assignment_id,course_id,audience_kind,title,score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure,solution_disclosure,class_statistics_disclosure) VALUES ($1,$2,$3,'course_wide','Physical duplicate-position assignment','after_submit','after_submit','after_submit','after_submit','never')")
        .bind(tenant.as_uuid()).bind(second_assignment).bind(course.as_uuid()).execute(&mut *tx).await.expect("second assignment for position uniqueness");
    rejected!(sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).bind(assignment.as_uuid()), "duplicate membership and position rejected");
    rejected!(sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,0)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).bind(second_assignment), "duplicate category position rejected");
    rejected!(sqlx::query("INSERT INTO course_grade_category_assignment (tenant_id,course_id,category_id,assignment_id,position) VALUES ($1,$2,$3,$4,-1)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(category).bind(assignment.as_uuid()), "negative assignment position rejected");
    for (label, minimum) in [
        (" ".to_owned(), 0),
        ("x".repeat(33), 0),
        ("A".to_owned(), -1),
        ("A".to_owned(), 10001),
    ] {
        rejected!(sqlx::query("INSERT INTO course_grade_letter_band (tenant_id,course_id,letter_band_id,label,minimum_basis_points) VALUES ($1,$2,$3,$4,$5)")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(id()).bind(label).bind(minimum), "letter-band scalar boundary rejected");
    }
    sqlx::query("INSERT INTO course_grade_letter_band (tenant_id,course_id,letter_band_id,label,minimum_basis_points) VALUES ($1,$2,$3,'A',9000)")
        .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(id()).execute(&mut *tx).await.expect("valid letter band");
    for (label, minimum) in [("A", 8000), ("B", 9000)] {
        rejected!(sqlx::query("INSERT INTO course_grade_letter_band (tenant_id,course_id,letter_band_id,label,minimum_basis_points) VALUES ($1,$2,$3,$4,$5)")
            .bind(tenant.as_uuid()).bind(course.as_uuid()).bind(id()).bind(label).bind(minimum), "duplicate letter label or minimum rejected");
    }
    // The roster upsert created this minimal summary row.  Update it to prove
    // the live widened physical check, not merely its pg_constraint text.
    for value in [-1000.0, 1000.0] {
        sqlx::query("UPDATE student_assignment_summary SET current_score=$3,best_score=$3,latest_score=$3,completed_run_count=0,total_question_attempts=0 WHERE tenant_id=$1 AND enrollment_id=$2")
            .bind(tenant.as_uuid()).bind(enrollment).bind(value).execute(&mut *tx).await.expect("summary score endpoint accepted");
    }
    for value in [-1000.0001, 1000.0001] {
        rejected!(sqlx::query("UPDATE student_assignment_summary SET current_score=$3 WHERE tenant_id=$1 AND enrollment_id=$2")
            .bind(tenant.as_uuid()).bind(enrollment).bind(value), "summary out-of-range score rejected");
    }
    rejected!(
        sqlx::query("UPDATE student_assignment_summary SET completed_run_count=-1 WHERE tenant_id=$1 AND enrollment_id=$2")
            .bind(tenant.as_uuid()).bind(enrollment),
        "summary negative completed-run counter rejected"
    );
    rejected!(
        sqlx::query("UPDATE student_assignment_summary SET total_question_attempts=-1 WHERE tenant_id=$1 AND enrollment_id=$2")
            .bind(tenant.as_uuid()).bind(enrollment),
        "summary negative attempt counter rejected"
    );
    let job = id();
    let payload = format!(r#"{{"kind":"export","delivery_object":"{}"}}"#, id());
    sqlx::query("INSERT INTO worker_job (job_id,tenant_id,payload,state,max_attempts) VALUES ($1,$2,$3::jsonb,'ready',1)")
        .bind(job).bind(tenant.as_uuid()).bind(payload).execute(&mut *tx).await.expect("minimal staging job");
    let staging = |value: f64| {
        sqlx::query("INSERT INTO assignment_summary_staging (tenant_id,job_id,assignment_id,scoring_generation,enrollment_id,current_score,best_score,latest_score,completed_run_count,total_question_attempts) VALUES ($1,$2,$3,1,$4,$5,$5,$5,0,0)")
        .bind(tenant.as_uuid()).bind(job).bind(assignment.as_uuid()).bind(enrollment).bind(value)
    };
    for value in [-1000.0, 1000.0] {
        staging(value)
            .execute(&mut *tx)
            .await
            .expect("staging score endpoint accepted");
        sqlx::query("DELETE FROM assignment_summary_staging WHERE tenant_id=$1 AND job_id=$2 AND enrollment_id=$3").bind(tenant.as_uuid()).bind(job).bind(enrollment).execute(&mut *tx).await.expect("staging endpoint cleanup");
    }
    for value in [-1000.0001, 1000.0001] {
        rejected!(staging(value), "staging out-of-range score rejected");
    }
    rejected!(
        sqlx::query("INSERT INTO assignment_summary_staging (tenant_id,job_id,assignment_id,scoring_generation,enrollment_id,completed_run_count,total_question_attempts) VALUES ($1,$2,$3,1,$4,-1,0)")
            .bind(tenant.as_uuid()).bind(job).bind(assignment.as_uuid()).bind(enrollment),
        "staging negative completed-run counter rejected"
    );
    rejected!(
        sqlx::query("INSERT INTO assignment_summary_staging (tenant_id,job_id,assignment_id,scoring_generation,enrollment_id,completed_run_count,total_question_attempts) VALUES ($1,$2,$3,1,$4,0,-1)")
            .bind(tenant.as_uuid()).bind(job).bind(assignment.as_uuid()).bind(enrollment),
        "staging negative attempt counter rejected"
    );
    tx.rollback()
        .await
        .expect("physical probes remain disposable");
}

async fn retention_deletes_audit(pool: &sqlx::PgPool, tenant: TenantId, course: CourseId) -> i64 {
    let mut tx = pool.begin().await.expect("retention transaction");
    sqlx::query("SET LOCAL ROLE ple_retention_broker")
        .execute(&mut *tx)
        .await
        .expect("retention role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .expect("retention tenant");
    sqlx::query("DELETE FROM course_total_export_audit WHERE tenant_id=$1 AND course_id=$2")
        .bind(tenant.as_uuid())
        .bind(course.as_uuid())
        .execute(&mut *tx)
        .await
        .expect("retention deletes course-total audit");
    let remaining: i64 = sqlx::query(
        "SELECT count(*) FROM course_total_export_audit WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&mut *tx)
    .await
    .expect("retention absence query")
    .try_get(0)
    .expect("remaining count");
    tx.commit().await.expect("retention commit");
    remaining
}

#[rustfmt::skip]
async fn postgres_course_grade_totals_use_only_summary_projection_and_preserve_transitions() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose(); let pool = lazy_pool(url).expect("PostgreSQL URL"); verify_application_schema(&pool).await.expect("migrated schema");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x66; 32]); let tenant = TenantId::from_uuid(id()); let context = TenantContext::from_authenticated_session(tenant); let instructor = UserId::from_uuid(id()); let token = session(&store, tenant, instructor).await; let course = create_fixture_course(&store, context, tenant, instructor).await; let reference = publish_fixture_question(&store, context, tenant, instructor).await;
    let records = [("Ten points", 10), ("Thirty points", 30), ("Zero-point extra", 0), ("Negative score", 10)].map(|(title, points)| numeric_assignment(tenant, course, reference, title, points)); let assignments = records.clone().map(|record| record.id);
    for record in records { create_published_assignment(&store, context, instructor, record, question_model::BaseAssignmentPolicy::default()).await.expect("Store creates and publishes numeric assignment"); }
    let student = UserId::from_uuid(id());
    store.upsert_course_member(context, instructor, UpsertCourseMember { course, user: student, display_name: "Numeric learner".into(), roster_contact: Some(CourseRosterContact { email: learning_data_access::AuthenticationEmail::parse("numeric.live@roosevelt.edu").expect("email"), roster_id: CourseRosterId::parse("900123458").expect("roster ID") }) }).await.expect("contact-bearing learner");
    for assignment in assignments {
        let issued = store.issue_assignment_entitlement(context, MaterializeAssignmentEntitlementCommand::for_instructor_action(student, course, assignment, instructor, EntitlementPurpose::InstructorIssue).expect("typed instructor issue")).await.expect("materialize numeric enrollment");
        assert!(matches!(issued, AssignmentEntitlementMaterialization::Granted(_)), "current learner receives numeric enrollment");
    }
    let scores = [(assignments[0], Some(0.8)), (assignments[1], None), (assignments[2], Some(5.0)), (assignments[3], Some(-0.1))];
    set_summary_scores(&pool, tenant, student, &scores, None).await;
    let total = store.course_gradebook_totals(context, token, course).await.expect("summary-only total");
    assert_eq!(total.rows[0].outcome.rounded_score, Some(0.24), "10/30-point weighting, missing zero, negative score, and zero-point extra credit contribute exactly once");
    set_summary_scores(&pool, tenant, student, &[(assignments[0], Some(0.8))], Some("recalculating")).await;
    assert_eq!(store.course_gradebook_totals(context, token, course).await.expect("recalculating row").rows[0].outcome.unavailable_reason, Some(domain::course_grade::CourseGradeUnavailableReason::Recalculating));
    set_summary_scores(&pool, tenant, student, &[(assignments[0], Some(0.8))], Some("failed")).await;
    assert_eq!(store.course_gradebook_totals(context, token, course).await.expect("failed row").rows[0].outcome.unavailable_reason, Some(domain::course_grade::CourseGradeUnavailableReason::Failed));
    set_summary_scores(&pool, tenant, student, &scores, Some("current")).await;
    let before = store.course_grade_scheme(context, token, course).await.expect("default scheme"); let home = GradeCategoryId::from_uuid(id()); let exam = GradeCategoryId::from_uuid(id());
    let weighted = CourseGradeScheme { mode: CourseGradeMode::WeightedCategories, rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero, categories: vec![WeightedGradeCategory { id: home, title: GradeCategoryTitle::new("Homework").expect("title"), position: 0, weight_basis_points: 5000, drop_lowest: 1 }, WeightedGradeCategory { id: exam, title: GradeCategoryTitle::new("Exam").expect("title"), position: 1, weight_basis_points: 5000, drop_lowest: 0 }], letter_bands: Vec::new() };
    let tied = [(assignments[0], Some(0.8)), (assignments[1], Some(0.8)), (assignments[2], Some(5.0)), (assignments[3], Some(-0.1))];
    set_summary_scores(&pool, tenant, student, &tied, None).await;
    let mapped = store
        .update_course_grade_scheme(
            context,
            token,
            UpdateCourseGradeScheme {
                course,
                expected_revision: before.revision,
                scheme: weighted.clone(),
                assignments: vec![
                    CourseGradeAssignmentMembership {
                        assignment: assignments[0],
                        included: true,
                        category: Some(home),
                        position: Some(0),
                    },
                    CourseGradeAssignmentMembership {
                        assignment: assignments[1],
                        included: true,
                        category: Some(home),
                        position: Some(1),
                    },
                    CourseGradeAssignmentMembership {
                        assignment: assignments[2],
                        included: true,
                        category: Some(exam),
                        position: Some(0),
                    },
                    CourseGradeAssignmentMembership {
                        assignment: assignments[3],
                        included: true,
                        category: Some(exam),
                        position: Some(1),
                    },
                ],
            },
        )
        .await
        .expect("weighted point aggregation");
    let weighted_total = store
        .course_gradebook_totals(context, token, course)
        .await
        .expect("weighted total");
    assert_eq!(
        (
            weighted_total.rows[0].outcome.rounded_score,
            weighted_total.rows[0]
                .outcome
                .dropped_assignment_ids
                .as_slice()
        ),
        (Some(0.6), &[assignments[1]][..]),
        "equal scores drop the later category position deterministically"
    );
    let new_assignment = numeric_assignment(tenant, course, reference, "New unmapped", 1);
    let new_id = new_assignment.id;
    create_published_assignment(&store, context, instructor, new_assignment, question_model::BaseAssignmentPolicy::default()).await.expect("new Store assignment");
    assert!(
        matches!(
            store.course_gradebook_totals(context, token, course).await,
            Err(StoreError::Unavailable(_))
        ),
        "new included weighted assignment is unavailable until remapped"
    );
    let remapped = store
        .course_grade_scheme(context, token, course)
        .await
        .expect("unmapped read remains visible");
    assert_eq!(
        remapped.revision,
        mapped
            .revision
            .next()
            .expect("assignment projection advances scheme revision"),
        "a new title-bearing assignment invalidates the prior strong token"
    );
    let mut memberships: Vec<_> = mapped
        .assignments
        .iter()
        .map(|entry| CourseGradeAssignmentMembership {
            assignment: entry.assignment,
            included: entry.included,
            category: entry.category,
            position: entry.position,
        })
        .collect();
    memberships.push(CourseGradeAssignmentMembership {
        assignment: new_id,
        included: false,
        category: None,
        position: None,
    });
    assert_eq!(
        store
            .update_course_grade_scheme(
                context,
                token,
                UpdateCourseGradeScheme {
                    course,
                    expected_revision: mapped.revision,
                    scheme: weighted.clone(),
                    assignments: memberships.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "the pre-assignment scheme token is stale before request validation"
    );
    store
        .update_course_grade_scheme(
            context,
            token,
            UpdateCourseGradeScheme {
                course,
                expected_revision: remapped.revision,
                scheme: weighted,
                assignments: memberships,
            },
        )
        .await
        .expect("revision-checked remap");
    let totals = store
        .course_gradebook_totals(context, token, course)
        .await
        .expect("remapped totals");
    let export = store
        .create_course_grade_export(context, token, course)
        .await
        .expect("export");
    assert_eq!(export.rows, totals.rows);
    assert_eq!(export.audit.row_count, 1);
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_course_grade_scheme_is_migrated_defaulted_revisioned_bounded_and_rls_fenced() {
    let runtime = load_acceptance_runtime();
    let database_url = runtime.admin_url().expose();
    let pool = lazy_pool(database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("S6 migration must be compatible");
    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x66; 32]);
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(id());
    let token = session(&store, tenant, instructor).await;

    // Inventory proves exact 1806; a Store course proves the new-course default.
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM _sqlx_migrations WHERE version = 2026081806 AND success",
        )
        .await,
        1,
        "migration 1806 must be applied exactly once",
    );
    let course = create_fixture_course(&store, context, tenant, instructor).await;
    let initial = store
        .course_grade_scheme(context, token, course)
        .await
        .expect("new course default scheme");
    assert_eq!(initial.revision, CourseGradeSchemeRevision::INITIAL);
    assert_eq!(initial.scheme.mode, CourseGradeMode::TotalPoints);
    assert_eq!(
        initial.scheme.rounding,
        CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero
    );
    assert!(initial.assignments.is_empty());
    let initial_marker = scheme_version_marker(&pool, tenant, course).await;
    assert_eq!(
        store
            .course_grade_scheme(foreign_context, token, course)
            .await,
        Err(StoreError::NotFound),
        "a foreign tenant cannot use a tenant-A instructor token"
    );
    assert_eq!(app_foreign_tenant_count(&pool, foreign_tenant).await, 0);

    let reference = publish_fixture_question(&store, context, tenant, instructor).await;
    let assignment_record = numeric_assignment(tenant, course, reference, "S6 empty assignment", 1);
    let assignment = assignment_record.id;
    create_published_assignment(
        &store,
        context,
        instructor,
        assignment_record,
        question_model::BaseAssignmentPolicy::default(),
    )
    .await
    .expect("Store creates title-bearing fixture assignment");
    let with_assignment = store
        .course_grade_scheme(context, token, course)
        .await
        .expect("default scheme incorporates current assignment");
    assert_eq!(with_assignment.assignments.len(), 1);
    assert_eq!(with_assignment.assignments[0].assignment, assignment);
    assert_eq!(with_assignment.assignments[0].title, "S6 empty assignment");
    assert_eq!(
        with_assignment.revision,
        initial
            .revision
            .next()
            .expect("assignment projection advances revision")
    );
    let category = GradeCategoryId::from_uuid(id());
    let weighted = CourseGradeScheme {
        mode: CourseGradeMode::WeightedCategories,
        rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero,
        categories: vec![WeightedGradeCategory {
            id: category,
            title: GradeCategoryTitle::new("Homework").expect("fixture title"),
            position: 0,
            weight_basis_points: 10_000,
            drop_lowest: 0,
        }],
        letter_bands: Vec::new(),
    };
    let weighted_saved = store
        .update_course_grade_scheme(
            context,
            token,
            UpdateCourseGradeScheme {
                course,
                expected_revision: with_assignment.revision,
                scheme: weighted,
                assignments: vec![CourseGradeAssignmentMembership {
                    assignment,
                    included: true,
                    category: Some(category),
                    position: Some(0),
                }],
            },
        )
        .await
        .expect("weighted mapping round trip");
    assert_eq!(
        weighted_saved.scheme.mode,
        CourseGradeMode::WeightedCategories
    );
    assert_eq!(weighted_saved.assignments[0].category, Some(category));
    let weighted_marker = scheme_version_marker(&pool, tenant, course).await;
    assert!(weighted_marker.0 >= initial_marker.0);
    assert_ne!(
        weighted_marker.1, initial_marker.1,
        "separate CAS write has a new tuple version"
    );

    // A no-activity total is a real Store calculation and export, with no run
    // or attempt fixture. The audit must be metadata only.
    let student = UserId::from_uuid(id());
    store
        .upsert_course_member(
            context,
            instructor,
            UpsertCourseMember {
                course,
                user: student,
                display_name: "S6 learner".into(),
                roster_contact: Some(CourseRosterContact {
                    email: learning_data_access::AuthenticationEmail::parse(
                        "s6.live@roosevelt.edu",
                    )
                    .expect("roster email"),
                    roster_id: CourseRosterId::parse("900123459").expect("roster ID"),
                }),
            },
        )
        .await
        .expect("active student fixture");
    let materialized = store
        .issue_assignment_entitlement(
            context,
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment,
                instructor,
                EntitlementPurpose::InstructorIssue,
            )
            .expect("typed instructor issue"),
        )
        .await
        .expect("materialize no-activity enrollment");
    let AssignmentEntitlementMaterialization::Granted(materialized) = materialized else {
        panic!("current course-wide learner must receive an enrollment")
    };
    let enrollment = materialized.enrollment.id.as_uuid();
    physical_scheme_and_score_guards(&pool, tenant, course, assignment, enrollment).await;
    let totals = store
        .course_gradebook_totals(context, token, course)
        .await
        .expect("summary-only totals");
    assert_eq!(totals.rows.len(), 1);
    let export = store
        .create_course_grade_export(context, token, course)
        .await
        .expect("synchronous bounded export");
    assert_eq!(export.audit.row_count, 1);
    app_cannot_mutate_audit(&pool, tenant, course).await;
    assert_eq!(
        app_without_tenant_count(&pool).await,
        0,
        "forced RLS denies no-context app reads"
    );
    assert_eq!(retention_deletes_audit(&pool, tenant, course).await, 0);
    let audit_after_delete = store
        .create_course_grade_export(context, token, course)
        .await
        .expect("export remains available after retention cleanup");
    assert_eq!(audit_after_delete.audit.row_count, 1);
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM information_schema.columns WHERE table_schema='public' \
             AND table_name='course_total_export_audit' AND column_name IN \
             ('roster_id','roster_email','display_name','student_id','enrollment_id')",
        )
        .await,
        0,
        "durable course export audit must remain PII-free",
    );

    // Whole-record CAS is strong: a successful replacement advances both
    // revision and timestamp; a replay of the original token conflicts.
    let total_memberships: Vec<CourseGradeAssignmentMembership> = with_assignment
        .assignments
        .iter()
        .map(|entry| CourseGradeAssignmentMembership {
            assignment: entry.assignment,
            included: entry.included,
            category: entry.category,
            position: entry.position,
        })
        .collect();
    let before_total_marker = scheme_version_marker(&pool, tenant, course).await;
    let saved = store
        .update_course_grade_scheme(
            context,
            token,
            UpdateCourseGradeScheme {
                course,
                expected_revision: weighted_saved.revision,
                scheme: with_assignment.scheme.clone(),
                assignments: total_memberships.clone(),
            },
        )
        .await
        .expect("total-points replacement");
    let after_total_marker = scheme_version_marker(&pool, tenant, course).await;
    // `updated_at` is transaction_timestamp(), so two very fast transactions
    // can theoretically share the same clock value.  The revision and xmin
    // together prove the successful CAS created a distinct tuple even then.
    assert!(after_total_marker.0 >= before_total_marker.0);
    assert_ne!(after_total_marker.1, before_total_marker.1);
    assert_eq!(
        saved.revision,
        weighted_saved
            .revision
            .next()
            .expect("total-points CAS advances revision")
    );
    assert!(matches!(
        store
            .update_course_grade_scheme(
                context,
                token,
                UpdateCourseGradeScheme {
                    course,
                    expected_revision: weighted_saved.revision,
                    scheme: with_assignment.scheme,
                    assignments: total_memberships,
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let stored_revision: i64 = sqlx::query_scalar(
        "SELECT revision FROM public.course_grade_scheme WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("stored scheme revision");
    assert_eq!(
        stored_revision,
        saved.revision.to_i64().expect("stored revision range")
    );

    // The database-facing schema contract is deliberately inspected directly:
    // Graphify does not extract SQL (tree_sitter_sql unavailable).  These
    // assertions cover the normalized mapping, mode, bounded score and audit
    // constraints without granting the application an RLS bypass.
    for relation in [
        "course_grade_scheme",
        "course_grade_category",
        "course_grade_category_assignment",
        "course_grade_letter_band",
        "course_total_export_audit",
    ] {
        let forced: bool = sqlx::query(
            "SELECT relforcerowsecurity FROM pg_class \
             WHERE oid = format('public.%I', $1)::regclass",
        )
        .bind(relation)
        .fetch_one(&pool)
        .await
        .expect("RLS catalog row")
        .try_get(0)
        .expect("RLS catalog value");
        assert!(forced, "{relation} is forced RLS");
    }
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM information_schema.role_table_grants \
             WHERE table_schema='public' AND table_name='course_total_export_audit' \
             AND grantee='ple_app' AND privilege_type='INSERT'",
        )
        .await,
        0,
        "application role uses the audited export capability instead of a direct table grant",
    );
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM pg_proc AS procedure_row \
             CROSS JOIN LATERAL aclexplode(COALESCE(\
                 procedure_row.proacl, acldefault('f', procedure_row.proowner)\
             )) AS privilege_row \
             WHERE procedure_row.oid IN (\
                 'public.ple_replace_course_grade_scheme_v1(uuid,character,uuid,bigint,jsonb)'::regprocedure, \
                 'public.ple_record_course_grade_export_audit_v1(uuid,character,uuid,uuid,integer,bigint,text,text)'::regprocedure\
             ) AND privilege_row.grantee='ple_app'::regrole \
               AND privilege_row.privilege_type='EXECUTE'",
        )
        .await,
        2,
        "application role receives exactly the two grade-control capabilities",
    );
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM pg_constraint WHERE conrelid='public.course_total_export_audit'::regclass \
             AND conname='course_total_export_audit_row_count_check' AND convalidated",
        )
        .await,
        1,
        "database audit cap is 500",
    );
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM pg_constraint WHERE conrelid='public.student_assignment_summary'::regclass \
             AND conname='student_assignment_summary_value_check' AND convalidated",
        )
        .await,
        1,
        "summary check is widened to the documented score domain",
    );

    // Retention is explicitly able to delete the PII-free audit, while no
    // grade operation needs run or attempt tables to be present.
    assert_eq!(
        scalar_i64(
            &pool,
            "SELECT count(*) FROM information_schema.role_table_grants \
             WHERE table_schema='public' AND table_name='course_total_export_audit' \
             AND grantee='ple_retention_broker' AND privilege_type='DELETE'",
        )
        .await,
        1,
        "retention broker can clean old course-total audits",
    );
    let weighted = CourseGradeScheme {
        mode: CourseGradeMode::WeightedCategories,
        rounding: CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero,
        categories: Vec::new(),
        letter_bands: Vec::new(),
    };
    let invalid_weighted = store
        .update_course_grade_scheme(
            context,
            token,
            UpdateCourseGradeScheme {
                course,
                expected_revision: saved.revision,
                scheme: weighted,
                assignments: Vec::<CourseGradeAssignmentMembership>::new(),
            },
        )
        .await;
    assert!(
        matches!(invalid_weighted, Err(StoreError::InvalidRecord(_))),
        "empty weighted schemes are rejected before SQL"
    );
    let capped_course = create_fixture_course(&store, context, tenant, instructor).await;
    for number in 0..501_u16 {
        let email = learning_data_access::AuthenticationEmail::parse(&format!(
            "cap-{number}@roosevelt.edu"
        ))
        .expect("cap fixture email");
        let roster_id =
            CourseRosterId::parse(&format!("99{number:07}")).expect("cap fixture roster ID");
        store
            .upsert_course_member(
                context,
                instructor,
                UpsertCourseMember {
                    course: capped_course,
                    user: UserId::from_uuid(id()),
                    display_name: format!("cap learner {number}"),
                    roster_contact: Some(CourseRosterContact { email, roster_id }),
                },
            )
            .await
            .expect("cap fixture roster member");
    }
    assert!(matches!(
        store
            .course_gradebook_totals(context, token, capped_course)
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    // The baseline calls this established test by name; retain the numeric
    // adapter/evaluator oracle in that same permanent execution path.
    postgres_course_grade_totals_use_only_summary_projection_and_preserve_transitions().await;
}
