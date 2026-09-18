//! Shared deterministic fixtures for the connected Blueprint Course oracle.

use super::*;

pub(super) struct FixturePoolIdIssuer(pub(super) AtomicUsize);

impl CourseInstancePoolIdIssuer for FixturePoolIdIssuer {
    fn issue_question_pool_id(&self) -> Result<QuestionId, StoreError> {
        const IDS: [&str; 8] = [
            "8K3M-69Q1",
            "9K3M-09Q2",
            "7K3M-T9Q3",
            "6K3M-19Q4",
            "5K3M-V9Q5",
            "4K3M-D9Q6",
            "3K3M-S9Q7",
            "2K3M-49Q8",
        ];
        let index = self.0.fetch_add(1, Ordering::SeqCst);
        IDS.get(index)
            .ok_or_else(|| StoreError::Unavailable("fixture Pool IDs exhausted".to_string()))?
            .parse()
            .map_err(|_| StoreError::InvalidRecord("fixture Pool ID is invalid".to_string()))
    }
}

pub(super) async fn authenticate_application_transaction(
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

/// Numeric references are relational inspection facts, never application input.
pub(super) async fn blueprint_reference_number(
    public_reference: &question_model::BlueprintCourseId,
) -> i64 {
    let mut inspection = adoption_inspection_connection().await;
    let reference = sqlx::query_scalar(
        "SELECT reference_number FROM ple_data.blueprint_course WHERE public_reference = $1",
    )
    .bind(public_reference.as_string())
    .fetch_one(&mut inspection)
    .await
    .expect("Blueprint relational identity");
    inspection
        .close()
        .await
        .expect("Blueprint inspection close");
    reference
}

pub(super) async fn adoption_inspection_connection() -> PgConnection {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let mut connection = PgConnection::connect(runtime.migration_url().expose())
        .await
        .expect("Blueprint fixture inspection connection");
    // ASVS 8.2.1: keep explicit fixture privilege here, never on an application pool.
    sqlx::query("SET ROLE ple_data_owner")
        .execute(&mut connection)
        .await
        .expect("Blueprint fixture inspection role");
    connection
}

/// Public commands use opaque IDs; numeric IDs belong only to relational inspection.
pub(super) async fn blueprint_public_reference(reference: i64) -> String {
    let mut inspection = adoption_inspection_connection().await;
    let public_reference = sqlx::query_scalar(
        "SELECT public_reference FROM ple_data.blueprint_course WHERE reference_number = $1",
    )
    .bind(reference)
    .fetch_one(&mut inspection)
    .await
    .expect("Blueprint public identity");
    inspection
        .close()
        .await
        .expect("Blueprint inspection close");
    public_reference
}

/// Exact immutable member pins for one local Pool Revision.
pub(super) async fn question_pool_member_pins(
    pool: &question_model::QuestionPoolRevisionReference,
) -> Vec<(String, i32)> {
    let mut inspection = adoption_inspection_connection().await;
    let pins = sqlx::query_as(
        "SELECT member.question_id, member.question_revision_number \
           FROM ple_data.question_pool AS pool \
           JOIN ple_data.question_pool_revision_member AS member \
             ON member.question_pool_id = pool.question_pool_id \
          WHERE pool.public_question_pool_id = $1 AND member.revision_number = $2 \
          ORDER BY member.member_position",
    )
    .bind(pool.question_pool_id.as_str())
    .bind(pool.revision_number.get() as i64)
    .fetch_all(&mut inspection)
    .await
    .expect("Pool member pins");
    inspection
        .close()
        .await
        .expect("Pool member inspection close");
    pins
}

/// The decoder must reject a sealed Revision whose stored content no longer
/// matches its immutable checksum.
pub(super) async fn assert_revision_checksum_mismatch(
    migration_url: &str,
    application_url: &str,
    reference: i64,
    blueprint_reference: question_model::BlueprintCourseId,
) {
    let tamper_pool = lazy_pool(application_url).expect("tamper application pool");
    let tamper_store = PostgresBlueprintCourseStore::new(tamper_pool.clone());
    let exact_revision = BlueprintRevision::new(3).expect("Revision three");
    let before_tamper = tamper_store
        .load_blueprint_revision(
            token(),
            question_model::BlueprintRevisionReference {
                reference: blueprint_reference.clone(),
                revision: exact_revision,
            },
        )
        .await
        .expect("stored Revision checksum before tamper");
    assert_eq!(before_tamper.content.modules.len(), 2);
    let mut tamper_connection = PgConnection::connect(migration_url)
        .await
        .expect("tamper connection");
    let mut tamper = tamper_connection.begin().await.expect("tamper transaction");
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
        tamper_store
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
    tamper_pool.close().await;
    tamper_connection
        .close()
        .await
        .expect("release tamper connection");
}

/// Compare durable Blueprint state across denied commands, including receipts.
pub(super) async fn blueprint_write_state(reference: i64) -> serde_json::Value {
    let mut inspection = adoption_inspection_connection().await;
    // ASVS 1.2.4: fixture identity remains a bound parameter.
    sqlx::query_scalar(
        "SELECT jsonb_build_object( \
            'metadata', to_jsonb(course), \
            'revisions', (SELECT jsonb_agg(to_jsonb(revision) ORDER BY blueprint_revision_number) \
                FROM ple_data.blueprint_course_revision AS revision \
                WHERE blueprint_course_reference_number = $1), \
            'revision_events', (SELECT jsonb_agg(to_jsonb(event) ORDER BY blueprint_revision_number) \
                FROM ple_data.blueprint_revision_event AS event \
                WHERE blueprint_course_reference_number = $1), \
            'metadata_events', (SELECT jsonb_agg(to_jsonb(event) ORDER BY occurred_at, blueprint_edit_number) \
                FROM ple_data.blueprint_metadata_event AS event \
                WHERE blueprint_course_reference_number = $1), \
            'save_receipts', (SELECT jsonb_agg(to_jsonb(receipt) ORDER BY request_checksum) \
                FROM ple_data.blueprint_course_save_receipt AS receipt \
                WHERE blueprint_course_reference_number = $1)) \
         FROM ple_data.blueprint_course AS course WHERE reference_number = $1",
    )
    .bind(reference)
    .fetch_one(&mut inspection)
    .await
    .expect("Blueprint durable write state")
}

pub(super) async fn save(
    url: &str,
    reference: i64,
    expected_revision: i64,
    checksum: Vec<u8>,
    content: &StoredBlueprintCourseContent,
) -> Result<(i64, bool), sqlx::Error> {
    let public_reference = blueprint_public_reference(reference).await;
    let encoded_content = database_content_json(content);
    let mut connection = PgConnection::connect(url)
        .await
        .expect("application connection");
    let mut transaction = connection.begin().await.expect("application transaction");
    authenticate_application_transaction(&mut transaction).await;
    let result = sqlx::query(
        "SELECT resulting_blueprint_revision_number, changed \
         FROM ple_api.save_blueprint_course($1, $2, $3, $4, $5, \
             (SELECT COALESCE(jsonb_agg(jsonb_build_object( \
                 'course_id', course_id, 'assessments', '[]'::jsonb)), '[]'::jsonb) \
                FROM ple_api.list_blueprint_daughter_course_ids($1)))",
    )
    .bind(public_reference)
    .bind(expected_revision)
    .bind(checksum)
    .bind(encoded_content)
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

/// PostgreSQL content keeps the exact canonical Question IDs.
fn database_content_json(content: &StoredBlueprintCourseContent) -> serde_json::Value {
    serde_json::to_value(content).expect("Blueprint content JSON")
}

pub(super) async fn transition_blueprint_availability(
    url: &str,
    reference: i64,
    expected_edit_number: Uuid,
    availability: &'static str,
    archive_confirmation_long_name: Option<&str>,
) -> Result<(BlueprintAvailability, Uuid), sqlx::Error> {
    let public_reference = blueprint_public_reference(reference).await;
    let mut connection = PgConnection::connect(url)
        .await
        .expect("Blueprint lifecycle application connection");
    let mut transaction = connection
        .begin()
        .await
        .expect("Blueprint lifecycle application transaction");
    authenticate_application_transaction(&mut transaction).await;
    let result = sqlx::query(
        "SELECT availability, blueprint_edit_number FROM ple_api.set_blueprint_availability($1, $2, $3, $4)",
    )
    .bind(public_reference)
    .bind(expected_edit_number)
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
            row.try_get("blueprint_edit_number")
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

pub(super) async fn near_now_term(url: &str) -> question_model::CourseTerm {
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

pub(super) fn error_code(error: &sqlx::Error) -> Option<String> {
    match error {
        sqlx::Error::Database(database) => database.code().map(|code| code.into_owned()),
        _ => None,
    }
}

pub(super) async fn assert_immutable_child(
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

pub(super) const INSTRUCTOR: u128 = 0xb100;
pub(super) const SESSION: u128 = 0xb101;
pub(super) const READER_INSTRUCTOR: u128 = 0xb102;
pub(super) const READER_SESSION: u128 = 0xb103;
pub(super) const QUESTION: &str = "ABCD-XEFG";
pub(super) const QUESTION_POOL: &str = "7654-Z321";

pub(super) fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

pub(super) fn token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xb2; 32])
}

pub(super) fn reader_token() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xb3; 32])
}

