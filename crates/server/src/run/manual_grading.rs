//! Instructor-only current manual-evaluation routes.
//!
//! The Store remains the authorization and concurrency boundary. This module
//! validates the HTTP representation and projects only the learner evidence
//! and current evaluation required for an instructor to act.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::header::{ETAG, IF_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use learning_data_access::{
    EvaluationRevision, ManualCredit, ManualEvaluationRecord, ManualEvaluationStatus,
    ManualGradeActionId, ManualGradingStore, SetManualGradeCommand,
};
use question_model::{QuestionAttemptId, StudentResponse};
use serde::{Deserialize, Serialize};

use super::{
    CatalogStore, RunBackend, RunRouteState, SessionStore, Store, error_response, no_store,
    resolve_request_session, store_error_response,
};

const IDEMPOTENCY_HEADER: &str = "idempotency-key";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ManualGradeRequest {
    credit_fraction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManualEvaluationView {
    attempt: QuestionAttemptId,
    response: StudentResponse,
    status: &'static str,
    credit_fraction: Option<String>,
    revision: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManualGradeReceiptView {
    action: uuid::Uuid,
    attempt: QuestionAttemptId,
    resulting_revision: u64,
    scoring_generation: u64,
    occurred_at: question_model::ActivityTimestamp,
}

pub(super) async fn get_manual_grade<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return super::auth_error_response(error),
    };
    let actor = authenticated.record.subject.user();
    let evaluation = match state
        .store
        .get_manual_evaluation_for_edit(authenticated.tenant_context, actor, attempt_id)
        .await
    {
        Ok(Some(evaluation)) => evaluation,
        // This Store call proves both existence and direct-instructor access.
        Ok(None)
        | Err(learning_data_access::StoreError::NotFound)
        | Err(learning_data_access::StoreError::Forbidden) => {
            return error_response(StatusCode::NOT_FOUND, "attempt not found");
        }
        Err(error) => return store_error_response(error),
    };
    let response = match current_response(
        state.store.as_ref(),
        authenticated.tenant_context,
        attempt_id,
    )
    .await
    {
        Ok(response) => response,
        Err(response) => return response,
    };
    manual_grade_response(evaluation, response)
}

pub(super) async fn put_manual_grade<S, B>(
    State(state): State<RunRouteState<S, B>>,
    headers: HeaderMap,
    Path(attempt_id): Path<QuestionAttemptId>,
    Json(request): Json<ManualGradeRequest>,
) -> Response
where
    S: Store + CatalogStore + ManualGradingStore + SessionStore + 'static,
    B: RunBackend + 'static,
{
    let authenticated = match resolve_request_session(state.store.as_ref(), &headers).await {
        Ok(authenticated) => authenticated,
        Err(error) => return super::auth_error_response(error),
    };
    let expected_revision = match required_revision(&headers) {
        Ok(revision) => revision,
        Err(RequiredRevisionError::Missing) => {
            return error_response(StatusCode::PRECONDITION_REQUIRED, "If-Match is required");
        }
        Err(RequiredRevisionError::Malformed) => {
            return error_response(StatusCode::BAD_REQUEST, "If-Match is invalid");
        }
    };
    let action = match manual_grade_action(&headers) {
        Ok(action) => action,
        Err(()) => return error_response(StatusCode::BAD_REQUEST, "idempotency-key is invalid"),
    };
    let credit = match canonical_credit(&request.credit_fraction) {
        Ok(credit) => credit,
        Err(()) => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "creditFraction must be a canonical bounded decimal",
            );
        }
    };
    let actor = authenticated.record.subject.user();
    let receipt = match state
        .store
        .set_manual_grade(
            authenticated.tenant_context,
            SetManualGradeCommand {
                action,
                actor,
                attempt: attempt_id,
                expected_revision,
                credit,
            },
        )
        .await
    {
        Ok(receipt) => receipt,
        Err(learning_data_access::StoreError::NotFound)
        | Err(learning_data_access::StoreError::Forbidden) => {
            return error_response(StatusCode::NOT_FOUND, "attempt not found");
        }
        // The compact Store contract deliberately keeps stale revisions and
        // a reused action with different content as the same conflict class.
        Err(error) => return store_error_response(error),
    };
    manual_grade_receipt_response(receipt)
}

