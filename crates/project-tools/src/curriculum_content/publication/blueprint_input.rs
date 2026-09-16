//! Current Blueprint input assembled from published curriculum identities.

use std::num::NonZeroU32;

use super::*;

pub(super) fn blueprint_input(
    manifest: &Manifest,
    published_pools: &PublishedPools,
    replacements: &ReplacementRevisions,
) -> Result<CreateBlueprintCourseInput> {
    let mut assessments = Vec::with_capacity(manifest.topics.len());
    for topic in &manifest.topics {
        let mut entries = Vec::with_capacity(topic.banks.len());
        for bank in &topic.banks {
            if replacement_source(manifest, topic, bank).is_some() {
                let revision = replacements
                    .get(&(topic.slug.clone(), bank.slug.clone()))
                    .with_context(|| {
                        format!(
                            "accepted canonical replacement is missing for {}/{}",
                            topic.slug, bank.slug
                        )
                    })?;
                entries.push(BlueprintAssessmentEntryInput::Fixed(
                    ReusableFixedQuestionInput {
                        question_id: revision.question_id.clone(),
                        points_possible: AssessmentPointValue::from_whole(1),
                        scoring_rule: AssessmentEntryScoringRule::Normal,
                        question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                        question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                    },
                ));
                continue;
            }
            let pool = published_pools
                .get(&(topic.slug.clone(), bank.slug.clone()))
                .with_context(|| {
                    format!(
                        "published curriculum Pool is missing for {}/{}",
                        topic.slug, bank.slug
                    )
                })?;
            entries.push(BlueprintAssessmentEntryInput::Pool(ReusablePoolInput {
                question_pool_id: pool.question_pool_revision.question_pool_id.clone(),
                selection_count: NonZeroU32::new(bank.selection_count)
                    .context("curriculum Pool selection count must be positive")?,
                points_per_item: AssessmentPointValue::from_whole(1),
                scoring_rule: AssessmentEntryScoringRule::Normal,
                selection_rule: QuestionPoolSelectionRule {
                    selected_question_order: QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
                },
                question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            }));
        }
        assessments.push(BlueprintAssessmentContentInput {
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
        });
    }
    let input = CreateBlueprintCourseInput {
        short_name: manifest.course.short_name.clone(),
        long_name: manifest.course.long_name.clone(),
        modules: vec![CreateBlueprintModuleInput {
            label: manifest.course.module_label.clone(),
            assessments,
        }],
    };
    input
        .validate()
        .map_err(|error| anyhow::anyhow!("curriculum Blueprint input is invalid: {error}"))?;
    Ok(input)
}

pub(super) fn replacement_source<'a>(
    manifest: &'a Manifest,
    topic: &Topic,
    bank: &super::super::Bank,
) -> Option<&'a ParameterizedSource> {
    manifest.parameterized_sources.iter().find(|source| {
        source.topic_slug == topic.slug
            && source.replaces_static_bank_slug.as_deref() == Some(bank.slug.as_str())
    })
}
