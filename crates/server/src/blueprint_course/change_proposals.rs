//! Authenticated participant discovery/review and one receiving-owner acceptance.

use super::{
    BlueprintCourseRouteState, change_proposal_view, concealed, instructor_session_hash,
    route_error, store_error_response, unavailable,
};
use axum::{
    Json,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use browser_api_contract::blueprint_change_proposal::{
    BlueprintChangeProposalAcceptanceRequest, BlueprintChangeProposalCreateRequest,
    BlueprintChangeProposalPageView,
};
use learning_data_access::{
    AcceptBlueprintChangeProposalInput, BlueprintChangeProposalListScope,
    BlueprintChangeProposalStore, CreateBlueprintChangeProposalInput, Cursor, PageRequest,
    PageSize, SessionTokenHash,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    cursor: Option<String>,
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
    cursor: Option<String>,
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProposalCursor {
    version: u8,
    query_fingerprint: String,
    page_size: u16,
    after: String,
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
            cursor: query.cursor,
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
    let fingerprint = query_fingerprint(&scope, &query, session);
    let page = match page_request(&query, &fingerprint) {
        Some(value) => value,
        None => return invalid_request(),
    };
    let size = page.size.get();
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
            let next_cursor = match page.next_cursor {
                Some(after) => match encode_cursor(&fingerprint, size, after) {
                    Some(value) => Some(value),
                    None => return unavailable(),
                },
                None => None,
            };
            // ASVS 4.1.1, 14.2.2: typed participant facts never cache Private evidence.
            crate::auth::no_store(
                Json(BlueprintChangeProposalPageView {
                    items: page
                        .items
                        .into_iter()
                        .map(change_proposal_view::summary)
                        .collect(),
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

fn scope_fields(
    scope: &BlueprintChangeProposalListScope,
) -> (Option<question_model::BlueprintCourseId>, bool) {
    match scope {
        BlueprintChangeProposalListScope::Mine => (None, true),
        BlueprintChangeProposalListScope::Target(blueprint_course_id) => {
            (Some(blueprint_course_id.clone()), false)
        }
    }
}

fn query_fingerprint(
    scope: &BlueprintChangeProposalListScope,
    query: &TargetQuery,
    session: SessionTokenHash,
) -> String {
    let (target, mine) = scope_fields(scope);
    let value = serde_json::to_vec(&(
        target,
        mine,
        query.text.trim(),
        query.sort,
        &query.status,
        session.to_string(),
    ))
    .expect("Blueprint Proposal query serializes");
    URL_SAFE_NO_PAD.encode(Sha256::digest(value))
}

fn page_request(query: &TargetQuery, fingerprint: &str) -> Option<PageRequest> {
    // ASVS 2.2.1/2/3: bind bounded continuation to the session, query, and page size.
    let size = PageSize::new(query.page_size.unwrap_or(20)).ok()?;
    let after = match query.cursor.as_ref() {
        Some(token) => {
            if token.len() > 512 {
                return None;
            }
            let bytes = URL_SAFE_NO_PAD.decode(token).ok()?;
            let cursor: ProposalCursor = serde_json::from_slice(&bytes).ok()?;
            let (timestamp, id) = cursor.after.split_once('|')?;
            if cursor.version != 2
                || cursor.query_fingerprint != fingerprint
                || cursor.page_size != size.get()
                || !valid_timestamp_key(timestamp)
                || parse_proposal_id(id).is_none()
            {
                return None;
            }
            Some(Cursor::parse(cursor.after).ok()?)
        }
        None => None,
    };
    Some(PageRequest { after, size })
}

fn encode_cursor(fingerprint: &str, page_size: u16, after: Cursor) -> Option<String> {
    serde_json::to_vec(&ProposalCursor {
        version: 2,
        query_fingerprint: fingerprint.to_owned(),
        page_size,
        after: after.as_str().to_owned(),
    })
    .ok()
    .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
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

fn valid_timestamp_key(value: &str) -> bool {
    // ASVS 2.2.1: reject malformed query calendars as 400 before SQL error mapping.
    let shape = value.len() == 27
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            10 => byte == b'T',
            13 | 16 => byte == b':',
            19 => byte == b'.',
            26 => byte == b'Z',
            _ => byte.is_ascii_digit(),
        });
    if !shape {
        return false;
    }
    let Ok(year) = value[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(month) = cookie::time::Month::try_from(month) else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    let Ok(hour) = value[11..13].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = value[14..16].parse::<u8>() else {
        return false;
    };
    let Ok(second) = value[17..19].parse::<u8>() else {
        return false;
    };
    let Ok(microsecond) = value[20..26].parse::<u32>() else {
        return false;
    };
    year > 0
        && cookie::time::Date::from_calendar_date(year, month, day).is_ok()
        && cookie::time::Time::from_hms_micro(hour, minute, second, microsecond).is_ok()
}

fn invalid_request() -> Response {
    route_error(
        StatusCode::BAD_REQUEST,
        "Blueprint Change Proposal request is invalid",
    )
}
