//! Assignment-definition request decoding, validation, and immutable publication resolution.

use axum::Json;
use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use learning_data_access::{AssignmentRecord, CatalogStore, SessionStore, Store, TenantContext};
use question_model::{
    AssignmentDeliveryState, AssignmentItem, AssignmentItemId, AssignmentSelectionCandidate,
    AssignmentSelectionGroup, AssignmentSelectionGroupId, Capability,
    MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES, PoolDrawAlgorithm, ProblemVersionRef, QuestionId,
};
use serde::Serialize;

use super::super::projection::{error_response, store_error_response};
use super::super::routing::{AssignmentEntryRequest, CourseRouteState};
use crate::auth::no_store;
use crate::http_refusal::{HttpRefusal, HttpResult};

const MAX_ASSIGNMENT_JSON_BYTES: usize = 64 * 1_024;

#[derive(Debug, Clone)]
pub(super) enum AssignmentPolicyValidationFact {
    SelectedProblemVariantsWithSelectionGroups,
    Capability {
        title: String,
        question_id: QuestionId,
        capability: Capability,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentCapabilityViolation {
    title: String,
    question_id: QuestionId,
    capability: Capability,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentValidationFailure {
    error: &'static str,
    violations: Vec<AssignmentCapabilityViolation>,
}

pub(super) async fn validate_assignment_request<S>(
    state: &CourseRouteState<S>,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> HttpResult<()>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    if !assignment.selection_groups.is_empty()
        && assignment.policies.variation == question_model::VariationPolicy::SelectedProblemVariants
    {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "selection groups are unavailable with the selected-problem variation policy",
        )
        .into());
    }
    let facts = collect_assignment_policy_validation_facts(state, context, assignment).await?;
    let violations = facts
        .into_iter()
        .filter_map(|fact| match fact {
            AssignmentPolicyValidationFact::SelectedProblemVariantsWithSelectionGroups => None,
            AssignmentPolicyValidationFact::Capability {
                title,
                question_id,
                capability,
            } => Some(AssignmentCapabilityViolation {
                title,
                question_id,
                capability,
            }),
        })
        .collect::<Vec<_>>();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(AssignmentValidationFailure {
                    error: "assignment configuration is not supported",
                    violations,
                }),
            )
                .into_response(),
        )
        .into())
    }
}

/// Collects the browser-safe facts that arise from a fully resolved
/// assignment policy. The caller owns the route-specific response envelope;
/// content saves retain their existing capability-only refusal contract.
pub(super) async fn collect_assignment_policy_validation_facts<S>(
    state: &CourseRouteState<S>,
    context: TenantContext,
    assignment: &AssignmentRecord,
) -> HttpResult<Vec<AssignmentPolicyValidationFact>>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    let mut facts = Vec::new();
    if !assignment.selection_groups.is_empty()
        && assignment.policies.variation == question_model::VariationPolicy::SelectedProblemVariants
    {
        facts.push(AssignmentPolicyValidationFact::SelectedProblemVariantsWithSelectionGroups);
    }
    let references = assignment.references().collect::<Vec<_>>();
    let mut selected = Vec::with_capacity(references.len());
    let mut display = std::collections::BTreeMap::new();
    for reference in &references {
        let Some(published) = state
            .store
            .get_catalog_problem(context, *reference)
            .await
            .map_err(|error| HttpRefusal::from(store_error_response(error)))?
        else {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment references a missing or hidden published version",
            )
            .into());
        };
        if !published.lifecycle.is_assignable() {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "assignment references a nonassignable published version",
            )
            .into());
        }
        display.insert(
            *reference,
            (
                published.question.metadata.title.clone(),
                published.question_id.clone(),
            ),
        );
        selected.push(domain::policy::AssignmentQuestionConfig {
            question: published.question,
            backend_capabilities: published.capabilities,
        });
    }
    facts.extend(
        domain::policy::validate_assignment_config(&domain::policy::AssignmentConfig {
            questions: selected,
            required_capabilities: Vec::new(),
        })
        .into_iter()
        .map(|violation| {
            let reference = assignment
                .references()
                .find(|reference| reference.version == violation.question)
                .expect("domain validation only reports a selected question version");
            let (title, question_id) = display
                .get(&reference)
                .expect("every selected question has its immutable title")
                .clone();
            AssignmentPolicyValidationFact::Capability {
                title,
                question_id,
                capability: violation.capability,
            }
        }),
    );
    Ok(facts)
}

