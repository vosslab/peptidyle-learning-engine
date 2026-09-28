//! Role-neutral, authenticated-self Account Settings and appearance seam.
//!
//! This route deliberately carries no Account identifier, supplied User Role,
//! avatar catalog, or image delivery. Those are separate concerns. PostgreSQL
//! derives the Account from the installed session for this read.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::to_bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::{IntoResponse, Response},
    routing::get,
};
use learning_data_access::{
    AccountAppearanceStore, AccountTimeZoneStore, SessionTokenHash, StoreError,
    postgres::{
        PostgresAccountAppearanceStore, PostgresAccountTimeZoneStore, PostgresSessionStore,
    },
};
use question_model::{AccountTimeZone, DisplayMode, Theme};
use serde::{Deserialize, Serialize};

use crate::auth::{AuthError, resolve_session};

#[derive(Clone)]
struct RouteState {
    sessions: Arc<PostgresSessionStore>,
    time_zones: PostgresAccountTimeZoneStore,
    appearance: PostgresAccountAppearanceStore,
}

/// Registers all-role self-only settings reads and independent preference writes.
pub fn profile_settings_router(
    sessions: Arc<PostgresSessionStore>,
    time_zones: PostgresAccountTimeZoneStore,
    appearance: PostgresAccountAppearanceStore,
) -> Router {
    Router::new()
        .route("/api/profile", get(read_profile_settings))
        .route(
            "/api/account/settings",
            get(read_account_settings).put(update_account_settings),
        )
        .route(
            "/api/account/appearance/display-mode-preference",
            axum::routing::put(update_display_mode_preference),
        )
        .route(
            "/api/instructor/personal-theme",
            axum::routing::put(update_instructor_personal_theme),
        )
        .with_state(RouteState {
            sessions,
            time_zones,
            appearance,
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileSettings {
    time_zone: AccountTimeZone,
    display_mode_preference: Option<DisplayMode>,
    personal_theme: Option<Theme>,
}

/// Closed, Account-owned preference payload. The browser can name only its
/// desired display zone; the server derives both Account and role from session.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateAccountSettingsInput {
    time_zone: AccountTimeZone,
}

/// Closed payload for an Account-owned explicit mode or browser-following clear.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateDisplayModePreferenceInput {
    // The explicit deserializer makes this nullable field required: JSON null
    // clears the preference, while an omitted member is rejected.
    #[serde(deserialize_with = "required_nullable_display_mode_preference")]
    display_mode_preference: Option<DisplayMode>,
}

fn required_nullable_display_mode_preference<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DisplayMode>, D::Error> {
    Option::<DisplayMode>::deserialize(deserializer)
}

/// Closed payload for an authenticated Instructor's global page Theme.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateInstructorPersonalThemeInput {
    theme: Theme,
}

const MAX_ACCOUNT_SETTINGS_UPDATE_BYTES: usize = 256;

async fn read_profile_settings(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let token = match self_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    // ASVS 4.1.1: Json supplies the matching application/json response type.
    account_settings_response(&state, token).await
}

async fn read_account_settings(State(state): State<RouteState>, headers: HeaderMap) -> Response {
    let token = match self_session_hash(&state, &headers).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    account_settings_response(&state, token).await
}

async fn update_account_settings(
    State(state): State<RouteState>,
    request: axum::extract::Request,
) -> Response {
    let token = match self_session_hash(&state, request.headers()).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    // ASVS 1.5.2, 2.2.1--2.2.2, and 4.1.4: accept one bounded,
    // closed JSON shape only after authenticating the self-only subject.
    if !has_content_type(request.headers(), "application/json") {
        return route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Account Settings is invalid",
        );
    }
    let input = match to_bytes(request.into_body(), MAX_ACCOUNT_SETTINGS_UPDATE_BYTES).await {
        Ok(bytes) => match serde_json::from_slice::<UpdateAccountSettingsInput>(&bytes) {
            Ok(input) => input,
            Err(_) => {
                return route_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Account Settings is invalid",
                );
            }
        },
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Account Settings is too large",
            );
        }
    };
    match state
        .time_zones
        .update_authenticated_account_time_zone(token, input.time_zone)
        .await
    {
        Ok(_) => account_settings_response(&state, token).await,
        Err(error) => store_error_response(error),
    }
}

async fn account_settings_response(state: &RouteState, token: SessionTokenHash) -> Response {
    let time_zone = match state
        .time_zones
        .authenticated_account_time_zone(token)
        .await
    {
        Ok(time_zone) => time_zone,
        Err(error) => return store_error_response(error),
    };
    match state
        .appearance
        .authenticated_account_appearance(token)
        .await
    {
        Ok(appearance) => crate::auth::no_store(
            Json(ProfileSettings {
                time_zone,
                display_mode_preference: appearance.display_mode_preference,
                personal_theme: appearance.personal_theme,
            })
            .into_response(),
        ),
        Err(error) => store_error_response(error),
    }
}

