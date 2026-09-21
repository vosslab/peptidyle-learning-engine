//! Blueprint Course view construction from Store records.

use std::collections::BTreeMap;

use learning_data_access::{
    QuestionLibraryStore, SessionTokenHash, StoredBlueprintAssessmentContent,
    StoredBlueprintAssessmentEntry, StoredBlueprintCourse, StoredBlueprintCourseContent,
};
use question_model::{
    BlueprintAssessmentContentView, BlueprintAssessmentEntryView,
    BlueprintCourseAssessmentContentView, BlueprintCourseSummaryView, BlueprintCourseView,
    BlueprintModuleView, BlueprintRevisionTuple, CreateBlueprintCourseInput, PublishedQuestionId,
    PublishedQuestionRevisionTuple, QuestionSearchResult, ReplaceBlueprintCourseContentInput,
    ReusablePoolView, ReusableQuestionView, ReusableSelectionAvailability,
};

use super::{BlueprintCourseRouteState, RouteLoadError};

pub(super) async fn view_from_record(
    state: &BlueprintCourseRouteState,
    session: SessionTokenHash,
    record: StoredBlueprintCourse,
) -> Result<BlueprintCourseView, RouteLoadError> {
    Ok(BlueprintCourseView {
        classification: record.classification,
        id: record.id.clone(),
        short_name: record.short_name,
        long_name: record.long_name,
        availability: record.availability,
        blueprint_edit_number: record.blueprint_edit_number,
        current_revision_tuple: BlueprintRevisionTuple {
            blueprint_course_id: record.id,
            revision_number: record.current_revision_number,
        },
        read_access: record.read_access,
        fork_source_tuple: record.fork_source_tuple,
        modules: content_modules(state, session, &record.content).await?,
    })
}
pub(super) fn summary_view(
    record: learning_data_access::StoredBlueprintCourseSummary,
) -> BlueprintCourseSummaryView {
    BlueprintCourseSummaryView {
        classification: record.classification,
        total_adoptions: record.total_adoptions,
        total_students_ever_enrolled: record.total_students_ever_enrolled,
        id: record.id.clone(),
        short_name: record.short_name,
        long_name: record.long_name,
        availability: record.availability,
        blueprint_edit_number: record.blueprint_edit_number,
        current_revision_tuple: BlueprintRevisionTuple {
            blueprint_course_id: record.id,
            revision_number: record.current_revision_number,
        },
        read_access: record.read_access,
    }
}
pub(super) async fn content_modules(
    state: &BlueprintCourseRouteState,
    session: SessionTokenHash,
    content: &StoredBlueprintCourseContent,
) -> Result<Vec<BlueprintModuleView>, RouteLoadError> {
    let mut entries = state
        .question_library
        .list_published_question_library_entries(session)
        .await
        .map_err(RouteLoadError::Store)?;
    let current_published_question_revision_tuples = entries
        .iter()
        .map(|entry| {
            (
                entry
                    .published_question_revision_tuple
                    .published_question_id
                    .clone(),
                entry.published_question_revision_tuple.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Discovery excludes archived lineages.  Retained Blueprint pins use the
    // exact historical Store path so an archive never breaks immutable
    // Blueprint Revision interpretation.
    for published_question_revision_tuple in content_published_question_revision_tuples(content) {
        if !entries.iter().any(|entry| {
            entry.published_question_revision_tuple == published_question_revision_tuple
        }) {
            entries.push(
                state
                    .question_library
                    .load_published_question_revision_library_entry(
                        session,
                        &published_question_revision_tuple,
                    )
                    .await
                    .map_err(RouteLoadError::Store)?,
            );
        }
    }
    let question_ids = entries
        .iter()
        .map(|entry| {
            entry
                .published_question_revision_tuple
                .published_question_id
                .clone()
        })
        .collect::<Vec<_>>();
    let evidence = crate::question_library::bulk_question_statistics(
        &state.question_library,
        session,
        true,
        &question_ids,
    )
    .await
    .map_err(|_| RouteLoadError::Unavailable)?;
    let questions = crate::question_library::answer_free_question_search_results(
        &state.objects,
        entries,
        &evidence,
    )
    .await
    .map_err(|_| RouteLoadError::Unavailable)?;
    content
        .modules
        .iter()
        .map(|module| {
            Ok(BlueprintModuleView {
                blueprint_module_id: module.blueprint_module_id,
                label: module.label.clone(),
                assessments: module
                    .assessments
                    .iter()
                    .map(|assessment| {
                        Ok(BlueprintCourseAssessmentContentView {
                            blueprint_assessment_id: assessment.blueprint_assessment_id,
                            content: assessment_content_view(
                                &assessment.content,
                                &questions,
                                &current_published_question_revision_tuples,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, RouteLoadError>>()?,
            })
        })
        .collect()
}

fn content_published_question_revision_tuples(
    content: &StoredBlueprintCourseContent,
) -> Vec<question_model::PublishedQuestionRevisionTuple> {
    content
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .flat_map(|assessment| assessment.content.entries.iter())
        .filter_map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Fixed {
                published_question_revision_tuple,
                ..
            } => Some(published_question_revision_tuple),
            StoredBlueprintAssessmentEntry::Pool { .. } => None,
        })
        .cloned()
        .collect()
}
fn assessment_content_view(
    content: &StoredBlueprintAssessmentContent,
    questions: &BTreeMap<PublishedQuestionRevisionTuple, QuestionSearchResult>,
    current_published_question_revision_tuples: &BTreeMap<
        PublishedQuestionId,
        PublishedQuestionRevisionTuple,
    >,
) -> Result<BlueprintAssessmentContentView, RouteLoadError> {
    let entries = content
        .entries
        .iter()
        .map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Fixed {
                published_question_revision_tuple,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssessmentEntryView::Fixed {
                question: Box::new(question_view(
                    published_question_revision_tuple,
                    questions,
                    current_published_question_revision_tuples,
                )?),
                points_possible: *points_possible,
                scoring_rule: *scoring_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            }),
            StoredBlueprintAssessmentEntry::Pool {
                question_pool_id,
                question_pool_edit_number,
                selection_count,
                points_per_item,
                scoring_rule,
                selection_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssessmentEntryView::Pool(ReusablePoolView {
                question_pool_id: question_pool_id.clone(),
                question_pool_edit_number: *question_pool_edit_number,
                selection_count: *selection_count,
                points_per_item: *points_per_item,
                scoring_rule: *scoring_rule,
                selection_rule: *selection_rule,
                question_attempt_limit: *question_attempt_limit,
                question_attempt_time_limit: *question_attempt_time_limit,
            })),
        })
        .collect::<Result<Vec<_>, RouteLoadError>>()?;
    Ok(BlueprintAssessmentContentView {
        assessment_type: content.assessment_type,
        title: content.title.clone(),
        instructions: content.instructions.clone(),
        entries,
        defaults: content.defaults.clone(),
    })
}
fn question_view(
    published_question_revision_tuple: &question_model::PublishedQuestionRevisionTuple,
    questions: &BTreeMap<PublishedQuestionRevisionTuple, QuestionSearchResult>,
    current_published_question_revision_tuples: &BTreeMap<
        PublishedQuestionId,
        PublishedQuestionRevisionTuple,
    >,
) -> Result<ReusableQuestionView, RouteLoadError> {
    Ok(ReusableQuestionView {
        published_question_revision_tuple: published_question_revision_tuple.clone(),
        question_library: question_search_result(published_question_revision_tuple, questions)?,
        selection_availability: selection_availability(
            published_question_revision_tuple,
            current_published_question_revision_tuples,
        ),
    })
}
fn question_search_result(
    published_question_revision_tuple: &question_model::PublishedQuestionRevisionTuple,
    questions: &BTreeMap<PublishedQuestionRevisionTuple, QuestionSearchResult>,
) -> Result<QuestionSearchResult, RouteLoadError> {
    exact_revision_value(published_question_revision_tuple, questions)
        .cloned()
        .ok_or(RouteLoadError::Unavailable)
}

pub(super) fn exact_revision_value<'a, T>(
    published_question_revision_tuple: &PublishedQuestionRevisionTuple,
    values: &'a BTreeMap<PublishedQuestionRevisionTuple, T>,
) -> Option<&'a T> {
    values.get(published_question_revision_tuple)
}
pub(super) fn selection_availability(
    published_question_revision_tuple: &question_model::PublishedQuestionRevisionTuple,
    current_published_question_revision_tuples: &BTreeMap<
        PublishedQuestionId,
        PublishedQuestionRevisionTuple,
    >,
) -> ReusableSelectionAvailability {
    if current_published_question_revision_tuples
        .get(&published_question_revision_tuple.published_question_id)
        == Some(published_question_revision_tuple)
    {
        ReusableSelectionAvailability::Available
    } else {
        ReusableSelectionAvailability::Retained
    }
}

// ASVS 2.2.1/2.2.2: reject a syntactically plausible but deployment-invalid ID
// before a persistence resolver can disclose whether a Question exists.
pub(super) fn valid_create_question_ids(input: &CreateBlueprintCourseInput) -> bool {
    input
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .all(valid_assessment_question_ids)
}
pub(super) fn valid_replace_question_ids(input: &ReplaceBlueprintCourseContentInput) -> bool {
    input
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .all(|assessment| valid_assessment_question_ids(&assessment.content))
}
fn valid_assessment_question_ids(input: &question_model::BlueprintAssessmentContentInput) -> bool {
    input.entries.iter().all(|entry| match entry {
        question_model::BlueprintAssessmentEntryInput::Fixed(_) => true,
        // ASVS 2.2.1/2.2.2: validate the exact Pool and any newly authored
        // member Question IDs before the Store resolves private state.
        question_model::BlueprintAssessmentEntryInput::Pool(_) => true,
    })
}
