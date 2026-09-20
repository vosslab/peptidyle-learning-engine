//! Connected oracle for atomic immutable Blueprint Revision lifecycle.

use super::*;

#[path = "lifecycle_privacy.rs"]
mod privacy;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    seed(&admin).await;
    admin.close().await;

    let application_pool = lazy_pool(&application_url).expect("application fixture pool");
    let pool_ids = Arc::new(FixturePoolIdIssuer(AtomicUsize::new(0)));
    let owner_store = PostgresBlueprintCourseStore::new(application_pool.clone())
        .with_question_pool_id_issuer(pool_ids.clone());
    let create_input = content_input("Revision one Assessment");
    let created = owner_store
        .create_blueprint_course(
            token(),
            RequestChecksum::from_bytes([0x11; 32]),
            create_input.clone(),
            Default::default(),
        )
        .await
        .expect("owner creates a new Blueprint through the application Store");
    assert_eq!(
        created.blueprint_revision_tuple.revision_number,
        BlueprintRevisionNumber::INITIAL
    );
    let replay = owner_store
        .create_blueprint_course(
            token(),
            RequestChecksum::from_bytes([0x11; 32]),
            create_input,
            Default::default(),
        )
        .await
        .expect("create request replay");
    assert_eq!(
        replay.blueprint_revision_tuple, created.blueprint_revision_tuple,
        "create request replay returns its original Revision"
    );
    let blueprint_course_id = created.blueprint_revision_tuple.blueprint_course_id;
    let blueprint_course_id_sql = blueprint_course_id_text(&blueprint_course_id).await;
    let reader_store = PostgresBlueprintCourseStore::new(application_pool.clone());
    promotion_boundary(&owner_store, blueprint_course_id.clone()).await;
    // Regression: a refactor could disclose Private immutable content or let
    // an adoption hide its source. These owner/non-owner lifecycle rules are
    // deliberate product and authorization contracts, so this connected
    // acceptance oracle earns permanent coverage. Failure action: repair the
    // lifecycle/persistence predicate; do not loosen this contract.
    let owner_private = owner_store
        .load_blueprint_course(token(), blueprint_course_id.clone())
        .await
        .expect("owner reads a new Private Blueprint");
    assert_eq!(owner_private.availability, BlueprintAvailability::Private);
    let revision_one = owner_private.content.clone();
    blueprint_course_postgres_exchange::assert_actual_role_round_trip(
        &owner_store,
        blueprint_course_id.clone(),
        &owner_private,
    )
    .await;
    privacy::assert_private_blueprint_is_owner_only(
        &owner_store,
        &reader_store,
        &blueprint_course_id,
    )
    .await;
    let (availability, public_blueprint_edit_number) = transition_blueprint_availability(
        &application_url,
        &blueprint_course_id_sql,
        owner_private.blueprint_edit_number.as_i64(),
        "public",
        None,
    )
    .await
    .expect("owner publishes Private Blueprint");
    assert_eq!(availability, BlueprintAvailability::Public);
    assert_eq!(
        reader_store
            .load_blueprint_course(reader_token(), blueprint_course_id.clone())
            .await
            .expect("Public Blueprint is readable by another Instructor")
            .availability,
        BlueprintAvailability::Public
    );
    let instance_store = PostgresCourseInstanceStore::new(application_pool.clone())
        .with_question_pool_id_issuer(pool_ids.clone());
    let adoption_term = near_now_term(&application_url).await;
    let adopted = instance_store
        .create_course_instance(
            reader_token(),
            CreateCourseInstanceInput {
                classification: question_model::CourseClassification {
                    discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                    subject_uuid: None,
                    topic_uuid: None,
                    subtopic_uuid: None,
                    tags: Vec::new(),
                },
                source: CourseInstanceCreationSource::Adopted {
                    blueprint_course: blueprint_course_id.clone(),
                    blueprint_revision_number: BlueprintRevisionNumber::new(1).expect("Revision 1"),
                },
                short_name: "ADOPT".into(),
                long_name: "Complete Blueprint adoption".into(),
                term: adoption_term.clone(),
                assigned_instructor: None,
            },
            Default::default(),
        )
        .await
        .expect("adopt all Blueprint Assessments");
    let independently_adopted = instance_store
        .create_course_instance(
            reader_token(),
            CreateCourseInstanceInput {
                classification: question_model::CourseClassification {
                    discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                    subject_uuid: None,
                    topic_uuid: None,
                    subtopic_uuid: None,
                    tags: Vec::new(),
                },
                source: CourseInstanceCreationSource::Adopted {
                    blueprint_course: blueprint_course_id.clone(),
                    blueprint_revision_number: BlueprintRevisionNumber::new(1).expect("Revision 1"),
                },
                short_name: "ADOPT-2".into(),
                long_name: "Independent Blueprint adoption".into(),
                term: adoption_term.clone(),
                assigned_instructor: None,
            },
            Default::default(),
        )
        .await
        .expect("independently adopt the same Blueprint Revision");
    let public_to_private = transition_blueprint_availability(
        &application_url,
        &blueprint_course_id_sql,
        public_blueprint_edit_number,
        "private",
        None,
    )
    .await
    .expect_err("adopted Public Blueprint remains Public");
    assert_eq!(error_code(&public_to_private).as_deref(), Some("55000"));
    let owner_public = owner_store
        .load_blueprint_course(token(), blueprint_course_id.clone())
        .await
        .expect("owner reads adopted Public Blueprint");
    let public_save_checksum = request(0x19);
    assert_eq!(
        save(
            &application_url,
            &blueprint_course_id_sql,
            1,
            public_save_checksum.clone(),
            &revision_one
        )
        .await
        .expect("Public Blueprint accepts a no-op Save"),
        (1, false)
    );
    let archived = owner_store
        .archive_blueprint(
            token(),
            blueprint_course_id.clone(),
            owner_public.blueprint_edit_number,
            "Revision acceptance Blueprint",
        )
        .await
        .expect("owner archives Blueprint after adoption");
    assert_eq!(archived.availability, BlueprintAvailability::Archived);
    // Regression: Archived writes must not succeed through receipt replay or
    // no-op shortcuts. Failure action: repair the locked lifecycle boundary,
    // preserving readable history and Public writes after restore.
    let archived_write_state = blueprint_write_state(&blueprint_course_id_sql).await;
    let mut changed_archived_content = revision_one.clone();
    changed_archived_content.modules[0].label = "Denied Archived edit".to_owned();
    for (checksum, content) in [
        (public_save_checksum, &revision_one),
        (request(0x1a), &revision_one),
        (request(0x1b), &changed_archived_content),
    ] {
        let denied = save(
            &application_url,
            &blueprint_course_id_sql,
            1,
            checksum,
            content,
        )
        .await
        .expect_err("Archived Save is denied");
        assert_eq!(
            error_code(&denied).as_deref(),
            Some("55000"),
            "Archived Blueprint denies replay, no-op and changed Saves"
        );
    }
    for short_name in ["REV-ACC", "DENIED"] {
        assert!(
            matches!(
                owner_store
                    .rename_blueprint_course(
                        token(),
                        blueprint_course_id.clone(),
                        archived.blueprint_edit_number,
                        RenameBlueprintCourseInput {
                            short_name: short_name.to_owned(),
                            long_name: "Revision acceptance Blueprint".to_owned(),
                        },
                    )
                    .await,
                Err(StoreError::LifecycleConflict)
            ),
            "Archived Blueprint denies no-op and changed renames"
        );
    }
    assert_eq!(
        blueprint_write_state(&blueprint_course_id_sql).await,
        archived_write_state,
        "denied Archived writes leave metadata, content, Revisions, events and receipts unchanged"
    );
    assert_eq!(
        reader_store
            .load_blueprint_course(reader_token(), blueprint_course_id.clone())
            .await
            .expect("Archived Blueprint stays readable by another Instructor")
            .availability,
        BlueprintAvailability::Archived
    );
    assert!(
        reader_store
            .list_blueprint_courses(reader_token(), discovery(false))
            .await
            .expect("ordinary Public discovery after archive")
            .items
            .iter()
            .all(|summary| summary.id != blueprint_course_id),
        "Archived Blueprint leaves ordinary discovery"
    );
    assert!(
        owner_store
            .list_blueprint_courses(token(), discovery(false))
            .await
            .expect("owner normal discovery after archive")
            .items
            .iter()
            .all(|summary| summary.id != blueprint_course_id),
        "even an owner's Archived Blueprint leaves normal discovery"
    );
    for session in [token(), reader_token()] {
        assert!(
            reader_store
                .list_blueprint_courses(session, discovery(true))
                .await
                .expect("explicit Archived discovery")
                .items
                .iter()
                .any(|summary| summary.id == blueprint_course_id),
            "Instructors can explicitly include Archived Blueprint history"
        );
    }
    assert!(
        reader_store
            .load_blueprint_revision(
                reader_token(),
                question_model::BlueprintRevisionTuple {
                    blueprint_course_id: blueprint_course_id.clone(),
                    revision_number: BlueprintRevisionNumber::INITIAL,
                },
            )
            .await
            .is_ok(),
        "Archived Blueprint keeps exact Revision history readable"
    );
    // The persistent contract is that an Archived Blueprint cannot create a
    // Course. Its HTTP status belongs to the route layer, so do not freeze a
    // storage-error taxonomy here.
    assert!(
        instance_store
            .create_course_instance(
                reader_token(),
                CreateCourseInstanceInput {
                    classification: question_model::CourseClassification {
                        discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                        subject_uuid: None,
                        topic_uuid: None,
                        subtopic_uuid: None,
                        tags: Vec::new()
                    },
                    source: CourseInstanceCreationSource::Adopted {
                        blueprint_course: blueprint_course_id.clone(),
                        blueprint_revision_number: BlueprintRevisionNumber::INITIAL,
                    },
                    short_name: "ARCH".into(),
                    long_name: "Archived Blueprint adoption denial".into(),
                    term: adoption_term,
                    assigned_instructor: None,
                },
                Default::default(),
            )
            .await
            .is_err(),
        "Archived Blueprint adoption is denied"
    );
    let restored = owner_store
        .restore_blueprint(
            token(),
            blueprint_course_id.clone(),
            archived.blueprint_edit_number,
        )
        .await
        .expect("owner restores Archived Blueprint to Public");
    assert_eq!(restored.availability, BlueprintAvailability::Public);
    assert_eq!(
        save(
            &application_url,
            &blueprint_course_id_sql,
            1,
            request(0x1c),
            &revision_one
        )
        .await
        .expect("restored Public Blueprint accepts Save"),
        (1, false)
    );
    let renamed = owner_store
        .rename_blueprint_course(
            token(),
            blueprint_course_id.clone(),
            restored.blueprint_edit_number,
            RenameBlueprintCourseInput {
                short_name: "RESTORED".to_owned(),
                long_name: "Revision acceptance Blueprint".to_owned(),
            },
        )
        .await
        .expect("restored Public Blueprint accepts rename");
    assert_eq!(renamed.short_name, "RESTORED");
    let mut inspection = adoption_inspection_connection().await;
    let adopted_course_instance_id = adopted.course.id.as_string();
    let independently_adopted_course_instance_id = independently_adopted.course.id.as_string();
    blueprint_course_postgres_adoption::assert_adoption_projection(
        &mut inspection,
        &adopted_course_instance_id,
        &blueprint_course_id_sql,
        1,
        &independently_adopted_course_instance_id,
    )
    .await;
    let mut enrollment = inspection.begin().await.expect("enrollment fixture");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *enrollment)
        .await
        .expect("membership owner");
    let course_id = adopted.course.id.as_string();
    sqlx::query("INSERT INTO ple_data.student_record (student_record_id, course_instance_id, student_account_id, created_at) VALUES ($1,$2,$3,clock_timestamp())")
        .bind(id(0xb105)).bind(&course_id).bind(student_account_id()).execute(&mut *enrollment).await.expect("Student Record");
    for episode in [0xb106, 0xb107] {
        sqlx::query("INSERT INTO ple_data.course_membership (course_membership_id,course_instance_id,account_id,role,student_record_id,joined_at) VALUES ($1,$2,$3,'student',$4,clock_timestamp())")
            .bind(id(episode)).bind(&course_id).bind(student_account_id()).bind(id(0xb105)).execute(&mut *enrollment).await.expect("membership episode");
        sqlx::query("INSERT INTO ple_data.course_membership_event (course_membership_event_id,course_membership_id,event_kind,occurred_at,reason) VALUES ($1,$2,'ended',clock_timestamp(),'verification departure')")
            .bind(id(episode + 0x10)).bind(id(episode)).execute(&mut *enrollment).await.expect("ended membership");
    }
    enrollment
        .commit()
        .await
        .expect("enrollment fixture commit");
    inspection.close().await.expect("adoption inspection close");

    let reader_list = reader_store
        .list_blueprint_courses(reader_token(), discovery(false))
        .await
        .expect("non-owner Instructor Blueprint list");
    let reader_summary = reader_list
        .items
        .iter()
        .find(|summary| summary.id == blueprint_course_id)
        .expect("Available Blueprint is discoverable by a non-owner Instructor");
    assert_eq!(
        reader_summary.read_access,
        question_model::BlueprintCourseReadAccess::ActiveInstructor
    );
    assert_eq!(reader_summary.total_adoptions, 2);
    assert_eq!(reader_summary.total_students_ever_enrolled, 1);
    let reader_view = reader_store
        .load_blueprint_course(reader_token(), blueprint_course_id.clone())
        .await
        .expect("non-owner Instructor Blueprint load");
    assert_eq!(
        reader_view.read_access,
        question_model::BlueprintCourseReadAccess::ActiveInstructor
    );

    let concurrent_left = changed_content(revision_one.clone(), "Concurrent left");
    let concurrent_right = changed_content(revision_one.clone(), "Concurrent right");
    let (left, right) = tokio::join!(
        save(
            &application_url,
            &blueprint_course_id_sql,
            1,
            request(0x12),
            &concurrent_left
        ),
        save(
            &application_url,
            &blueprint_course_id_sql,
            1,
            request(0x13),
            &concurrent_right
        ),
    );
    let successes = [left.as_ref().ok(), right.as_ref().ok()];
    assert_eq!(
        successes.iter().filter(|result| result.is_some()).count(),
        1
    );
    let stale = [left.as_ref().err(), right.as_ref().err()]
        .into_iter()
        .flatten()
        .next()
        .expect("one concurrent Save loses");
    assert_eq!(error_code(stale).as_deref(), Some("40001"));
    let current_content = if left.is_ok() {
        concurrent_left
    } else {
        concurrent_right
    };

    let mut inspection = PgConnection::connect(migration_url)
        .await
        .expect("inspection connection");
    sqlx::query("SET ROLE ple_api_owner")
        .execute(&mut inspection)
        .await
        .expect("inspection role");
    let revision_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.blueprint_course_revision \
         WHERE blueprint_course_id = $1",
    )
    .bind(&blueprint_course_id_sql)
    .fetch_one(&mut inspection)
    .await
    .expect("Revision count");
    assert_eq!(
        revision_count, 2,
        "one concurrent changed Save creates one new Revision"
    );

    let no_op = save(
        &application_url,
        &blueprint_course_id_sql,
        2,
        request(0x14),
        &current_content,
    )
    .await
    .expect("canonical no-op Save");
    assert_eq!(no_op, (2, false));
    let no_op_replay = save(
        &application_url,
        &blueprint_course_id_sql,
        2,
        request(0x14),
        &current_content,
    )
    .await
    .expect("canonical no-op Save replay");
    assert_eq!(
        no_op_replay, no_op,
        "Save replay returns its original receipt"
    );

    let retained_assessment = current_content.modules[0].assessments[0].blueprint_assessment_id;
    let retained_module = current_content.modules[0].blueprint_module_id;
    let moved_input = ReplaceBlueprintCourseContentInput {
        modules: vec![
            BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_id: retained_module,
                },
                label: "Module alpha".to_owned(),
                assessments: vec![BlueprintAssessmentReplacementInput {
                    choice: BlueprintAssessmentEditChoice::New,
                    content: assessment_input("Replacement in Module alpha"),
                }],
            },
            BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::New,
                label: "Module beta".to_owned(),
                assessments: vec![BlueprintAssessmentReplacementInput {
                    choice: BlueprintAssessmentEditChoice::Retained {
                        blueprint_assessment_id: retained_assessment,
                    },
                    content: retained_assessment_input(
                        &current_content.modules[0].assessments[0].content,
                        "Moved retained Assessment",
                    ),
                }],
            },
        ],
    };
    let append_store = PostgresBlueprintCourseStore::new(application_pool.clone())
        .with_question_pool_id_issuer(pool_ids.clone());
    let moved_receipt = append_store
        .save_blueprint_course(
            token(),
            blueprint_course_id.clone(),
            BlueprintRevisionNumber::new(2).expect("Revision two"),
            question_model::RequestChecksum::from_bytes([0x15; 32]),
            moved_input,
            Default::default(),
        )
        .await
        .expect("Save moving retained Assessment and materializing a new daughter Assessment");
    assert_eq!(
        moved_receipt.blueprint_revision_tuple.revision_number,
        BlueprintRevisionNumber::new(3).expect("Revision three")
    );
    let moved = (3, moved_receipt.changed);
    let moved_content = append_store
        .load_blueprint_revision(token(), moved_receipt.blueprint_revision_tuple)
        .await
        .expect("sealed moved content")
        .content;
    assert_eq!(moved, (3, true));
    let moved_replay = save(
        &application_url,
        &blueprint_course_id_sql,
        2,
        request(0x15),
        &moved_content,
    )
    .await
    .expect("changed Save replay");
    assert_eq!(
        moved_replay, moved,
        "changed Save replay returns its original receipt"
    );
    let module_rows: Vec<Uuid> = sqlx::query_scalar(
        "SELECT blueprint_module_id FROM ple_data.blueprint_revision_assessment \
         WHERE blueprint_course_id = $1 \
           AND blueprint_assessment_id = $2 \
           AND blueprint_revision_number IN (2, 3) ORDER BY blueprint_revision_number",
    )
    .bind(&blueprint_course_id_sql)
    .bind(retained_assessment.as_uuid())
    .fetch_all(&mut inspection)
    .await
    .expect("retained Assessment lineage");
    assert_eq!(module_rows.len(), 2);
    assert_ne!(
        module_rows[0], module_rows[1],
        "retained Assessment moved Modules"
    );

    let sealed_insert: &'static str = "INSERT INTO ple_data.blueprint_revision_module \
        (blueprint_course_id, blueprint_revision_number, blueprint_module_id, module_position) \
        VALUES ($1, $2, '00000000-0000-0000-0000-00000000b122', 9)";
    let sealed_update = "UPDATE ple_data.blueprint_revision_assessment \
        SET assessment_position = assessment_position + 10 \
        WHERE blueprint_course_id = $1 AND blueprint_revision_number = $2";
    let sealed_delete = "DELETE FROM ple_data.blueprint_revision_question_pin \
        WHERE blueprint_course_id = $1 AND blueprint_revision_number = $2";
    assert_immutable_child(&mut inspection, sealed_insert, &blueprint_course_id_sql, 3).await;
    assert_immutable_child(&mut inspection, sealed_update, &blueprint_course_id_sql, 3).await;
    assert_immutable_child(&mut inspection, sealed_delete, &blueprint_course_id_sql, 3).await;

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
             'CI0000000' || ple_private.crockford_checksum_character('CI0000000'), \
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
    let _creator_pid = creator_pid_receiver.await.expect("creator PID");
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
    assert_eq!(error_code(&stale_course).as_deref(), Some("40001"));
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
    blueprint_course_postgres_append::assert_new_assessment_save_preserves_daughter_work().await;

    assert_revision_checksum_mismatch(
        migration_url,
        &application_url,
        &blueprint_course_id_sql,
        blueprint_course_id,
    )
    .await;
}
