#[cfg(test)]
use std::cell::Cell;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::header::IF_MATCH;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use domain::policy::PublicationViolation;
use learning_data_access::{
    CatalogStore, OwnerCorrectionAuthority, OwnerCorrectionStore, PublishDraftCommand,
    PublishedProblemRecord, SessionStore, Store, StoreError, WorkspaceDraftRevision,
};
use question_model::{
    DraftQuestionSource, ProblemId, ProblemVersionRef, PublicationScope, QuestionSource, UserRole,
    VersionId, WorkspaceId,
};
use serde::{Deserialize, Serialize};

use crate::auth::{auth_error_response, no_store, resolve_request_session};

use super::routes::CatalogRouteState;
use super::{BackendRegistry, BackendRegistryError, PublicReviewGate};
use super::{error_response, store_error_response};

// Kept behind a small helper so publication has one auditable point at which
// durable identities can be minted. In particular, source preparation must
// finish before this function is reached.
pub(crate) fn mint_publication_reference(revises: Option<ProblemVersionRef>) -> ProblemVersionRef {
    #[cfg(test)]
    {
        PUBLICATION_MINT_COUNT.with(|count| count.set(count.get() + 1));
    }
    ProblemVersionRef {
        problem: revises.map_or_else(ProblemId::generate, |reference| reference.problem),
        version: VersionId::generate(),
    }
}

