//! Typed PostgreSQL binding for bounded Question Library discovery.

use question_model::{
    LibrarySearchKind, PublishedQuestionFilter, PublishedQuestionId, QuestionBackend,
    QuestionLicense, QuestionType,
};
use sqlx::Row;

use super::{PostgresQuestionLibraryStore, invalid, map_sqlx_error};
use crate::question_library::{
    QuestionLibraryBackendRestriction, QuestionLibrarySearchCursorPosition,
    QuestionLibrarySearchFacets, QuestionLibrarySearchPage, QuestionLibrarySearchRequest,
    QuestionLibrarySearchScope, QuestionLibrarySearchSort, QuestionLibraryTextField,
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
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)",
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
        .bind(&request.discipline_uuids)
        .bind(&request.subject_uuids)
        .bind(&request.topic_uuids)
        .bind(&request.subtopic_uuids)
        .bind(request.cross_discipline)
        .bind(
            request
                .bloom_cognitive_processes
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>(),
        )
        .bind(
            request
                .bloom_knowledge_dimensions
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>(),
        )
        .bind(question_types(&request.question_types)?)
        .bind(question_licenses(&request.question_licenses)?)
        .bind(request.authored_by_current_account)
        .bind(match request.kind {
            LibrarySearchKind::Both => "both",
            LibrarySearchKind::Questions => "questions",
            LibrarySearchKind::Pools => "pools",
        })
        .bind(match request.questions {
            PublishedQuestionFilter::InNoPool => "no_pool",
            PublishedQuestionFilter::All => "all",
        })
        .bind(
            request
                .owner_account_id
                .as_ref()
                .map(question_model::AccountId::as_str),
        )
        .bind(sort_name(request.sort))
        .bind(after_text_key(request.after.as_ref()))
        .bind(after_published_at(request.after.as_ref()))
        .bind(after_id(request.after.as_ref()))
        .bind(i32::from(request.page_size) + 1)
        .bind(scope_name(request.scope))
        .bind(after_starred_at(request.after.as_ref()))
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
            .map(super::mixed::decode_search_entry)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let next_position = if items.len() > limit {
            items.truncate(limit);
            let item = items
                .last()
                .ok_or_else(|| invalid("Question Library page"))?;
            Some(cursor_position(item, request.sort)?)
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
        )
        | (
            QuestionLibrarySearchSort::StarredDateAscending
            | QuestionLibrarySearchSort::StarredDateDescending,
            Some(QuestionLibrarySearchCursorPosition::StarredDate { .. }),
        ) => true,
        (
            sort,
            Some(QuestionLibrarySearchCursorPosition::Text {
                sort: position_sort,
                key,
                ..
            }),
        ) => {
            sort == *position_sort
                && sort != QuestionLibrarySearchSort::PublishedNewest
                && sort != QuestionLibrarySearchSort::StarredDateAscending
                && sort != QuestionLibrarySearchSort::StarredDateDescending
                && !key.is_empty()
        }
        _ => false,
    };
    if !cursor_matches_sort
        || (request.scope == QuestionLibrarySearchScope::Public
            && matches!(
                request.sort,
                QuestionLibrarySearchSort::StarredDateAscending
                    | QuestionLibrarySearchSort::StarredDateDescending
            ))
        || (request.scope == QuestionLibrarySearchScope::CurrentStarred
            && request.kind != LibrarySearchKind::Questions)
    {
        return Err(invalid("Question Library cursor"));
    }
    Ok(())
}

pub(super) fn text_terms_json(
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

pub(super) fn backends(
    restriction: &QuestionLibraryBackendRestriction,
) -> Option<Vec<&'static str>> {
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

pub(super) fn question_types(values: &[QuestionType]) -> Result<Vec<String>, StoreError> {
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

pub(super) fn question_licenses(values: &[QuestionLicense]) -> Result<Vec<String>, StoreError> {
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
        QuestionLibrarySearchSort::TitleDescending => "title_descending",
        QuestionLibrarySearchSort::KindAscending => "kind_ascending",
        QuestionLibrarySearchSort::KindDescending => "kind_descending",
        QuestionLibrarySearchSort::DisciplineAscending => "discipline_ascending",
        QuestionLibrarySearchSort::DisciplineDescending => "discipline_descending",
        QuestionLibrarySearchSort::QuestionTypeAscending => "question_type_ascending",
        QuestionLibrarySearchSort::QuestionTypeDescending => "question_type_descending",
        QuestionLibrarySearchSort::PublishedNewest => "published_newest",
        QuestionLibrarySearchSort::StarredDateAscending => "starred_date_ascending",
        QuestionLibrarySearchSort::StarredDateDescending => "starred_date_descending",
    }
}

pub(super) fn scope_name(scope: QuestionLibrarySearchScope) -> &'static str {
    match scope {
        QuestionLibrarySearchScope::Public => "public",
        QuestionLibrarySearchScope::CurrentStarred => "current_starred",
    }
}

