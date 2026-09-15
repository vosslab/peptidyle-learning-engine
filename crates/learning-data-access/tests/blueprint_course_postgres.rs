#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for immutable Blueprint Revision persistence.

use std::collections::BTreeMap;

use learning_data_access::postgres::{
    PostgresBlueprintCourseStore, PostgresCourseInstanceStore, lazy_pool,
};
use learning_data_access::{
    BlueprintCourseStore, CourseInstanceCreationSource, CourseInstanceStore,
    CreateCourseInstanceInput, SessionTokenHash, StoreError, StoredBlueprintCourseContent,
};
use question_model::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, BlueprintAssessmentContentInput, BlueprintAssessmentDefaults,
    BlueprintAssessmentEditChoice, BlueprintAssessmentEntryInput,
    BlueprintAssessmentReplacementInput, BlueprintAvailability, BlueprintCourseReference,
    BlueprintModuleEditChoice, BlueprintModuleReplacementInput, BlueprintRevision,
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, LateWorkRule, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionId, QuestionRevisionNumber, QuestionRevisionReference,
    ReplaceBlueprintCourseContentInput, ReusableFixedQuestionInput, StudentFeedbackReleaseRule,
};
use sqlx::{Connection, PgConnection, Row};
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

#[path = "blueprint_course_postgres/support.rs"]
mod blueprint_course_postgres_support;
use blueprint_course_postgres_support::*;
#[path = "blueprint_course_postgres/adoption.rs"]
mod blueprint_course_postgres_adoption;

async fn authenticate_application_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) {
    sqlx::query("SET LOCAL ROLE ple_auth")
        .execute(&mut **transaction)
        .await
        .expect("authentication role");
    let installed: Option<Uuid> = sqlx::query_scalar(
        "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
    )
    .bind(token().to_string())
    .fetch_optional(&mut **transaction)
    .await
    .expect("session resolution");
    assert_eq!(installed, Some(id(SESSION)), "fixture session installs");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut **transaction)
        .await
        .expect("application role");
}

