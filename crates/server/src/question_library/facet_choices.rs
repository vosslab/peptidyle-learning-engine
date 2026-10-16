//! Read-only choices over the complete authorized Library query.

use super::*;
use learning_data_access::{QuestionLibraryBackendRestriction, QuestionLibraryFacetChoiceRequest};
use question_model::{Capability, LibraryFacetChoice, LibraryFacetChoicePage, LibraryFacetField};
use serde::Serialize;

pub(super) async fn facet_choices_route(
    State(state): State<QuestionLibraryRouteState>,
    headers: HeaderMap,
    Path(field): Path<String>,
    Query(transport): Query<LibraryObjectSearchQuery>,
) -> Response {
    let Ok(field) = field.parse::<LibraryFacetField>() else {
        return route_error(
            StatusCode::BAD_REQUEST,
            "Question Library facet field is invalid",
        );
    };
    let choice_search = match normalized_choice_search(transport.choice_search.as_deref()) {
        Ok(value) => value,
        Err(()) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Library choice search is invalid",
            );
        }
    };
    let query = match LibraryObjectSearchRequest::try_from(transport) {
        Ok(value) if value.page_size.is_some_and(|size| size <= 100) => value,
        _ => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Library choice query is invalid",
            );
        }
    };
    let (session_hash, _) = match library_reader_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let after_key = match facet_choice_paging::decode(&query, field, choice_search.as_deref()) {
        Ok(value) => value,
        Err(()) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                "Question Library choice continuation is invalid",
            );
        }
    };
    if let Err(response) =
        classification::validate(&state.classifications, session_hash, &query).await
    {
        return response;
    }
    let mut effective = query.clone();
    match field {
        LibraryFacetField::Backends => effective.backends.clear(),
        LibraryFacetField::Capabilities => effective.capabilities.clear(),
        _ => {}
    }
    let mut request = mixed_search::store_search_request(&effective, None);
    // Backend capabilities are declared by adapters. Count compatible Backend
    // rows over the complete collection, then fold through that same owner.
    if field == LibraryFacetField::Capabilities {
        request.page_size = 100;
        request.backends = eligible_backends(&effective);
    }
    if field == LibraryFacetField::Backends {
        request.backends = if query.capabilities.is_empty() {
            QuestionLibraryBackendRestriction::Any
        } else {
            eligible_backends(&effective)
        };
    }
    let selected_keys = selected_keys(&query, field);
    let result = match state
        .store
        .list_question_library_facet_choices(
            session_hash,
            QuestionLibraryFacetChoiceRequest {
                search: request,
                field,
                choice_search: if field == LibraryFacetField::Capabilities {
                    None
                } else {
                    choice_search.clone()
                },
                after_key: if field == LibraryFacetField::Capabilities {
                    None
                } else {
                    after_key
                },
                selected_keys: if field == LibraryFacetField::Capabilities {
                    Vec::new()
                } else {
                    selected_keys.clone()
                },
            },
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    let (choices, selected, next_cursor) = if field == LibraryFacetField::Capabilities {
        let all = Capability::ALL
            .into_iter()
            .filter_map(|capability| {
                let count = result
                    .choices
                    .iter()
                    .filter_map(|backend_choice| {
                        let backend = match backend_choice.key.as_str() {
                            "ple" => QuestionBackend::Ple,
                            "webwork" => QuestionBackend::Webwork,
                            "imathas" => QuestionBackend::Imathas,
                            _ => return None,
                        };
                        facets::backend_capabilities(backend)
                            .supports(capability)
                            .then_some(backend_choice.count)
                    })
                    .sum::<u64>();
                (count > 0).then(|| LibraryFacetChoice {
                    key: capability.as_str().to_owned(),
                    label: capability.as_str().to_owned(),
                    count,
                })
            })
            .collect::<Vec<_>>();
        let selected = selected_keys
            .iter()
            .map(|key| LibraryFacetChoice {
                key: key.clone(),
                label: key.clone(),
                count: all
                    .iter()
                    .find(|choice| choice.key == *key)
                    .map_or(0, |choice| choice.count),
            })
            .collect();
        let visible = all
            .into_iter()
            .filter(|choice| {
                choice_search
                    .as_ref()
                    .is_none_or(|search| choice.label.to_lowercase().contains(search))
            })
            .collect();
        (visible, selected, None)
    } else {
        let next_cursor = result
            .next_key
            .map(|key| facet_choice_paging::encode(&query, field, choice_search.as_deref(), key));
        (result.choices, result.selected, next_cursor)
    };
    crate::auth::no_store(
        Json(LibraryFacetChoicePage {
            choices,
            selected,
            next_cursor,
        })
        .into_response(),
    )
}

pub(crate) fn normalized_choice_search(value: Option<&str>) -> Result<Option<String>, ()> {
    let normalized = value.map(|value| {
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    });
    if normalized
        .as_ref()
        .is_some_and(|value| value.chars().count() > 128)
    {
        return Err(());
    }
    Ok(normalized.filter(|value| !value.is_empty()))
}

fn json_key<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("closed Library choice value serializes")
        .as_str()
        .expect("closed Library choice value is a string")
        .to_owned()
}

pub(crate) fn selected_keys(
    query: &LibraryObjectSearchRequest,
    field: LibraryFacetField,
) -> Vec<String> {
    match field {
        LibraryFacetField::Kind => match query.kind {
            question_model::LibrarySearchKind::Both => Vec::new(),
            question_model::LibrarySearchKind::Questions => vec!["questions".to_owned()],
            question_model::LibrarySearchKind::Pools => vec!["pools".to_owned()],
        },
        LibraryFacetField::AuthorNames => query.author_names.clone(),
        LibraryFacetField::Backends => query
            .backends
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
        LibraryFacetField::Tags => query.tags.clone(),
        LibraryFacetField::NameSubjects => query.subjects.clone(),
        LibraryFacetField::NameTopics => query.topics.clone(),
        LibraryFacetField::DisciplineUuids => query
            .discipline_uuids
            .iter()
            .map(ToString::to_string)
            .collect(),
        LibraryFacetField::SubjectUuids => query
            .subject_uuids
            .iter()
            .map(ToString::to_string)
            .collect(),
        LibraryFacetField::TopicUuids => {
            query.topic_uuids.iter().map(ToString::to_string).collect()
        }
        LibraryFacetField::SubtopicUuids => query
            .subtopic_uuids
            .iter()
            .map(ToString::to_string)
            .collect(),
        LibraryFacetField::BloomCognitiveProcesses => query
            .bloom_cognitive_processes
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
        LibraryFacetField::BloomKnowledgeDimensions => query
            .bloom_knowledge_dimensions
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
        LibraryFacetField::QuestionTypes => query.question_types.iter().map(json_key).collect(),
        LibraryFacetField::Capabilities => query
            .capabilities
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
        LibraryFacetField::QuestionLicenses => {
            query.question_licenses.iter().map(json_key).collect()
        }
    }
}
