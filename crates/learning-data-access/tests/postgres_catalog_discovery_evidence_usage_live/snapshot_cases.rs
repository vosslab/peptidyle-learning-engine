use sqlx::{PgPool, Postgres, Row, Transaction};

use super::Fixture;

async fn titles(
    pool: &PgPool,
    fixture: &Fixture,
    session: &str,
    token: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(session)
        .execute(&mut *transaction)
        .await?;
    let values = sqlx::query_scalar(
        "SELECT course_title FROM ple_instructor_catalog_usage_snapshot_rows($1,$2,$3)",
    )
    .bind(fixture.tenant)
    .bind(session)
    .bind(token)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(values)
}

async fn assert_retained_course_hidden(pool: &PgPool, fixture: &Fixture, token: &str) {
    sqlx::query(
        "INSERT INTO course_retention \
         (tenant_id,course_id,ended_at,notify_days,archive_days,delete_days, \
          assignment_disposition,generation,lifecycle) \
         VALUES ($1,$2,transaction_timestamp(),1,2,3,'retain',1,'archived')",
    )
    .bind(fixture.tenant)
    .bind(fixture.actor_course)
    .execute(pool)
    .await
    .expect("make actor course records retention-inaccessible");

    let mut app = pool.begin().await.expect("begin retention visibility read");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *app)
        .await
        .expect("assume app");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *app)
        .await
        .expect("set tenant");
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.actor_session)
        .execute(&mut *app)
        .await
        .expect("set session");
    let named_courses: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_instructor_catalog_course_usage($1,$2,$3,NULL,100)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.actor_session)
    .bind(fixture.question_id)
    .fetch_one(&mut *app)
    .await
    .expect("read retention-filtered named course usage");
    let summary = sqlx::query(
        "SELECT own_course_count,own_assignment_count \
         FROM ple_instructor_catalog_usage_summary($1,$2,$3)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.actor_session)
    .bind(fixture.question_id)
    .fetch_one(&mut *app)
    .await
    .expect("read retention-filtered own usage counts");
    assert_eq!(named_courses, 0);
    assert_eq!(summary.get::<i64, _>("own_course_count"), 0);
    assert_eq!(summary.get::<i64, _>("own_assignment_count"), 0);
    app.commit()
        .await
        .expect("commit retention visibility read");
    assert!(
        titles(pool, fixture, &fixture.actor_session, token)
            .await
            .expect("valid snapshot rechecks current retention")
            .is_empty()
    );
}

fn assert_code(error: &sqlx::Error, code: &str) {
    assert_eq!(
        error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some(code)
    );
}

