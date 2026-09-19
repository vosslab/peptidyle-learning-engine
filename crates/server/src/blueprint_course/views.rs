//! Blueprint Course view construction from Store records.

use std::collections::BTreeMap;

use learning_data_access::{
    QuestionLibraryStore, SessionTokenHash, StoredBlueprintAssessmentContent,
    StoredBlueprintAssessmentEntry, StoredBlueprintCourse, StoredBlueprintCourseContent,
};
use question_model::{
    BlueprintAssessmentContentView, BlueprintAssessmentEntryView,
    BlueprintCourseAssessmentContentView, BlueprintCourseSummaryView, BlueprintCourseView,
    BlueprintModuleView, BlueprintRevisionReference, CreateBlueprintCourseInput, QuestionId,
    QuestionRevisionReference, QuestionSearchResult, ReplaceBlueprintCourseContentInput,
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
        current_revision: BlueprintRevisionReference {
            blueprint_course_id: record.id,
            revision: record.current_revision,
        },
        read_access: record.read_access,
        fork_source: record.fork_source,
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
        current_revision: BlueprintRevisionReference {
            blueprint_course_id: record.id,
            revision: record.current_revision,
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
    let current_question_revisions = entries
        .iter()
        .map(|entry| {
            (
                entry.question_revision.question_id.clone(),
                entry.question_revision.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Discovery excludes archived lineages.  Retained Blueprint pins use the
    // exact historical Store path so an archive never breaks immutable
    // Blueprint Revision interpretation.
    for reference in content_question_revisions(content) {
        if !entries
            .iter()
            .any(|entry| entry.question_revision == reference)
        {
            entries.push(
                state
                    .question_library
                    .load_published_question_revision_library_entry(session, &reference)
                    .await
                    .map_err(RouteLoadError::Store)?,
            );
        }
    }
    let question_ids = entries
        .iter()
        .map(|entry| entry.question_revision.question_id.clone())
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
                blueprint_module_reference: module.blueprint_module_reference,
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
                                &current_question_revisions,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, RouteLoadError>>()?,
            })
        })
        .collect()
}

fn content_question_revisions(
    content: &StoredBlueprintCourseContent,
) -> Vec<question_model::QuestionRevisionReference> {
    content
        .modules
        .iter()
        .flat_map(|module| module.assessments.iter())
        .flat_map(|assessment| assessment.content.entries.iter())
        .filter_map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Fixed {
                question_revision, ..
            } => Some(question_revision),
            StoredBlueprintAssessmentEntry::Pool { .. } => None,
        })
        .cloned()
        .collect()
}
fn assessment_content_view(
    content: &StoredBlueprintAssessmentContent,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> Result<BlueprintAssessmentContentView, RouteLoadError> {
    let entries = content
        .entries
        .iter()
        .map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Fixed {
                question_revision,
                points_possible,
                scoring_rule,
                question_attempt_limit,
                question_attempt_time_limit,
            } => Ok(BlueprintAssessmentEntryView::Fixed {
                question: Box::new(question_view(
                    question_revision,
                    questions,
                    current_question_revisions,
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
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> Result<ReusableQuestionView, RouteLoadError> {
    Ok(ReusableQuestionView {
        reference: reference.clone(),
        question_library: question_search_result(reference, questions)?,
        selection_availability: selection_availability(reference, current_question_revisions),
    })
}
fn question_search_result(
    reference: &question_model::QuestionRevisionReference,
    questions: &BTreeMap<QuestionRevisionReference, QuestionSearchResult>,
) -> Result<QuestionSearchResult, RouteLoadError> {
    exact_revision_value(reference, questions)
        .cloned()
        .ok_or(RouteLoadError::Unavailable)
}

pub(super) fn exact_revision_value<'a, T>(
    reference: &QuestionRevisionReference,
    values: &'a BTreeMap<QuestionRevisionReference, T>,
) -> Option<&'a T> {
    values.get(reference)
}
pub(super) fn selection_availability(
    reference: &question_model::QuestionRevisionReference,
    current_question_revisions: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> ReusableSelectionAvailability {
    if current_question_revisions.get(&reference.question_id) == Some(reference) {
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