pub(super) fn request(value: u8) -> Vec<u8> {
    vec![value; 32]
}

fn question_id() -> QuestionId {
    QUESTION.parse().expect("closed Question ID fixture")
}

fn question_pool_id() -> QuestionId {
    QUESTION_POOL.parse().expect("closed Pool ID fixture")
}

pub(super) fn content_input(title: &str) -> CreateBlueprintCourseInput {
    CreateBlueprintCourseInput {
        classification: question_model::CourseClassification {
            discipline_uuid: uuid::Uuid::from_u128(0xcc01),
            subject_uuid: None,
            topic_uuid: None,
            subtopic_uuid: None,
            tags: Vec::new(),
        },
        short_name: "REV-ACC".to_owned(),
        long_name: "Revision acceptance Blueprint".to_owned(),
        modules: vec![CreateBlueprintModuleInput {
            label: "Module alpha".to_owned(),
            assessments: vec![BlueprintAssessmentContentInput {
                assessment_type: question_model::AssessmentType::RegularAssignment,
                title: title.to_owned(),
                instructions: AssessmentInstructions::try_new("Read the prompt.".to_owned())
                    .expect("fixture instructions"),
                entries: vec![
                    BlueprintAssessmentEntryInput::Pool(question_model::ReusablePoolInput {
                        pool: question_model::BlueprintPoolInputChoice::Import {
                            question_pool_revision: question_model::QuestionPoolRevisionReference {
                                question_pool_id: question_pool_id(),
                                revision_number: question_model::QuestionPoolRevisionNumber::new(1)
                                    .expect("fixture Pool Revision"),
                            },
                        },
                        selection_count: std::num::NonZeroU32::new(1)
                            .expect("positive fixture Pool selection count"),
                        points_per_item: AssessmentPointValue::from_whole(3),
                        scoring_rule: AssessmentEntryScoringRule::ExtraCredit,
                        selection_rule: question_model::QuestionPoolSelectionRule {
                            selected_question_order:
                                question_model::QuestionPoolSelectedQuestionOrder::RandomOrder,
                        },
                        question_attempt_limit: QuestionAttemptLimit {
                            max_attempts: Some(2),
                        },
                        question_attempt_time_limit: QuestionAttemptTimeLimit::Limited {
                            seconds: 120,
                            grace_seconds: 20,
                        },
                    }),
                    BlueprintAssessmentEntryInput::Fixed(ReusableFixedQuestionInput {
                        published_question: QuestionRevisionReference {
                            question_id: question_id(),
                            revision_number: QuestionRevisionNumber::new(1)
                                .expect("fixture Question Revision"),
                        },
                        points_possible: AssessmentPointValue::from_whole(2),
                        scoring_rule: AssessmentEntryScoringRule::Normal,
                        question_attempt_limit: QuestionAttemptLimit {
                            max_attempts: Some(4),
                        },
                        question_attempt_time_limit: QuestionAttemptTimeLimit::Limited {
                            seconds: 90,
                            grace_seconds: 15,
                        },
                    }),
                ],
                defaults: BlueprintAssessmentDefaults {
                    assessment_attempt_time_limit_seconds: std::num::NonZeroU32::new(900),
                    attempt_limit: std::num::NonZeroU32::new(3),
                    late_work_rule: LateWorkRule::MarkLate,
                    activity_rules: AssessmentActivityRules {
                        question_variation_rule:
                            question_model::AssessmentQuestionVariationRule::ReuseVariation,
                        assessment_question_order_rule:
                            question_model::AssessmentQuestionOrderRule::AuthoredOrder,
                    },
                    student_feedback_release_rule: StudentFeedbackReleaseRule {
                        score: question_model::StudentFeedbackReleaseTiming::DuringAttempt,
                        per_item_correctness:
                            question_model::StudentFeedbackReleaseTiming::AfterSubmit,
                        submitted_response: question_model::StudentFeedbackReleaseTiming::AfterDue,
                        question_answer: question_model::StudentFeedbackReleaseTiming::Never,
                        question_answer_explanation:
                            question_model::StudentFeedbackReleaseTiming::AfterClose,
                        class_statistics: question_model::StudentFeedbackReleaseTiming::AfterDue,
                    },
                },
            }],
        }],
    }
}

