//! Typed PostgreSQL binding for bounded Question Library discovery.

use question_model::{PublishedQuestionId, QuestionBackend, QuestionLicense, QuestionType};
use sqlx::Row;

use super::{PostgresQuestionLibraryStore, decode_entry, invalid, map_sqlx_error};
use crate::question_library::{
    QuestionLibraryBackendRestriction, QuestionLibrarySearchCursorPosition,
    QuestionLibrarySearchFacets, QuestionLibrarySearchPage, QuestionLibrarySearchRequest,
    QuestionLibrarySearchSort, QuestionLibraryTextField,
};
use crate::{SessionTokenHash, StoreError};

impl PostgresQuestionLibraryStore {
    pub(super) async fn search_published_question_library_entries(
        &self,
        session: SessionTokenHash,
        request: QuestionLibrarySearchRequest,
    ) -> Result<QuestionLibrarySearchPage, StoreError> {
        validate_request(&request)?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: every predicate and cursor component stays a typed bind.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.search_question_library_entries(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, \
                $15, $16, $17, $18, $19, $20, $21, $22, $23)",
        )
        .bind(
            request
                .exact_question_id
                .as_ref()
                .map(PublishedQuestionId::as_str),
        )
        .bind(text_terms_json(&request)?)
        .bind(&request.author_names)
        .bind(backends(&request.backends))
        .bind(&request.tags)
        .bind(&request.subjects)
        .bind(&request.topics)
        .bind(request.discipline_uuid)
        .bind(request.subject_uuid)
        .bind(request.topic_uuid)
        .bind(request.subtopic_uuid)
        .bind(request.cross_discipline)
        .bind(request.bloom_cognitive_process.map(|value| value.as_str()))
        .bind(
            request
                .bloom_knowledge_dimension
                .map(|value| value.as_str()),
        )
        .bind(question_types(&request.question_types)?)
        .bind(question_licenses(&request.question_licenses)?)
        .bind(request.used_in_current_account_courses)
        .bind(request.authored_by_current_account)
        .bind(sort_name(request.sort))
        .bind(after_title(request.after.as_ref()))
        .bind(after_published_at(request.after.as_ref()))
        .bind(after_id(request.after.as_ref()))
        .bind(i32::from(request.page_size) + 1)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let facets = rows
            .first()
            .ok_or_else(|| invalid("Question Library facets"))?
            .try_get::<serde_json::Value, _>("facets")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                serde_json::from_value::<QuestionLibrarySearchFacets>(value)
                    .map_err(|_| invalid("Question Library facets"))
            })?;
        let limit = usize::from(request.page_size);
        let mut items = rows
            .iter()
            .filter(|row| {
                row.try_get::<Option<String>, _>("published_question_id")
                    .map(|value| value.is_some())
                    .unwrap_or(false)
            })
            .map(decode_entry)
            .collect::<Result<Vec<_>, _>>()?;
        let next_position = if items.len() > limit {
            items.truncate(limit);
            let item = items
                .last()
                .ok_or_else(|| invalid("Question Library page"))?;
            Some(cursor_position(item, request.sort))
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QuestionLibrarySearchPage {
            items,
            next_position,
            facets,
        })
    }
}

fn validate_request(request: &QuestionLibrarySearchRequest) -> Result<(), StoreError> {
    if request.page_size == 0 || request.page_size > question_model::MAX_DISCOVERY_PAGE_SIZE as u16
    {
        return Err(invalid("Question Library page size"));
    }
    let cursor_matches_sort = match (request.sort, request.after.as_ref()) {
        (_, None)
        | (
            QuestionLibrarySearchSort::PublishedNewest,
            Some(QuestionLibrarySearchCursorPosition::PublishedNewest { .. }),
        ) => true,
        (
            QuestionLibrarySearchSort::TitleAscending,
            Some(QuestionLibrarySearchCursorPosition::TitleAscending { title, .. }),
        ) => !title.is_empty(),
        _ => false,
    };
    if !cursor_matches_sort {
        return Err(invalid("Question Library cursor"));
    }
    Ok(())
}