fn after_starred_at(after: Option<&QuestionLibrarySearchCursorPosition>) -> Option<i64> {
    match after {
        Some(QuestionLibrarySearchCursorPosition::StarredDate {
            starred_at_micros, ..
        }) => Some(*starred_at_micros),
        _ => None,
    }
}

fn after_text_key(after: Option<&QuestionLibrarySearchCursorPosition>) -> Option<&str> {
    match after {
        Some(QuestionLibrarySearchCursorPosition::Text { key, .. }) => Some(key),
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
        Some(QuestionLibrarySearchCursorPosition::Text { public_id, .. })
        | Some(QuestionLibrarySearchCursorPosition::PublishedNewest { public_id, .. })
        | Some(QuestionLibrarySearchCursorPosition::StarredDate { public_id, .. }) => {
            Some(public_id.as_str())
        }
        None => None,
    }
}

fn cursor_position(
    item: &crate::LibrarySearchEntry,
    sort: QuestionLibrarySearchSort,
) -> Result<QuestionLibrarySearchCursorPosition, StoreError> {
    let public_id = item
        .public_id()
        .parse()
        .map_err(|_| invalid("Library cursor ID"))?;
    Ok(match sort {
        QuestionLibrarySearchSort::PublishedNewest => {
            QuestionLibrarySearchCursorPosition::PublishedNewest {
                published_at_millis: item.sort_time().as_unix_millis(),
                public_id,
            }
        }
        QuestionLibrarySearchSort::StarredDateAscending
        | QuestionLibrarySearchSort::StarredDateDescending => {
            QuestionLibrarySearchCursorPosition::StarredDate {
                starred_at_micros: item
                    .starred_at_micros()
                    .ok_or_else(|| invalid("Starred date"))?,
                public_id,
            }
        }
        sort => QuestionLibrarySearchCursorPosition::Text {
            sort,
            key: sort_text_key(item, sort).to_owned(),
            public_id,
        },
    })
}

fn sort_text_key(item: &crate::LibrarySearchEntry, sort: QuestionLibrarySearchSort) -> &str {
    match sort {
        QuestionLibrarySearchSort::TitleAscending | QuestionLibrarySearchSort::TitleDescending => {
            item.title()
        }
        QuestionLibrarySearchSort::KindAscending | QuestionLibrarySearchSort::KindDescending => {
            match item {
                crate::LibrarySearchEntry::Question { .. } => "Published Question",
                crate::LibrarySearchEntry::Pool { .. } => "Question Pool",
            }
        }
        QuestionLibrarySearchSort::DisciplineAscending
        | QuestionLibrarySearchSort::DisciplineDescending => match item {
            crate::LibrarySearchEntry::Question { entry, .. } => &entry.discipline_name,
            crate::LibrarySearchEntry::Pool { summary, .. } => &summary.metadata.discipline_name,
        },
        QuestionLibrarySearchSort::QuestionTypeAscending
        | QuestionLibrarySearchSort::QuestionTypeDescending => {
            let question_type = match item {
                crate::LibrarySearchEntry::Question { entry, .. } => entry.question_type,
                crate::LibrarySearchEntry::Pool { summary, .. } => summary.question_type,
            };
            match question_type {
                QuestionType::MultipleChoice => "Multiple choice",
                QuestionType::MultipleAnswer => "Multiple answer",
                QuestionType::FillInBlank => "Fill in the blank",
                QuestionType::MultipleFillInBlank => "Multiple fill in the blank",
                QuestionType::Numeric => "Numeric",
                QuestionType::Matching => "Matching",
                QuestionType::Ordering => "Ordering",
                QuestionType::Hotspot => "Hotspot",
            }
        }
        QuestionLibrarySearchSort::PublishedNewest => {
            unreachable!("timestamp sort has no text key")
        }
        QuestionLibrarySearchSort::StarredDateAscending
        | QuestionLibrarySearchSort::StarredDateDescending => {
            unreachable!("timestamp sort has no text key")
        }
    }
}