pub(super) fn assessment_input(title: &str) -> BlueprintAssessmentContentInput {
    content_input(title).modules.remove(0).assessments.remove(0)
}

/// Reuse an Assessment-owned Pool from the loaded expected head.
pub(super) fn retained_assessment_input(
    prior: &learning_data_access::StoredBlueprintAssessmentContent,
    title: &str,
) -> BlueprintAssessmentContentInput {
    let mut input = assessment_input(title);
    let mut prior_pools = prior.entries.iter().filter_map(|entry| match entry {
        learning_data_access::StoredBlueprintAssessmentEntry::Pool {
            question_pool_revision,
            ..
        } => Some(question_pool_revision.clone()),
        learning_data_access::StoredBlueprintAssessmentEntry::Fixed { .. } => None,
    });
    for entry in &mut input.entries {
        let question_model::BlueprintAssessmentEntryInput::Pool(pool) = entry else {
            continue;
        };
        pool.pool = question_model::BlueprintPoolInputChoice::Retained {
            question_pool_revision: prior_pools.next().expect("retained fixture Pool"),
            members: None,
            interchangeability_attested: false,
        };
    }
    assert!(
        prior_pools.next().is_none(),
        "retained fixture Assessment Pool shape"
    );
    input
}

pub(super) fn changed_content(
    mut content: StoredBlueprintCourseContent,
    title: &str,
) -> StoredBlueprintCourseContent {
    content.modules[0].assessments[0].content.title = title.to_owned();
    content
}

