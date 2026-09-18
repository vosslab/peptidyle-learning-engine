//! Ordinary Store-backed creation of the fixed Live Demo Blueprint Course.

use anyhow::{Context, Result, ensure};
use learning_data_access::{
    BlueprintCourseStore, SessionTokenHash, StoredBlueprintAssessmentEntry,
    StoredBlueprintCourseContent,
    postgres::{PostgresBlueprintCourseStore, lazy_pool},
};
use question_model::{
    AssessmentActivityRules, AssessmentEntryScoringRule, AssessmentInstructions,
    AssessmentPointValue, BlueprintAssessmentContentInput, BlueprintAssessmentDefaults,
    BlueprintAssessmentEntryInput, BlueprintAvailability, BlueprintRevision,
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, LateWorkRule, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionRevisionReference, RequestChecksum,
    ReusableFixedQuestionInput,
};

use crate::{
    installation_data::{
        LIVE_DEMO_ASSESSMENT_TITLE, LIVE_DEMO_COURSE_LONG_NAME, LIVE_DEMO_COURSE_SHORT_NAME,
    },
    pilot_content,
};

const LIVE_DEMO_BLUEPRINT_MODULE_LABEL: &str = "Chapter 1 reviewed practice";
const LIVE_DEMO_BLUEPRINT_INSTRUCTIONS: &str =
    "Complete the four reviewed Chapter 1 practice questions.";
const LIVE_DEMO_BLUEPRINT_REQUEST_CHECKSUM: RequestChecksum = RequestChecksum::from_bytes([
    0x8e, 0x44, 0x1b, 0xad, 0x29, 0x3e, 0x23, 0x32, 0x13, 0x22, 0x1a, 0xa4, 0xd5, 0x6e, 0xd2, 0xb5,
    0x02, 0xc8, 0x9c, 0xb7, 0xd5, 0x55, 0x26, 0x9f, 0x94, 0xfc, 0x45, 0x3a, 0x1c, 0xbe, 0x0f, 0x6c,
]);

/// Store-generated references consumed by the dependent Live Demo SQL graph.
pub(crate) struct LiveDemoBlueprintManifestReferences {
    pub(crate) blueprint_public_reference: String,
    pub(crate) assessment_reference: String,
}

/// Creates, or replays, and publishes the immutable Revision 1 used by the Live Demo.
///
/// The input is built only from the validated Pilot publication mapping. The
/// Store owns Question pinning, child identities, creation receipts, and the
/// authenticated Instructor transaction.
pub(crate) fn create_live_demo_blueprint(
    session: SessionTokenHash,
    publications: &str,
) -> Result<LiveDemoBlueprintManifestReferences> {
    let questions = pilot_content::validated_ple_question_json_revisions(publications)
        .context("resolving the reviewed PLE Question JSON Pilot publications")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the Live Demo Blueprint runtime")?;
    runtime.block_on(async move {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be set for Live Demo Blueprint creation")?;
        let pool =
            lazy_pool(&database_url).context("Live Demo Blueprint database URL is invalid")?;
        let classification_store =
            learning_data_access::postgres::PostgresContentClassificationStore::new(pool.clone());
        // Authored Course metadata, not inferred from its Question collection.
        let resolved = pilot_content::AuthoredClassification {
            discipline: "Biology".to_owned(),
            subject: "Biochemistry".to_owned(),
            topic: None,
            subtopic: None,
        }
        .resolve(&classification_store, session)
        .await?;
        let classification = question_model::CourseClassification {
            discipline_uuid: resolved.discipline_uuid,
            subject_uuid: Some(resolved.subject_uuid),
            topic_uuid: resolved.topic_uuid,
            subtopic_uuid: resolved.subtopic_uuid,
            tags: Vec::new(),
        };
        let input = live_demo_blueprint_input(questions.clone(), classification.clone())?;
        let store = PostgresBlueprintCourseStore::new(pool);
        let receipt = store
            .create_blueprint_course(
                session,
                LIVE_DEMO_BLUEPRINT_REQUEST_CHECKSUM,
                input.clone(),
                Default::default(),
            )
            .await
            .context("creating the ordinary Live Demo Blueprint Course")?;
        ensure!(
            receipt.blueprint_revision.revision == BlueprintRevision::INITIAL,
            "Live Demo Blueprint creation did not return Revision 1"
        );
        let blueprint = store
            .load_blueprint_course(session, receipt.blueprint_revision.reference.clone())
            .await
            .context("reloading the ordinary Live Demo Blueprint Course")?;
        ensure!(
            matches!(
                blueprint.availability,
                BlueprintAvailability::Private | BlueprintAvailability::Public
            ),
            "Live Demo Blueprint is neither Private nor Public"
        );
        ensure!(
            blueprint.current_revision == BlueprintRevision::INITIAL,
            "Live Demo Blueprint current Revision is not Revision 1"
        );
        ensure!(
            blueprint.short_name == LIVE_DEMO_COURSE_SHORT_NAME
                && blueprint.long_name == LIVE_DEMO_COURSE_LONG_NAME,
            "Live Demo Blueprint lineage names differ from the fixed course names"
        );
        let assessment_reference = validate_loaded_content(&blueprint.content, &input, &questions)?;
        ensure!(
            blueprint.classification == classification,
            "Live Demo Blueprint classification conflicts with the authored Course"
        );
        if blueprint.availability == BlueprintAvailability::Private {
            store
                .publish_blueprint(
                    session,
                    receipt.blueprint_revision.reference.clone(),
                    blueprint.blueprint_edit_number,
                )
                .await
                .context("publishing the ordinary Live Demo Blueprint Course")?;
        }
        let blueprint = store
            .load_blueprint_course(session, receipt.blueprint_revision.reference.clone())
            .await
            .context("reloading the published Live Demo Blueprint Course")?;
        ensure!(
            blueprint.availability == BlueprintAvailability::Public,
            "Live Demo Blueprint is not Public before Course adoption"
        );
        Ok(LiveDemoBlueprintManifestReferences {
            blueprint_public_reference: receipt.blueprint_revision.reference.to_string(),
            assessment_reference: assessment_reference.to_string(),
        })
    })
}

