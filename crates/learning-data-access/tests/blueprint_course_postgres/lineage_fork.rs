//! Connected oracle for ordinary Blueprint parentage, Revision history, and fork retries.

use super::*;
use learning_data_access::{
    StoredBlueprintAssessmentContent, StoredBlueprintAssessmentEntry, StoredBlueprintCourseContent,
};
use question_model::{
    BlueprintAssessmentContentInput, BlueprintAssessmentEditChoice,
    BlueprintAssessmentReplacementInput, BlueprintCourseRevisionTuple, BlueprintModuleEditChoice,
    BlueprintModuleReplacementInput, BlueprintRevisionNumber, ReplaceBlueprintCourseContentInput,
    ReusableFixedQuestionInput, ReusablePoolInput,
};

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn blueprint_forks_are_ordinary_lineages_with_immediate_parent_and_normal_revisions() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    seed_if_needed(&admin).await;
    admin.close().await;

    let application_url = std::env::var("DATABASE_URL").expect("application database URL");
    let application_pool = lazy_pool(&application_url).expect("application pool");
    let course_store = PostgresBlueprintCourseStore::new(application_pool.clone());
    let lineage_store = PostgresBlueprintLineageStore::new(application_pool.clone());

    let created = course_store
        .create_blueprint_course(
            token(),
            RequestChecksum::from_bytes([0xd1; 32]),
            content_input("Source Revision One"),
        )
        .await
        .expect("owner creates the ordinary source Blueprint");
    let source_id = created
        .blueprint_course_revision_tuple
        .blueprint_course_id
        .clone();
    let source_tuple = created.blueprint_course_revision_tuple;
    assert_eq!(
        course_store
            .load_blueprint_course(token(), source_id.clone())
            .await
            .expect("owner reads the new root Blueprint")
            .fork_source_tuple,
        None,
        "an ordinary root Blueprint has no parent tuple"
    );
    course_store
        .publish_blueprint(token(), source_id.clone(), created.blueprint_edit_number)
        .await
        .expect("owner publishes the source Blueprint");

    let request_checksum = RequestChecksum::from_bytes([0xd2; 32]);
    let fork_source = BlueprintForkSource {
        blueprint_course_revision_tuple: source_tuple.clone(),
    };
    let first = lineage_store
        .fork_blueprint_course(token(), fork_source.clone(), request_checksum)
        .await
        .expect("owner forks the exact source Revision");
    let retry = lineage_store
        .fork_blueprint_course(token(), fork_source, request_checksum)
        .await
        .expect("same fork request is replayed");
    assert_eq!(
        retry, first,
        "replay returns the same ordinary Blueprint result"
    );
    let mismatched_retry = lineage_store
        .fork_blueprint_course(
            token(),
            BlueprintForkSource {
                blueprint_course_revision_tuple: BlueprintCourseRevisionTuple {
                    blueprint_course_id: source_id.clone(),
                    revision_number: BlueprintRevisionNumber::new(2)
                        .expect("positive source Revision"),
                },
            },
            request_checksum,
        )
        .await;
    assert!(
        mismatched_retry.is_err(),
        "a request checksum cannot replay against a different source tuple"
    );
    assert_eq!(
        first.blueprint_course_revision_tuple,
        BlueprintCourseRevisionTuple {
            blueprint_course_id: first
                .blueprint_course_revision_tuple
                .blueprint_course_id
                .clone(),
            revision_number: BlueprintRevisionNumber::INITIAL,
        },
        "a fork has its own identity and begins at Revision 1"
    );
    assert_ne!(
        first.blueprint_course_revision_tuple.blueprint_course_id, source_id,
        "the fork is a new ordinary Blueprint lineage"
    );

    let child_id = first
        .blueprint_course_revision_tuple
        .blueprint_course_id
        .clone();
    let child = course_store
        .load_blueprint_course(token(), child_id.clone())
        .await
        .expect("owner reads the fork as an ordinary Blueprint");
    assert_eq!(child.fork_source_tuple, Some(source_tuple.clone()));
    let replacement = replacement_with_title(&child.content, "Fork Revision Two");
    let saved = course_store
        .save_blueprint_course(
            token(),
            child_id.clone(),
            BlueprintRevisionNumber::INITIAL,
            RequestChecksum::from_bytes([0xd3; 32]),
            replacement,
        )
        .await
        .expect("fork accepts an ordinary successor Revision");
    assert!(saved.changed);
    assert_eq!(
        saved.blueprint_course_revision_tuple.blueprint_course_id,
        child_id
    );
    assert_eq!(
        saved
            .blueprint_course_revision_tuple
            .revision_number
            .value(),
        2
    );
    assert_eq!(
        course_store
            .load_blueprint_course(token(), source_id.clone())
            .await
            .expect("source remains independently readable")
            .current_revision_number,
        BlueprintRevisionNumber::INITIAL,
        "saving the fork does not copy or advance parent history"
    );

    let updated_child = course_store
        .load_blueprint_course(token(), child_id.clone())
        .await
        .expect("owner reads the fork's new head");
    course_store
        .publish_blueprint(
            token(),
            child_id.clone(),
            updated_child.blueprint_edit_number,
        )
        .await
        .expect("owner publishes the revised child for its own fork");
    let child_tuple = BlueprintCourseRevisionTuple {
        blueprint_course_id: child_id.clone(),
        revision_number: saved.blueprint_course_revision_tuple.revision_number,
    };
    let grandchild = lineage_store
        .fork_blueprint_course(
            token(),
            BlueprintForkSource {
                blueprint_course_revision_tuple: child_tuple.clone(),
            },
            RequestChecksum::from_bytes([0xd4; 32]),
        )
        .await
        .expect("owner forks the immediate child Revision");
    assert_eq!(
        grandchild.blueprint_course_revision_tuple.revision_number,
        BlueprintRevisionNumber::INITIAL
    );
    let grandchild_view = course_store
        .load_blueprint_course(
            token(),
            grandchild
                .blueprint_course_revision_tuple
                .blueprint_course_id
                .clone(),
        )
        .await
        .expect("owner reads the second-generation Blueprint");
    assert_eq!(grandchild_view.fork_source_tuple, Some(child_tuple));

    application_pool.close().await;
}

