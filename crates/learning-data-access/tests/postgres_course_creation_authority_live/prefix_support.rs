//! Shared exact-prefix fixtures for the course-creation authority oracle.

use super::*;

pub(super) async fn installer<'a>(pool: &'a PgPool) -> Transaction<'a, Postgres> {
    let mut transaction = pool.begin().await.expect("installer transaction");
    become_installer(&mut transaction).await;
    transaction
}

pub(super) async fn become_installer(transaction: &mut Transaction<'_, Postgres>) {
    sqlx::query("SET LOCAL ROLE ple_base_course_installer")
        .execute(&mut **transaction)
        .await
        .expect("installer role");
}

pub(super) async fn owner_role(transaction: &mut Transaction<'_, Postgres>) {
    sqlx::query("RESET ROLE")
        .execute(&mut **transaction)
        .await
        .expect("fixture owner role");
}

pub(super) async fn create_as_instructor(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    course: Uuid,
    actor: Uuid,
    session: &str,
) -> Result<(Uuid, Uuid), sqlx::Error> {
    create_course(transaction, tenant, course, actor, session, false).await
}

pub(super) async fn create_as_sysadmin(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    course: Uuid,
    actor: Uuid,
    session: &str,
) -> Result<(Uuid, Uuid), sqlx::Error> {
    create_course(transaction, tenant, course, actor, session, true).await
}

async fn create_course(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    course: Uuid,
    actor: Uuid,
    session: &str,
    sysadmin: bool,
) -> Result<(Uuid, Uuid), sqlx::Error> {
    sqlx::query("SAVEPOINT course_creation_call")
        .execute(&mut **transaction)
        .await
        .expect("course creation savepoint");
    let statement = if sysadmin {
        "SELECT * FROM public.ple_create_course_as_sysadmin_v1($1,$2,$3,DATE '2026-08-24',DATE '2026-12-18','America/Chicago',$4,$5)"
    } else {
        "SELECT * FROM public.ple_create_course_as_instructor_v1($1,$2,$3,DATE '2026-08-24',DATE '2026-12-18','America/Chicago',$4,$5)"
    };
    let result = sqlx::query_as(statement)
        .bind(tenant)
        .bind(course)
        .bind("Authority course")
        .bind(actor)
        .bind(session)
        .fetch_one(&mut **transaction)
        .await;
    if result.is_err() {
        sqlx::query("ROLLBACK TO SAVEPOINT course_creation_call")
            .execute(&mut **transaction)
            .await
            .expect("course creation rejection rollback");
    }
    sqlx::query("RELEASE SAVEPOINT course_creation_call")
        .execute(&mut **transaction)
        .await
        .expect("course creation savepoint release");
    result
}

pub(super) async fn assert_complete_aggregate(
    pool: &PgPool,
    tenant: Uuid,
    course: Uuid,
    actor: Uuid,
    scheme_revision: i32,
) {
    let row = sqlx::query("SELECT (SELECT count(*) FROM course WHERE tenant_id=$1 AND course_id=$2), (SELECT count(*) FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND role='instructor' AND status='active'), (SELECT count(*) FROM course_roster_state WHERE tenant_id=$1 AND course_id=$2 AND revision=1 AND signup_posture='invitation_only'), (SELECT count(*) FROM course_group_membership_policy WHERE tenant_id=$1 AND course_id=$2 AND (purpose,multiple_membership,revision) IN (('section','warn',1),('lab','allow',1),('cohort','allow',1),('accommodation','allow',1),('work','allow',1))), (SELECT count(*) FROM course_grade_scheme WHERE tenant_id=$1 AND course_id=$2 AND mode='total_points' AND rounding='four_decimal_places_half_away_from_zero' AND revision=$4), (SELECT count(*) FROM course_appearance WHERE tenant_id=$1 AND course_id=$2 AND theme_id='grass' AND current_banner_delivery_id IS NULL AND banner_alt_kind IS NULL AND banner_alt_text IS NULL AND revision=1)")
        .bind(tenant).bind(course).bind(actor).bind(scheme_revision).fetch_one(pool).await.expect("aggregate query");
    assert_eq!(row.try_get::<i64, _>(0).expect("course count"), 1);
    assert_eq!(row.try_get::<i64, _>(1).expect("membership count"), 1);
    assert_eq!(row.try_get::<i64, _>(2).expect("roster count"), 1);
    assert_eq!(row.try_get::<i64, _>(3).expect("policy count"), 5);
    assert_eq!(row.try_get::<i64, _>(4).expect("scheme count"), 1);
    assert_eq!(row.try_get::<i64, _>(5).expect("appearance count"), 1);
}

pub(super) async fn assert_no_course(pool: &PgPool, tenant: Uuid, course: Uuid) {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.course WHERE tenant_id=$1 AND course_id=$2",
    )
    .bind(tenant)
    .bind(course)
    .fetch_one(pool)
    .await
    .expect("course absence query");
    assert_eq!(
        count, 0,
        "denied request leaves no partial course aggregate"
    );
}

