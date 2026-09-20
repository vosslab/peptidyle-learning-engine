//! Answer-free, no-write Instructor Student View delivery.
//!
//! Every read reauthorizes the current direct Course Instructor relationship and
//! exact Assessment Entry pin through the Store. This module creates no Student
//! identity, Student Work, Assessment Attempt, response, submission, or grade.

use std::{num::NonZeroU32, str::FromStr, sync::Arc};

use adapter_ple::{PleQuestionBackend, ResolvedPleQuestionJsonSource};
use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use axum::{
    Json, Router,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use browser_api_contract::assessment_delivery::StudentQuestionPresentation;
use domain::{QuestionPoolSelectionEntropy, select_question_pool_items};
use learning_data_access::{
    InstructorStudentViewSnapshotEntry, InstructorStudentViewSource, InstructorStudentViewStore,
    SessionStore, SessionTokenHash, StoreError,
    postgres::{PostgresInstructorStudentViewStore, PostgresSessionStore},
};
use objects::s3::S3ObjectStore;
use question_model::{
    AssessmentEditNumber, AssessmentEntryAvailability, AssessmentId, AssessmentQuestionOrderRule,
    CourseInstanceId, InstructorStudentView, InstructorStudentViewEntry,
    InstructorStudentViewNotShownReason, InstructorStudentViewQuestion, ProductRole, QuestionId,
    QuestionPresentationResponseFormat, QuestionRevisionNumber, QuestionRevisionTuple,
};

use crate::auth::{AuthError, resolve_session};

const WEBWORK_SOURCE_MEDIA_TYPE: &str = "text/x-wework-pg";

#[derive(Clone)]
struct StateData {
    sessions: Arc<PostgresSessionStore>,
    store: PostgresInstructorStudentViewStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
    browser_origin: Arc<str>,
}

enum PendingEntry {
    Presented {
        authored_position: u32,
        questions: Vec<QuestionRevisionTuple>,
    },
    NotShown {
        authored_position: u32,
    },
}

/// Registers the canonical Instructor Student View manifest and per-Question reads.
pub fn assessment_student_view_router(
    sessions: Arc<PostgresSessionStore>,
    store: PostgresInstructorStudentViewStore,
    objects: S3ObjectStore,
    webwork: Arc<WebworkAdapter<HttpWebworkRenderer>>,
    browser_origin: Arc<str>,
) -> Router {
    // ASVS 4.1.4 and 8.3.1: the trusted server exposes GET only for this
    // read-only capability; no preview mutation route exists.
    Router::new()
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/student-view",
            get(manifest),
        )
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/student-view/entries/{authored_position}/questions/{question_id}/revisions/{revision_number}/presentation",
            get(presentation),
        )
        .route(
            "/api/course-instances/{course_instance_id}/assessments/{assessment_id}/student-view/entries/{authored_position}/questions/{question_id}/revisions/{revision_number}/document",
            get(document),
        )
        .with_state(StateData {
            sessions,
            store,
            objects,
            webwork,
            browser_origin,
        })
}