fn replacement_with_title(
    content: &StoredBlueprintCourseContent,
    title: &str,
) -> ReplaceBlueprintCourseContentInput {
    ReplaceBlueprintCourseContentInput {
        modules: content
            .modules
            .iter()
            .map(|module| BlueprintModuleReplacementInput {
                choice: BlueprintModuleEditChoice::Retained {
                    blueprint_module_id: module.blueprint_module_id,
                },
                label: module.label.clone(),
                assessments: module
                    .assessments
                    .iter()
                    .map(|assessment| BlueprintAssessmentReplacementInput {
                        choice: BlueprintAssessmentEditChoice::Retained {
                            blueprint_assessment_id: assessment.blueprint_assessment_id,
                        },
                        content: assessment_content_with_title(&assessment.content, title),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn assessment_content_with_title(
    content: &StoredBlueprintAssessmentContent,
    title: &str,
) -> BlueprintAssessmentContentInput {
    BlueprintAssessmentContentInput {
        assessment_type: content.assessment_type,
        title: title.to_owned(),
        instructions: content.instructions.clone(),
        entries: content
            .entries
            .iter()
            .map(|entry| match entry {
                StoredBlueprintAssessmentEntry::Fixed {
                    published_question_revision_tuple,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => question_model::BlueprintAssessmentEntryInput::Fixed(
                    ReusableFixedQuestionInput {
                        published_question_revision_tuple: published_question_revision_tuple
                            .clone(),
                        points_possible: *points_possible,
                        scoring_rule: *scoring_rule,
                        question_attempt_limit: *question_attempt_limit,
                        question_attempt_time_limit: *question_attempt_time_limit,
                    },
                ),
                StoredBlueprintAssessmentEntry::Pool {
                    question_pool_id,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => question_model::BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                    question_pool_id: question_pool_id.clone(),
                    selection_count: *selection_count,
                    points_per_item: *points_per_item,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                }),
            })
            .collect(),
        defaults: content.defaults.clone(),
    }
}
