//! Derived current-pair comparison; no comparison state or content is saved.

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use browser_api_contract::blueprint_course::{
    BlueprintComparisonAssessment, BlueprintComparisonAssessmentRelationship,
    BlueprintComparisonModule, BlueprintComparisonNames, BlueprintComparisonSide,
    BlueprintComparisonView,
};
use learning_data_access::{BlueprintComparisonSources, BlueprintLineageStore, StoreError};
use question_model::blueprint_course::{BlueprintComparisonInventory, compare_blueprint_courses};

use super::{
    BlueprintCourseRouteState, instructor_session_hash, parse_blueprint_course_id,
    store_error_response,
};

pub(super) async fn load_comparison(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    Path((left, right)): Path<(String, String)>,
) -> Response {
    // ASVS 2.2.1, 2.2.2: validate both identifiers at the trusted boundary.
    let left = match parse_blueprint_course_id(&left) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let right = match parse_blueprint_course_id(&right) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    // ASVS 8.2.2, 8.3.1: the pair reader establishes lineage relationship and
    // current visibility of both sides using the requesting Instructor.
    let sources = match state
        .lineage
        .load_blueprint_comparison_sources(session, left, right)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error_response(error),
    };
    match project_comparison(sources) {
        // ASVS 4.1.1, 14.3.2: typed JSON, never browser-cache private content.
        Ok(view) => crate::auth::no_store(Json(view).into_response()),
        Err(error) => store_error_response(error),
    }
}

fn project_comparison(
    sources: BlueprintComparisonSources,
) -> Result<BlueprintComparisonView, StoreError> {
    let comparison = compare_blueprint_courses(
        &sources.left.content.to_domain()?,
        &sources.right.content.to_domain()?,
        &sources.pool_memberships,
    )
    .map_err(|_| StoreError::InvalidRecord("Blueprint comparison content is invalid".into()))?;
    Ok(BlueprintComparisonView {
        left: comparison_side(
            comparison.left,
            sources.left.blueprint_revision_tuple,
            sources.left_short_name,
            sources.left_long_name,
            sources.left_blueprint_edit_number,
        ),
        right: comparison_side(
            comparison.right,
            sources.right.blueprint_revision_tuple,
            sources.right_short_name,
            sources.right_long_name,
            sources.right_blueprint_edit_number,
        ),
        assessment_relationships: comparison
            .relationships
            .into_iter()
            .map(|row| BlueprintComparisonAssessmentRelationship {
                left_assessment_id: row.left_assessment_id,
                right_assessment_id: row.right_assessment_id,
                shared_question_ids: row.shared_question_ids,
            })
            .collect(),
        shared_question_ids: comparison.shared_question_ids,
        left_only_question_ids: comparison.left_only_question_ids,
        right_only_question_ids: comparison.right_only_question_ids,
    })
}

pub(super) fn comparison_side(
    inventory: BlueprintComparisonInventory,
    current_revision: question_model::BlueprintRevisionTuple,
    short_name: String,
    long_name: String,
    blueprint_edit_number: question_model::BlueprintEditNumber,
) -> BlueprintComparisonSide {
    BlueprintComparisonSide {
        current_revision,
        names: BlueprintComparisonNames {
            short_name,
            long_name,
        },
        blueprint_edit_number,
        modules: inventory
            .modules
            .into_iter()
            .map(|row| BlueprintComparisonModule {
                blueprint_module_id: row.blueprint_module_id,
                position: row.position,
                label: row.label,
            })
            .collect(),
        assessments: inventory
            .assessments
            .into_iter()
            .map(|row| BlueprintComparisonAssessment {
                blueprint_assessment_id: row.blueprint_assessment_id,
                blueprint_module_id: row.blueprint_module_id,
                position: row.position,
                content: row.content,
                question_ids: row.question_ids,
            })
            .collect(),
    }
}
