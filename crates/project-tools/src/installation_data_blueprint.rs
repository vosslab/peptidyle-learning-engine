//! Ordinary Store-backed creation of the fixed Live Demo Blueprint Course.

use anyhow::{Context, Result, ensure};
use learning_data_access::{
    BlueprintCourseStore, SessionTokenHash, StoredBlueprintAssignmentEntry,
    StoredBlueprintCourseContent,
    postgres::{PostgresBlueprintCourseStore, lazy_pool},
};
use question_model::{
    AssignmentActivityRules, AssignmentEntryScoringRule, AssignmentInstructions,
    AssignmentPointValue, BlueprintAssignmentContentInput, BlueprintAssignmentDefaults,
    BlueprintAssignmentEntryInput, BlueprintAvailability, BlueprintRevision,
    CreateBlueprintCourseInput, CreateBlueprintModuleInput, LateWorkRule, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionRevisionReference, RequestChecksum,
    ReusableFixedQuestionInput,
};

use crate::{
    installation_data::{
        LIVE_DEMO_ASSIGNMENT_TITLE, LIVE_DEMO_COURSE_LONG_NAME, LIVE_DEMO_COURSE_SHORT_NAME,
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
    pub(crate) blueprint_reference: String,
    pub(crate) assignment_reference: String,
}

/// Creates, or replays, the immutable Revision 1 used by the Live Demo.
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
    let input = live_demo_blueprint_input(questions.clone())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the Live Demo Blueprint runtime")?;
    runtime.block_on(async move {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be set for Live Demo Blueprint creation")?;
        let pool =
            lazy_pool(&database_url).context("Live Demo Blueprint database URL is invalid")?;
        let store = PostgresBlueprintCourseStore::new(pool);
        let receipt = store
            .create_blueprint_course(session, LIVE_DEMO_BLUEPRINT_REQUEST_CHECKSUM, input.clone())
            .await
            .context("creating the ordinary Live Demo Blueprint Course")?;
        ensure!(
            receipt.blueprint_revision.revision == BlueprintRevision::INITIAL,
            "Live Demo Blueprint creation did not return Revision 1"
        );
        let blueprint = store
            .load_blueprint_course(session, receipt.blueprint_revision.reference)
            .await
            .context("reloading the ordinary Live Demo Blueprint Course")?;
        ensure!(
            blueprint.availability == BlueprintAvailability::Available,
            "Live Demo Blueprint Revision 1 is not Available"
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
        let assignment_reference = validate_loaded_content(&blueprint.content, &input, &questions)?;
        Ok(LiveDemoBlueprintManifestReferences {
            blueprint_reference: receipt.blueprint_revision.reference.number().to_string(),
            assignment_reference: assignment_reference.to_string(),
        })
    })
}

/// Compares every authored field after a create receipt replay. The Store owns
/// the module and Assignment UUIDs, so this deliberately ignores only those
/// two generated identities while rejecting all other semantic drift.
fn validate_loaded_content(
    content: &StoredBlueprintCourseContent,
    expected: &CreateBlueprintCourseInput,
    questions: &[QuestionRevisionReference],
) -> Result<question_model::BlueprintAssignmentReference> {
    ensure!(
        content.modules.len() == 1 && expected.modules.len() == 1,
        "Live Demo Blueprint must contain exactly one Module"
    );
    let actual_module = &content.modules[0];
    let expected_module = &expected.modules[0];
    ensure!(
        actual_module.label == expected_module.label
            && actual_module.assignments.len() == 1
            && expected_module.assignments.len() == 1,
        "Live Demo Blueprint Module content differs from the fixed definition"
    );
    let actual_assignment = &actual_module.assignments[0];
    let expected_assignment = &expected_module.assignments[0];
    ensure!(
        actual_assignment.content.title == expected_assignment.title
            && actual_assignment.content.instructions == expected_assignment.instructions
            && actual_assignment.content.defaults == expected_assignment.defaults,
        "Live Demo Blueprint Assignment content differs from the fixed definition"
    );
    ensure!(
        actual_assignment.content.entries.len() == expected_assignment.entries.len()
            && actual_assignment.content.entries.len() == questions.len(),
        "Live Demo Blueprint Question entries differ from the fixed definition"
    );
    for ((actual, expected), question) in actual_assignment
        .content
        .entries
        .iter()
        .zip(&expected_assignment.entries)
        .zip(questions)
    {
        let (
            StoredBlueprintAssignmentEntry::Fixed {
                question_revision,
                points_possible: actual_points,
                scoring_rule: actual_scoring,
                question_attempt_limit: actual_attempt_limit,
                question_attempt_time_limit: actual_time_limit,
            },
            BlueprintAssignmentEntryInput::Fixed(expected_fixed),
        ) = (actual, expected)
        else {
            anyhow::bail!("Live Demo Blueprint entries must remain fixed Questions");
        };
        ensure!(
            question_revision == question && expected_fixed.question_id == question.question_id,
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
    Ok(actual_assignment.blueprint_assignment_reference)
}

fn live_demo_blueprint_input(
    questions: Vec<QuestionRevisionReference>,
) -> Result<CreateBlueprintCourseInput> {
    ensure!(
        questions.len() == 4,
        "Live Demo Blueprint requires exactly four PLE Question JSON publications"
    );
    let input = CreateBlueprintCourseInput {
        short_name: LIVE_DEMO_COURSE_SHORT_NAME.to_owned(),
        long_name: LIVE_DEMO_COURSE_LONG_NAME.to_owned(),
        modules: vec![CreateBlueprintModuleInput {
            label: LIVE_DEMO_BLUEPRINT_MODULE_LABEL.to_owned(),
            assignments: vec![BlueprintAssignmentContentInput {
                title: LIVE_DEMO_ASSIGNMENT_TITLE.to_owned(),
                instructions: AssignmentInstructions::try_new(
                    LIVE_DEMO_BLUEPRINT_INSTRUCTIONS.to_owned(),
                )
                .expect("the fixed Live Demo instructions are valid"),
                entries: questions
                    .into_iter()
                    .map(|question| {
                        BlueprintAssignmentEntryInput::Fixed(ReusableFixedQuestionInput {
                            question_id: question.question_id,
                            points_possible: AssignmentPointValue::from_whole(1),
                            scoring_rule: AssignmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        })
                    })
                    .collect(),
                defaults: BlueprintAssignmentDefaults {
                    assignment_attempt_time_limit_seconds: None,
                    attempt_limit: None,
                    late_work_rule: LateWorkRule::Accept,
                    activity_rules: AssignmentActivityRules::default(),
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
        StoredBlueprintAssignment, StoredBlueprintAssignmentContent, StoredBlueprintModule,
    };
    use question_model::{BlueprintAssignmentReference, BlueprintModuleReference};
    use uuid::Uuid;

    fn question(number: u8) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: format!("A{number}B-CDEF")
                .parse()
                .expect("test Question ID is valid"),
            revision_number: question_model::QuestionRevisionNumber::new(1)
                .expect("test Revision is valid"),
        }
    }

    #[test]
    fn live_demo_input_contains_one_assignment_with_four_fixed_questions() {
        let input = live_demo_blueprint_input((1..=4).map(question).collect()).unwrap();
        assert_eq!(input.modules.len(), 1);
        assert_eq!(input.modules[0].assignments.len(), 1);
        assert_eq!(input.modules[0].assignments[0].entries.len(), 4);
        assert_eq!(input.modules[0].label, LIVE_DEMO_BLUEPRINT_MODULE_LABEL);
    }

    #[test]
    fn live_demo_input_rejects_an_incomplete_pilot_mapping() {
        assert!(live_demo_blueprint_input((1..=3).map(question).collect()).is_err());
    }

    fn stored_content(
        input: &CreateBlueprintCourseInput,
        questions: &[QuestionRevisionReference],
    ) -> StoredBlueprintCourseContent {
        let assignment = &input.modules[0].assignments[0];
        StoredBlueprintCourseContent {
            modules: vec![StoredBlueprintModule {
                blueprint_module_reference: BlueprintModuleReference::from_uuid(Uuid::from_u128(1)),
                label: input.modules[0].label.clone(),
                assignments: vec![StoredBlueprintAssignment {
                    blueprint_assignment_reference: BlueprintAssignmentReference::from_uuid(
                        Uuid::from_u128(2),
                    ),
                    content: StoredBlueprintAssignmentContent {
                        title: assignment.title.clone(),
                        instructions: assignment.instructions.clone(),
                        entries: assignment
                            .entries
                            .iter()
                            .zip(questions)
                            .map(|(entry, question)| match entry {
                                BlueprintAssignmentEntryInput::Fixed(fixed) => {
                                    StoredBlueprintAssignmentEntry::Fixed {
                                        question_revision: question.clone(),
                                        points_possible: fixed.points_possible,
                                        scoring_rule: fixed.scoring_rule,
                                        question_attempt_limit: fixed.question_attempt_limit,
                                        question_attempt_time_limit: fixed
                                            .question_attempt_time_limit,
                                    }
                                }
                                BlueprintAssignmentEntryInput::Pool(_) => {
                                    panic!("fixed Live Demo input does not contain Question Pools")
                                }
                            })
                            .collect(),
                        defaults: assignment.defaults.clone(),
                    },
                }],
            }],
        }
    }

    #[test]
    fn loaded_content_allows_only_store_generated_child_identity_differences() {
        let questions = (1..=4).map(question).collect::<Vec<_>>();
        let input = live_demo_blueprint_input(questions.clone()).unwrap();
        let content = stored_content(&input, &questions);

        let assignment = validate_loaded_content(&content, &input, &questions).unwrap();

        assert_eq!(assignment.as_uuid(), Uuid::from_u128(2));
    }

    #[test]
    fn loaded_content_rejects_replayed_semantic_drift() {
        let questions = (1..=4).map(question).collect::<Vec<_>>();
        let input = live_demo_blueprint_input(questions.clone()).unwrap();
        let mut content = stored_content(&input, &questions);
        content.modules[0].assignments[0].content.title = "Changed title".to_owned();

        assert!(validate_loaded_content(&content, &input, &questions).is_err());
    }
}
