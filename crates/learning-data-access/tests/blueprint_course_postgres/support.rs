//! Shared deterministic fixtures for the connected Blueprint Course oracle.

use super::*;

pub(super) const INSTRUCTOR: u128 = 0xb100;
pub(super) const SESSION: u128 = 0xb101;
pub(super) const READER_INSTRUCTOR: u128 = 0xb102;
pub(super) const READER_SESSION: u128 = 0xb103;
pub(super) const QUESTION: &str = "ABCDEFG3";

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

fn content_input(title: &str) -> CreateBlueprintCourseInput {
    CreateBlueprintCourseInput {
        short_name: "REV-ACC".to_owned(),
        long_name: "Revision acceptance Blueprint".to_owned(),
        modules: vec![CreateBlueprintModuleInput {
            label: "Module alpha".to_owned(),
            assessments: vec![BlueprintAssessmentContentInput {
                title: title.to_owned(),
                instructions: AssessmentInstructions::try_new("Read the prompt.".to_owned())
                    .expect("fixture instructions"),
                entries: vec![
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
                    BlueprintAssessmentEntryInput::Pool(question_model::ReusablePoolInput {
                        items: vec![question_id()],
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
                ],
                defaults: BlueprintAssessmentDefaults {
                    assessment_attempt_time_limit_seconds: std::num::NonZeroU32::new(900),
                    attempt_limit: std::num::NonZeroU32::new(3),
                    late_work_rule: LateWorkRule::MarkLate,
                    activity_rules: AssessmentActivityRules {
                        assessment_completion_rule:
                            question_model::AssessmentCompletionRule::ScoreAtLeast {
                                fraction: 0.75,
                            },
                        assessment_attempt_grade_rule:
                            question_model::AssessmentAttemptGradeRule::Latest,
                        assessment_attempt_continuation_rule:
                            question_model::AssessmentAttemptContinuationRule::Capped {
                                max_additional_assessment_attempts: 2,
                            },
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
                        question_feedback: question_model::StudentFeedbackReleaseTiming::AfterClose,
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
    StoredBlueprintCourseContent::from_create(content_input("Revision one Assessment"), &pins)
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
    transaction.commit().await.expect("fixture commit");
}