/// Resolves the one ordered browser definition into a complete internal
/// assignment definition. The browser can name only public Question IDs and
/// teaching choices; all stable identities, immutable publication bindings,
/// and draw algorithm choices originate here on the authenticated server.
pub(super) async fn resolve_assignment_entries<S>(
    state: &CourseRouteState<S>,
    context: TenantContext,
    entries: Vec<AssignmentEntryRequest>,
    current: Option<&AssignmentRecord>,
) -> HttpResult<(Vec<AssignmentItem>, Vec<AssignmentSelectionGroup>)>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    // ASVS 2.2.1 and 2.2.2: enforce documented resource limits at the
    // trusted server boundary before catalog resolution can consume work.
    validate_assignment_entry_cardinalities(&entries)?;
    let mut seen_question_ids = std::collections::BTreeSet::new();
    let mut seen_positions = std::collections::BTreeSet::new();
    let mut used_fixed_ids = std::collections::BTreeSet::new();
    let mut used_group_ids = std::collections::BTreeSet::new();
    let mut items = Vec::new();
    let mut groups = Vec::new();

    for entry in entries {
        match entry {
            AssignmentEntryRequest::Fixed {
                question_id,
                position,
                points_possible,
                delivery_state,
                scoring_mode,
            } => {
                if !seen_positions.insert(position) {
                    return Err(error_response(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "each assignment entry needs a distinct position",
                    )
                    .into());
                }
                let reference = resolve_one_assignable_question_id(
                    state,
                    context,
                    question_id,
                    &mut seen_question_ids,
                )
                .await?;
                let id = current
                    .and_then(|record| {
                        record.items.iter().find(|item| {
                            item.reference == reference && !used_fixed_ids.contains(&item.id)
                        })
                    })
                    .map(|item| item.id)
                    .unwrap_or_else(AssignmentItemId::generate);
                used_fixed_ids.insert(id);
                items.push(AssignmentItem {
                    id,
                    reference,
                    position,
                    points_possible,
                    delivery_state,
                    scoring_mode,
                });
            }
            AssignmentEntryRequest::SelectionGroup {
                candidate_question_ids,
                position,
                draw_count,
                points_per_item,
                ordering,
            } => {
                if !seen_positions.insert(position) {
                    return Err(error_response(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "each assignment entry needs a distinct position",
                    )
                    .into());
                }
                let mut references = Vec::with_capacity(candidate_question_ids.len());
                for question_id in candidate_question_ids {
                    references.push(
                        resolve_one_assignable_question_id(
                            state,
                            context,
                            question_id,
                            &mut seen_question_ids,
                        )
                        .await?,
                    );
                }
                let prior = current.and_then(|record| {
                    record.selection_groups.iter().find(|group| {
                        !used_group_ids.contains(&group.id)
                            && group
                                .candidates
                                .iter()
                                .map(|candidate| candidate.reference)
                                .eq(references.iter().copied())
                    })
                });
                let id = prior
                    .map(|group| group.id)
                    .unwrap_or_else(AssignmentSelectionGroupId::generate);
                used_group_ids.insert(id);
                let candidates = references
                    .into_iter()
                    .enumerate()
                    .map(|(candidate_position, reference)| {
                        let prior_candidate = prior.and_then(|group| {
                            group
                                .candidates
                                .get(candidate_position)
                                .filter(|candidate| candidate.reference == reference)
                        });
                        AssignmentSelectionCandidate {
                            id: prior_candidate
                                .map(|candidate| candidate.id)
                                .unwrap_or_else(AssignmentItemId::generate),
                            position: u32::try_from(candidate_position)
                                .expect("request body candidate count fits u32"),
                            reference,
                            delivery_state: prior_candidate
                                .map(|candidate| candidate.delivery_state)
                                .unwrap_or(AssignmentDeliveryState::Active),
                        }
                    })
                    .collect();
                groups.push(AssignmentSelectionGroup {
                    id,
                    position,
                    draw_count,
                    points_per_item,
                    ordering,
                    algorithm: PoolDrawAlgorithm::V1,
                    candidates,
                });
            }
        }
    }
    if !seen_positions
        .iter()
        .copied()
        .eq(0..u32::try_from(seen_positions.len()).expect("request body entry count fits u32"))
    {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "assignment entries must use consecutive positions starting at zero",
        )
        .into());
    }
    Ok((items, groups))
}

