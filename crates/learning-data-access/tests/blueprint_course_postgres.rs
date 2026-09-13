#![cfg(feature = "postgres")]

//! Connected PostgreSQL oracle for immutable Blueprint Revision persistence.

use std::collections::BTreeMap;

use learning_data_access::postgres::{PostgresBlueprintCourseStore, lazy_pool};
use learning_data_access::{
    BlueprintCourseStore, SessionTokenHash, StoreError, StoredBlueprintCourseContent,
};
use question_model::{
    AssignmentActivityRules, AssignmentEntryScoringRule, AssignmentInstructions,
    AssignmentPointValue, BlueprintAssignmentContentInput, BlueprintAssignmentDefaults,
    BlueprintAssignmentEditChoice, BlueprintAssignmentEntryInput,
    BlueprintAssignmentReplacementInput, BlueprintCourseReference, BlueprintModuleEditChoice,
    BlueprintModuleReplacementInput, BlueprintRevision, CreateBlueprintCourseInput,
    CreateBlueprintModuleInput, LateWorkRule, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionId, QuestionRevisionNumber, QuestionRevisionReference, RelativeAssignmentSchedule,
    ReplaceBlueprintCourseContentInput, ReusableFixedQuestionInput, StudentFeedbackReleaseRule,
};
use sqlx::{Connection, PgConnection, Row};
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

const INSTRUCTOR: u128 = 0xb100;
const SESSION: u128 = 0xb101;
const READER_INSTRUCTOR: u128 = 0xb102;
const READER_SESSION: u128 = 0xb103;
const QUESTION: &str = "ABCDE12";

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xb2; 32])
}

fn reader_token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xb3; 32])
}

fn request(value: u8) -> Vec<u8> {
    vec![value; 32]
}

fn question_id() -> QuestionId {
    QUESTION.parse().expect("closed Question ID fixture")
}

fn content_input(title: &str) -> CreateBlueprintCourseInput {
    CreateBlueprintCourseInput {
        short_name: "REV-ACC".to_owned(),
        long_name: "Revision acceptance Blueprint".to_owned(),
        modules: vec![CreateBlueprintModuleInput {
            label: "Module alpha".to_owned(),
            assignments: vec![BlueprintAssignmentContentInput {
                title: title.to_owned(),
                instructions: AssignmentInstructions::try_new("Read the prompt.".to_owned())
                    .expect("fixture instructions"),
                entries: vec![BlueprintAssignmentEntryInput::Fixed(
                    ReusableFixedQuestionInput {
                        question_id: question_id(),
                        points_possible: AssignmentPointValue::from_whole(2),
                        scoring_rule: AssignmentEntryScoringRule::Normal,
                        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                    },
                )],
                defaults: BlueprintAssignmentDefaults {
                    assignment_attempt_time_limit_seconds: None,
                    attempt_limit: None,
                    late_work_rule: LateWorkRule::Accept,
                    activity_rules: AssignmentActivityRules::default(),
                    student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                },
                schedule: RelativeAssignmentSchedule::default(),
            }],
        }],
    }
}

fn initial_content() -> StoredBlueprintCourseContent {
    let question = question_id();
    let pins = BTreeMap::from([(
        question.clone(),
        QuestionRevisionReference {
            question_id: question,
            revision_number: QuestionRevisionNumber::new(1).expect("fixture Question Revision"),
        },
    )]);
    StoredBlueprintCourseContent::from_create(content_input("Revision one Assignment"), &pins)
        .expect("closed Blueprint content fixture")
}

fn pins() -> BTreeMap<QuestionId, QuestionRevisionReference> {
    let question = question_id();
    BTreeMap::from([(
        question.clone(),
        QuestionRevisionReference {
            question_id: question,
            revision_number: QuestionRevisionNumber::new(1).expect("fixture Question Revision"),
        },
    )])
}

fn assignment_input(title: &str) -> BlueprintAssignmentContentInput {
    content_input(title).modules.remove(0).assignments.remove(0)
}

fn changed_content(
    mut content: StoredBlueprintCourseContent,
    title: &str,
) -> StoredBlueprintCourseContent {
    content.modules[0].assignments[0].content.title = title.to_owned();
    content
}

