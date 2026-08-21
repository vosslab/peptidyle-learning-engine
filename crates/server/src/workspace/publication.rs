use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use learning_data_access::{CatalogStore, SessionStore, Store, WorkspaceDraftRevision};
use question_model::catalog::QuestionBackend;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, TaxonomyTerm};
use question_model::{DraftQuestionDefinition, WorkspaceId};
use serde::Serialize;

use crate::auth::{auth_error_response, resolve_request_session};
use crate::catalog::{BackendRegistry, BackendRegistryError};

use super::state::WorkspaceRouteState;
use super::support::{
    error_response, may_author_workspaces, revisioned_response, store_error_response,
};

/// Validates whether the stored draft can cross the publication capability
/// boundary. The request is intentionally bodyless: the server validates the
/// exact persisted draft, never a browser-supplied shadow copy.
pub(super) async fn validate_publication<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
    body: Bytes,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    if !body.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "publication validation does not accept a request body",
        );
    }
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let draft = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => return store_error_response(error),
    };
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
    if violations.is_empty()
        && let Err(message) = crate::catalog::prepare_published_source(draft.record.question.source)
    {
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    revisioned_response(draft.revision, PublicationValidationReport { violations })
}

/// Returns the safe, semantic before/after projection used by the publishing
/// confirmation. Source locators, artifacts, providers, and grading material
/// are deliberately absent even though this is an author-only route.
pub(super) async fn publication_diff<S, B>(
    State(state): State<WorkspaceRouteState<S, B>>,
    headers: HeaderMap,
    Path(workspace): Path<WorkspaceId>,
) -> Response
where
    S: Store + CatalogStore + SessionStore + 'static,
    B: BackendRegistry + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return auth_error_response(error),
    };
    if !may_author_workspaces(authenticated.record.subject.roles()) {
        return error_response(
            StatusCode::FORBIDDEN,
            "workspace authoring is not authorized",
        );
    }
    let draft = match state
        .store
        .get_draft(
            authenticated.tenant_context,
            authenticated.record.subject.user(),
            workspace,
        )
        .await
    {
        Ok(Some(draft)) => draft,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "workspace not found"),
        Err(error) => return store_error_response(error),
    };
    let revision = draft.revision;
    let draft = draft.record;
    let current = PublicationSemanticProjection::from_draft(&draft.question);
    revisioned_response(
        revision,
        PublicationDiff {
            draft_revision: revision,
            baseline: PublicationDiffBaseline::NewQuestion,
            current,
            // A new question has no published baseline. The browser's strict
            // semantic-diff contract still receives the explicit empty set.
            changed: Vec::new(),
        },
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationValidationReport {
    violations: Vec<domain::policy::PublicationViolation>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationDiff {
    /// The exact private draft revision represented by this comparison. The
    /// browser must revalidate after a draft save changes this value before it
    /// can treat the confirmation as current.
    draft_revision: WorkspaceDraftRevision,
    baseline: PublicationDiffBaseline,
    current: PublicationSemanticProjection,
    changed: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum PublicationDiffBaseline {
    NewQuestion,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationSemanticProjection {
    source_backend: QuestionBackend,
    title: String,
    prompt: PromptShape,
    response: ResponseShape,
    attempt_policy: AttemptPolicy,
    timing_policy: TimingPolicy,
    randomization: RandomizationShape,
    metadata: MetadataShape,
}

impl PublicationSemanticProjection {
    fn from_draft(question: &DraftQuestionDefinition) -> Self {
        Self::from_content(
            QuestionBackend::from(&question.source),
            &question.metadata.title,
            &question.prompt,
            &question.response,
            question.attempt_policy,
            question.timing_policy,
            &question.randomization,
            &question.metadata,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_content(
        source_backend: QuestionBackend,
        title: &str,
        prompt: &[ContentBlock],
        response: &ResponseDefinition,
        attempt_policy: AttemptPolicy,
        timing_policy: TimingPolicy,
        randomization: &RandomizationDefinition,
        metadata: &question_model::QuestionMetadata,
    ) -> Self {
        Self {
            source_backend,
            title: title.to_string(),
            prompt: PromptShape::from_blocks(prompt),
            response: ResponseShape::from_definition(response),
            attempt_policy,
            timing_policy,
            randomization: RandomizationShape::from_definition(randomization),
            metadata: MetadataShape::from_metadata(metadata),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptShape {
    blocks: Vec<PromptBlockKind>,
}

impl PromptShape {
    fn from_blocks(blocks: &[ContentBlock]) -> Self {
        Self {
            blocks: blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { .. } => PromptBlockKind::Text,
                    ContentBlock::Math { .. } => PromptBlockKind::Math,
                    ContentBlock::Image { .. } => PromptBlockKind::Image,
                    ContentBlock::Code { .. } => PromptBlockKind::Code,
                    ContentBlock::Table { .. } => PromptBlockKind::Table,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum PromptBlockKind {
    Text,
    Math,
    Image,
    Code,
    Table,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseShape {
    kind: ResponseKind,
    option_count: Option<usize>,
}

impl ResponseShape {
    fn from_definition(definition: &ResponseDefinition) -> Self {
        match definition {
            ResponseDefinition::Numeric { .. } => Self {
                kind: ResponseKind::Numeric,
                option_count: None,
            },
            ResponseDefinition::MultipleChoice { choices, .. } => Self {
                kind: ResponseKind::MultipleChoice,
                option_count: Some(choices.len()),
            },
            ResponseDefinition::ShortText { .. } => Self {
                kind: ResponseKind::ShortText,
                option_count: None,
            },
            ResponseDefinition::MultiBlank { blanks } => Self {
                kind: ResponseKind::MultiBlank,
                option_count: Some(blanks.len()),
            },
            ResponseDefinition::Matching { prompts, .. } => Self {
                kind: ResponseKind::Matching,
                option_count: Some(prompts.len()),
            },
            ResponseDefinition::Ordering { items } => Self {
                kind: ResponseKind::Ordering,
                option_count: Some(items.len()),
            },
            ResponseDefinition::Hotspot { regions, .. } => Self {
                kind: ResponseKind::Hotspot,
                option_count: Some(regions.len()),
            },
            ResponseDefinition::FileUpload { .. } => Self {
                kind: ResponseKind::FileUpload,
                option_count: None,
            },
            ResponseDefinition::ExternalTool {} => Self {
                kind: ResponseKind::ExternalTool,
                option_count: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum ResponseKind {
    Numeric,
    MultipleChoice,
    ShortText,
    MultiBlank,
    Matching,
    Ordering,
    Hotspot,
    FileUpload,
    ExternalTool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RandomizationShape {
    kind: RandomizationKind,
}

impl RandomizationShape {
    fn from_definition(definition: &RandomizationDefinition) -> Self {
        let kind = match definition {
            RandomizationDefinition::Static => RandomizationKind::Static,
            RandomizationDefinition::Seeded { .. } => RandomizationKind::Seeded,
        };
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum RandomizationKind {
    Static,
    Seeded,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetadataShape {
    tags: Vec<String>,
    taxonomy: Vec<TaxonomyTerm>,
    license: License,
    language: String,
}

impl MetadataShape {
    fn from_metadata(metadata: &question_model::QuestionMetadata) -> Self {
        Self {
            tags: metadata
                .tags
                .iter()
                .map(|tag| tag.as_str().to_string())
                .collect(),
            taxonomy: metadata.taxonomy.clone(),
            license: metadata.license.clone(),
            language: metadata.language.clone(),
        }
    }
}
