//! Facet choices from the same private, actor-scoped Starred matcher.

use super::{
    Deserialize, HeaderMap, IntoResponse, Json, LibraryObjectSearchRequest, LibrarySearchKind,
    Path, PublishedQuestionFilter, QuestionLibrarySearchScope, RawQuery, Response, RouteState,
    Serialize, StarredSort, State, StatusCode, instructor_session, parse_starred_query,
    route_error, store_error_response,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    QuestionLibraryBackendRestriction, QuestionLibraryFacetChoiceRequest, QuestionLibraryStore,
};
use question_model::{
    Capability, LibraryFacetChoice, LibraryFacetChoicePage, LibraryFacetField,
    LibraryObjectSearchFilter, QuestionBackend,
};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChoiceCursor {
    version: u8,
    digest: [u8; 32],
    key: String,
}

fn choice_digest(
    query: &LibraryObjectSearchRequest,
    sort: StarredSort,
    actor: &str,
    field: LibraryFacetField,
    search: Option<&str>,
) -> [u8; 32] {
    let filter = LibraryObjectSearchFilter::from_query(query.clone())
        .expect("validated Starred facet query normalizes");
    Sha256::digest(
        serde_json::to_vec(&(filter, sort, actor, field, search))
            .expect("Starred choice query serializes"),
    )
    .into()
}

fn decode_choice_cursor(value: Option<&str>, digest: [u8; 32]) -> Result<Option<String>, ()> {
    let Some(value) = value else { return Ok(None) };
    if value.len() > question_model::MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES {
        return Err(());
    }
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| ())?;
    if bytes.len() > 4096 || URL_SAFE_NO_PAD.encode(&bytes) != value {
        return Err(());
    }
    let cursor: ChoiceCursor = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if cursor.version != 1 || cursor.digest != digest || cursor.key.is_empty() {
        return Err(());
    }
    Ok(Some(cursor.key))
}

fn encode_choice_cursor(key: String, digest: [u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&ChoiceCursor {
            version: 1,
            digest,
            key,
        })
        .expect("Starred choice cursor serializes"),
    )
}

pub(super) async fn list_starred_facet_choices(
    State(state): State<RouteState>,
    headers: HeaderMap,
    Path(raw_field): Path<String>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let Ok(field) = raw_field.parse::<LibraryFacetField>() else {
        return route_error(StatusCode::BAD_REQUEST, "Starred facet field is invalid");
    };
    let Ok(query) = parse_starred_query(raw_query.as_deref()) else {
        return route_error(StatusCode::BAD_REQUEST, "Starred facet query is invalid");
    };
    let choice_search = match crate::question_library::facet_choices::normalized_choice_search(
        query.library.choice_search.as_deref(),
    ) {
        Ok(value) => value,
        Err(()) => return route_error(StatusCode::BAD_REQUEST, "Starred facet search is invalid"),
    };
    let sort = query.starred_sort;
    let mut public_query = match LibraryObjectSearchRequest::try_from(query.library) {
        Ok(value) if value.page_size.is_some_and(|size| size <= 100) => value,
        _ => return route_error(StatusCode::BAD_REQUEST, "Starred facet query is invalid"),
    };
    public_query.kind = LibrarySearchKind::Questions;
    public_query.questions = PublishedQuestionFilter::All;
    let (session, actor) = match instructor_session(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let digest = choice_digest(&public_query, sort, &actor, field, choice_search.as_deref());
    let after_key = match decode_choice_cursor(public_query.cursor.as_deref(), digest) {
        Ok(value) => value,
        Err(()) => return route_error(StatusCode::BAD_REQUEST, "Starred facet cursor is invalid"),
    };
    let mut effective = public_query.clone();
    match field {
        LibraryFacetField::Backends => effective.backends.clear(),
        LibraryFacetField::Capabilities => effective.capabilities.clear(),
        _ => {}
    }
    let mut request = crate::question_library::mixed_search::store_search_request(&effective);
    request.scope = QuestionLibrarySearchScope::CurrentStarred;
    if field == LibraryFacetField::Backends {
        request.backends = if public_query.capabilities.is_empty() {
            QuestionLibraryBackendRestriction::Any
        } else {
            crate::question_library::eligible_backends(&effective)
        };
    }
    let selected_keys = crate::question_library::facet_choices::selected_keys(&public_query, field);
    let result = match state
        .library
        .list_question_library_facet_choices(
            session,
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
        let all: Vec<_> = Capability::ALL
            .into_iter()
            .filter_map(|capability| {
                let count = result
                    .choices
                    .iter()
                    .filter_map(|choice| {
                        let backend = match choice.key.as_str() {
                            "ple" => QuestionBackend::Ple,
                            "webwork" => QuestionBackend::Webwork,
                            "imathas" => QuestionBackend::Imathas,
                            _ => return None,
                        };
                        crate::question_library::facets::backend_capabilities(backend)
                            .supports(capability)
                            .then_some(choice.count)
                    })
                    .sum::<u64>();
                (count > 0).then(|| LibraryFacetChoice {
                    key: capability.as_str().to_owned(),
                    label: capability.as_str().to_owned(),
                    count,
                })
            })
            .collect();
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
        let choices = all
            .into_iter()
            .filter(|choice| {
                choice_search
                    .as_ref()
                    .is_none_or(|search| choice.label.to_lowercase().contains(search))
            })
            .collect();
        (choices, selected, None)
    } else {
        let next_cursor = result.next_key.map(|key| encode_choice_cursor(key, digest));
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