pub(super) async fn run(pool: &PgPool, fixture: &Fixture, mut usage: Transaction<'_, Postgres>) {
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.sysadmin_session)
        .execute(&mut *usage)
        .await
        .expect("present Morgan Sysadmin session");
    let sysadmin_snapshot = sqlx::query(
        "SELECT snapshot_token,row_count \
         FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,5000)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.sysadmin_session)
    .fetch_one(&mut *usage)
    .await
    .expect("reuse Morgan's valid empty usage snapshot");
    let sysadmin_token: String = sysadmin_snapshot.get("snapshot_token");
    assert_eq!(sysadmin_snapshot.get::<i32, _>("row_count"), 0);
    let sysadmin_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_instructor_catalog_usage_snapshot_rows($1,$2,$3)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.sysadmin_session)
    .bind(&sysadmin_token)
    .fetch_one(&mut *usage)
    .await
    .expect("Morgan's zero-membership snapshot remains a valid empty cursor");
    assert_eq!(sysadmin_rows, 0);
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.actor_session)
        .execute(&mut *usage)
        .await
        .expect("restore Instructor session for named-course snapshot");
    let snapshot = sqlx::query(
        "SELECT snapshot_token,row_count FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,5000)")
        .bind(fixture.tenant).bind(&fixture.actor_session).fetch_one(&mut *usage).await
        .expect("begin bounded actor usage snapshot");
    let token: String = snapshot.get("snapshot_token");
    assert_eq!(snapshot.get::<i32, _>("row_count"), 2);
    let reused: String = sqlx::query_scalar(
        "SELECT snapshot_token FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,5000)",
    )
    .bind(fixture.tenant)
    .bind(&fixture.actor_session)
    .fetch_one(&mut *usage)
    .await
    .expect("reuse unchanged snapshot digest");
    assert_eq!(token, reused);
    usage.commit().await.expect("commit usage snapshot");

    sqlx::query("UPDATE course SET title='Mutated after snapshot' WHERE course_id=$1")
        .bind(fixture.actor_course)
        .execute(pool)
        .await
        .expect("mutate current course");
    assert!(
        titles(pool, fixture, &fixture.actor_session, &token)
            .await
            .expect("read frozen snapshot")
            .contains(&"Actor-owned D1 course".to_string())
    );
    let error = titles(pool, fixture, &fixture.foreign_session, &token)
        .await
        .expect_err("foreign same-tenant session cannot read another actor snapshot");
    assert_code(&error, "22023");
    let error = titles(pool, fixture, &fixture.actor_session, &"f".repeat(64))
        .await
        .expect_err("reject tampered token");
    assert_code(&error, "22023");

    let mut bounded = pool.begin().await.expect("begin bound refusal");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *bounded)
        .await
        .expect("assume app");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *bounded)
        .await
        .expect("set tenant");
    sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
        .bind(&fixture.actor_session)
        .execute(&mut *bounded)
        .await
        .expect("set session");
    let error =
        sqlx::query("SELECT * FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,1)")
            .bind(fixture.tenant)
            .bind(&fixture.actor_session)
            .fetch_all(&mut *bounded)
            .await
            .expect_err("refuse over-bound snapshot");
    assert_code(&error, "54000");
    drop(bounded);

    let mut newest = token.clone();
    let mut changed_tokens = Vec::new();
    for state in 0..8 {
        sqlx::query("UPDATE course SET title=$1 WHERE course_id=$2")
            .bind(format!("Snapshot state {state}"))
            .bind(fixture.actor_course)
            .execute(pool)
            .await
            .expect("mutate snapshot digest input");
        let mut begin = pool.begin().await.expect("begin capped snapshot");
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *begin)
            .await
            .expect("assume app");
        sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
            .bind(fixture.tenant.to_string())
            .execute(&mut *begin)
            .await
            .expect("set tenant");
        sqlx::query("SELECT set_config('ple.session_hash',$1,true)")
            .bind(&fixture.actor_session)
            .execute(&mut *begin)
            .await
            .expect("set session");
        newest = sqlx::query_scalar(
            "SELECT snapshot_token FROM ple_begin_instructor_catalog_usage_snapshot($1,$2,300,5000)")
            .bind(fixture.tenant).bind(&fixture.actor_session).fetch_one(&mut *begin).await
            .expect("create changed snapshot");
        changed_tokens.push(newest.clone());
        begin.commit().await.expect("commit changed snapshot");
    }
    let error = titles(pool, fixture, &fixture.actor_session, &token)
        .await
        .expect_err("oldest snapshot evicted");
    assert_code(&error, "22023");

    sqlx::query("UPDATE catalog_usage_snapshot SET expires_at=created_at+interval '1 microsecond' WHERE snapshot_token=$1")
        .bind(&newest).execute(pool).await.expect("expire snapshot");
    let error = titles(pool, fixture, &fixture.actor_session, &newest)
        .await
        .expect_err("expired snapshot invalid");
    assert_code(&error, "22023");
    let membership_token = &changed_tokens[changed_tokens.len() - 2];
    assert_retained_course_hidden(pool, fixture, membership_token).await;
    sqlx::query(
        "UPDATE course_member SET status='revoked',revoked_at=transaction_timestamp() \
         WHERE tenant_id=$1 AND user_id=$2",
    )
    .bind(fixture.tenant)
    .bind(fixture.actor)
    .execute(pool)
    .await
    .expect("revoke membership");
    let error = titles(pool, fixture, &fixture.actor_session, membership_token)
        .await
        .expect_err("membership loss invalidates snapshot");
    assert_code(&error, "22023");
}
