//! One globally ordered Library page, with source reads restricted to Question rows.

use super::*;
use learning_data_access::{LibrarySearchEntry, QuestionLibrarySearchCursorPosition};
use question_model::{LibrarySearchClassification, LibrarySearchResult};

/// One Question Library search after the session is known.
///
/// The opaque cursor is checked before any source read. Native Question source
/// is resolved only for the store's returned page.
pub(super) async fn search_library_objects<S, O>(
    store: &S,
    objects: &O,
    session_hash: SessionTokenHash,
    is_instructor: bool,
    query: LibraryObjectSearchRequest,
    after: Option<QuestionLibrarySearchCursorPosition>,
) -> Response
where
    S: QuestionLibraryStore + QuestionLibraryPageStatistics,
    O: ObjectStore,
{
    let request = store_search_request(&query, after);
    let search = match store
        .search_published_question_library_entries(session_hash, request)
        .await
    {
        Ok(search) => search,
        Err(error) => return store_error_response(error),
    };
    let next_cursor = search
        .next_position
        .clone()
        .map(|position| paging::encode_position(position, &query));
    let (items, facets) =
        match materialize_search_page(store, objects, session_hash, is_instructor, search).await {
            Ok(value) => value,
            Err(response) => return *response,
        };
    crate::auth::no_store(
        Json(LibraryObjectSearchPage {
            items,
            next_cursor,
            facets,
        })
        .into_response(),
    )
}

/// The shared result projection for public Library and private Starred pages.
pub(crate) async fn materialize_search_page<S, O>(
    store: &S,
    objects: &O,
    session_hash: SessionTokenHash,
    is_instructor: bool,
    search: learning_data_access::QuestionLibrarySearchPage,
) -> Result<
    (
        Vec<LibrarySearchResult>,
        question_model::QuestionSearchFacets,
    ),
    Box<Response>,
>
where
    S: QuestionLibraryStore + QuestionLibraryPageStatistics,
    O: ObjectStore,
{
    let facets = facets::from_store(search.facets);
    let questions = search
        .items
        .iter()
        .filter_map(|item| match item {
            LibrarySearchEntry::Question { entry, .. } => Some((**entry).clone()),
            LibrarySearchEntry::Pool { .. } => None,
        })
        .collect();
    let summaries = match entries_to_summaries(objects, questions).await {
        Ok(summaries) => summaries,
        Err(()) => {
            return Err(Box::new(route_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Library unavailable",
            )));
        }
    };
    let question_ids = summaries
        .iter()
        .map(|entry| entry.summary.question_id.clone())
        .collect::<Vec<_>>();
    let evidence = if question_ids.is_empty() {
        BTreeMap::new()
    } else {
        match store
            .page_statistics(session_hash, is_instructor, &question_ids)
            .await
        {
            Ok(value) => value,
            Err(response) => return Err(Box::new(response)),
        }
    };
    let mut questions = summaries.into_iter();
    let mut items = Vec::with_capacity(search.items.len());
    for item in search.items {
        items.push(match item {
            LibrarySearchEntry::Question {
                entry: source_entry,
                owner_account_id,
                ..
            } => {
                let Some(entry) = questions.next() else {
                    return Err(Box::new(unavailable()));
                };
                let question = QuestionSearchResult {
                    evidence: usage_statistics::evidence_for(&entry.summary.question_id, &evidence),
                    summary: entry.summary,
                    discipline_name: entry.discipline.unwrap_or_default(),
                    discipline_is_retired: entry.discipline_is_retired,
                };
                let metadata = &source_entry.shared_metadata;
                let classification = LibrarySearchClassification {
                    discipline_uuid: metadata.discipline_uuid,
                    discipline_name: source_entry.discipline_name,
                    subject_uuid: metadata.subject_uuid,
                    subject_name: source_entry.subject_name,
                    topic_uuid: metadata.topic_uuid,
                    topic_name: source_entry.topic_name,
                    subtopic_uuid: metadata.subtopic_uuid,
                    subtopic_name: source_entry.subtopic_name,
                };
                LibrarySearchResult::Question {
                    question: Box::new(question),
                    owner_account_id,
                    classification,
                }
            }
            LibrarySearchEntry::Pool {
                summary,
                subject_name,
                topic_name,
                subtopic_name,
                ..
            } => {
                let metadata = &summary.metadata;
                let classification = LibrarySearchClassification {
                    discipline_uuid: metadata.discipline_uuid,
                    discipline_name: metadata.discipline_name.clone(),
                    subject_uuid: metadata.subject_uuid,
                    subject_name,
                    topic_uuid: metadata.topic_uuid,
                    topic_name,
                    subtopic_uuid: metadata.subtopic_uuid,
                    subtopic_name,
                };
                LibrarySearchResult::Pool {
                    pool: summary,
                    classification,
                }
            }
        });
    }
    Ok((items, facets))
}

pub(crate) fn store_search_request(
    query: &LibraryObjectSearchRequest,
    after: Option<QuestionLibrarySearchCursorPosition>,
) -> QuestionLibrarySearchRequest {
    let text_query = search_query::QuestionTextQuery::parse(query.text.as_deref());
    let (exact_question_id, text_terms) = text_query.into_store_terms();
    let page_size = query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    QuestionLibrarySearchRequest {
        scope: learning_data_access::QuestionLibrarySearchScope::Public,
        kind: query.kind,
        questions: query.questions,
        owner_account_id: query.owner_account_id.clone(),
        exact_question_id,
        text_terms,
        author_names: query.author_names.clone(),
        backends: eligible_backends(query),
        tags: query.tags.clone(),
        subjects: query.subjects.clone(),
        topics: query.topics.clone(),
        discipline_uuids: query.discipline_uuids.clone(),
        subject_uuids: query.subject_uuids.clone(),
        topic_uuids: query.topic_uuids.clone(),
        subtopic_uuids: query.subtopic_uuids.clone(),
        cross_discipline: query.cross_discipline,
        bloom_cognitive_processes: query.bloom_cognitive_processes.clone(),
        bloom_knowledge_dimensions: query.bloom_knowledge_dimensions.clone(),
        question_types: query.question_types.clone(),
        question_licenses: query.question_licenses.clone(),
        authored_by_current_account: query.authorship
            == question_model::QuestionSearchAuthorship::AuthoredByCurrentAccount,
        sort: paging::store_sort(query.sort),
        page_size,
        after,
    }
}

/// Resolves an actual visible object under the same Library reader authority.
pub(super) async fn library_object_kind(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(public_id): Path<String>,
) -> Response {
    let (session, _) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(public_id) = public_id.parse::<question_model::LibraryObjectId>() else {
        return concealed();
    };
    match state
        .store
        .load_library_object_kind(session, &public_id)
        .await
    {
        Ok(kind) => crate::auth::no_store(
            Json(question_model::LibraryObjectKindResponse { kind }).into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}