async fn update_display_mode_preference(
    State(state): State<RouteState>,
    request: axum::extract::Request,
) -> Response {
    let token = match self_session_hash(&state, request.headers()).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let input = match decode_json_body::<UpdateDisplayModePreferenceInput>(
        request,
        "Display mode preference",
    )
    .await
    {
        Ok(input) => input,
        Err(response) => return *response,
    };
    match state
        .appearance
        .update_authenticated_account_display_mode_preference(token, input.display_mode_preference)
        .await
    {
        Ok(_) => account_settings_response(&state, token).await,
        Err(error) => store_error_response(error),
    }
}

async fn update_instructor_personal_theme(
    State(state): State<RouteState>,
    request: axum::extract::Request,
) -> Response {
    let token = match self_session_hash(&state, request.headers()).await {
        Ok(token) => token,
        Err(response) => return *response,
    };
    let input = match decode_json_body::<UpdateInstructorPersonalThemeInput>(
        request,
        "Instructor personal theme",
    )
    .await
    {
        Ok(input) => input,
        Err(response) => return *response,
    };
    match state
        .appearance
        .update_authenticated_instructor_personal_theme(token, input.theme)
        .await
    {
        Ok(_) => account_settings_response(&state, token).await,
        Err(error) => store_error_response(error),
    }
}

async fn decode_json_body<T: serde::de::DeserializeOwned>(
    request: axum::extract::Request,
    label: &'static str,
) -> Result<T, Box<Response>> {
    // ASVS 1.5.2, 2.2.1--2.2.2, and 4.1.4: authenticate first, then accept
    // one bounded closed JSON shape for this self-only preference write.
    if !has_content_type(request.headers(), "application/json") {
        return Err(Box::new(route_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Account Settings is invalid",
        )));
    }
    match to_bytes(request.into_body(), MAX_ACCOUNT_SETTINGS_UPDATE_BYTES).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
            Box::new(route_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                match label {
                    "Display mode preference" => "Display mode preference is invalid",
                    "Instructor personal theme" => "Instructor personal theme is invalid",
                    _ => "Account Settings is invalid",
                },
            ))
        }),
        Err(_) => Err(Box::new(route_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Account Settings is too large",
        ))),
    }
}

async fn self_session_hash(
    state: &RouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        // ASVS 8.2.2 and 8.3.1: this authenticates one current server-derived
        // Account. The browser never supplies the Account or role to authorize.
        Ok(session) => Ok(session.session_hash),
        Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Profile Settings authentication unavailable",
        ))),
    }
}

fn joined_cookie_header(headers: &HeaderMap) -> Option<String> {
    let values = headers
        .get_all(COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then(|| values.join("; "))
}

fn has_content_type(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected))
        })
}

fn store_error_response(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::InvalidRecord(_) => route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Profile Settings is invalid",
        ),
        StoreError::Conflict | StoreError::RetryableTransaction => {
            route_error(StatusCode::PRECONDITION_FAILED, "Profile Settings changed")
        }
        StoreError::LifecycleConflict | StoreError::AlreadyExists => {
            route_error(StatusCode::CONFLICT, "Profile Settings conflict")
        }
        StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Profile Settings unavailable",
        ),
    }
}

fn concealed() -> Response {
    route_error(StatusCode::NOT_FOUND, "Profile Settings not found")
}

fn route_error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use super::{DisplayMode, UpdateDisplayModePreferenceInput};

    #[test]
    fn display_mode_preference_requires_one_closed_nullable_field() {
        for (body, expected) in [
            (
                r#"{"displayModePreference":"light"}"#,
                Some(DisplayMode::Light),
            ),
            (
                r#"{"displayModePreference":"dark"}"#,
                Some(DisplayMode::Dark),
            ),
            (r#"{"displayModePreference":null}"#, None),
        ] {
            let input = serde_json::from_str::<UpdateDisplayModePreferenceInput>(body)
                .expect("valid required nullable display-mode preference");
            assert_eq!(input.display_mode_preference, expected);
        }

        assert!(serde_json::from_str::<UpdateDisplayModePreferenceInput>(r#"{}"#).is_err());
        assert!(
            serde_json::from_str::<UpdateDisplayModePreferenceInput>(
                r#"{"displayModePreference":"light","unexpected":true}"#
            )
            .is_err()
        );
    }
}
