//! Authenticated canonical Blueprint Course import and export routes.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use learning_data_access::BlueprintCourseStore;
use question_model::{CanonicalBlueprintAssessmentEntry, CanonicalBlueprintCourse};

use super::{
    BlueprintCourseRouteState, RouteLoadError, instructor_session_hash, load_view, parse_reference,
    request_checksum,
    responses::{blueprint_response, concealed, store_error_response, unavailable},
};
use crate::question_publication::HmacQuestionIdIssuer;

pub(super) async fn export(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let reference = match parse_reference(&reference) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match state
        .blueprints
        .export_blueprint_course(session, reference)
        .await
    {
        // ASVS 15.3.1: the canonical DTO is an allowlisted reusable-content
        // projection with no lineage, owner, stewardship, or operational state.
        Ok(exchange) => crate::auth::no_store(Json(exchange).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn import(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Json(exchange): Json<CanonicalBlueprintCourse>,
) -> Response {
    // ASVS 2.2.2 and 8.3.1: validate deployment-bound public identifiers and
    // authorize at the server before the Store performs the trusted import.
    if !valid_question_ids(&state.question_id_issuer, &exchange) {
        return concealed();
    }
    let checksum = match request_checksum("import-blueprint-course", &headers, &exchange) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let receipt = match state
        .blueprints
        .import_blueprint_course(session, checksum, exchange)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match load_view(&state, session, receipt.blueprint_revision.reference).await {
        Ok(view) => blueprint_response(StatusCode::CREATED, view),
        Err(RouteLoadError::Store(error)) => store_error_response(error),
        Err(RouteLoadError::Unavailable) => unavailable(),
    }
}

fn valid_question_ids(issuer: &HmacQuestionIdIssuer, exchange: &CanonicalBlueprintCourse) -> bool {
    exchange
        .modules()
        .iter()
        .flat_map(|module| module.assessments())
        .flat_map(|assessment| assessment.entries())
        .all(|entry| match entry {
            CanonicalBlueprintAssessmentEntry::Fixed {
                published_question, ..
            } => issuer.validates_question_id(&published_question.question_id),
            CanonicalBlueprintAssessmentEntry::Pool {
                question_pool_revision,
                ..
            } => issuer.validates_question_id(&question_pool_revision.question_pool_id),
        })
}
