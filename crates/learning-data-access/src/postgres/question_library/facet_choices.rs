//! Typed, session-authorized facet enumeration without Question source reads.

use question_model::{
    LibraryFacetChoice, LibrarySearchKind, PublishedQuestionFilter, PublishedQuestionId,
};
use sqlx::Row;

use super::{PostgresQuestionLibraryStore, invalid, map_sqlx_error};
use crate::{
    QuestionLibraryFacetChoiceRequest, QuestionLibraryFacetChoiceResult, SessionTokenHash,
    StoreError,
};

impl PostgresQuestionLibraryStore {
    pub(super) async fn list_question_library_facet_choices(
        &self,
        session: SessionTokenHash,
        request: QuestionLibraryFacetChoiceRequest,
    ) -> Result<QuestionLibraryFacetChoiceResult, StoreError> {
        if request.search.page_size == 0 || request.search.page_size > 100 {
            return Err(invalid("Question Library choice page size"));
        }
        let search = &request.search;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: every selector, search string, and continuation key is bound.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.question_library_facet_choices(\
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,\
                $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)",
        )
        .bind(
            search
                .exact_question_id
                .as_ref()
                .map(PublishedQuestionId::as_str),
        )
        .bind(super::search::text_terms_json(search)?)
        .bind(&search.author_names)
        .bind(super::search::backends(&search.backends))
        .bind(&search.tags)
        .bind(&search.subjects)
        .bind(&search.topics)
        .bind(&search.discipline_uuids)
        .bind(&search.subject_uuids)
        .bind(&search.topic_uuids)
        .bind(&search.subtopic_uuids)
        .bind(search.cross_discipline)
        .bind(
            search
                .bloom_cognitive_processes
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>(),
        )
        .bind(
            search
                .bloom_knowledge_dimensions
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>(),
        )
        .bind(super::search::question_types(&search.question_types)?)
        .bind(super::search::question_licenses(&search.question_licenses)?)
        .bind(search.authored_by_current_account)
        .bind(
            search
                .owner_account_id
                .as_ref()
                .map(question_model::AccountId::as_str),
        )
        .bind(match search.kind {
            LibrarySearchKind::Both => "both",
            LibrarySearchKind::Questions => "questions",
            LibrarySearchKind::Pools => "pools",
        })
        .bind(match search.questions {
            PublishedQuestionFilter::InNoPool => "no_pool",
            PublishedQuestionFilter::All => "all",
        })
        .bind(request.field.as_str())
        .bind(request.choice_search.as_deref())
        .bind(request.after_key.as_deref())
        .bind(i32::from(search.page_size) + 1)
        .bind(&request.selected_keys)
        .bind(super::search::scope_name(search.scope))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let mut choices = Vec::new();
        let mut selected = Vec::new();
        for row in rows {
            let count: i64 = row.try_get("choice_count").map_err(map_sqlx_error)?;
            let count =
                u64::try_from(count).map_err(|_| invalid("Question Library choice count"))?;
            let choice = LibraryFacetChoice {
                key: row.try_get("key").map_err(map_sqlx_error)?,
                label: row.try_get("label").map_err(map_sqlx_error)?,
                count,
            };
            if row
                .try_get::<bool, _>("is_selected")
                .map_err(map_sqlx_error)?
            {
                selected.push(choice);
            } else {
                choices.push(choice);
            }
        }
        let next_key = if choices.len() > usize::from(search.page_size) {
            choices.truncate(usize::from(search.page_size));
            choices.last().map(|choice| choice.key.clone())
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(QuestionLibraryFacetChoiceResult {
            choices,
            selected,
            next_key,
        })
    }
}
