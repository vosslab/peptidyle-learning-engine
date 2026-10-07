use super::*;

pub(super) async fn assert_head_lock_race_and_revision_seal(
    application_pool: &sqlx::Pool<sqlx::Postgres>,
    inspection: PgConnection,
    migration_url: &str,
    application_url: String,
    blueprint_course_id_sql: String,
    moved_content: StoredBlueprintCourseContent,
) {
    // These store operations are finished. Close their shared fixture pool
    // before the two direct application connections required by the head race;
    // retaining unrelated idle pools must not consume the login's real limit.
    application_pool.close().await;

    // Hold the old head exclusively, let Course Instance creation block behind
    // it, then advance the head. The blocked creator must recheck and reject
    // the superseded Revision rather than committing stale provenance.
    let revision_three_content = moved_content;
    let revision_four_content =
        changed_content(revision_three_content, "Head advances while creating");
    inspection
        .close()
        .await
        .expect("release inspection before fixture holder");
    let mut holder_connection = PgConnection::connect(migration_url)
        .await
        .expect("fixture holder connection");
    let mut holder = holder_connection
        .begin()
        .await
        .expect("fixture holder transaction");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *holder)
        .await
        .expect("fixture holder API role");
    let holder_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *holder)
        .await
        .expect("fixture holder backend PID");
    sqlx::query(
        "SELECT 1 FROM ple_data.blueprint_course WHERE blueprint_course_id = $1 FOR UPDATE",
    )
    .bind(&blueprint_course_id_sql)
    .execute(&mut *holder)
    .await
    .expect("hold Blueprint head");
    let (save_pid_sender, save_pid_receiver) = oneshot::channel();
    let save_url = application_url.clone();
    let save_content = revision_four_content.clone();
    let save_blueprint_course_id =
        blueprint_course_id_text_from_str(&blueprint_course_id_sql).await;
    let saver = tokio::spawn(async move {
        let mut connection = PgConnection::connect(&save_url)
            .await
            .expect("Save application connection");
        let mut transaction = connection.begin().await.expect("Save transaction");
        authenticate_application_transaction(&mut transaction).await;
        let save_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *transaction)
            .await
            .expect("Save backend PID");
        save_pid_sender.send(save_pid).expect("Save PID receiver");
        let row = sqlx::query(
            "SELECT resulting_blueprint_revision_number, changed \
            FROM ple_api.save_blueprint_course($1, 3, $2, $3, $4, \
                (SELECT COALESCE(jsonb_agg(jsonb_build_object( \
                    'course_instance_id', course_instance_id, 'assessments', '[]'::jsonb)), \
                    '[]'::jsonb) \
                   FROM ple_api.list_blueprint_daughter_course_ids($1)))",
        )
        .bind(save_blueprint_course_id)
        .bind(request(0x16))
        .bind(serde_json::to_value(&save_content).expect("content JSON"))
        .bind(
            save_content
                .checksum()
                .expect("content checksum")
                .as_bytes()
                .to_vec(),
        )
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok::<_, sqlx::Error>(row)
    });
    let save_pid = save_pid_receiver.await.expect("Save PID");
    let mut save_blocked = false;
    for _ in 0..100 {
        save_blocked = sqlx::query_scalar::<_, bool>("SELECT $1 = ANY(pg_blocking_pids($2))")
            .bind(holder_pid)
            .bind(save_pid)
            .fetch_one(&mut *holder)
            .await
            .expect("Save lock inspection");
        if save_blocked {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        save_blocked,
        "ordinary Save waits behind the fixture head lock"
    );
    let (creator_pid_sender, creator_pid_receiver) = oneshot::channel();
    let create_url = application_url.clone();
    let create_blueprint_course_id =
        blueprint_course_id_text_from_str(&blueprint_course_id_sql).await;
    let creator = tokio::spawn(async move {
        let mut connection = PgConnection::connect(&create_url)
            .await
            .expect("creator application connection");
        let mut transaction = connection.begin().await.expect("creator transaction");
        authenticate_application_transaction(&mut transaction).await;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *transaction)
            .await
            .expect("creator backend PID");
        creator_pid_sender.send(pid).expect("creator PID receiver");
        sqlx::query(
            "SELECT course_instance_id FROM ple_api.create_course_instance(\
             'CI0000000Y', \
             '00000000-0000-0000-0000-00000000b131', \
             '00000000-0000-0000-0000-00000000b132', \
             '00000000-0000-0000-0000-00000000b133', \
             'adopted', $1, 3, 'RACE-C', 'Concurrent head Course', \
             current_date, current_date + 1, NULL, '[]'::jsonb, '00000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[])",
        )
        .bind(create_blueprint_course_id)
        .fetch_one(&mut *transaction)
        .await
    });
    let creator_pid = creator_pid_receiver.await.expect("creator PID");
    let mut creator_blocked = false;
    for _ in 0..100 {
        creator_blocked = sqlx::query_scalar::<_, bool>("SELECT $1 = ANY(pg_blocking_pids($2))")
            .bind(save_pid)
            .bind(creator_pid)
            .fetch_one(&mut *holder)
            .await
            .expect("Course creation queue inspection");
        if creator_blocked {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if !creator_blocked {
        if creator.is_finished() {
            match creator.await {
                Ok(Ok(_)) => panic!(
                    "Course creation unexpectedly completed before the queued Blueprint Save"
                ),
                Ok(Err(error)) => panic!(
                    "Course creation completed before the queued Blueprint Save with SQLSTATE {:?}: {error}",
                    error_code(&error)
                ),
                Err(error) => {
                    panic!("Course creation task ended before the queued Blueprint Save: {error}")
                }
            }
        }
        panic!(
            "Course creation PID {creator_pid} remained active without queuing behind Save PID {save_pid}"
        );
    }
    holder.commit().await.expect("release fixture head lock");
    let saved = saver
        .await
        .expect("Save task completion")
        .expect("ordinary Save succeeds");
    assert_eq!(
        saved
            .try_get::<i64, _>("resulting_blueprint_revision_number")
            .expect("saved Revision"),
        4
    );
    let stale_course = creator
        .await
        .expect("creator task completion")
        .expect_err("stale Course Instance is rejected");
    assert_eq!(
        error_code(&stale_course).as_deref(),
        Some("40001"),
        "stale Course Instance rejection: {stale_course}"
    );
    holder_connection
        .close()
        .await
        .expect("release fixture holder connection");

    // A Revision is built as header, children, then receipt event in one
    // transaction. A second writer cannot append a child while that aggregate
    // is uncommitted, and cannot append it after the receipt seals it.
    let mut construction_connection = PgConnection::connect(migration_url)
        .await
        .expect("construction connection");
    let mut construction = construction_connection
        .begin()
        .await
        .expect("construction transaction");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *construction)
        .await
        .expect("construction role");
    let revision_five_content = revision_four_content;
    let revision_five_json = serde_json::to_value(&revision_five_content).expect("content JSON");
    let revision_five_checksum = revision_five_content
        .checksum()
        .expect("content checksum")
        .as_bytes()
        .to_vec();
    sqlx::query(
        "INSERT INTO ple_data.blueprint_course_revision \
         (blueprint_course_id, blueprint_revision_number, content, content_checksum, saved_at) \
         VALUES ($1, 5, $2, $3, clock_timestamp())",
    )
    .bind(&blueprint_course_id_sql)
    .bind(&revision_five_json)
    .bind(&revision_five_checksum)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Revision header");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_question_pin \
         SELECT $1, 5, pins.content_path, pins.published_question_id, pins.question_revision_number \
           FROM ple_data.blueprint_content_question_pins($2) AS pins",
    )
    .bind(&blueprint_course_id_sql)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Question pins");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_module \
         SELECT $1, 5, modules.blueprint_module_id, modules.module_position \
           FROM ple_data.blueprint_content_modules($2) AS modules",
    )
    .bind(&blueprint_course_id_sql)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Modules");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_assessment \
         SELECT $1, 5, assessments.blueprint_module_id, \
                assessments.blueprint_assessment_id, assessments.assessment_position \
           FROM ple_data.blueprint_content_assessments($2) AS assessments",
    )
    .bind(&blueprint_course_id_sql)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Assessments");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_id, blueprint_revision_number, actor_account_id, request_checksum, occurred_at) \
         VALUES ($1, 5, $2, $3, clock_timestamp())",
    )
    .bind(&blueprint_course_id_sql)
    .bind(instructor_account_id())
    .bind(request(0x17))
    .execute(&mut *construction)
    .await
    .expect("uncommitted Revision receipt");

    let competing_url = migration_url.to_owned();
    let competing_blueprint_course_id = blueprint_course_id_sql.clone();
    let mut competing_child = tokio::spawn(async move {
        let mut connection = PgConnection::connect(&competing_url)
            .await
            .expect("competing connection");
        sqlx::query("SET ROLE ple_api_owner")
            .execute(&mut connection)
            .await
            .expect("competing role");
        sqlx::query(
            "INSERT INTO ple_data.blueprint_revision_module \
             (blueprint_course_id, blueprint_revision_number, blueprint_module_id, module_position) \
             VALUES ($1, 5, '00000000-0000-0000-0000-00000000b140', 99)",
        )
        .bind(&competing_blueprint_course_id)
        .execute(&mut connection)
        .await
    });
    let immediate = timeout(Duration::from_millis(250), &mut competing_child).await;
    construction
        .commit()
        .await
        .expect("complete constructed Revision");
    let construction_race_error = match immediate {
        Ok(result) => result
            .expect("competing child task completion")
            .expect_err("invisible uncommitted Revision rejects a child append"),
        Err(_) => competing_child
            .await
            .expect("competing child task completion after seal")
            .expect_err("sealed Revision rejects a waiting child append"),
    };
    assert!(
        matches!(
            error_code(&construction_race_error).as_deref(),
            Some("23503") | Some("55000")
        ),
        "concurrent child append is rejected before construction visibility or after sealing"
    );
    construction_connection
        .close()
        .await
        .expect("release construction fixture connection");
    let mut final_inspection = PgConnection::connect(migration_url)
        .await
        .expect("final inspection connection");
    sqlx::query("SET ROLE ple_api_owner")
        .execute(&mut final_inspection)
        .await
        .expect("final inspection role");
    let competing_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.blueprint_revision_module \
         WHERE blueprint_course_id = $1 AND blueprint_revision_number = 5 \
           AND blueprint_module_id = '00000000-0000-0000-0000-00000000b140'",
    )
    .bind(&blueprint_course_id_sql)
    .fetch_one(&mut final_inspection)
    .await
    .expect("competing child inspection");
    assert_eq!(
        competing_rows, 0,
        "rejected child did not enter the sealed Revision"
    );
    final_inspection
        .close()
        .await
        .expect("release final inspection before append fixture");
}