async fn manifest(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment)): Path<(String, String)>,
) -> Response {
    let (course, assessment) = match route_ids(&course, &assessment) {
        Some(value) => value,
        None => return concealed(),
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let snapshot = match state
        .store
        .load_instructor_student_view_snapshot(token, course, assessment)
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    let edit_number = snapshot.assessment_edit_number;
    let view = match project_manifest(snapshot) {
        Ok(value) => value,
        Err(()) => return unavailable(),
    };
    quoted_edit_number_response(Json(view).into_response(), edit_number)
}

async fn presentation(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment, authored_position, question_id, revision_number)): Path<(
        String,
        String,
        u32,
        String,
        u32,
    )>,
) -> Response {
    let expected = match edit_header(&headers) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Some((course, assessment, question_revision_tuple)) =
        route_question_ids(&course, &assessment, &question_id, revision_number)
    else {
        return concealed();
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let source = match state
        .store
        .load_instructor_student_view_question_source(
            token,
            course,
            assessment,
            expected,
            authored_position,
            question_revision_tuple.clone(),
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    let response = match answer_free_presentation(&state, source, &question_revision_tuple).await {
        Ok(value) => Json(value).into_response(),
        Err(()) => return unavailable(),
    };
    quoted_edit_number_response(response, expected)
}

async fn document(
    State(state): State<StateData>,
    headers: HeaderMap,
    Path((course, assessment, authored_position, question_id, revision_number)): Path<(
        String,
        String,
        u32,
        String,
        u32,
    )>,
    RawQuery(raw_query): RawQuery,
) -> Response {
    let expected = match document_edit_query(raw_query.as_deref()) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Some((course, assessment, question_revision_tuple)) =
        route_question_ids(&course, &assessment, &question_id, revision_number)
    else {
        return concealed();
    };
    let token = match instructor(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let source = match state
        .store
        .load_instructor_student_view_question_source(
            token,
            course,
            assessment,
            expected,
            authored_position,
            question_revision_tuple.clone(),
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    answer_free_document(&state, source, &question_revision_tuple).await
}

fn project_manifest(
    snapshot: learning_data_access::InstructorStudentViewSnapshot,
) -> Result<InstructorStudentView, ()> {
    let mut pending = Vec::with_capacity(snapshot.entries.len());
    for entry in snapshot.entries {
        let expected_authored_position = u32::try_from(pending.len()).map_err(|_| ())?;
        let actual_authored_position = match &entry {
            InstructorStudentViewSnapshotEntry::Fixed {
                authored_position, ..
            }
            | InstructorStudentViewSnapshotEntry::Pool {
                authored_position, ..
            } => *authored_position,
        };
        if actual_authored_position != expected_authored_position {
            return Err(());
        }
        let projected = match entry {
            InstructorStudentViewSnapshotEntry::Fixed {
                authored_position,
                availability: AssessmentEntryAvailability::Retired,
                ..
            }
            | InstructorStudentViewSnapshotEntry::Pool {
                authored_position,
                availability: AssessmentEntryAvailability::Retired,
                ..
            } => PendingEntry::NotShown { authored_position },
            InstructorStudentViewSnapshotEntry::Fixed {
                authored_position,
                availability: AssessmentEntryAvailability::Available,
                question_revision_tuple,
            } => {
                verified_question_revision_tuple(&question_revision_tuple)?;
                PendingEntry::Presented {
                    authored_position,
                    questions: vec![question_revision_tuple],
                }
            }
            InstructorStudentViewSnapshotEntry::Pool {
                authored_position,
                availability: AssessmentEntryAvailability::Available,
                assessment_entry,
                members,
            } => {
                for member in &members {
                    verified_question_revision_tuple(&member.question_revision_tuple)?;
                }
                let selected =
                    select_question_pool_items(&assessment_entry, &members, selection_entropy()?)
                        .map_err(|_| ())?;
                PendingEntry::Presented {
                    authored_position,
                    questions: selected
                        .into_iter()
                        .map(|item| item.question_revision_tuple)
                        .collect(),
                }
            }
        };
        pending.push(projected);
    }

    let positions = flattened_positions(&pending, snapshot.assessment_question_order_rule)?;
    let mut entries = Vec::with_capacity(pending.len());
    for (entry_index, entry) in pending.into_iter().enumerate() {
        entries.push(match entry {
            PendingEntry::NotShown { authored_position } => InstructorStudentViewEntry::NotShown {
                authored_position,
                reason: InstructorStudentViewNotShownReason::AssessmentEntryUnavailable,
            },
            PendingEntry::Presented {
                authored_position,
                questions,
            } => InstructorStudentViewEntry::Presented {
                authored_position,
                questions: questions
                    .into_iter()
                    .enumerate()
                    .map(|(question_index, question_revision_tuple)| {
                        Ok(InstructorStudentViewQuestion {
                            position: NonZeroU32::new(positions[entry_index][question_index])
                                .ok_or(())?,
                            question_revision_tuple,
                        })
                    })
                    .collect::<Result<Vec<_>, ()>>()?,
            },
        });
    }
    Ok(InstructorStudentView {
        assessment_edit_number: snapshot.assessment_edit_number,
        status: snapshot.status,
        title: snapshot.title,
        instructions: snapshot.instructions,
        display_time_zone: snapshot.display_time_zone,
        delivery: snapshot.delivery,
        entries,
    })
}

fn flattened_positions(
    entries: &[PendingEntry],
    order: AssessmentQuestionOrderRule,
) -> Result<Vec<Vec<u32>>, ()> {
    let mut flattened = Vec::new();
    let mut positions = Vec::with_capacity(entries.len());
    for (entry_index, entry) in entries.iter().enumerate() {
        let question_count = match entry {
            PendingEntry::Presented { questions, .. } => questions.len(),
            PendingEntry::NotShown { .. } => 0,
        };
        positions.push(vec![0; question_count]);
        flattened.extend((0..question_count).map(|question_index| (entry_index, question_index)));
    }
    if order == AssessmentQuestionOrderRule::Shuffled {
        shuffle(&mut flattened)?;
    }
    for (index, (entry_index, question_index)) in flattened.into_iter().enumerate() {
        positions[entry_index][question_index] = u32::try_from(index + 1).map_err(|_| ())?;
    }
    Ok(positions)
}

fn selection_entropy() -> Result<QuestionPoolSelectionEntropy, ()> {
    // ASVS 11.5.1: selection uses 256 bits from the operating-system CSPRNG;
    // the entropy is neither accepted from nor returned to the browser.
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    Ok(QuestionPoolSelectionEntropy::from_bytes(bytes))
}

fn shuffle<T>(values: &mut [T]) -> Result<(), ()> {
    for position in 0..values.len() {
        let remaining = values.len() - position;
        let selected = position + random_index(remaining)?;
        values.swap(position, selected);
    }
    Ok(())
}

fn random_index(bound: usize) -> Result<usize, ()> {
    let bound = u64::try_from(bound).map_err(|_| ())?;
    let rejection_threshold = bound.wrapping_neg() % bound;
    loop {
        let mut bytes = [0_u8; 8];
        getrandom::fill(&mut bytes).map_err(|_| ())?;
        let value = u64::from_be_bytes(bytes);
        if value >= rejection_threshold {
            return usize::try_from(value % bound).map_err(|_| ());
        }
    }
}

async fn answer_free_presentation(
    state: &StateData,
    source: InstructorStudentViewSource,
    expected: &QuestionRevisionTuple,
) -> Result<StudentQuestionPresentation, ()> {
    match source {
        InstructorStudentViewSource::Ple {
            question_revision_tuple,
            source_object_id,
            source_object_checksum,
            source_media_type,
            question_image_renditions,
        } => {
            if &question_revision_tuple != expected
                || source_media_type != adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE
            {
                return Err(());
            }
            let resolved = ResolvedPleQuestionJsonSource::resolve(
                &state.objects,
                question_revision_tuple.clone(),
                source_object_id,
                source_object_checksum,
            )
            .await
            .map_err(|_| ())?;
            let presentation = PleQuestionBackend::new().preview_question_json(&resolved);
            let built = question_model::presentation::build_question_presentation(
                &presentation,
                &crate::assessment_delivery::question_image_renditions_from_ready(
                    &question_image_renditions,
                ),
            )
            .map_err(|_| ())?;
            Ok(StudentQuestionPresentation {
                question_revision_tuple,
                author_content_digest: built.presentation.author_content_digest,
                prompt: built.presentation.prompt,
                response: built.presentation.response,
            })
        }
        InstructorStudentViewSource::Webwork {
            question_revision_tuple,
            source_object_id,
            source_object_checksum,
            source_media_type,
            webwork_pg_path,
            ..
        } => {
            if &question_revision_tuple != expected
                || source_media_type != WEBWORK_SOURCE_MEDIA_TYPE
            {
                return Err(());
            }
            let binding =
                WebworkQuestionSourceBinding::new(question_revision_tuple.clone(), webwork_pg_path)
                    .map_err(|_| ())?;
            ResolvedWebworkQuestionSource::resolve(
                &state.objects,
                binding,
                source_object_id,
                source_object_checksum,
            )
            .await
            .map_err(|_| ())?;
            Ok(StudentQuestionPresentation {
                question_revision_tuple,
                author_content_digest: None,
                prompt: Vec::new(),
                response: QuestionPresentationResponseFormat::BackendOwned {},
            })
        }
        // iMathAS remains a PLE backend, but its production server composition is not
        // configured yet. Reauthorization has already occurred; fail closed without
        // inventing a profile or exposing a partial presentation.
        InstructorStudentViewSource::Imathas { .. } => Err(()),
    }
}

async fn answer_free_document(
    state: &StateData,
    source: InstructorStudentViewSource,
    expected: &QuestionRevisionTuple,
) -> Response {
    match source {
        InstructorStudentViewSource::Ple {
            question_revision_tuple,
            source_object_id,
            source_object_checksum,
            source_media_type,
            ..
        } => {
            if &question_revision_tuple != expected
                || source_media_type != adapter_ple::question_json::PLE_QUESTION_JSON_MEDIA_TYPE
            {
                return unavailable();
            }
            let resolved = match ResolvedPleQuestionJsonSource::resolve(
                &state.objects,
                question_revision_tuple,
                source_object_id,
                source_object_checksum,
            )
            .await
            {
                Ok(value) => value,
                Err(_) => return unavailable(),
            };
            let presentation = PleQuestionBackend::new().preview_question_json(&resolved);
            let Some(author_content) = presentation.author_content else {
                return concealed();
            };
            crate::author_content_document_route::author_content_document_response(
                &author_content,
                state.browser_origin.as_ref(),
            )
        }
        InstructorStudentViewSource::Webwork {
            question_revision_tuple,
            source_object_id,
            source_object_checksum,
            source_media_type,
            webwork_pg_path,
            ..
        } => {
            if &question_revision_tuple != expected
                || source_media_type != WEBWORK_SOURCE_MEDIA_TYPE
            {
                return unavailable();
            }
            let binding = match WebworkQuestionSourceBinding::new(
                question_revision_tuple.clone(),
                webwork_pg_path,
            ) {
                Ok(value) => value,
                Err(_) => return unavailable(),
            };
            let resolved = match ResolvedWebworkQuestionSource::resolve(
                &state.objects,
                binding,
                source_object_id,
                source_object_checksum,
            )
            .await
            {
                Ok(value) => value,
                Err(_) => return unavailable(),
            };
            let document = match state
                .webwork
                .preview_document(
                    match question_seed() {
                        Ok(value) => value,
                        Err(()) => return unavailable(),
                    },
                    &resolved,
                )
                .await
            {
                Ok(value) => value,
                Err(_) => return unavailable(),
            };
            match String::from_utf8(document) {
                Ok(value) => {
                    crate::webwork_document_route::preview_backend_document_response(value)
                }
                Err(_) => unavailable(),
            }
        }
        InstructorStudentViewSource::Imathas { .. } => unavailable(),
    }
}

fn question_seed() -> Result<question_model::generation::QuestionSeed, ()> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    Ok(question_model::generation::QuestionSeed::new(
        u64::from_be_bytes(bytes),
    ))
}

fn route_ids(course: &str, assessment: &str) -> Option<(CourseInstanceId, AssessmentId)> {
    Some((course.parse().ok()?, assessment.parse().ok()?))
}

fn route_question_ids(
    course: &str,
    assessment: &str,
    question_id: &str,
    revision_number: u32,
) -> Option<(CourseInstanceId, AssessmentId, QuestionRevisionTuple)> {
    let (course, assessment) = route_ids(course, assessment)?;
    let question_id = question_id.parse::<QuestionId>().ok()?;
    // ASVS 2.2.1/2: parse the exact checksum-bearing ID before any Question lookup.
    Some((
        course,
        assessment,
        QuestionRevisionTuple {
            question_id,
            revision_number: QuestionRevisionNumber::new(revision_number).ok()?,
        },
    ))
}

fn verified_question_revision_tuple(
    question_revision_tuple: &QuestionRevisionTuple,
) -> Result<(), ()> {
    question_revision_tuple
        .question_id
        .as_str()
        .parse::<QuestionId>()
        .map(|_| ())
        .map_err(|_| ())
}

fn edit_header(headers: &HeaderMap) -> Result<AssessmentEditNumber, Box<Response>> {
    // ASVS 2.2.1 and 2.2.2: accept only the strong, canonical saved-edit
    // precondition at the trusted request boundary.
    let value = headers.get(header::IF_MATCH).ok_or_else(|| {
        Box::new(error(
            StatusCode::PRECONDITION_REQUIRED,
            "Assessment Edit Number is required",
        ))
    })?;
    let value = value.to_str().map_err(|_| {
        Box::new(error(
            StatusCode::BAD_REQUEST,
            "Assessment Edit Number is invalid",
        ))
    })?;
    let value = value
        .strip_prefix('"')
        .and_then(|candidate| candidate.strip_suffix('"'))
        .ok_or_else(|| {
            Box::new(error(
                StatusCode::BAD_REQUEST,
                "Assessment Edit Number is invalid",
            ))
        })?;
    value.parse().map_err(|_| {
        Box::new(error(
            StatusCode::BAD_REQUEST,
            "Assessment Edit Number is invalid",
        ))
    })
}

fn document_edit_query(raw_query: Option<&str>) -> Result<AssessmentEditNumber, Box<Response>> {
    let raw_query = raw_query.ok_or_else(|| {
        Box::new(error(
            StatusCode::PRECONDITION_REQUIRED,
            "Assessment Edit Number is required",
        ))
    })?;
    let pairs = url::form_urlencoded::parse(raw_query.as_bytes()).collect::<Vec<_>>();
    let [(name, value)] = pairs.as_slice() else {
        return Err(Box::new(error(
            StatusCode::BAD_REQUEST,
            "Assessment Edit Number is invalid",
        )));
    };
    if name != "assessmentEditNumber" {
        return Err(Box::new(error(
            StatusCode::BAD_REQUEST,
            "Assessment Edit Number is invalid",
        )));
    }
    AssessmentEditNumber::from_str(value).map_err(|_| {
        Box::new(error(
            StatusCode::BAD_REQUEST,
            "Assessment Edit Number is invalid",
        ))
    })
}

async fn instructor(
    state: &StateData,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    instructor_with_sessions(state.sessions.as_ref(), headers).await
}

async fn instructor_with_sessions(
    sessions: &dyn SessionStore,
    headers: &HeaderMap,
) -> Result<SessionTokenHash, Box<Response>> {
    let cookie = headers
        .get_all(header::COOKIE)
        .iter()
        .map(|value| value.to_str().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|values| !values.is_empty())
        .map(|values| values.join("; "));
    match resolve_session(sessions, cookie.as_deref()).await {
        Ok(value) if value.record.product_role == ProductRole::Instructor => Ok(value.session_hash),
        Ok(_) | Err(AuthError::Unauthenticated) => Err(Box::new(concealed())),
        Err(AuthError::Unavailable(_) | AuthError::Randomness(_)) => Err(Box::new(unavailable())),
    }
}

fn quoted_edit_number_response(response: Response, edit_number: AssessmentEditNumber) -> Response {
    let mut response = crate::auth::no_store(response);
    match HeaderValue::from_str(&format!("\"{}\"", edit_number.value())) {
        Ok(value) => {
            response.headers_mut().insert(header::ETAG, value);
            response
        }
        Err(_) => unavailable(),
    }
}

fn store_error(store_error: StoreError) -> Response {
    // ASVS 16.5.1 and 16.5.3: authorization and backend failures never expose
    // source, record identity, query detail, or partial render output.
    match store_error {
        StoreError::NotFound | StoreError::Forbidden | StoreError::OwnershipMismatch => concealed(),
        StoreError::Conflict | StoreError::RetryableTransaction => error(
            StatusCode::PRECONDITION_FAILED,
            "Instructor Student View changed",
        ),
        StoreError::InvalidRecord(_)
        | StoreError::AlreadyExists
        | StoreError::LifecycleConflict
        | StoreError::AssessmentActivity(_)
        | StoreError::TimedOut
        | StoreError::LeaseLost
        | StoreError::Unavailable(_) => unavailable(),
    }
}

fn concealed() -> Response {
    error(StatusCode::NOT_FOUND, "Instructor Student View unavailable")
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "Instructor Student View unavailable",
    )
}

fn error(status: StatusCode, message: &'static str) -> Response {
    crate::auth::no_store((status, message).into_response())
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use learning_data_access::{SessionId, SessionLifetime, SessionRecord};
    use question_model::{AccountId, Timestamp};
    use uuid::Uuid;

    use super::*;

    struct RoleSessionStore {
        expected_hash: SessionTokenHash,
        role: ProductRole,
    }

    #[async_trait]
    impl SessionStore for RoleSessionStore {
        async fn create_session(
            &self,
            _: SessionTokenHash,
            _: AccountId,
            _: SessionLifetime,
        ) -> Result<SessionRecord, StoreError> {
            panic!("Student View authentication must not create a session")
        }

        async fn resolve_session(
            &self,
            token_hash: SessionTokenHash,
        ) -> Result<Option<SessionRecord>, StoreError> {
            if token_hash != self.expected_hash {
                return Ok(None);
            }
            Ok(Some(SessionRecord {
                id: SessionId::from_uuid(Uuid::from_u128(1)),
                token_hash,
                account: AccountId::from_debug_serial(2),
                product_role: self.role,
                created_at: Timestamp::from_unix_millis(1),
                expires_at: Timestamp::from_unix_millis(2),
            }))
        }

        async fn revoke_session(&self, _: SessionTokenHash) -> Result<(), StoreError> {
            panic!("Student View authentication must not revoke a session")
        }
    }

    fn session_headers() -> (HeaderMap, SessionTokenHash) {
        let token = [7_u8; 32];
        let token_hash = SessionTokenHash::compute(&token);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("ple_session={}", URL_SAFE_NO_PAD.encode(token)))
                .expect("fixed cookie is valid"),
        );
        (headers, token_hash)
    }

    #[tokio::test]
    async fn student_view_authenticates_only_instructors_without_session_writes() {
        let (headers, expected_hash) = session_headers();
        let instructor_store = RoleSessionStore {
            expected_hash,
            role: ProductRole::Instructor,
        };
        assert_eq!(
            instructor_with_sessions(&instructor_store, &headers)
                .await
                .expect("Instructor session is admitted"),
            expected_hash
        );

        let student_store = RoleSessionStore {
            expected_hash,
            role: ProductRole::Student,
        };
        let response = match instructor_with_sessions(&student_store, &headers).await {
            Ok(_) => panic!("Student Product Role must not enter Instructor Student View"),
            Err(response) => response,
        };
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
    }
}