pub(super) fn recipe(base: Uuid, genetics: Uuid) -> (BaseCoursePeople, Value) {
    let people = BaseCoursePeople {
        avery: id(),
        elena: id(),
        jack: id(),
        mary: id(),
        morgan: id(),
    };
    let recipe = json!({
        "schemaVersion": 1,
        "participants": {"avery": people.avery, "elena": people.elena, "jack": people.jack, "mary": people.mary, "morgan": people.morgan},
        "courses": {
            "baseCourse": {"id": base, "title": "Biochemistry Base Course", "termStart": "2026-01-01", "termEnd": "2099-12-31", "timeZone": "America/Chicago", "initialInstructor": people.elena},
            "geneticsPractice": {"id": genetics, "title": "Genetics Practice Course", "termStart": "2026-01-01", "termEnd": "2099-12-31", "timeZone": "America/Chicago", "initialInstructor": people.morgan}
        },
        "graph": {
            "workspace": id(), "problem": id(), "version": id(),
            "assignment": id(), "assignmentItem": id(), "maryRun": id(),
            "maryAttempt": id(), "jackRun": id(), "jackAttempt": id()
        },
    });
    (people, recipe)
}

pub(super) async fn acquire(installer: &mut Transaction<'_, Postgres>) {
    sqlx::query("SELECT public.ple_base_course_install_acquire_lock_v1()")
        .execute(&mut **installer)
        .await
        .expect("installer lock");
}

pub(super) async fn prepare(
    installer: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    recipe: Value,
) -> Result<(String, Uuid, String), sqlx::Error> {
    sqlx::query("SAVEPOINT base_course_prepare")
        .execute(&mut **installer)
        .await
        .expect("prepare savepoint");
    let result = sqlx::query_as("SELECT * FROM public.ple_base_course_install_prepare_v2($1,'base-course-v1','[]'::jsonb,$2)")
        .bind(tenant).bind(recipe).fetch_one(&mut **installer).await;
    if result.is_err() {
        sqlx::query("ROLLBACK TO SAVEPOINT base_course_prepare")
            .execute(&mut **installer)
            .await
            .expect("prepare rejection rollback");
    }
    sqlx::query("RELEASE SAVEPOINT base_course_prepare")
        .execute(&mut **installer)
        .await
        .expect("prepare savepoint release");
    result
}

pub(super) async fn installer_state(installer: &mut Transaction<'_, Postgres>) -> Option<String> {
    sqlx::query_scalar("SELECT state FROM public.ple_base_course_install_read_v2()")
        .fetch_optional(&mut **installer)
        .await
        .expect("installer lifecycle read capability")
}

pub(super) async fn assert_seed_refused(
    installer: &mut Transaction<'_, Postgres>,
    generation: Uuid,
    slot: &str,
) {
    let row = sqlx::query(
        "SELECT seed_outcome,course_id,instructor_membership_id,failure_kind \
         FROM public.ple_base_course_install_seed_course_v2($1,$2)",
    )
    .bind(generation)
    .bind(slot)
    .fetch_one(&mut **installer)
    .await
    .expect("typed course-prefix refusal");
    assert_eq!(row.try_get::<String, _>(0).unwrap(), "refused");
    assert!(row.try_get::<Option<Uuid>, _>(1).unwrap().is_none());
    assert!(row.try_get::<Option<Uuid>, _>(2).unwrap().is_none());
    assert_eq!(
        row.try_get::<String, _>(3).unwrap(),
        "course_aggregate_conflict"
    );
}

pub(super) async fn assert_seed_exact_prefix(
    installer: &mut Transaction<'_, Postgres>,
    generation: Uuid,
    slot: &str,
) {
    let row = sqlx::query(
        "SELECT seed_outcome,course_id,instructor_membership_id,failure_kind \
         FROM public.ple_base_course_install_seed_course_v2($1,$2)",
    )
    .bind(generation)
    .bind(slot)
    .fetch_one(&mut **installer)
    .await
    .expect("exact course-prefix witness");
    assert_eq!(row.try_get::<String, _>(0).unwrap(), "exact_prefix");
    assert!(row.try_get::<Uuid, _>(1).is_ok());
    assert!(row.try_get::<Uuid, _>(2).is_ok());
    assert!(row.try_get::<Option<String>, _>(3).unwrap().is_none());
}

pub(super) async fn insert_phase_assignment(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    assignment: Uuid,
    course: Uuid,
    title: &str,
) {
    sqlx::query("INSERT INTO public.assignment(tenant_id,assignment_id,course_id,title,audience_kind,score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure,solution_disclosure,class_statistics_disclosure) VALUES($1,$2,$3,$4,'course_wide','after_submit','after_submit','after_submit','after_submit','never')")
        .bind(tenant).bind(assignment).bind(course).bind(title)
        .execute(&mut **transaction).await.expect("phase assignment fixture");
}