async fn seed(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'instructor', clock_timestamp())",
    )
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("Instructor account");
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'instructor', clock_timestamp())",
    )
    .bind(id(READER_INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("reader Instructor account");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(SESSION))
    .bind(id(INSTRUCTOR))
    .bind(token().to_string())
    .execute(&mut *transaction)
    .await
    .expect("Instructor session");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(READER_SESSION))
    .bind(id(READER_INSTRUCTOR))
    .bind(reader_token().to_string())
    .execute(&mut *transaction)
    .await
    .expect("reader Instructor session");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("data fixture role");
    sqlx::query(
        "SELECT setval(\
             'ple_data.blueprint_course_reference_number_seq', \
             GREATEST(COALESCE((SELECT max(reference_number) FROM ple_data.blueprint_course), 1), 1), \
             true\
         )",
    )
    .execute(&mut *transaction)
    .await
    .expect("Blueprint reference sequence follows fixed fixtures");
    sqlx::query(
        "INSERT INTO ple_data.published_question (question_id, created_at) \
         VALUES ($1, clock_timestamp())",
    )
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("Published Question");
    sqlx::query(
        "INSERT INTO ple_data.question_revision \
         (question_id, revision_number, backend, question_type, published_at) \
         VALUES ($1, 1, 'ple', 'multipleChoice', clock_timestamp())",
    )
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("Question Revision");
    transaction.commit().await.expect("fixture commit");
}

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

    let reader_store =
        PostgresBlueprintCourseStore::new(lazy_pool(&application_url).expect("reader pool"));
    let blueprint_reference = format!("BP-{reference}")
        .parse::<BlueprintCourseReference>()
        .expect("Blueprint reference");
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

    let retained_assignment =
        current_content.modules[0].assignments[0].blueprint_assignment_reference;
    let retained_module = current_content.modules[0].blueprint_module_reference;
    let moved_input = ReplaceBlueprintCourseContentInput {
        modules: vec![
            BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_reference: retained_module,
                },
                label: "Module alpha".to_owned(),
                assignments: vec![BlueprintAssignmentReplacementInput {
                    choice: BlueprintAssignmentEditChoice::New,
                    content: assignment_input("Replacement in Module alpha"),
                }],
            },
            BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::New,
                label: "Module beta".to_owned(),
                assignments: vec![BlueprintAssignmentReplacementInput {
                    choice: BlueprintAssignmentEditChoice::Retained {
                        blueprint_assignment_reference: retained_assignment,
                    },
                    content: assignment_input("Moved retained Assignment"),
                }],
            },
        ],
    };
    let moved_content =
        StoredBlueprintCourseContent::from_replace(moved_input, &current_content, &pins())
            .expect("retained Assignment may move into a new Module");
    let moved = save(
        &application_url,
        reference,
        2,
        request(0x15),
        &moved_content,
    )
    .await
    .expect("Save moving retained Assignment");
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
        "SELECT blueprint_module_reference FROM ple_data.blueprint_revision_assignment \
         WHERE blueprint_course_reference_number = $1 \
           AND blueprint_assignment_reference = $2 \
           AND blueprint_revision_number IN (2, 3) ORDER BY blueprint_revision_number",
    )
    .bind(reference)
    .bind(retained_assignment.as_uuid())
    .fetch_all(&mut inspection)
    .await
    .expect("retained Assignment lineage");
    assert_eq!(module_rows.len(), 2);
    assert_ne!(
        module_rows[0], module_rows[1],
        "retained Assignment moved Modules"
    );

    let sealed_insert: &'static str = "INSERT INTO ple_data.blueprint_revision_module \
        (blueprint_course_reference_number, blueprint_revision_number, blueprint_module_reference, module_position) \
        VALUES ($1, $2, '00000000-0000-0000-0000-00000000b122', 9)";
    let sealed_update = "UPDATE ple_data.blueprint_revision_assignment \
        SET assignment_position = assignment_position + 10 \
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
             '{modules,0,assignments,0,blueprint_assignment_reference}', \
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
             $1, 3, 'RACE-C', 'Concurrent head Course', \
             '2030-01-01'::date, '2030-05-01'::date, NULL)",
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
        "INSERT INTO ple_data.blueprint_revision_assignment \
         SELECT $1, 5, assignments.blueprint_module_reference, \
                assignments.blueprint_assignment_reference, assignments.assignment_position \
           FROM ple_data.blueprint_content_assignments($2) AS assignments",
    )
    .bind(reference)
    .bind(&revision_five_json)
    .execute(&mut *construction)
    .await
    .expect("uncommitted Assignments");
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