/// Dispatches publication through the only route that can carry the
/// authenticated original-owner capability.  Every publication surface calls
/// this helper so a revision can neither silently use ordinary publication nor
/// be forgotten by a backend-specific route.
pub(crate) async fn dispatch_publication<S>(
    store: &S,
    authenticated: &crate::auth::AuthenticatedSession,
    command: PublishDraftCommand,
) -> Result<PublishedProblemRecord, StoreError>
where
    S: CatalogStore + OwnerCorrectionStore,
{
    if command.expected_draft.revises.is_some() {
        OwnerCorrectionStore::publish_owner_correction(
            store,
            authenticated.tenant_context,
            OwnerCorrectionAuthority {
                actor: authenticated.record.subject.user(),
                session: authenticated.session_hash(),
            },
            command,
        )
        .await
    } else {
        CatalogStore::publish_draft(
            store,
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            command,
        )
        .await
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static PUBLICATION_MINT_COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Converts a draft locator into publication inputs before any immutable ID is
/// minted. The generic route only owns native publication: every external
/// backend needs a server-prepared immutable source artifact supplied by its
/// dedicated import/broker workflow.
///
/// This deliberately runs before [`mint_publication_reference`]: an
/// unprepared external source (currently iMathAS) is a refused draft state,
/// not a partially-created published identity.
pub(crate) fn prepare_published_source(
    source: DraftQuestionSource,
) -> Result<QuestionSource, &'static str> {
    match source {
        DraftQuestionSource::Native { family } => Ok(QuestionSource::Native { family }),
        DraftQuestionSource::Imathas { .. } => {
            Err("iMathAS publication requires a verified source snapshot and integration profile")
        }
        DraftQuestionSource::Webwork { .. }
        | DraftQuestionSource::Qti { .. }
        | DraftQuestionSource::H5p { .. } => {
            Err("external publication requires a server-prepared immutable source artifact")
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PublishProblemRequest {
    scope: PublicationScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublishRevisionError {
    Missing,
    Malformed,
}

/// Parses the one strong workspace ETag required for publication.
///
/// Publication consumes an exact mutable draft to mint an immutable catalog
/// version. Requiring the current revision prevents a stale browser tab from
/// publishing a collaborator's later edit.
fn required_publish_revision(
    headers: &HeaderMap,
) -> Result<WorkspaceDraftRevision, PublishRevisionError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(PublishRevisionError::Missing);
    };
    if values.next().is_some() {
        return Err(PublishRevisionError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| PublishRevisionError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(PublishRevisionError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(PublishRevisionError::Malformed);
    }
    let numeric = value
        .parse::<u64>()
        .map_err(|_| PublishRevisionError::Malformed)?;
    if numeric == 0 || numeric > i64::MAX as u64 {
        return Err(PublishRevisionError::Malformed);
    }
    serde_json::from_str(value).map_err(|_| PublishRevisionError::Malformed)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationValidationFailure {
    error: &'static str,
    violations: Vec<PublicationViolation>,
}

pub(super) async fn publish_problem<S, B, R>(
    State(state): State<CatalogRouteState<S, B, R>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    Json(request): Json<PublishProblemRequest>,
) -> Response
where
    S: Store + CatalogStore + OwnerCorrectionStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
    R: PublicReviewGate + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_publish(authenticated.record.subject.roles(), request.scope) {
        return error_response(StatusCode::FORBIDDEN, "publication is not authorized");
    }
    let expected_revision = match required_publish_revision(&headers) {
        Ok(revision) => revision,
        Err(PublishRevisionError::Missing) => {
            return error_response(
                StatusCode::PRECONDITION_REQUIRED,
                "If-Match is required to publish a workspace",
            );
        }
        Err(PublishRevisionError::Malformed) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "If-Match must contain one strong workspace revision",
            );
        }
    };
    let publisher = authenticated.record.subject.user();
    let draft = match state
        .store
        .get_draft(authenticated.tenant_context, publisher, workspace)
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "draft not found"),
        Err(error) => return store_error_response(error),
    };
    if draft.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    // Storage validates this for normal writes, but old imports or a repaired
    // database can still contain a legacy record. Refuse it at the HTTP
    // boundary before source preparation or immutable ID minting.
    if draft.record.question.metadata.validate_title().is_err() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question title is invalid",
        );
    }
    let capabilities = match state.backends.capabilities(&draft.record.question.source) {
        Ok(capabilities) => capabilities,
        Err(BackendRegistryError::Unsupported) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question backend is not registered",
            );
        }
        Err(BackendRegistryError::Unavailable(_)) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "backend registry unavailable",
            );
        }
    };
    let violations =
        domain::policy::validate_draft_for_publication(&draft.record.question, &capabilities);
    if !violations.is_empty() {
        return no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PublicationValidationFailure {
                    error: "publication validation failed",
                    violations,
                }),
            )
                .into_response(),
        );
    }
    if request.scope == PublicationScope::Public {
        match state
            .review_gate
            .allows_publication(authenticated.tenant_context, publisher, &draft.record)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                return error_response(
                    StatusCode::FORBIDDEN,
                    "public publication requires institutional review",
                );
            }
            Err(_) => {
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "publication review unavailable",
                );
            }
        }
    }
    // A review gate can await an external institutional workflow, and adapter
    // declarations can change while that happens. Re-read the actor-visible
    // draft immediately before source preparation and identity minting. The
    // Store repeats the exact-record comparison in its publication
    // transaction, closing the remaining race between this read and commit.
    let current_draft = match state
        .store
        .get_draft(authenticated.tenant_context, publisher, workspace)
        .await
    {
        Ok(Some(current_draft)) => current_draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "draft not found"),
        Err(error) => return store_error_response(error),
    };
    if current_draft.revision != expected_revision {
        return error_response(StatusCode::CONFLICT, "draft changed; reload it");
    }
    let draft = current_draft.record;
    if draft.question.metadata.validate_title().is_err() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "question title is invalid",
        );
    }
    let capabilities = match state.backends.capabilities(&draft.question.source) {
        Ok(capabilities) => capabilities,
        Err(BackendRegistryError::Unsupported) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "question backend is not registered",
            );
        }
        Err(BackendRegistryError::Unavailable(_)) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "backend registry unavailable",
            );
        }
    };
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        return no_store(
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PublicationValidationFailure {
                    error: "publication validation failed",
                    violations,
                }),
            )
                .into_response(),
        );
    }
    // Validate and freeze the source before minting either immutable ID. An
    // iMathAS locator has to be prepared by its server-owned integration into
    // a snapshot-bearing QuestionSource first.
    let published_source = match prepare_published_source(draft.question.source.clone()) {
        Ok(source) => source,
        Err(message) => {
            return error_response(StatusCode::UNPROCESSABLE_ENTITY, message);
        }
    };
    let publication = mint_publication_reference(draft.revises);
    let command = PublishDraftCommand {
        expected_draft: draft,
        expected_revision,
        publication,
        published_source,
        // Source-backed adapters are not wired to this generic route yet;
        // storage rejects them before a version can be minted.
        source_artifact: None,
        qti_promotion: None,
        flat_question_promotion: None,
        publisher,
        scope: request.scope,
        capabilities,
    };
    match dispatch_publication(state.store.as_ref(), &authenticated, command).await {
        Ok(record) => no_store((StatusCode::CREATED, Json(record.question)).into_response()),
        Err(error) => store_error_response(error),
    }
}

pub(crate) fn may_publish(roles: &[UserRole], scope: PublicationScope) -> bool {
    match scope {
        PublicationScope::Institution => roles
            .iter()
            .any(|role| matches!(role, UserRole::Instructor | UserRole::Sysadmin)),
        PublicationScope::Public => roles
            .iter()
            .any(|role| matches!(role, UserRole::Instructor | UserRole::Sysadmin)),
    }
}
