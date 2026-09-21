//! HTTP helpers and publication error mapping for private Authoring routes.

use axum::{
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CONTENT_TYPE, COOKIE, ETAG, IF_MATCH},
    },
    response::{IntoResponse, Response},
};
use learning_data_access::{
    DraftQuestionEditNumber, DraftQuestionUuid, SessionTokenHash, StoreError,
};
use question_model::{
    PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionAuthor, QuestionAuthorDisplayName,
    QuestionAuthorship, QuestionRevisionNumber,
};
use uuid::Uuid;

use super::{AuthoringRouteState, PLE_QUESTION_JSON_MEDIA_TYPE, concealed, private_error};
use crate::auth::{AuthError, resolve_session};

pub(super) fn existing_parent_published_question_revision_tuple(
    question_id: String,
    parent_revision_number: u32,
) -> Result<PublishedQuestionRevisionTuple, ()> {
    let question_id = question_id.parse::<PublishedQuestionId>().map_err(|_| ())?;
    let revision_number = QuestionRevisionNumber::new(parent_revision_number).map_err(|_| ())?;
    Ok(PublishedQuestionRevisionTuple {
        published_question_id: question_id,
        revision_number,
    })
}

pub(super) fn question_authorship(authors: Vec<String>) -> Result<QuestionAuthorship, ()> {
    let authors = authors
        .into_iter()
        .map(|name| {
            QuestionAuthorDisplayName::new(name).map(|display_name| QuestionAuthor { display_name })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    QuestionAuthorship::new(authors).map_err(|_| ())
}

pub(crate) fn parse_draft_question_uuid(value: &str) -> Result<DraftQuestionUuid, Box<Response>> {
    let parsed = Uuid::parse_str(value).map_err(|_| Box::new(concealed()))?;
    (parsed.to_string() == value)
        .then_some(DraftQuestionUuid::from_uuid(parsed))
        .ok_or_else(|| Box::new(concealed()))
}

pub(crate) fn expected_edit_number(
    headers: &HeaderMap,
) -> Result<DraftQuestionEditNumber, Box<Response>> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(header) = values.next() else {
        return Err(Box::new(private_error(
            StatusCode::PRECONDITION_REQUIRED,
            "Draft Question Edit Number is required",
        )));
    };
    let value = match (header.to_str(), values.next()) {
        (Ok(value), None) => value,
        _ => {
            return Err(Box::new(private_error(
                StatusCode::BAD_REQUEST,
                "Draft Question Edit Number is invalid",
            )));
        }
    };
    let Some(number) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(Box::new(private_error(
            StatusCode::BAD_REQUEST,
            "Draft Question Edit Number is invalid",
        )));
    };
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| DraftQuestionEditNumber::new(number).ok())
        .ok_or_else(|| {
            Box::new(private_error(
                StatusCode::BAD_REQUEST,
                "Draft Question Edit Number is invalid",
            ))
        })
}

pub(super) fn etag(edit_number: DraftQuestionEditNumber) -> Result<HeaderValue, ()> {
    HeaderValue::from_str(&format!("\"{}\"", edit_number.as_postgres_bigint())).map_err(|_| ())
}

pub(super) fn edit_number_response(edit_number: DraftQuestionEditNumber) -> Response {
    let mut response = crate::auth::no_store(StatusCode::NO_CONTENT.into_response());
    match etag(edit_number) {
        Ok(value) => response.headers_mut().insert(ETAG, value),
        Err(()) => {
            return private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Authoring draft is unavailable",
            );
        }
    };
    response
}

pub(super) fn is_ple_question_json_request(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim() == PLE_QUESTION_JSON_MEDIA_TYPE)
        })
}

pub(crate) async fn instructor_session_hash(
    state: &AuthoringRouteState,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    match resolve_session(
        state.sessions.as_ref(),
        joined_cookie_header(headers).as_deref(),
    )
    .await
    {
        Ok(session) if session.record.product_role == question_model::ProductRole::Instructor => {
            Ok(session.session_hash)
        }
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(private_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Authoring authentication is unavailable",
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

pub(crate) fn private_store_error(error: StoreError) -> Response {
    match error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before this operation",
        ),
        StoreError::LifecycleConflict => {
            private_error(StatusCode::CONFLICT, "Draft Question lifecycle conflict")
        }
        StoreError::InvalidRecord(_) => private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question is invalid",
        ),
        StoreError::AlreadyExists => {
            private_error(StatusCode::CONFLICT, "Draft Question operation conflicts")
        }
        StoreError::LeaseLost
        | StoreError::Unavailable(_)
        | StoreError::AssessmentActivity(_)
        | StoreError::TimedOut => {
            private_error(StatusCode::SERVICE_UNAVAILABLE, "Authoring is unavailable")
        }
    }
}

pub(super) fn publication_error(
    error: crate::question_publication::QuestionPublicationError,
) -> Response {
    match error {
        crate::question_publication::QuestionPublicationError::StaleQuestionRevision => {
            private_error(
                StatusCode::PRECONDITION_FAILED,
                "Question Revision changed before publication",
            )
        }
        crate::question_publication::QuestionPublicationError::Store(
            StoreError::Conflict | StoreError::RetryableTransaction,
        ) => private_error(
            StatusCode::PRECONDITION_FAILED,
            "Draft Question changed before publication",
        ),
        crate::question_publication::QuestionPublicationError::Store(
            StoreError::LifecycleConflict,
        ) => private_error(
            StatusCode::CONFLICT,
            "Question publication lifecycle conflict",
        ),
        crate::question_publication::QuestionPublicationError::Store(error) => {
            private_store_error(error)
        }
        crate::question_publication::QuestionPublicationError::IdentityCollisions
        | crate::question_publication::QuestionPublicationError::ObjectStore(_)
        | crate::question_publication::QuestionPublicationError::SourceObjectRecordMismatch
        | crate::question_publication::QuestionPublicationError::QuestionIdIssuance(_) => {
            private_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Question Publication is unavailable",
            )
        }
    }
}
