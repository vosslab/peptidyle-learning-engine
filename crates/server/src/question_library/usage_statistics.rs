//! Instructor-only Question Library usage statistics attachment.

use std::collections::BTreeMap;

use axum::response::Response;
use learning_data_access::{
    PublishedQuestionLibraryEntry, SessionTokenHash, postgres::PostgresQuestionLibraryStore,
};
use objects::s3::S3ObjectStore;
use question_model::{
    QuestionDetails, QuestionDetailsPromptView, QuestionId, QuestionRevisionReference,
    QuestionSearchResult, QuestionStatistics, QuestionUsageTotals, QuestionUseDetails,
    QuestionUseSummary, ReusableQuestionView, ReusableSelectionAvailability,
};

use super::{
    ResolvedQuestionLibraryEntry, answer_free_question_library_entry, store_error_response,
};

pub(crate) async fn answer_free_question_search_results(
    objects: &S3ObjectStore,
    entries: Vec<PublishedQuestionLibraryEntry>,
    evidence_by_question: &BTreeMap<QuestionId, QuestionStatistics>,
) -> Result<BTreeMap<QuestionRevisionReference, QuestionSearchResult>, ()> {
    let mut results = BTreeMap::new();
    for entry in entries {
        let question_id = entry.question_revision.question_id.clone();
        let question_revision = entry.question_revision.clone();
        let resolved = answer_free_question_library_entry(objects, entry).await?;
        results.insert(
            question_revision,
            search_result(resolved, evidence_for(&question_id, evidence_by_question)),
        );
    }
    Ok(results)
}

pub(crate) async fn answer_free_reusable_question_view(
    objects: &S3ObjectStore,
    entry: PublishedQuestionLibraryEntry,
    evidence: QuestionStatistics,
) -> Result<ReusableQuestionView, ()> {
    let selection_availability = match entry.availability {
        question_model::QuestionAvailability::Available => ReusableSelectionAvailability::Available,
        question_model::QuestionAvailability::Archived => ReusableSelectionAvailability::Retained,
    };
    let reference = entry.question_revision.clone();
    let resolved = answer_free_question_library_entry(objects, entry).await?;
    Ok(ReusableQuestionView {
        reference,
        question_library: search_result(resolved, evidence),
        selection_availability,
    })
}

pub(super) fn details_from_resolved(
    resolved: ResolvedQuestionLibraryEntry,
    evidence: QuestionStatistics,
) -> QuestionDetails {
    QuestionDetails {
        summary: resolved.summary,
        discipline_name: resolved.discipline.unwrap_or_default(),
        subject_name: resolved.subject.unwrap_or_default(),
        discipline_is_retired: resolved.discipline_is_retired,
        prompt: QuestionDetailsPromptView::Static {
            blocks: resolved.prompt,
        },
        response_preview: resolved.response_preview,
        evidence,
        usage: QuestionUseDetails {
            summary: QuestionUseSummary {
                global_course_count: 0,
                global_assessment_count: 0,
                own_course_count: 0,
                own_assessment_count: 0,
            },
            own_courses: Vec::new(),
            own_courses_truncated: false,
        },
    }
}

fn search_result(
    resolved: ResolvedQuestionLibraryEntry,
    evidence: QuestionStatistics,
) -> QuestionSearchResult {
    QuestionSearchResult {
        summary: resolved.summary,
        discipline_name: resolved.discipline.unwrap_or_default(),
        discipline_is_retired: resolved.discipline_is_retired,
        evidence,
    }
}

#[allow(clippy::result_large_err)]
pub(crate) async fn bulk_question_statistics(
    store: &PostgresQuestionLibraryStore,
    session_hash: SessionTokenHash,
    is_instructor: bool,
    question_ids: &[QuestionId],
) -> Result<BTreeMap<QuestionId, QuestionStatistics>, Response> {
    if !is_instructor {
        return Ok(BTreeMap::new());
    }
    let rows = store
        .load_question_usage_statistics(session_hash, question_ids)
        .await
        .map_err(store_error_response)?;
    Ok(rows
        .into_iter()
        .map(|(question_id, totals)| (question_id, totals.into_available(None, None)))
        .collect())
}

#[allow(clippy::result_large_err)]
pub(super) async fn question_detail_statistics(
    store: &PostgresQuestionLibraryStore,
    session_hash: SessionTokenHash,
    is_instructor: bool,
    question_id: &QuestionId,
) -> Result<QuestionStatistics, Response> {
    if !is_instructor {
        return Ok(QuestionStatistics::Unavailable);
    }
    let rows = store
        .load_question_usage_statistics(session_hash, std::slice::from_ref(question_id))
        .await
        .map_err(store_error_response)?;
    let totals = rows
        .into_iter()
        .find(|(id, _)| id == question_id)
        .map(|(_, totals)| totals)
        .unwrap_or_else(QuestionUsageTotals::empty);
    let revisions = store
        .load_question_revision_usage_statistics(session_hash, question_id)
        .await
        .map_err(store_error_response)?;
    Ok(totals.into_available(Some(revisions), None))
}

pub(crate) fn evidence_for(
    question_id: &QuestionId,
    evidence_by_question: &BTreeMap<QuestionId, QuestionStatistics>,
) -> QuestionStatistics {
    evidence_by_question
        .get(question_id)
        .cloned()
        .unwrap_or(QuestionStatistics::Unavailable)
}