async fn create(
    url: &str,
    blueprint_id: Uuid,
    checksum: Vec<u8>,
    content: &StoredBlueprintCourseContent,
) -> (i64, i64) {
    let mut connection = PgConnection::connect(url)
        .await
        .expect("application connection");
    let mut transaction = connection.begin().await.expect("application transaction");
    authenticate_application_transaction(&mut transaction).await;
    let row = sqlx::query(
        "SELECT reference_number, blueprint_revision_number \
         FROM ple_api.create_blueprint_course($1, $2, 'REV-ACC', \
              'Revision acceptance Blueprint', $3, $4)",
    )
    .bind(blueprint_id)
    .bind(checksum)
    .bind(serde_json::to_value(content).expect("content JSON"))
    .bind(
        content
            .checksum()
            .expect("content checksum")
            .as_bytes()
            .to_vec(),
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("Blueprint creation");
    transaction
        .commit()
        .await
        .expect("Blueprint creation commit");
    (
        row.try_get("reference_number")
            .expect("Blueprint reference"),
        row.try_get("blueprint_revision_number")
            .expect("Blueprint Revision"),
    )
}

async fn save(
    url: &str,
    reference: i64,
    expected_revision: i64,
    checksum: Vec<u8>,
    content: &StoredBlueprintCourseContent,
) -> Result<(i64, bool), sqlx::Error> {
    let mut connection = PgConnection::connect(url)
        .await
        .expect("application connection");
    let mut transaction = connection.begin().await.expect("application transaction");
    authenticate_application_transaction(&mut transaction).await;
    let result = sqlx::query(
        "SELECT resulting_blueprint_revision_number, changed \
         FROM ple_api.save_blueprint_course($1, $2, $3, $4, $5)",
    )
    .bind(reference)
    .bind(expected_revision)
    .bind(checksum)
    .bind(serde_json::to_value(content).expect("content JSON"))
    .bind(
        content
            .checksum()
            .expect("content checksum")
            .as_bytes()
            .to_vec(),
    )
    .fetch_one(&mut *transaction)
    .await
    .map(|row| {
        (
            row.try_get("resulting_blueprint_revision_number")
                .expect("result Revision"),
            row.try_get("changed").expect("change flag"),
        )
    });
    match result {
        Ok(value) => {
            transaction.commit().await.expect("Blueprint Save commit");
            Ok(value)
        }
        Err(error) => Err(error),
    }
}

async fn transition_blueprint_availability(
    url: &str,
    reference: i64,
    expected_metadata_etag: Uuid,
    availability: &'static str,
    archive_confirmation_long_name: Option<&str>,
) -> Result<(BlueprintAvailability, Uuid), sqlx::Error> {
    let mut connection = PgConnection::connect(url)
        .await
        .expect("Blueprint lifecycle application connection");
    let mut transaction = connection
        .begin()
        .await
        .expect("Blueprint lifecycle application transaction");
    authenticate_application_transaction(&mut transaction).await;
    let result = sqlx::query(
        "SELECT availability, metadata_etag FROM ple_api.set_blueprint_availability($1, $2, $3, $4)",
    )
    .bind(reference)
    .bind(expected_metadata_etag)
    .bind(availability)
    .bind(archive_confirmation_long_name)
    .fetch_one(&mut *transaction)
    .await
    .map(|row| {
        let availability = match row
            .try_get::<String, _>("availability")
            .expect("Blueprint lifecycle availability")
            .as_str()
        {
            "private" => BlueprintAvailability::Private,
            "public" => BlueprintAvailability::Public,
            "archived" => BlueprintAvailability::Archived,
            value => panic!("unexpected Blueprint lifecycle availability: {value}"),
        };
        (
            availability,
            row.try_get("metadata_etag")
                .expect("Blueprint lifecycle metadata ETag"),
        )
    });
    match result {
        Ok(value) => {
            transaction
                .commit()
                .await
                .expect("Blueprint lifecycle transition commit");
            Ok(value)
        }
        Err(error) => Err(error),
    }
}

async fn near_now_term(url: &str) -> question_model::CourseTerm {
    let mut connection = PgConnection::connect(url)
        .await
        .expect("term-clock connection");
    let (starts_on, ends_on): (String, String) =
        sqlx::query_as("SELECT current_date::text, (current_date + 1)::text")
            .fetch_one(&mut connection)
            .await
            .expect("database current term dates");
    question_model::CourseTerm::from_parts(&starts_on, &ends_on).expect("near-now Course term")
}

fn error_code(error: &sqlx::Error) -> Option<String> {
    match error {
        sqlx::Error::Database(database) => database.code().map(|code| code.into_owned()),
        _ => None,
    }
}

async fn assert_immutable_child(
    connection: &mut PgConnection,
    sql: &'static str,
    reference: i64,
    revision: i64,
) {
    let error = sqlx::query(sql)
        .bind(reference)
        .bind(revision)
        .execute(&mut *connection)
        .await
        .expect_err("sealed Revision child mutation must fail");
    assert_eq!(error_code(&error).as_deref(), Some("55000"));
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL 17 acceptance runtime"]
async fn revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    seed(&admin).await;
    admin.close().await;

    let revision_one = initial_content();
    let create_checksum = request(0x11);
    let (reference, revision) = create(
        &application_url,
        id(0xb110),
        create_checksum.clone(),
        &revision_one,
    )
    .await;
    assert_eq!(revision, 1);
    let replay = create(&application_url, id(0xb111), create_checksum, &revision_one).await;
    assert_eq!(
        replay,
        (reference, 1),
        "create request replay returns its original Revision"
    );

    let blueprint_reference = format!("BP-{reference}")
        .parse::<BlueprintCourseReference>()
        .expect("Blueprint reference");
    let owner_store =
        PostgresBlueprintCourseStore::new(lazy_pool(&application_url).expect("owner pool"));
    let reader_store =
        PostgresBlueprintCourseStore::new(lazy_pool(&application_url).expect("reader pool"));
    // Regression: a refactor could disclose Private immutable content or let
    // an adoption hide its source. These owner/non-owner lifecycle rules are
    // deliberate product and authorization contracts, so this connected
    // acceptance oracle earns permanent coverage. Failure action: repair the
    // lifecycle/persistence predicate; do not loosen this contract.
    let owner_private = owner_store
        .load_blueprint_course(token(), blueprint_reference)
        .await
        .expect("owner reads a new Private Blueprint");
    assert_eq!(owner_private.availability, BlueprintAvailability::Private);
    assert!(
        reader_store
            .list_blueprint_courses(reader_token())
            .await
            .expect("non-owner Private Blueprint list")
            .iter()
            .all(|summary| summary.reference != blueprint_reference),
        "Private Blueprint is absent from non-owner discovery"
    );
    assert!(matches!(
        reader_store
            .load_blueprint_course(reader_token(), blueprint_reference)
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        reader_store
            .load_blueprint_revision(
                reader_token(),
                question_model::BlueprintRevisionReference {
                    reference: blueprint_reference,
                    revision: BlueprintRevision::INITIAL,
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
    let (availability, public_metadata_etag) = transition_blueprint_availability(
        &application_url,
        reference,
        owner_private.metadata_etag.into_uuid(),
        "public",
        None,
    )
    .await
    .expect("owner publishes Private Blueprint");
    assert_eq!(availability, BlueprintAvailability::Public);
    assert_eq!(
        reader_store
            .load_blueprint_course(reader_token(), blueprint_reference)
            .await
            .expect("Public Blueprint is readable by another Instructor")
            .availability,
        BlueprintAvailability::Public
    );
    let instance_store =
        PostgresCourseInstanceStore::new(lazy_pool(&application_url).expect("adoption pool"));
    let adoption_term = near_now_term(&application_url).await;
    let adopted = instance_store
        .create_course_instance(
            reader_token(),
            CreateCourseInstanceInput {
                source: CourseInstanceCreationSource::Adopted {
                    blueprint_course: blueprint_reference,
                    blueprint_revision: BlueprintRevision::new(1).expect("Revision 1"),
                },
                short_name: "ADOPT".into(),
                long_name: "Complete Blueprint adoption".into(),
                term: adoption_term.clone(),
                assigned_instructor: None,
            },
        )
        .await
        .expect("adopt all Blueprint Assessments");
    let independently_adopted = instance_store
        .create_course_instance(
            reader_token(),
            CreateCourseInstanceInput {
                source: CourseInstanceCreationSource::Adopted {
                    blueprint_course: blueprint_reference,
                    blueprint_revision: BlueprintRevision::new(1).expect("Revision 1"),
                },
                short_name: "ADOPT-2".into(),
                long_name: "Independent Blueprint adoption".into(),
                term: adoption_term.clone(),
                assigned_instructor: None,
            },
        )
        .await
        .expect("independently adopt the same Blueprint Revision");
    let public_to_private = transition_blueprint_availability(
        &application_url,
        reference,
        public_metadata_etag,
        "private",
        None,
    )
    .await
    .expect_err("adopted Public Blueprint remains Public");
    assert_eq!(error_code(&public_to_private).as_deref(), Some("55000"));
    let owner_public = owner_store
        .load_blueprint_course(token(), blueprint_reference)
        .await
        .expect("owner reads adopted Public Blueprint");
    let archived = owner_store
        .archive_blueprint(
            token(),
            blueprint_reference,
            owner_public.metadata_etag,
            "Revision acceptance Blueprint",
        )
        .await
        .expect("owner archives Blueprint after adoption");
    assert_eq!(archived.availability, BlueprintAvailability::Archived);
    assert_eq!(
        reader_store
            .load_blueprint_course(reader_token(), blueprint_reference)
            .await
            .expect("Archived Blueprint stays readable by another Instructor")
            .availability,
        BlueprintAvailability::Archived
    );
    assert!(
        reader_store
            .list_blueprint_courses(reader_token())
            .await
            .expect("ordinary Public discovery after archive")
            .iter()
            .all(|summary| summary.reference != blueprint_reference),
        "Archived Blueprint leaves ordinary discovery"
    );
    assert!(
        reader_store
            .load_blueprint_revision(
                reader_token(),
                question_model::BlueprintRevisionReference {
                    reference: blueprint_reference,
                    revision: BlueprintRevision::INITIAL,
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
                    source: CourseInstanceCreationSource::Adopted {
                        blueprint_course: blueprint_reference,
                        blueprint_revision: BlueprintRevision::INITIAL,
                    },
                    short_name: "ARCH".into(),
                    long_name: "Archived Blueprint adoption denial".into(),
                    term: adoption_term,
                    assigned_instructor: None,
                },
            )
            .await
            .is_err(),
        "Archived Blueprint adoption is denied"
    );
    let restored = owner_store
        .restore_blueprint(token(), blueprint_reference, archived.metadata_etag)
        .await
        .expect("owner restores Archived Blueprint to Public");
    assert_eq!(restored.availability, BlueprintAvailability::Public);
    // The disposable C73 harness may provide its administrator URL solely for
    // this post-operation relational oracle. Product writes above remain the
    // normal authenticated application path.
    let inspection_url = std::env::var("C73_ADOPTION_INSPECTION_DATABASE_URL")
        .unwrap_or_else(|_| migration_url.to_owned());
    let inspection = lazy_pool(&inspection_url).expect("adoption inspection pool");
    blueprint_course_postgres_adoption::assert_adoption_projection(
        &inspection,
        i64::from(adopted.course.reference.number()),
        reference,
        1,
        i64::from(independently_adopted.course.reference.number()),
    )
    .await;
    let mut enrollment = inspection.begin().await.expect("enrollment fixture");
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *enrollment)
        .await
        .expect("membership owner");
    let course_id: Uuid = sqlx::query_scalar(
        "SELECT course_id FROM ple_data.course_instance WHERE reference_number=$1",
    )
    .bind(i64::from(adopted.course.reference.number()))
    .fetch_one(&mut *enrollment)
    .await
    .expect("adopted course identity");
    sqlx::query("INSERT INTO ple_data.student_record (student_record_id, course_id, student_account_id, created_at) VALUES ($1,$2,$3,clock_timestamp())")
        .bind(id(0xb105)).bind(course_id).bind(id(0xb104)).execute(&mut *enrollment).await.expect("Student Record");
    for episode in [0xb106, 0xb107] {
        sqlx::query("INSERT INTO ple_data.course_membership (membership_id,course_id,account_id,role,student_record_id,joined_at) VALUES ($1,$2,$3,'student',$4,clock_timestamp())")
            .bind(id(episode)).bind(course_id).bind(id(0xb104)).bind(id(0xb105)).execute(&mut *enrollment).await.expect("membership episode");
        sqlx::query("INSERT INTO ple_data.course_membership_event (course_membership_event_id,membership_id,event_kind,occurred_at,reason) VALUES ($1,$2,'ended',clock_timestamp(),'verification departure')")
            .bind(id(episode + 0x10)).bind(id(episode)).execute(&mut *enrollment).await.expect("ended membership");
    }
    enrollment
        .commit()
        .await
        .expect("enrollment fixture commit");
    inspection.close().await;

    let reader_list = reader_store
        .list_blueprint_courses(reader_token())
        .await
        .expect("non-owner Instructor Blueprint list");
    let reader_summary = reader_list
        .iter()
        .find(|summary| summary.reference == blueprint_reference)
        .expect("Available Blueprint is discoverable by a non-owner Instructor");
    assert_eq!(
        reader_summary.read_access,
        question_model::BlueprintCourseReadAccess::ActiveInstructor
    );
    assert_eq!(reader_summary.total_adoptions, 2);
    assert_eq!(reader_summary.total_students_ever_enrolled, 1);
    let reader_view = reader_store
        .load_blueprint_course(reader_token(), blueprint_reference)
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
            reference,
            1,
            request(0x12),
            &concurrent_left
        ),
        save(
            &application_url,
            reference,
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
         WHERE blueprint_course_reference_number = $1",
    )
    .bind(reference)
    .fetch_one(&mut inspection)
    .await
    .expect("Revision count");
    assert_eq!(
        revision_count, 2,
        "one concurrent changed Save creates one new Revision"
    );

    let no_op = save(
        &application_url,
        reference,
        2,
        request(0x14),
        &current_content,
    )
    .await
    .expect("canonical no-op Save");
    assert_eq!(no_op, (2, false));
    let no_op_replay = save(
        &application_url,
        reference,
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

    let retained_assessment =
        current_content.modules[0].assessments[0].blueprint_assessment_reference;
    let retained_module = current_content.modules[0].blueprint_module_reference;
    let moved_input = ReplaceBlueprintCourseContentInput {
        modules: vec![
            BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_reference: retained_module,
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
                        blueprint_assessment_reference: retained_assessment,
                    },
                    content: assessment_input("Moved retained Assessment"),
                }],
            },
        ],
    };
    let moved_content =
        StoredBlueprintCourseContent::from_replace(moved_input, &current_content, &pins())
            .expect("retained Assessment may move into a new Module");
    let moved = save(
        &application_url,
        reference,
        2,
        request(0x15),
        &moved_content,
    )
    .await
    .expect("Save moving retained Assessment");
    assert_eq!(moved, (3, true));
    let moved_replay = save(
        &application_url,
        reference,
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
        "SELECT blueprint_module_reference FROM ple_data.blueprint_revision_assessment \
         WHERE blueprint_course_reference_number = $1 \
           AND blueprint_assessment_reference = $2 \
           AND blueprint_revision_number IN (2, 3) ORDER BY blueprint_revision_number",
    )
    .bind(reference)
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
        (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, module_position) \
        VALUES ($1, $2, '00000000-0000-0000-0000-00000000b122', 9)";
    let sealed_update = "UPDATE ple_data.blueprint_revision_assessment \
        SET assessment_position = assessment_position + 10 \
        WHERE blueprint_course_reference_number = $1 AND blueprint_revision_number = $2";
    let sealed_delete = "DELETE FROM ple_data.blueprint_revision_question_pin \
        WHERE blueprint_course_reference_number = $1 AND blueprint_revision_number = $2";
    assert_immutable_child(&mut inspection, sealed_insert, reference, 3).await;
    assert_immutable_child(&mut inspection, sealed_update, reference, 3).await;
    assert_immutable_child(&mut inspection, sealed_delete, reference, 3).await;

    let store =
        PostgresBlueprintCourseStore::new(lazy_pool(&application_url).expect("application pool"));
    let blueprint_reference = format!("BP-{reference}")
        .parse::<BlueprintCourseReference>()
        .expect("Blueprint reference");
    let exact_revision = BlueprintRevision::new(3).expect("Revision three");
    let before_tamper = store
        .load_blueprint_revision(
            token(),
            question_model::BlueprintRevisionReference {
                reference: blueprint_reference,
                revision: exact_revision,
            },
        )
        .await
        .expect("stored Revision checksum before tamper");
    assert_eq!(before_tamper.content.modules.len(), 2);

    let mut tamper = inspection.begin().await.expect("tamper transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *tamper)
        .await
        .expect("tamper owner role");
    sqlx::query("ALTER TABLE ple_data.blueprint_course_revision NO FORCE ROW LEVEL SECURITY")
        .execute(&mut *tamper)
        .await
        .expect("controlled tamper harness temporarily permits owner inspection");
    sqlx::query("ALTER TABLE ple_data.blueprint_course_revision DISABLE TRIGGER USER")
        .execute(&mut *tamper)
        .await
        .expect("controlled tamper harness disables immutability trigger");
    let tampered = sqlx::query(
        "UPDATE ple_data.blueprint_course_revision \
         SET content = jsonb_set(content, \
             '{modules,0,assessments,0,blueprint_assessment_reference}', \
             to_jsonb('00000000-0000-0000-0000-00000000b123'::text)) \
         WHERE blueprint_course_reference_number = $1 AND blueprint_revision_number = 3",
    )
    .bind(reference)
    .execute(&mut *tamper)
    .await
    .expect("controlled identity tamper");
    assert_eq!(
        tampered.rows_affected(),
        1,
        "controlled tamper changed one Revision"
    );
    sqlx::query("ALTER TABLE ple_data.blueprint_course_revision ENABLE TRIGGER USER")
        .execute(&mut *tamper)
        .await
        .expect("tamper harness restores trigger");
    sqlx::query("ALTER TABLE ple_data.blueprint_course_revision FORCE ROW LEVEL SECURITY")
        .execute(&mut *tamper)
        .await
        .expect("tamper harness restores forced RLS");
    tamper.commit().await.expect("tamper commit");
    assert!(matches!(
        store
            .load_blueprint_revision(
                token(),
                question_model::BlueprintRevisionReference {
                    reference: blueprint_reference,
                    revision: exact_revision,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

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
    sqlx::query("SELECT 1 FROM ple_data.blueprint_course WHERE reference_number = $1 FOR UPDATE")
        .bind(reference)
        .execute(&mut *holder)
        .await
        .expect("hold Blueprint head");
    let (save_pid_sender, save_pid_receiver) = oneshot::channel();
    let save_url = application_url.clone();
    let save_content = revision_four_content.clone();
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
        let row = sqlx::query("SELECT resulting_blueprint_revision_number, changed FROM ple_api.save_blueprint_course($1, 3, $2, $3, $4)")
            .bind(reference).bind(request(0x16))
            .bind(serde_json::to_value(&save_content).expect("content JSON"))
            .bind(save_content.checksum().expect("content checksum").as_bytes().to_vec())
            .fetch_one(&mut *transaction).await?;
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
            "SELECT reference_number FROM ple_api.create_course_instance(\
             '00000000-0000-0000-0000-00000000b130', \
             '00000000-0000-0000-0000-00000000b131', \
             '00000000-0000-0000-0000-00000000b132', \
             '00000000-0000-0000-0000-00000000b133', \
             'adopted', $1, 3, 'RACE-C', 'Concurrent head Course', \
             current_date, current_date + 1, NULL, '[]'::jsonb)",
        )
        .bind(reference)
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
         (blueprint_course_reference_number, blueprint_revision_number, content, content_checksum, saved_at) \
         VALUES ($1, 5, $2, $3, clock_timestamp())",
    )
    .bind(reference)
    .bind(&revision_five_json)
    .bind(&revision_five_checksum)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Revision header");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_question_pin \
         SELECT $1, 5, pins.content_path, pins.question_id, pins.question_revision_number \
           FROM ple_data.blueprint_content_question_pins($2) AS pins",
    )
    .bind(reference)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Question pins");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_module \
         SELECT $1, 5, modules.blueprint_module_reference, modules.module_position \
           FROM ple_data.blueprint_content_modules($2) AS modules",
    )
    .bind(reference)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Modules");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_assessment \
         SELECT $1, 5, assessments.blueprint_module_reference, \
                assessments.blueprint_assessment_reference, assessments.assessment_position \
           FROM ple_data.blueprint_content_assessments($2) AS assessments",
    )
    .bind(reference)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Assessments");
    sqlx::query(
        "INSERT INTO ple_data.blueprint_revision_event \
         (blueprint_course_reference_number, blueprint_revision_number, actor_account_id, request_checksum, occurred_at) \
         VALUES ($1, 5, $2, $3, clock_timestamp())",
    )
    .bind(reference)
    .bind(id(INSTRUCTOR))
    .bind(request(0x17))
    .execute(&mut *construction)
    .await
    .expect("uncommitted Revision receipt");

    let competing_url = migration_url.to_owned();
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
             (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, module_position) \
             VALUES ($1, 5, '00000000-0000-0000-0000-00000000b140', 99)",
        )
        .bind(reference)
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
    let mut final_inspection = PgConnection::connect(migration_url)
        .await
        .expect("final inspection connection");
    sqlx::query("SET ROLE ple_api_owner")
        .execute(&mut final_inspection)
        .await
        .expect("final inspection role");
    let competing_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ple_data.blueprint_revision_module \
         WHERE blueprint_course_reference_number = $1 AND blueprint_revision_number = 5 \
           AND blueprint_module_reference = '00000000-0000-0000-0000-00000000b140'",
    )
    .bind(reference)
    .fetch_one(&mut final_inspection)
    .await
    .expect("competing child inspection");
    assert_eq!(
        competing_rows, 0,
        "rejected child did not enter the sealed Revision"
    );
}