pub(super) async fn seed(admin: &sqlx::postgres::PgPool) {
    let mut transaction = admin.begin().await.expect("fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("classification fixture owner");
    sqlx::query("INSERT INTO ple_data.content_discipline (discipline_uuid, name) VALUES ('00000000-0000-0000-0000-00000000cc01', 'Course fixture discipline') ON CONFLICT (discipline_uuid) DO NOTHING").execute(&mut *transaction).await.expect("explicit fixture Discipline");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private fixture role");
    for account in [INSTRUCTOR, READER_INSTRUCTOR] {
        sqlx::query(
            "INSERT INTO ple_private.account (account_id, product_role, created_at) \
             VALUES ($1, 'instructor', clock_timestamp())",
        )
        .bind(id(account))
        .execute(&mut *transaction)
        .await
        .expect("Instructor account");
    }
    sqlx::query(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ($1, 'student', clock_timestamp())",
    )
    .bind(id(0xb104))
    .execute(&mut *transaction)
    .await
    .expect("Student fixture");
    for (session, account, token) in [
        (SESSION, INSTRUCTOR, token()),
        (READER_SESSION, READER_INSTRUCTOR, reader_token()),
    ] {
        sqlx::query(
            "INSERT INTO ple_private.authenticated_session \
             (session_id, account_id, product_role, token_hash, created_at, expires_at) \
             VALUES ($1, $2, 'instructor', decode($3, 'hex'), clock_timestamp(), \
                     clock_timestamp() + interval '1 hour')",
        )
        .bind(id(session))
        .bind(id(account))
        .bind(token.to_string())
        .execute(&mut *transaction)
        .await
        .expect("Instructor session");
    }
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
    sqlx::query(
        "INSERT INTO ple_data.content_subject (subject_uuid, name) \
         VALUES ($1, 'Blueprint fixture Subject') ON CONFLICT (subject_uuid) DO NOTHING",
    )
    .bind(id(0xcc02))
    .execute(&mut *transaction)
    .await
    .expect("explicit fixture Subject");
    sqlx::query(
        "INSERT INTO ple_data.content_subject_discipline (subject_uuid, discipline_uuid) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id(0xcc02))
    .bind(id(0xcc01))
    .execute(&mut *transaction)
    .await
    .expect("explicit fixture Subject Discipline association");
    sqlx::query(
        "INSERT INTO ple_data.published_question_metadata (\
             question_id, question_title, question_description, language, \
             discipline_uuid, subject_uuid, created_at, updated_at\
         ) VALUES ($1, 'Blueprint fixture Question', 'Blueprint fixture Question description', \
                   'en', $2, $3, clock_timestamp(), clock_timestamp())",
    )
    .bind(QUESTION)
    .bind(id(0xcc01))
    .bind(id(0xcc02))
    .execute(&mut *transaction)
    .await
    .expect("explicit first Question metadata");
    // This fixture names an ordinary Question Library Revision. Keep its
    // immutable publication evidence complete so the selected Question stays
    // valid for both ordinary Blueprint creation and canonical exchange import.
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private Question publication fixture role");
    sqlx::query(
        "INSERT INTO ple_private.object_record (\
             object_id, object_address, object_storage_area, object_data_class, \
             sha256, size_bytes, media_type, created_at\
         ) SELECT $1, jsonb_build_object(\
                 'kind', 'questionSource', \
                 'questionRevision', jsonb_build_object('questionId', $2, 'revisionNumber', 1), \
                 'object', $1\
             ), 'private-content', 'question-source', decode(repeat('b1', 32), 'hex'), \
             1, 'application/json', revision.published_at \
           FROM ple_data.question_revision AS revision \
          WHERE revision.question_id = $2 AND revision.revision_number = 1",
    )
    .bind(id(0xb107))
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("exact Question source Object Record");
    sqlx::query(
        "INSERT INTO ple_private.question_revision_source_binding (\
             question_id, revision_number, backend, question_format, source_object_id, \
             source_object_checksum, created_at\
         ) SELECT $1, 1, 'ple', 'pleQuestionJson', $2, repeat('b1', 32), revision.published_at \
           FROM ple_data.question_revision AS revision \
          WHERE revision.question_id = $1 AND revision.revision_number = 1",
    )
    .bind(QUESTION)
    .bind(id(0xb107))
    .execute(&mut *transaction)
    .await
    .expect("exact Question source binding");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_acceptance (\
             question_id, revision_number, parent_revision_number, editor_account_id, \
             accepted_by_account_id, accepted_at, reason_for_edit\
         ) SELECT $1, 1, NULL, $2, $2, revision.published_at, 'Initial publication' \
           FROM ple_data.question_revision AS revision \
          WHERE revision.question_id = $1 AND revision.revision_number = 1",
    )
    .bind(QUESTION)
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("Question Revision acceptance");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_authorship (\
             question_id, revision_number, author_position, author_display_name, author_account_id\
         ) VALUES ($1, 1, 1, 'Blueprint fixture Instructor', $2)",
    )
    .bind(QUESTION)
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("Question Revision authorship");
    sqlx::query(
        "INSERT INTO ple_data.question_revision_license (question_id, revision_number, spdx_expression) \
         VALUES ($1, 1, 'CC-BY-4.0')",
    )
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("Question Revision license");
    sqlx::query(
        "INSERT INTO ple_data.question_ownership_event (\
             question_ownership_event_id, question_id, owner_account_id, recorded_by_account_id, \
             event_kind, occurred_at\
         ) SELECT $1, $2, $3, $3, 'initial', revision.published_at \
           FROM ple_data.question_revision AS revision \
          WHERE revision.question_id = $2 AND revision.revision_number = 1",
    )
    .bind(id(0xb108))
    .bind(QUESTION)
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("initial Question ownership");
    sqlx::query(
        "INSERT INTO ple_data.question_publication_event (\
             event_id, question_id, revision_number, actor_account_id, occurred_at\
         ) SELECT $1, $2, 1, $3, revision.published_at \
           FROM ple_data.question_revision AS revision \
          WHERE revision.question_id = $2 AND revision.revision_number = 1",
    )
    .bind(id(0xb109))
    .bind(QUESTION)
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("Question publication event");
    sqlx::query(
        "INSERT INTO ple_data.question_availability_event (\
             event_id, question_id, actor_account_id, availability, edit_number, reason, occurred_at\
         ) SELECT $1, $2, $3, 'available', 1, NULL, revision.published_at \
           FROM ple_data.question_revision AS revision \
          WHERE revision.question_id = $2 AND revision.revision_number = 1",
    )
    .bind(id(0xb10a))
    .bind(QUESTION)
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("initial Question availability");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *transaction)
        .await
        .expect("data Question Pool fixture role");
    sqlx::query(
        "INSERT INTO ple_data.question_pool (\
             question_pool_id, public_question_pool_id, blueprint_edit_number, current_revision_number, created_at, \
             title, description, discipline_uuid, subject_uuid\
         ) SELECT $1, $2, $3, 1, clock_timestamp(), \
                  'Blueprint fixture Pool', 'Blueprint fixture Pool description', \
                  metadata.discipline_uuid, metadata.subject_uuid \
             FROM ple_data.published_question_metadata AS metadata WHERE metadata.question_id = $4",
    )
    .bind(id(0xb105))
    .bind(QUESTION_POOL)
    .bind(id(0xb106))
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("Published Question Pool");
    sqlx::query(
        "INSERT INTO ple_data.question_pool_revision (\
             question_pool_id, revision_number, member_count, interchangeability_attested_by_account_id, \
             interchangeability_attested_at, created_at\
         ) VALUES ($1, 1, 1, $2, clock_timestamp(), clock_timestamp())",
    )
    .bind(id(0xb105))
    .bind(id(INSTRUCTOR))
    .execute(&mut *transaction)
    .await
    .expect("Published Question Pool Revision");
    sqlx::query(
        "INSERT INTO ple_data.question_pool_revision_member (\
             question_pool_id, revision_number, member_position, question_id, question_revision_number\
         ) VALUES ($1, 1, 1, $2, 1)",
    )
    .bind(id(0xb105))
    .bind(QUESTION)
    .execute(&mut *transaction)
    .await
    .expect("Published Question Pool member");
    transaction.commit().await.expect("fixture commit");
}
