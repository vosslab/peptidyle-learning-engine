//! Authenticated participant discovery/review and one receiving-owner acceptance.

use super::{
    BlueprintCourseRouteState, change_proposal_view, concealed, instructor_session_hash,
    route_error, store_error_response,
};
use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_change_proposal::{
    BlueprintChangeProposalAcceptanceRequest, BlueprintChangeProposalCreateRequest,
    BlueprintChangeProposalPageView,
};
use learning_data_access::{
    AcceptBlueprintChangeProposalInput, BlueprintChangeProposalListScope,
    BlueprintChangeProposalStore, CreateBlueprintChangeProposalInput, NumberedPageRequest,
    PageNumber, PageSize, SessionTokenHash,
};
use serde::{Deserialize, Serialize};

pub(super) async fn create(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(target): Path<String>,
    Json(input): Json<BlueprintChangeProposalCreateRequest>,
) -> Response {
    let blueprint_course_id = match target.parse::<question_model::BlueprintCourseId>() {
        Ok(value) => value,
        Err(_) => return invalid_request(),
    };
    if blueprint_course_id != input.target_revision_tuple.blueprint_course_id {
        return invalid_request();
    }
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.2, 2.3.3: Store commit owns exact basis authorization and evidence.
    let proposal = match state
        .blueprints
        .create_blueprint_change_proposal(
            session,
            CreateBlueprintChangeProposalInput {
                source_revision_tuple: input.source_revision_tuple,
                source_blueprint_edit_number: input.source_blueprint_edit_number,
                target_revision_tuple: input.target_revision_tuple,
                target_blueprint_edit_number: input.target_blueprint_edit_number,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    load_detail(&state, session, proposal.proposal_id, StatusCode::CREATED).await
}

pub(super) async fn detail(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let id = match parse_proposal_id(&id) {
        Some(value) => value,
        None => return invalid_request(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    load_detail(&state, session, id, StatusCode::OK).await
}

async fn load_detail(
    state: &BlueprintCourseRouteState,
    session: SessionTokenHash,
    id: uuid::Uuid,
    status: StatusCode,
) -> Response {
    // ASVS 8.2.2/3, 8.3.1/2: exact participant loader, not ordinary source/history.
    match state
        .blueprints
        .read_blueprint_change_proposal_review(session, id)
        .await
    {
        Ok(Some(review)) => match change_proposal_view::detail(review) {
            Ok(view) => crate::auth::no_store((status, Json(view)).into_response()),
            Err(error) => store_error_response(error),
        },
        Ok(None) => concealed(),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn accept(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<BlueprintChangeProposalAcceptanceRequest>,
) -> Response {
    let id = match parse_proposal_id(&id) {
        Some(value) => value,
        None => return invalid_request(),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 2.3.1/3, 8.2.2: final owner decision uses the existing guarded transaction.
    match state
        .blueprints
        .accept_blueprint_change_proposal(
            session,
            AcceptBlueprintChangeProposalInput {
                proposal_id: id,
                expected_target_revision_tuple: input.expected_target_revision_tuple,
                expected_target_blueprint_edit_number: input.expected_target_blueprint_edit_number,
                decision: change_proposal_view::store_decision(input.decision),
            },
        )
        .await
    {
        Ok(record) => {
            crate::auth::no_store(Json(change_proposal_view::accepted(record)).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TargetQuery {
    page_number: Option<u32>,
    page_size: Option<u16>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    sort: ProposalSort,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MineQuery {
    scope: MineScope,
    page_number: Option<u32>,
    page_size: Option<u16>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    sort: ProposalSort,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProposalSort {
    #[default]
    CreatedDescending,
    CreatedAscending,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MineScope {
    Mine,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TargetFacetQuery {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MineFacetQuery {
    scope: MineScope,
    #[serde(default)]
    text: String,
}

pub(super) async fn target_status_choices(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(target): Path<String>,
    query: Result<Query<TargetFacetQuery>, QueryRejection>,
) -> Response {
    let (Ok(target), Ok(Query(query))) = (target.parse(), query) else {
        return invalid_request();
    };
    status_choices(
        &state,
        &headers,
        BlueprintChangeProposalListScope::Target(target),
        &query.text,
    )
    .await
}

pub(super) async fn mine_status_choices(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    query: Result<Query<MineFacetQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return invalid_request();
    };
    let MineScope::Mine = query.scope;
    status_choices(
        &state,
        &headers,
        BlueprintChangeProposalListScope::Mine,
        &query.text,
    )
    .await
}

async fn status_choices(
    state: &BlueprintCourseRouteState,
    headers: &HeaderMap,
    scope: BlueprintChangeProposalListScope,
    text: &str,
) -> Response {
    // ASVS 2.2.1/2, 8.2.1/2: validate browser criteria and derive actor from the session.
    let text = text.trim();
    if text.len() > 160 || text.chars().any(|ch| matches!(ch, '%' | '_' | '\\')) {
        return invalid_request();
    }
    let session = match instructor_session_hash(state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .blueprint_change_proposal_status_counts(session, scope, text)
        .await
    {
        Ok([proposed, stale, accepted]) => crate::auth::no_store(
            Json(serde_json::json!({
                "proposed": proposed, "stale": stale, "accepted": accepted,
            }))
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn list_target(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(target): Path<String>,
    query: Result<Query<TargetQuery>, QueryRejection>,
) -> Response {
    let query = match query {
        Ok(Query(value)) => value,
        Err(_) => return invalid_request(),
    };
    let target = match target.parse() {
        Ok(value) => value,
        Err(_) => return invalid_request(),
    };
    list(
        &state,
        &headers,
        BlueprintChangeProposalListScope::Target(target),
        query,
    )
    .await
}

pub(super) async fn list_mine(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    query: Result<Query<MineQuery>, QueryRejection>,
) -> Response {
    let query = match query {
        Ok(Query(value)) => value,
        Err(_) => return invalid_request(),
    };
    let MineScope::Mine = query.scope;
    list(
        &state,
        &headers,
        BlueprintChangeProposalListScope::Mine,
        TargetQuery {
            page_number: query.page_number,
            page_size: query.page_size,
            text: query.text,
            sort: query.sort,
            status: query.status,
        },
    )
    .await
}

async fn list(
    state: &BlueprintCourseRouteState,
    headers: &HeaderMap,
    scope: BlueprintChangeProposalListScope,
    query: TargetQuery,
) -> Response {
    let text = query.text.trim();
    if text.len() > 160
        || text.chars().any(|ch| matches!(ch, '%' | '_' | '\\'))
        || !valid_statuses(&query.status)
    {
        return invalid_request();
    }
    let session = match instructor_session_hash(state, headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let page = match page_request(&query) {
        Some(value) => value,
        None => return invalid_request(),
    };
    match state
        .blueprints
        .list_blueprint_change_proposals(
            session,
            scope.clone(),
            page,
            text,
            query.sort == ProposalSort::CreatedAscending,
            &query.status,
        )
        .await
    {
        Ok(page) => {
            // ASVS 4.1.1, 14.2.2: typed participant facts never cache Private evidence.
            crate::auth::no_store(
                Json(BlueprintChangeProposalPageView {
                    items: page
                        .items
                        .into_iter()
                        .map(change_proposal_view::summary)
                        .collect(),
                    page_number: page.page_number.get(),
                    page_size: page.page_size.get(),
                    total_count: page.total_count,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

fn page_request(query: &TargetQuery) -> Option<NumberedPageRequest> {
    Some(NumberedPageRequest {
        page_number: PageNumber::new(query.page_number.unwrap_or(1)).ok()?,
        page_size: PageSize::new(query.page_size.unwrap_or(50)).ok()?,
    })
}

fn parse_proposal_id(value: &str) -> Option<uuid::Uuid> {
    uuid::Uuid::parse_str(value)
        .ok()
        .filter(|id| id.to_string() == value)
}

fn valid_statuses(value: &str) -> bool {
    matches!(
        value,
        "" | "accepted"
            | "proposed"
            | "stale"
            | "accepted,proposed"
            | "accepted,stale"
            | "proposed,stale"
            | "accepted,proposed,stale"
    )
}

fn invalid_request() -> Response {
    route_error(
        StatusCode::BAD_REQUEST,
        "Blueprint Change Proposal request is invalid",
    )
}