fn text_terms_json(
    request: &QuestionLibrarySearchRequest,
) -> Result<serde_json::Value, StoreError> {
    let terms = request
        .text_terms
        .iter()
        .map(|term| {
            serde_json::json!({
                "field": match term.field {
                    QuestionLibraryTextField::Any => "any",
                    QuestionLibraryTextField::Discipline => "discipline",
                    QuestionLibraryTextField::Subtopic => "subtopic",
                    QuestionLibraryTextField::Subject => "subject",
                    QuestionLibraryTextField::Topic => "topic",
                    QuestionLibraryTextField::Tags => "tags",
                    QuestionLibraryTextField::QuestionType => "question_type",
                    QuestionLibraryTextField::Author => "author",
                },
                "value": term.value,
                "excluded": term.excluded,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::Value::Array(terms))
}

fn backends(restriction: &QuestionLibraryBackendRestriction) -> Option<Vec<&'static str>> {
    match restriction {
        QuestionLibraryBackendRestriction::Any => None,
        QuestionLibraryBackendRestriction::Only(values) => Some(
            values
                .iter()
                .copied()
                .map(QuestionBackend::as_str)
                .collect(),
        ),
    }
}

fn question_types(values: &[QuestionType]) -> Result<Vec<String>, StoreError> {
    values
        .iter()
        .map(|value| serde_json::to_value(value).map_err(|_| invalid("Question type")))
        .map(|value| {
            value.and_then(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("Question type"))
            })
        })
        .collect()
}

fn question_licenses(values: &[QuestionLicense]) -> Result<Vec<String>, StoreError> {
    values
        .iter()
        .map(|value| serde_json::to_value(value).map_err(|_| invalid("Question license")))
        .map(|value| {
            value.and_then(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("Question license"))
            })
        })
        .collect()
}

fn sort_name(sort: QuestionLibrarySearchSort) -> &'static str {
    match sort {
        QuestionLibrarySearchSort::TitleAscending => "title_ascending",
        QuestionLibrarySearchSort::PublishedNewest => "published_newest",
    }
}

fn after_title(after: Option<&QuestionLibrarySearchCursorPosition>) -> Option<&str> {
    match after {
        Some(QuestionLibrarySearchCursorPosition::TitleAscending { title, .. }) => Some(title),
        _ => None,
    }
}

fn after_published_at(after: Option<&QuestionLibrarySearchCursorPosition>) -> Option<i64> {
    match after {
        Some(QuestionLibrarySearchCursorPosition::PublishedNewest {
            published_at_millis,
            ..
        }) => Some(*published_at_millis),
        _ => None,
    }
}

fn after_id(after: Option<&QuestionLibrarySearchCursorPosition>) -> Option<&str> {
    match after {
        Some(QuestionLibrarySearchCursorPosition::TitleAscending { question_id, .. })
        | Some(QuestionLibrarySearchCursorPosition::PublishedNewest { question_id, .. }) => {
            Some(question_id.as_str())
        }
        None => None,
    }
}

fn cursor_position(
    item: &crate::PublishedQuestionLibraryEntry,
    sort: QuestionLibrarySearchSort,
) -> QuestionLibrarySearchCursorPosition {
    let question_id = item
        .published_question_revision_tuple
        .published_question_id
        .clone();
    match sort {
        QuestionLibrarySearchSort::TitleAscending => {
            QuestionLibrarySearchCursorPosition::TitleAscending {
                title: item.question_title.clone(),
                question_id,
            }
        }
        QuestionLibrarySearchSort::PublishedNewest => {
            QuestionLibrarySearchCursorPosition::PublishedNewest {
                published_at_millis: item.published_at.as_unix_millis(),
                question_id,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::PublishedQuestionId;

    fn request() -> QuestionLibrarySearchRequest {
        QuestionLibrarySearchRequest {
            exact_question_id: None,
            text_terms: Vec::new(),
            author_names: Vec::new(),
            backends: QuestionLibraryBackendRestriction::Any,
            tags: Vec::new(),
            subjects: Vec::new(),
            topics: Vec::new(),
            discipline_uuid: None,
            subject_uuid: None,
            topic_uuid: None,
            subtopic_uuid: None,
            cross_discipline: false,
            bloom_cognitive_process: None,
            bloom_knowledge_dimension: None,
            question_types: Vec::new(),
            question_licenses: Vec::new(),
            used_in_current_account_courses: false,
            authored_by_current_account: false,
            sort: QuestionLibrarySearchSort::TitleAscending,
            page_size: 50,
            after: None,
        }
    }

    #[test]
    fn typed_search_rejects_a_cursor_for_the_other_order() {
        let mut query = request();
        query.after = Some(QuestionLibrarySearchCursorPosition::PublishedNewest {
            published_at_millis: 1,
            question_id: PublishedQuestionId::from_random_identifier("P100000")
                .expect("Question ID"),
        });
        assert!(validate_request(&query).is_err());
    }
}
