//! Typed PostgreSQL binding for bounded Question Library discovery.

use crate::postgres::connection::begin_authenticated_transaction;
use question_model::{
    LibrarySearchKind, PublishedQuestionFilter, PublishedQuestionId, QuestionBackend,
    QuestionLicense, QuestionType,
};
use sqlx::Row;

use super::{PostgresQuestionLibraryStore, invalid, map_sqlx_error};
use crate::question_library::{
    QuestionLibraryBackendRestriction, QuestionLibrarySearchFacets, QuestionLibrarySearchPage,
    QuestionLibrarySearchRequest, QuestionLibrarySearchScope, QuestionLibrarySearchSort,
    QuestionLibraryTextField,
};
use crate::{NumberedPage, PageNumber, SessionTokenHash, StoreError};

impl PostgresQuestionLibraryStore {
    pub(super) async fn search_published_question_library_entries(
        &self,
        session: SessionTokenHash,
        request: QuestionLibrarySearchRequest,
    ) -> Result<QuestionLibrarySearchPage, StoreError> {
        validate_request(&request)?;
        let mut transaction = begin_authenticated_transaction(&self.pool, session).await?;
        // ASVS 1.2.4: every predicate and numbered-page component stays a typed bind.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.search_question_library_entries(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, \
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25)",
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
        .bind(i64::from(request.page.page_number.get()))
        .bind(i32::from(request.page.page_size.get()))
        .bind(scope_name(request.scope))
        .bind(
            request
                .excluded_question_ids
                .iter()
                .map(PublishedQuestionId::as_str)
                .collect::<Vec<_>>(),
        )
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
        let page_row = rows
            .first()
            .ok_or_else(|| invalid("Question Library page"))?;
        let total_count = page_row
            .try_get::<i64, _>("total_count")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                u64::try_from(value).map_err(|_| invalid("Question Library count"))
            })?;
        let page_number = page_row
            .try_get::<i64, _>("resolved_page_number")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                u32::try_from(value)
                    .ok()
                    .and_then(|value| PageNumber::new(value).ok())
                    .ok_or_else(|| invalid("Question Library page"))
            })?;
        let page_size = page_row
            .try_get::<i32, _>("resolved_page_size")
            .map_err(map_sqlx_error)
            .and_then(|value| {
                u16::try_from(value)
                    .ok()
                    .and_then(|value| crate::DiscoveryPageSize::new(value).ok())
                    .ok_or_else(|| invalid("Question Library page size"))
            })?;
        let mut items = rows
            .iter()
            .map(super::mixed::decode_search_entry)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        super::image_renditions::load_image_bindings(
            &mut transaction,
            items.iter_mut().filter_map(|item| match item {
                crate::LibrarySearchEntry::Question { entry, .. } => Some(entry.as_mut()),
                crate::LibrarySearchEntry::Pool { .. } => None,
            }),
        )
        .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QuestionLibrarySearchPage {
            page: NumberedPage {
                items,
                page_number,
                page_size,
                total_count,
            },
            facets,
        })
    }
}

fn validate_request(request: &QuestionLibrarySearchRequest) -> Result<(), StoreError> {
    if (request.scope == QuestionLibrarySearchScope::Public
        && matches!(
            request.sort,
            QuestionLibrarySearchSort::StarredDateAscending
                | QuestionLibrarySearchSort::StarredDateDescending
        ))
        || (request.scope == QuestionLibrarySearchScope::CurrentStarred
            && request.kind != LibrarySearchKind::Questions)
    {
        return Err(invalid("Question Library request"));
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
