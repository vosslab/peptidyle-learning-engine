//! Direct Fixed-entry Blueprint input for the fresh canonical catalog.

use super::*;
use question_model::{
    AssessmentActivityRules, AssessmentInstructions, AssessmentPointValue,
    BlueprintAssessmentContentInput, BlueprintAssessmentDefaults, CreateBlueprintModuleInput,
    LateWorkRule, ReusableFixedQuestionInput, StudentFeedbackReleaseRule,
};

pub(super) fn blueprint_input(
    manifest: &Manifest,
    revisions: &SourceRevisions,
) -> Result<CreateBlueprintCourseInput> {
    let assessments = manifest
        .topics
        .iter()
        .map(|topic| -> Result<_> {
            let entries = topic
                .source_ids
                .iter()
                .map(|source_id| {
                    let revision = revisions.get(source_id).with_context(|| {
                        format!("canonical Genetics revision is missing for source {source_id}")
                    })?;
                    Ok(BlueprintAssessmentEntryInput::Fixed(
                        ReusableFixedQuestionInput {
                            published_question: revision.clone(),
                            points_possible: AssessmentPointValue::from_whole(1),
                            scoring_rule: AssessmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        },
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(BlueprintAssessmentContentInput {
                assessment_type: manifest.course.assessment_type,
                title: topic.title.clone(),
                instructions: AssessmentInstructions::try_new(topic.instructions.clone())
                    .map_err(|_| anyhow::anyhow!("curriculum topic instructions are invalid"))?,
                entries,
                defaults: BlueprintAssessmentDefaults {
                    assessment_attempt_time_limit_seconds: None,
                    attempt_limit: None,
                    late_work_rule: LateWorkRule::Reject,
                    activity_rules: AssessmentActivityRules::default(),
                    student_feedback_release_rule: StudentFeedbackReleaseRule::for_assessment_type(
                        manifest.course.assessment_type,
                    ),
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let input = CreateBlueprintCourseInput {
        short_name: manifest.course.short_name.clone(),
        long_name: manifest.course.long_name.clone(),
        modules: vec![CreateBlueprintModuleInput {
            label: manifest.course.module_label.clone(),
            assessments,
        }],
    };
    input.validate().map_err(|error| {
        anyhow::anyhow!("canonical Genetics Blueprint input is invalid: {error}")
    })?;
    Ok(input)
}
