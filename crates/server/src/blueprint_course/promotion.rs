//! Narrow Sysadmin-only lineage promotion, independent of Blueprint Revisions.

use super::{
    BlueprintCourseRouteState, concealed, expected_metadata_etag, joined_cookie_header,
    route_error, store_error_response, unavailable,
};
use crate::auth::{AuthError, resolve_session};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header::ETAG},
    response::{IntoResponse, Response},
};
use learning_data_access::{BlueprintPromotionStore, SessionTokenHash};
use question_model::{BlueprintCourseReference, ProductRole};
use serde::Deserialize;

// ASVS 2.2.1/8.2.3: exact boolean command; no arbitrary metadata assignment.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PromotionInput {
    promoted: bool,
}

async fn sysadmin_session_hash(
    state: &BlueprintCourseRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    // ASVS 8.2.1/8.3.1: early admission only; SQL repeats active-role authorization.
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == ProductRole::Sysadmin => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

pub(super) async fn load_promotion(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> Response {
    let session = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let reference = match reference.parse::<BlueprintCourseReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    match state
        .blueprints
        .load_blueprint_promotion(session, reference)
        .await
    {
        Ok(value) => promotion_response(value),
        Err(error) => store_error_response(error),
    }
}

pub(super) async fn set_promotion(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    input: Result<Json<PromotionInput>, JsonRejection>,
) -> Response {
    let session = match sysadmin_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let reference = match reference.parse::<BlueprintCourseReference>() {
        Ok(value) => value,
        Err(_) => return concealed(),
    };
    let expected = match expected_metadata_etag(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Json(input) = match input {
        Ok(value) => value,
        Err(_) => return route_error(StatusCode::BAD_REQUEST, "Blueprint promotion is invalid"),
    };
    match state
        .blueprints
        .set_blueprint_promotion(session, reference, expected, input.promoted)
        .await
    {
        Ok(value) => promotion_response(value),
        Err(error) => store_error_response(error),
    }
}

fn promotion_response(value: learning_data_access::StoredBlueprintPromotion) -> Response {
    let etag = value.metadata_etag;
    let mut response = crate::auth::no_store(Json(value).into_response());
    match HeaderValue::from_str(&format!("\"{etag}\"")) {
        Ok(value) => {
            response.headers_mut().insert(ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promotion_command_accepts_only_an_exact_boolean_field() {
        assert!(
            serde_json::from_str::<PromotionInput>(r#"{"promoted":true}"#)
                .expect("exact promotion command")
                .promoted
        );
        for input in [
            r#"{"promoted":"true"}"#,
            r#"{"promoted":null}"#,
            r#"{"promoted":false,"availability":"public"}"#,
            r#"{}"#,
        ] {
            assert!(serde_json::from_str::<PromotionInput>(input).is_err());
        }
    }
}
