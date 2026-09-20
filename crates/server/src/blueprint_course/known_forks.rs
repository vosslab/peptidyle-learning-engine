//! Ordinary-visibility projection of readable direct Blueprint forks.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_course::BlueprintKnownForkView;
use learning_data_access::{BlueprintLineageStore, StoredKnownBlueprintFork};

use super::{
    BlueprintCourseRouteState, instructor_session_hash, parse_blueprint_course_id,
    store_error_response,
};

pub(super) async fn list_known_forks(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path(blueprint_course_id): Path<String>,
) -> Response {
    let blueprint_course_id = match parse_blueprint_course_id(&blueprint_course_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.2/3, 8.3.1: the attested Store filters source and child
    // visibility, including Private children hidden from the source owner.
    match state
        .lineage
        .list_known_blueprint_forks(session, blueprint_course_id)
        .await
    {
        Ok(forks) => {
            let views: Vec<BlueprintKnownForkView> = forks.into_iter().map(fork_view).collect();
            // ASVS 4.1.1, 14.3.2: typed JSON; authenticated names are not cached.
            crate::auth::no_store(Json(views).into_response())
        }
        Err(error) => store_error_response(error),
    }
}

fn fork_view(fork: StoredKnownBlueprintFork) -> BlueprintKnownForkView {
    BlueprintKnownForkView {
        id: fork.id,
        short_name: fork.short_name,
        long_name: fork.long_name,
        availability: fork.availability,
        current_revision_number: fork.current_revision_number,
        source_revision_number: fork.source_revision_number,
        owner_display_name: fork.owner_display_name,
    }
}