/// Compares every authored field after a create receipt replay. The Store owns
/// the module and Assessment UUIDs, so this deliberately ignores only those
/// two generated identities while rejecting all other semantic drift.
fn validate_loaded_content(
    content: &StoredBlueprintCourseContent,
    expected: &CreateBlueprintCourseInput,
    questions: &[QuestionRevisionReference],
) -> Result<question_model::BlueprintAssessmentId> {
    ensure!(
        content.modules.len() == 1 && expected.modules.len() == 1,
        "Live Demo Blueprint must contain exactly one Module"
    );
    let actual_module = &content.modules[0];
    let expected_module = &expected.modules[0];
    ensure!(
        actual_module.label == expected_module.label
            && actual_module.assessments.len() == 1
            && expected_module.assessments.len() == 1,
        "Live Demo Blueprint Module content differs from the fixed definition"
    );
    let actual_assessment = &actual_module.assessments[0];
    let expected_assessment = &expected_module.assessments[0];
    ensure!(
        actual_assessment.content.title == expected_assessment.title
            && actual_assessment.content.instructions == expected_assessment.instructions
            && actual_assessment.content.defaults == expected_assessment.defaults,
        "Live Demo Blueprint Assessment content differs from the fixed definition"
    );
    ensure!(
        actual_assessment.content.entries.len() == expected_assessment.entries.len()
            && actual_assessment.content.entries.len() == questions.len(),
        "Live Demo Blueprint Question entries differ from the fixed definition"
    );
    for ((actual, expected), question) in actual_assessment
        .content
        .entries
        .iter()
        .zip(&expected_assessment.entries)
        .zip(questions)
    {
        let (
            StoredBlueprintAssessmentEntry::Fixed {
                question_revision,
                points_possible: actual_points,
                scoring_rule: actual_scoring,
                question_attempt_limit: actual_attempt_limit,
                question_attempt_time_limit: actual_time_limit,
            },
            BlueprintAssessmentEntryInput::Fixed(expected_fixed),
        ) = (actual, expected)
        else {
            anyhow::bail!("Live Demo Blueprint entries must remain fixed Questions");
        };
        ensure!(
            question_revision == question && &expected_fixed.published_question == question,
            "Live Demo Blueprint Question pins differ from the reviewed Pilot publications"
        );
        ensure!(
            actual_points == &expected_fixed.points_possible
                && actual_scoring == &expected_fixed.scoring_rule
                && actual_attempt_limit == &expected_fixed.question_attempt_limit
                && actual_time_limit == &expected_fixed.question_attempt_time_limit,
            "Live Demo Blueprint Question policy differs from the fixed definition"
        );
    }
    Ok(actual_assessment.blueprint_assessment_reference)
}