/// Validates every browser-controlled collection bound before any Question ID
/// enters the catalog resolver. PostgreSQL repeats these limits in its private
/// definition codec as defense in depth.
fn validate_assignment_entry_cardinalities(entries: &[AssignmentEntryRequest]) -> HttpResult<()> {
    if entries.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Use no more than 1024 ordered assignment entries.",
        )
        .into());
    }
    let mut total_candidates = 0_usize;
    for entry in entries {
        let AssignmentEntryRequest::SelectionGroup {
            candidate_question_ids,
            draw_count,
            ..
        } = entry
        else {
            continue;
        };
        if candidate_question_ids.is_empty() {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Add at least one candidate Question ID to each selection group.",
            )
            .into());
        }
        if candidate_question_ids.len() > MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Keep each selection group to 1024 candidate Question IDs or fewer.",
            )
            .into());
        }
        total_candidates = total_candidates
            .checked_add(candidate_question_ids.len())
            .ok_or_else(|| {
                HttpRefusal::from(error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Keep the assignment within 8192 total candidate Question IDs.",
                ))
            })?;
        if total_candidates > MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Keep the assignment within 8192 total candidate Question IDs.",
            )
            .into());
        }
        if *draw_count == 0
            || usize::try_from(*draw_count)
                .ok()
                .is_none_or(|count| count > candidate_question_ids.len())
        {
            return Err(error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Set a draw count that fits the candidate Question IDs in its selection group.",
            )
            .into());
        }
    }
    Ok(())
}

/// Consumes an Instructor-authorized authoring request as one bounded JSON value.
pub(super) async fn assignment_json_body(request: Request) -> HttpResult<serde_json::Value> {
    // ASVS 1.5.2: deserialize one closed JSON value only after authorization
    // and after enforcing the route's explicit resource bound.
    if !request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim() == "application/json")
        })
    {
        return Err(error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Use the assignment editor to send JSON.",
        )
        .into());
    }
    let bytes = to_bytes(request.into_body(), MAX_ASSIGNMENT_JSON_BYTES + 1)
        .await
        .map_err(|_| {
            HttpRefusal::from(error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Use a smaller assignment definition, then try again.",
            ))
        })?;
    if bytes.len() > MAX_ASSIGNMENT_JSON_BYTES {
        return Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Use a smaller assignment definition, then try again.",
        )
        .into());
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Use the assignment editor to send a complete valid assignment definition.",
        )
        .into()
    })
}

async fn resolve_one_assignable_question_id<S>(
    state: &CourseRouteState<S>,
    context: TenantContext,
    question_id: QuestionId,
    seen: &mut std::collections::BTreeSet<QuestionId>,
) -> HttpResult<ProblemVersionRef>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    if !seen.insert(question_id.clone()) {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "each Question ID can appear once in an assignment definition",
        )
        .into());
    }
    let Some(record) = state
        .store
        .resolve_catalog_problem(context, question_model::ProblemDisplayRef { question_id })
        .await
        .map_err(|error| HttpRefusal::from(store_error_response(error)))?
    else {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "assignment Question ID is unavailable",
        )
        .into());
    };
    if !record.lifecycle.is_assignable() {
        return Err(error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "assignment Question ID is not assignable",
        )
        .into());
    }
    Ok(ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    })
}

/// Resolves the one browser-supplied public Question ID accepted by a focused
/// fixed-slot replacement. The Store command receives only this server-owned
/// immutable publication reference.
pub(super) async fn resolve_assignable_question_id<S>(
    state: &CourseRouteState<S>,
    context: TenantContext,
    question_id: QuestionId,
) -> HttpResult<ProblemVersionRef>
where
    S: Store + CatalogStore + SessionStore + 'static,
{
    // ASVS 2.2.1 and 2.2.2: validate the closed browser request at the
    // trusted boundary, then resolve the public locator server-side.
    let mut seen = std::collections::BTreeSet::new();
    resolve_one_assignable_question_id(state, context, question_id, &mut seen).await
}
