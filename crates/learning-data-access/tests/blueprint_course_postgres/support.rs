//! Shared deterministic fixtures for the connected Blueprint Course oracle.

use super::*;

pub(super) struct FixturePoolIdIssuer(pub(super) AtomicUsize);

impl CourseInstancePoolIdIssuer for FixturePoolIdIssuer {
    fn issue_question_pool_id(&self) -> Result<QuestionId, StoreError> {
        const IDS: [&str; 2] = ["8K3M-X9Q1", "9K3M-X9Q2"];
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

pub(super) async fn create(
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

pub(super) async fn save(
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

pub(super) async fn transition_blueprint_availability(
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
pub(super) const QUESTION: &str = "ABCDEFG3";
pub(super) const QUESTION_POOL: &str = "7654X321";

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
    "7654-X321".parse().expect("closed Pool ID fixture")
}

fn content_input(title: &str) -> CreateBlueprintCourseInput {
    CreateBlueprintCourseInput {
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
                        question_pool_id: question_pool_id(),
                        selection_count: 1,
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
                        question_id: question_id(),
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
                        assessment_attempt_grade_rule:
                            question_model::AssessmentAttemptGradeRule::Latest,
                        question_pool_reuse_rule:
                            question_model::QuestionPoolReuseRule::SelectAgain,
                        question_variation_rule:
                            question_model::AssessmentQuestionVariationRule::ReuseVariation,
                        assessment_attempt_resume_rule:
                            question_model::AssessmentAttemptResumeRule::SingleSession,
                        assessment_question_display_rule:
                            question_model::AssessmentQuestionDisplayRule::AllQuestions,
                        assessment_navigation_rule:
                            question_model::AssessmentNavigationRule::ForwardOnly,
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

pub(super) fn initial_content() -> StoredBlueprintCourseContent {
    let question = question_id();
    let pins = BTreeMap::from([(
        question.clone(),
        QuestionRevisionReference {
            question_id: question,
            revision_number: QuestionRevisionNumber::new(1).expect("fixture Question Revision"),
        },
    )]);
    let pools = BTreeMap::from([(
        question_pool_id(),
        question_model::QuestionPoolRevisionReference {
            question_pool_id: question_pool_id(),
            revision_number: question_model::QuestionPoolRevisionNumber::new(1)
                .expect("fixture Pool Revision"),
        },
    )]);
    StoredBlueprintCourseContent::from_create(
        content_input("Revision one Assessment"),
        &pins,
        &pools,
    )
    .expect("closed Blueprint content fixture")
}

pub(super) fn pins() -> BTreeMap<QuestionId, QuestionRevisionReference> {
    let question = question_id();
    BTreeMap::from([(
        question.clone(),
        QuestionRevisionReference {
            question_id: question,
            revision_number: QuestionRevisionNumber::new(1).expect("fixture Question Revision"),
        },
    )])
}

pub(super) fn assessment_input(title: &str) -> BlueprintAssessmentContentInput {
    content_input(title).modules.remove(0).assessments.remove(0)
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
        "INSERT INTO ple_data.question_pool (\
             question_pool_id, public_question_pool_id, metadata_etag, current_revision_number, created_at\
         ) VALUES ($1, $2, $3, 1, clock_timestamp())",
    )
    .bind(id(0xb105))
    .bind(QUESTION_POOL)
    .bind(id(0xb106))
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
