//! Co-Instructor ownership proof for a later Assessment Blueprint update.

use super::*;

pub(super) async fn assert_later_apply_uses_current_instructor(
    application_pool: sqlx::PgPool,
    course_instance_id: &str,
    co_instructor_membership_id: uuid::Uuid,
) {
    let mut inspection = adoption_inspection_connection().await;
    let assessment_id: String = sqlx::query_scalar(
        "SELECT assessment_id FROM ple_data.assessment \
          WHERE course_instance_id = $1 AND origin_kind = 'adopted'",
    )
    .bind(course_instance_id)
    .fetch_one(&mut inspection)
    .await
    .expect("creator Course adopted Assessment");
    let (old_pool_id, old_pool_owner): (String, String) = sqlx::query_as(
        "SELECT pool.question_pool_id, pool.owner_account_id FROM ple_data.assessment_entry AS entry \
          JOIN ple_data.assessment_entry_pool AS pool_entry \
            ON pool_entry.assessment_entry_id = entry.assessment_entry_id \
          JOIN ple_data.question_pool AS pool ON pool.question_pool_id = pool_entry.question_pool_id \
         WHERE entry.assessment_id = $1 AND entry.entry_kind = 'question_pool'",
    )
    .bind(&assessment_id)
    .fetch_one(&mut inspection)
    .await
    .expect("original adopted Pool reference");
    let live_assessments = PostgresLiveAssessmentStore::new(application_pool);
    let course_instance_id: question_model::CourseInstanceId =
        course_instance_id.parse().expect("creator Course ID");
    let assessment_id: question_model::AssessmentId =
        assessment_id.parse().expect("creator Assessment ID");
    let review = live_assessments
        .review_assessment_blueprint_update(
            reader_token(),
            course_instance_id.clone(),
            assessment_id.clone(),
        )
        .await
        .expect("active co-Instructor reviews current Blueprint update");
    assert_eq!(
        review
            .source_blueprint_course_revision_tuple
            .revision_number,
        BlueprintRevisionNumber::new(2).expect("Revision two"),
        "co-Instructor review reads the later source Revision"
    );
    let applied = live_assessments
        .apply_assessment_blueprint_update(
            reader_token(),
            course_instance_id,
            assessment_id.clone(),
            ApplyAssessmentBlueprintUpdateInput {
                expected_source_blueprint_course_revision_tuple: review
                    .source_blueprint_course_revision_tuple,
                expected_assessment_edit_number: review.assessment.assessment_edit_number,
            },
        )
        .await
        .expect("active co-Instructor applies the later Blueprint update");
    assert!(
        applied.assessment_edit_number > review.assessment.assessment_edit_number,
        "changed source update advances the live Assessment Edit Number"
    );
    let (new_pool_id, new_pool_owner, applying_instructor): (String, String, String) =
        sqlx::query_as(
            "SELECT pool.question_pool_id, pool.owner_account_id, membership.account_id \
               FROM ple_data.assessment_entry AS entry \
               JOIN ple_data.assessment_entry_pool AS pool_entry \
                 ON pool_entry.assessment_entry_id = entry.assessment_entry_id \
               JOIN ple_data.question_pool AS pool ON pool.question_pool_id = pool_entry.question_pool_id \
               JOIN ple_data.course_membership AS membership \
                 ON membership.course_membership_id = $2 \
              WHERE entry.assessment_id = $1 AND entry.entry_kind = 'question_pool'",
        )
        .bind(assessment_id.as_string())
        .bind(co_instructor_membership_id)
        .fetch_one(&mut inspection)
        .await
        .expect("updated Assessment Pool reference and applying co-Instructor");
    assert_eq!(
        new_pool_id, old_pool_id,
        "applying a changed Blueprint Assessment preserves its referenced Pool"
    );
    assert_eq!(
        new_pool_owner, old_pool_owner,
        "applying a changed Blueprint Assessment preserves the Pool owner"
    );
    assert_ne!(
        new_pool_owner, applying_instructor,
        "applying as a co-Instructor does not take ownership of the referenced Pool"
    );
    inspection
        .close()
        .await
        .expect("co-Instructor inspection close");
}