async fn current_response<S>(
    store: &S,
    context: learning_data_access::TenantContext,
    attempt: QuestionAttemptId,
) -> Result<StudentResponse, Response>
where
    S: Store + ?Sized,
{
    let attempt = store
        .get_question_attempt(context, attempt)
        .await
        .map_err(store_error_response)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "attempt not found"))?;
    attempt
        .response
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "attempt not found"))
}

fn manual_grade_response(
    evaluation: ManualEvaluationRecord,
    response: StudentResponse,
) -> Response {
    let body = ManualEvaluationView {
        attempt: evaluation.attempt,
        response,
        status: match evaluation.status {
            ManualEvaluationStatus::NeedsManualGrading => "needsManualGrading",
            ManualEvaluationStatus::Graded => "graded",
        },
        credit_fraction: evaluation
            .credit
            .as_ref()
            .map(|credit| credit.as_canonical_decimal().to_string()),
        revision: evaluation.revision.as_u64(),
    };
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", evaluation.revision.as_u64()))
            .expect("positive evaluation revision is an ETag"),
    );
    no_store(response)
}

fn manual_grade_receipt_response(receipt: learning_data_access::ManualGradeReceipt) -> Response {
    let body = ManualGradeReceiptView {
        action: receipt.action.as_uuid(),
        attempt: receipt.attempt,
        resulting_revision: receipt.resulting_revision.as_u64(),
        scoring_generation: receipt.scoring_generation.value(),
        occurred_at: receipt.occurred_at,
    };
    let mut response = Json(body).into_response();
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&format!("\"{}\"", receipt.resulting_revision.as_u64()))
            .expect("positive evaluation revision is an ETag"),
    );
    no_store(response)
}

#[derive(Clone, Copy)]
enum RequiredRevisionError {
    Missing,
    Malformed,
}

fn required_revision(headers: &HeaderMap) -> Result<EvaluationRevision, RequiredRevisionError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(RequiredRevisionError::Missing);
    };
    if values.next().is_some() {
        return Err(RequiredRevisionError::Malformed);
    }
    let value = value
        .to_str()
        .map_err(|_| RequiredRevisionError::Malformed)?;
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(RequiredRevisionError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RequiredRevisionError::Malformed);
    }
    let value = value
        .parse::<u64>()
        .map_err(|_| RequiredRevisionError::Malformed)?;
    EvaluationRevision::from_u64(value).ok_or(RequiredRevisionError::Malformed)
}

fn manual_grade_action(headers: &HeaderMap) -> Result<ManualGradeActionId, ()> {
    let mut values = headers.get_all(IDEMPOTENCY_HEADER).iter();
    let Some(value) = values.next() else {
        return Err(());
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.to_str().map_err(|_| ())?;
    let action = uuid::Uuid::parse_str(value).map_err(|_| ())?;
    Ok(ManualGradeActionId::from_uuid(action))
}

fn canonical_credit(value: &str) -> Result<ManualCredit, ()> {
    if !strict_decimal(value) {
        return Err(());
    }
    let credit = ManualCredit::parse(value).map_err(|_| ())?;
    Ok(credit)
}

fn strict_decimal(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let Some(first) = value.as_bytes().first() else {
        return false;
    };
    let (whole, fraction) = match value.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (value, None),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (*first == b'0' && whole.len() != 1)
    {
        return false;
    }
    fraction.is_none_or(|fraction| {
        !fraction.is_empty()
            && fraction.len() <= 12
            && fraction.bytes().all(|byte| byte.is_ascii_digit())
    })
}