fn live_demo_blueprint_input(
    questions: Vec<QuestionRevisionReference>,
    classification: question_model::CourseClassification,
) -> Result<CreateBlueprintCourseInput> {
    ensure!(
        questions.len() == 4,
        "Live Demo Blueprint requires exactly four PLE Question JSON publications"
    );
    let input = CreateBlueprintCourseInput {
        classification,
        short_name: LIVE_DEMO_COURSE_SHORT_NAME.to_owned(),
        long_name: LIVE_DEMO_COURSE_LONG_NAME.to_owned(),
        modules: vec![CreateBlueprintModuleInput {
            label: LIVE_DEMO_BLUEPRINT_MODULE_LABEL.to_owned(),
            assessments: vec![BlueprintAssessmentContentInput {
                assessment_type: question_model::AssessmentType::PracticeQuestionAssignment,
                title: LIVE_DEMO_ASSESSMENT_TITLE.to_owned(),
                instructions: AssessmentInstructions::try_new(
                    LIVE_DEMO_BLUEPRINT_INSTRUCTIONS.to_owned(),
                )
                .expect("the fixed Live Demo instructions are valid"),
                entries: questions
                    .into_iter()
                    .map(|question| {
                        BlueprintAssessmentEntryInput::Fixed(ReusableFixedQuestionInput {
                            published_question: question,
                            points_possible: AssessmentPointValue::from_whole(1),
                            scoring_rule: AssessmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        })
                    })
                    .collect(),
                defaults: BlueprintAssessmentDefaults {
                    assessment_attempt_time_limit_seconds: None,
                    attempt_limit: None,
                    late_work_rule: LateWorkRule::Accept,
                    activity_rules: AssessmentActivityRules::default(),
                    student_feedback_release_rule: Default::default(),
                },
            }],
        }],
    };
    input
        .validate()
        .map_err(|error| anyhow::anyhow!("fixed Live Demo Blueprint input is invalid: {error}"))?;
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use learning_data_access::{
        StoredBlueprintAssessment, StoredBlueprintAssessmentContent, StoredBlueprintModule,
    };
    use question_model::{BlueprintAssessmentId, BlueprintModuleReference};
    use uuid::Uuid;

    fn question(number: u8) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: question_model::QuestionId::from_random_identifier(format!(
                "ABCDEF{number}"
            ))
            .expect("test Question ID is valid"),
            revision_number: question_model::QuestionRevisionNumber::new(1)
                .expect("test Revision is valid"),
        }
    }

    #[test]
    fn live_demo_input_contains_one_assessment_with_four_fixed_questions() {
        let input = live_demo_blueprint_input(
            (1..=4).map(question).collect(),
            question_model::CourseClassification {
                discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(input.modules.len(), 1);
        assert_eq!(input.modules[0].assessments.len(), 1);
        assert_eq!(
            input.modules[0].assessments[0].assessment_type,
            question_model::AssessmentType::PracticeQuestionAssignment
        );
        assert_eq!(input.modules[0].assessments[0].entries.len(), 4);
        assert_eq!(input.modules[0].label, LIVE_DEMO_BLUEPRINT_MODULE_LABEL);
    }

    #[test]
    fn live_demo_input_rejects_an_incomplete_pilot_mapping() {
        assert!(
            live_demo_blueprint_input(
                (1..=3).map(question).collect(),
                question_model::CourseClassification {
                    discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                    subject_uuid: None,
                    topic_uuid: None,
                    subtopic_uuid: None,
                    tags: Vec::new()
                }
            )
            .is_err()
        );
    }

    fn stored_content(
        input: &CreateBlueprintCourseInput,
        questions: &[QuestionRevisionReference],
    ) -> StoredBlueprintCourseContent {
        let assessment = &input.modules[0].assessments[0];
        StoredBlueprintCourseContent {
            modules: vec![StoredBlueprintModule {
                blueprint_module_reference: BlueprintModuleReference::from_uuid(Uuid::from_u128(1)),
                label: input.modules[0].label.clone(),
                assessments: vec![StoredBlueprintAssessment {
                    blueprint_assessment_reference: BlueprintAssessmentId::from_uuid(
                        Uuid::from_u128(2),
                    ),
                    content: StoredBlueprintAssessmentContent {
                        assessment_type: assessment.assessment_type,
                        title: assessment.title.clone(),
                        instructions: assessment.instructions.clone(),
                        entries: assessment
                            .entries
                            .iter()
                            .zip(questions)
                            .map(|(entry, question)| match entry {
                                BlueprintAssessmentEntryInput::Fixed(fixed) => {
                                    StoredBlueprintAssessmentEntry::Fixed {
                                        question_revision: question.clone(),
                                        points_possible: fixed.points_possible,
                                        scoring_rule: fixed.scoring_rule,
                                        question_attempt_limit: fixed.question_attempt_limit,
                                        question_attempt_time_limit: fixed
                                            .question_attempt_time_limit,
                                    }
                                }
                                BlueprintAssessmentEntryInput::Pool(_) => {
                                    panic!("fixed Live Demo input does not contain Question Pools")
                                }
                            })
                            .collect(),
                        defaults: assessment.defaults.clone(),
                    },
                }],
            }],
        }
    }

    #[test]
    fn loaded_content_allows_only_store_generated_child_identity_differences() {
        let questions = (1..=4).map(question).collect::<Vec<_>>();
        let input = live_demo_blueprint_input(
            questions.clone(),
            question_model::CourseClassification {
                discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
        )
        .unwrap();
        let content = stored_content(&input, &questions);

        let assessment = validate_loaded_content(&content, &input, &questions).unwrap();

        assert_eq!(assessment.as_uuid(), Uuid::from_u128(2));
    }

    #[test]
    fn loaded_content_rejects_replayed_semantic_drift() {
        let questions = (1..=4).map(question).collect::<Vec<_>>();
        let input = live_demo_blueprint_input(
            questions.clone(),
            question_model::CourseClassification {
                discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                tags: Vec::new(),
            },
        )
        .unwrap();
        let mut content = stored_content(&input, &questions);
        content.modules[0].assessments[0].content.title = "Changed title".to_owned();

        assert!(validate_loaded_content(&content, &input, &questions).is_err());
    }
}
