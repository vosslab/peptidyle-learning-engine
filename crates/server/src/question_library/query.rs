//! Strict HTTP transport and trusted normalization for Library search.
use axum::http::StatusCode;
use question_model::{
    Capability, QuestionBackend, QuestionSearchAuthorship, QuestionSearchCourseUse,
    QuestionSearchRequest, QuestionSearchSort,
};
use serde::Deserialize;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

/// URL form of the current Question Search request.
///
/// The model's transport form intentionally has no defaults because saved
/// searches must record every field. HTTP uses defaults for omitted filters,
/// then immediately constructs the same strict model value.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QuestionSearchQuery {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    author_names: Vec<String>,
    #[serde(default)]
    backends: Vec<QuestionBackend>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    subjects: Vec<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    discipline_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    subject_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    topic_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    subtopic_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    cross_discipline: bool,
    #[serde(default)]
    question_types: Vec<question_model::QuestionType>,
    #[serde(default)]
    capabilities: Vec<Capability>,
    #[serde(default)]
    question_licenses: Vec<question_model::QuestionLicense>,
    #[serde(default)]
    used_in_my_courses: QuestionSearchCourseUse,
    #[serde(default)]
    authorship: QuestionSearchAuthorship,
    #[serde(default)]
    sort: QuestionSearchSort,
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    page_size: Option<u16>,
}

impl TryFrom<QuestionSearchQuery> for QuestionSearchRequest {
    type Error = (StatusCode, &'static str);

    /// ASVS 2.2.1 and 2.2.2: applies the model's positive limits and
    /// normalization again at the trusted service boundary.
    fn try_from(query: QuestionSearchQuery) -> Result<Self, Self::Error> {
        if query
            .page_size
            .is_some_and(|size| size == 0 || size > MAX_PAGE_SIZE)
        {
            return Err((
                StatusCode::BAD_REQUEST,
                "Question Library page size is invalid",
            ));
        }
        QuestionSearchRequest {
            text: query.text,
            author_names: query.author_names,
            backends: query.backends,
            tags: query.tags,
            subjects: query.subjects,
            topics: query.topics,
            discipline_uuid: query.discipline_uuid,
            subject_uuid: query.subject_uuid,
            topic_uuid: query.topic_uuid,
            subtopic_uuid: query.subtopic_uuid,
            cross_discipline: query.cross_discipline,
            question_types: query.question_types,
            capabilities: query.capabilities,
            question_licenses: query.question_licenses,
            used_in_my_courses: query.used_in_my_courses,
            authorship: query.authorship,
            sort: query.sort,
            cursor: query.cursor,
            page_size: query.page_size.or(Some(DEFAULT_PAGE_SIZE)),
        }
        .normalized()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Question Library query is invalid"))
    }
}
