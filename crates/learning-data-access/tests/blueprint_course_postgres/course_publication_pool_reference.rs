//! Connected PostgreSQL oracle for ordinary Course-to-Blueprint Pool references.

use super::*;
use learning_data_access::postgres::{
    PostgresCourseBlueprintPublicationStore, PostgresQuestionPoolCreationStore,
};
use learning_data_access::{
    CourseBlueprintPublicationStore, CourseInstanceCreationSource, CreateCourseInstanceInput,
    CreateQuestionPoolInput, QuestionPoolCreationStore, StoredBlueprintAssessmentEntry,
};
use question_model::{
    AssessmentEntry, AssessmentEntryAvailability, AssessmentEntryId, AssessmentEntryScoringRule,
    AssessmentInstructions, AssessmentPointValue, AssessmentTitle, AssessmentType,
    CourseClassification, CreateBlueprintFromCourseInstanceInput, PublishedQuestionId,
    PublishedQuestionRevisionTuple, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionPoolAssessmentEntry, QuestionPoolEditNumber, QuestionRevisionNumber, RequestChecksum,
};
use std::num::NonZeroU32;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn course_publication_preserves_an_ordinary_pool_reference() {
    let fixture_nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos()
        .to_string();
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let migration_url = runtime.migration_url().expose();
    let admin = lazy_pool(migration_url).expect("migration pool");
    seed_if_needed(&admin).await;
    admin.close().await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&application_url).expect("application pool");
    let question_id: PublishedQuestionId = QUESTION.parse().expect("fixture Question ID");
    let pool_identifier = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_OID,
        format!("{fixture_nonce}:pool").as_bytes(),
    )
    .simple()
    .to_string()
    .to_uppercase();
    let pool_id = question_model::QuestionPoolId::from_random_identifier(&pool_identifier[..7])
        .expect("ordinary Pool ID");
    let pool_created = PostgresQuestionPoolCreationStore::new(application.clone())
        .create_question_pool(
            token(),
            CreateQuestionPoolInput {
                title: "M11 Course publication Pool".to_owned(),
                description: "Ordinary Pool referenced by a Course Assessment".to_owned(),
                question_pool_id: pool_id.clone(),
                members: vec![PublishedQuestionRevisionTuple {
                    published_question_id: question_id,
                    revision_number: QuestionRevisionNumber::new(1).expect("Question Revision"),
                }],
                tags: Vec::new(),
            },
        )
        .await
        .expect("create ordinary Pool through its Store");
    assert_eq!(pool_created.question_pool_id, pool_id);

    let course_store = PostgresCourseInstanceStore::new(application.clone());
    let course = course_store
        .create_course_instance(
            token(),
            CreateCourseInstanceInput {
                classification: CourseClassification {
                    discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                    subject_uuid: None,
                    topic_uuid: None,
                    subtopic_uuid: None,
                    tags: Vec::new(),
                },
                source: CourseInstanceCreationSource::Empty,
                short_name: "M11-PUB".to_owned(),
                long_name: "M11 Course Publication Pool Reference".to_owned(),
                term: near_now_term(migration_url).await,
                assigned_instructor_account_id: None,
            },
        )
        .await
        .expect("create Empty Course through its Store")
        .course_instance;
    let assessment_store = PostgresLiveAssessmentStore::new(application.clone());
    let assessment = assessment_store
        .create_live_assessment(
            token(),
            course.id.clone(),
            learning_data_access::CreateLiveAssessmentInput {
                assessment_type: AssessmentType::RegularAssignment,
                title: AssessmentTitle::try_new("Pool reference publication".to_owned())
                    .expect("Assessment title"),
                instructions: AssessmentInstructions::default(),
            },
        )
        .await
        .expect("create Assessment through its Store");
    let saved_assessment = assessment_store
        .save_live_assessment(
            token(),
            course.id.clone(),
            assessment.id.clone(),
            learning_data_access::SaveLiveAssessmentInput {
                expected_assessment_edit_number: assessment.assessment_edit_number,
                title: assessment.title.clone(),
                instructions: AssessmentInstructions::default(),
                due_at: None,
                available_at: None,
                closes_at: None,
                late_work_rule: question_model::LateWorkRule::Reject,
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                activity_rules: question_model::AssessmentActivityRules::default(),
                student_feedback_release_rule: question_model::StudentFeedbackReleaseRule::default(
                ),
                entries: vec![AssessmentEntry::QuestionPool(QuestionPoolAssessmentEntry {
                    id: AssessmentEntryId::from_uuid(uuid::Uuid::new_v5(
                        &uuid::Uuid::NAMESPACE_OID,
                        format!("{fixture_nonce}:assessment-entry").as_bytes(),
                    )),
                    question_pool_id: pool_id.clone(),
                    question_pool_edit_number: QuestionPoolEditNumber::new(1)
                        .expect("Pool Edit Number"),
                    availability: AssessmentEntryAvailability::Available,
                    scoring_rule: AssessmentEntryScoringRule::Normal,
                    selection_count: NonZeroU32::new(2).expect("selection count"),
                    points_per_item: AssessmentPointValue::from_whole(1),
                    question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                    question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                })],
            },
        )
        .await
        .expect("save Pool reference through the Assessment Store");
    assert_eq!(
        saved_assessment.status,
        question_model::AssessmentStatus::Unreleased
    );

    let mut inspection = adoption_inspection_connection().await;
    let (source_owner_before, child_forks_before): (String, i64) = sqlx::query_as(
        "SELECT pool.owner_account_id::text, \
                (SELECT count(*) FROM ple_data.question_pool AS child \
                  WHERE child.source_question_pool_id = pool.question_pool_id) \
           FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = $1",
    )
    .bind(pool_id.as_str())
    .fetch_one(&mut inspection)
    .await
    .expect("source Pool owner and fork count before publication");
    assert_eq!(
        child_forks_before, 0,
        "new ordinary Pool has no source-linked children"
    );

    let publication = PostgresCourseBlueprintPublicationStore::new(application.clone());
    let checksum_first = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_OID,
        format!("{fixture_nonce}:request-checksum:first").as_bytes(),
    );
    let checksum_second = uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_OID,
        format!("{fixture_nonce}:request-checksum:second").as_bytes(),
    );
    let mut checksum_bytes = [0; 32];
    checksum_bytes[..16].copy_from_slice(checksum_first.as_bytes());
    checksum_bytes[16..].copy_from_slice(checksum_second.as_bytes());
    let receipt = publication
        .create_blueprint_from_course_instance(
            token(),
            course.id.clone(),
            RequestChecksum::from_bytes(checksum_bytes),
            CreateBlueprintFromCourseInstanceInput {
                classification: CourseClassification {
                    discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                    subject_uuid: None,
                    topic_uuid: None,
                    subtopic_uuid: None,
                    tags: Vec::new(),
                },
                short_name: "M11-BLUE".to_owned(),
                long_name: "M11 Published Course Structure".to_owned(),
            },
        )
        .await
        .expect("publish Course through the publication Store");

    let blueprints = PostgresBlueprintCourseStore::new(application.clone());
    let blueprint = blueprints
        .load_blueprint_course(
            token(),
            receipt
                .blueprint_course_revision_tuple
                .blueprint_course_id
                .clone(),
        )
        .await
        .expect("reload persisted Blueprint through its Store");
    let pool_entry = &blueprint.content.modules[0].assessments[0].content.entries[0];
    match pool_entry {
        StoredBlueprintAssessmentEntry::Pool {
            question_pool_id: persisted_pool_id,
            selection_count,
            ..
        } => {
            assert_eq!(
                persisted_pool_id, &pool_id,
                "publication preserves the ordinary Pool ID"
            );
            assert_eq!(
                selection_count.get(),
                2,
                "publication preserves requested selection count"
            );
        }
        other => panic!("published Assessment entry is not a Pool reference: {other:?}"),
    }

    let course_id = course.id.as_string();
    let mut provenance_inspection = inspection
        .begin()
        .await
        .expect("Blueprint provenance inspection transaction");
    // FORCE RLS grants provenance reads to the API owner, not the data owner.
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *provenance_inspection)
        .await
        .expect("Blueprint provenance inspection role");
    let source_course: String = sqlx::query_scalar(
        "SELECT source.source_course_instance_id::text \
           FROM ple_data.blueprint_course_instance_source AS source \
          WHERE source.blueprint_course_id = $1",
    )
    .bind(blueprint.id.as_string())
    .fetch_one(&mut *provenance_inspection)
    .await
    .expect("Blueprint source Course provenance");
    provenance_inspection
        .commit()
        .await
        .expect("Blueprint provenance inspection commit");
    assert_eq!(
        source_course, course_id,
        "Blueprint records its source Course"
    );

    let (source_owner_after, child_forks_after): (String, i64) = sqlx::query_as(
        "SELECT pool.owner_account_id::text, \
                (SELECT count(*) FROM ple_data.question_pool AS child \
                  WHERE child.source_question_pool_id = pool.question_pool_id) \
           FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = $1",
    )
    .bind(pool_id.as_str())
    .fetch_one(&mut inspection)
    .await
    .expect("source Pool owner and fork count after publication");
    assert_eq!(
        source_owner_after, source_owner_before,
        "publication leaves Pool owner unchanged"
    );
    assert_eq!(
        source_owner_after,
        instructor_account_id(),
        "source Pool remains Instructor-owned"
    );
    assert_eq!(
        child_forks_after, child_forks_before,
        "publication creates no source-linked Pool fork"
    );
    inspection
        .close()
        .await
        .expect("inspection connection close");
    application.close().await;
}
