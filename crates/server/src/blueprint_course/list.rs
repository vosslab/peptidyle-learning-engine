//! Literal name narrowing and bounded, visibility-reattested Blueprint pages.

use super::{
    BlueprintCourseRouteState, instructor_session_hash, route_error, store_error_response,
    summary_view, unavailable,
};
use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    BlueprintCourseListRequest, BlueprintCourseStore, Cursor, PageRequest, PageSize,
};
use question_model::{BlueprintCourseId, BlueprintCourseSummaryView};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ListQuery {
    #[serde(default)]
    include_archived: bool,
    #[serde(default)]
    public_only: bool,
    #[serde(default)]
    promoted_only: bool,
    #[serde(default)]
    query: String,
    cursor: Option<String>,
    page_size: Option<u16>,
    discipline_uuid: Option<Uuid>,
    subject_uuid: Option<Uuid>,
    topic_uuid: Option<Uuid>,
    subtopic_uuid: Option<Uuid>,
    #[serde(default)]
    cross_discipline: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListCursor {
    version: u8,
    query: String,
    include_archived: bool,
    public_only: bool,
    promoted_only: bool,
    page_size: u16,
    after: (String, BlueprintCourseId),
    discipline_uuid: Option<Uuid>,
    subject_uuid: Option<Uuid>,
    topic_uuid: Option<Uuid>,
    subtopic_uuid: Option<Uuid>,
    cross_discipline: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    items: Vec<BlueprintCourseSummaryView>,
    next_cursor: Option<String>,
}

pub(super) async fn list_blueprints(
    State(state): State<BlueprintCourseRouteState>,
    headers: HeaderMap,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Response {
    let request = match query.ok().and_then(|Query(query)| list_request(query)) {
        Some(value) => value,
        None => return route_error(StatusCode::BAD_REQUEST, "Blueprint Course page is invalid"),
    };
    let session = match instructor_session_hash(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let binding = (
        request.query.clone(),
        request.include_archived,
        request.public_only,
        request.promoted_only,
        request.page.size.get(),
        request.discipline_uuid,
        request.subject_uuid,
        request.topic_uuid,
        request.subtopic_uuid,
        request.cross_discipline,
    );
    match state
        .blueprints
        .list_blueprint_courses(session, request)
        .await
    {
        Ok(page) => {
            let next_cursor = match page.next_cursor {
                Some(after) => {
                    let after = match serde_json::from_str(after.as_str()) {
                        Ok(value) => value,
                        Err(_) => return unavailable(),
                    };
                    let cursor = ListCursor {
                        version: 3,
                        query: binding.0,
                        include_archived: binding.1,
                        public_only: binding.2,
                        promoted_only: binding.3,
                        page_size: binding.4,
                        after,
                        discipline_uuid: binding.5,
                        subject_uuid: binding.6,
                        topic_uuid: binding.7,
                        subtopic_uuid: binding.8,
                        cross_discipline: binding.9,
                    };
                    match serde_json::to_vec(&cursor) {
                        Ok(bytes) => Some(URL_SAFE_NO_PAD.encode(bytes)),
                        Err(_) => return unavailable(),
                    }
                }
                None => None,
            };
            crate::auth::no_store(
                Json(ListResponse {
                    items: page.items.into_iter().map(summary_view).collect(),
                    next_cursor,
                })
                .into_response(),
            )
        }
        Err(error) => store_error_response(error),
    }
}

fn list_request(query: ListQuery) -> Option<BlueprintCourseListRequest> {
    // ASVS 2.2.1/2/3: trusted bounds and cursor binding to the full normalized request.
    if query.query.chars().count() > 256
        || query.query.chars().any(char::is_control)
        || (query.public_only && query.include_archived)
        || (query.subject_uuid.is_some() && query.discipline_uuid.is_none())
        || (query.topic_uuid.is_some() && query.subject_uuid.is_none())
        || (query.subtopic_uuid.is_some() && query.topic_uuid.is_none())
        || (query.cross_discipline
            && (query.discipline_uuid.is_none() || query.subject_uuid.is_none()))
    {
        return None;
    }
    let normalized = query.query.trim().to_lowercase();
    if normalized.chars().count() > 256 {
        return None;
    }
    let size = PageSize::new(query.page_size.unwrap_or(50)).ok()?;
    let after = match query.cursor {
        Some(token) => {
            if token.len() > 8192 {
                return None;
            }
            let cursor: ListCursor =
                serde_json::from_slice(&URL_SAFE_NO_PAD.decode(token).ok()?).ok()?;
            if cursor.version != 3
                || cursor.query != normalized
                || cursor.include_archived != query.include_archived
                || cursor.public_only != query.public_only
                || cursor.promoted_only != query.promoted_only
                || cursor.page_size != size.get()
                || cursor.discipline_uuid != query.discipline_uuid
                || cursor.subject_uuid != query.subject_uuid
                || cursor.topic_uuid != query.topic_uuid
                || cursor.subtopic_uuid != query.subtopic_uuid
                || cursor.cross_discipline != query.cross_discipline
                || cursor.after.0.chars().count() > 500
            {
                return None;
            }
            Some(Cursor::parse(serde_json::to_string(&cursor.after).ok()?).ok()?)
        }
        None => None,
    };
    Some(BlueprintCourseListRequest {
        page: PageRequest { after, size },
        query: normalized,
        include_archived: query.include_archived,
        public_only: query.public_only,
        promoted_only: query.promoted_only,
        discipline_uuid: query.discipline_uuid,
        subject_uuid: query.subject_uuid,
        topic_uuid: query.topic_uuid,
        subtopic_uuid: query.subtopic_uuid,
        cross_discipline: query.cross_discipline,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_requires_complete_typed_parent_chain() {
        let empty = serde_json::from_str::<ListQuery>(r#"{}"#).expect("defaults");
        assert!(!empty.cross_discipline);
        assert!(list_request(empty).is_some());
        for invalid in [
            r#"{"disciplineUuid":"bad"}"#,
            r#"{"crossDiscipline":"true"}"#,
            r#"{"crossDiscipline":true}"#,
            r#"{"subjectUuid":"00000000-0000-0000-0000-000000000001"}"#,
            r#"{"topicUuid":"00000000-0000-0000-0000-000000000001"}"#,
            r#"{"subtopicUuid":"00000000-0000-0000-0000-000000000001"}"#,
        ] {
            assert!(
                serde_json::from_str::<ListQuery>(invalid)
                    .ok()
                    .and_then(list_request)
                    .is_none()
            );
        }
    }

    #[test]
    fn classification_continuation_binds_every_filter_and_rejects_old_version() {
        let id = |value| Some(Uuid::from_u128(value));
        let cursor = ListCursor {
            version: 3,
            query: "enzyme".to_owned(),
            include_archived: false,
            public_only: true,
            promoted_only: true,
            page_size: 50,
            after: (
                "Blueprint".to_owned(),
                "BPABCDEFGJ".parse().expect("Blueprint Course ID"),
            ),
            discipline_uuid: id(1),
            subject_uuid: id(2),
            topic_uuid: id(3),
            subtopic_uuid: id(4),
            cross_discipline: true,
        };
        let mut value = serde_json::to_value(&cursor).expect("cursor value");
        value.as_object_mut().expect("object").remove("version");
        value.as_object_mut().expect("object").remove("after");
        value["query"] = serde_json::json!(" Enzyme ");
        value["cursor"] =
            serde_json::json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("cursor")));
        assert!(list_request(serde_json::from_value(value.clone()).expect("query")).is_some());
        for field in [
            "disciplineUuid",
            "subjectUuid",
            "topicUuid",
            "subtopicUuid",
            "crossDiscipline",
        ] {
            let mut changed = value.clone();
            changed[field] = if field == "crossDiscipline" {
                serde_json::json!(false)
            } else {
                serde_json::json!(Uuid::from_u128(9))
            };
            assert!(
                list_request(serde_json::from_value(changed).expect("changed query")).is_none()
            );
        }
        let mut old = cursor;
        old.version = 2;
        value["cursor"] = serde_json::json!(
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&old).expect("old cursor"))
        );
        assert!(list_request(serde_json::from_value(value).expect("query")).is_none());
    }

    #[test]
    fn continuation_cannot_change_the_applied_promotion_filter() {
        let after = (
            "Blueprint".to_owned(),
            "BPABCDEFGJ".parse().expect("Blueprint Course ID"),
        );
        let token = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&ListCursor {
                version: 3,
                query: String::new(),
                include_archived: false,
                public_only: true,
                promoted_only: true,
                page_size: 50,
                after,
                discipline_uuid: None,
                subject_uuid: None,
                topic_uuid: None,
                subtopic_uuid: None,
                cross_discipline: false,
            })
            .expect("cursor"),
        );
        let query = |promoted_only| ListQuery {
            include_archived: false,
            public_only: true,
            promoted_only,
            query: String::new(),
            cursor: Some(token.clone()),
            page_size: None,
            discipline_uuid: None,
            subject_uuid: None,
            topic_uuid: None,
            subtopic_uuid: None,
            cross_discipline: false,
        };
        assert!(list_request(query(true)).is_some());
        assert!(list_request(query(false)).is_none());
        assert!(
            !serde_json::from_str::<ListQuery>(r#"{}"#)
                .expect("default list")
                .promoted_only
        );
    }
}
