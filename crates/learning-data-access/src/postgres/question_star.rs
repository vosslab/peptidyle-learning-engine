//! PostgreSQL implementation of the authenticated Question Star boundary.

use async_trait::async_trait;
use question_model::{PublishedQuestionId, validate_question_title};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::{
    Cursor, DiscoveryPageRequest, Page, QuestionStarProjection, QuestionStarStore,
    QuestionStarredInstructor, SessionTokenHash, StarredQuestionSummary, StoreError,
};

/// PostgreSQL Store for self-only Instructor Star state and public counts.
#[derive(Clone)]
pub struct PostgresQuestionStarStore {
    pool: Pool,
}

impl PostgresQuestionStarStore {
    /// Binds the attested API pool to the Star procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }

    fn projection(row: &sqlx::postgres::PgRow) -> Result<QuestionStarProjection, StoreError> {
        let count: i64 = row.try_get("star_count").map_err(map_sqlx_error)?;
        Ok(QuestionStarProjection {
            viewer_has_starred: row.try_get("viewer_has_starred").map_err(map_sqlx_error)?,
            star_count: u64::try_from(count)
                .map_err(|_| StoreError::InvalidRecord("Question Star count".to_owned()))?,
            starred_instructors: row
                .try_get::<Vec<String>, _>("starred_instructor_display_names")
                .map_err(map_sqlx_error)?
                .into_iter()
                .map(validated_display_name)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    async fn read_in(
        tx: &mut Transaction<'_, Postgres>,
        question_id: &PublishedQuestionId,
    ) -> Result<QuestionStarProjection, StoreError> {
        // ASVS 8.2.1--8.3.1: this API procedure derives the subject from the
        // installed session; it accepts no Account ID or client role claim.
        let row = sqlx::query("SELECT * FROM ple_api.read_current_question_star($1)")
            .bind(question_id.as_str())
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
        Self::projection(&row)
    }
}

fn validated_question_title(value: String) -> Result<String, StoreError> {
    if value != value.trim()
        || value.chars().any(char::is_control)
        || validate_question_title(&value).is_err()
    {
        return Err(StoreError::InvalidRecord(
            "Starred Question title".to_owned(),
        ));
    }
    Ok(value)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StarCursor {
    page_size: u16,
    starred_at_micros: i64,
    published_question_id: String,
}

fn starred_question_page(
    rows: &[sqlx::postgres::PgRow],
    page_size: usize,
    page_size_value: u16,
) -> Result<Page<StarredQuestionSummary>, StoreError> {
    let has_next = rows.len() > page_size;
    let visible_rows = rows.iter().take(page_size).collect::<Vec<_>>();
    let mut items = Vec::with_capacity(visible_rows.len());
    for row in &visible_rows {
        let question_id = row
            .try_get::<String, _>("published_question_id")
            .map_err(map_sqlx_error)?
            .parse::<PublishedQuestionId>()
            .map_err(|_| StoreError::InvalidRecord("Starred Question identity".to_owned()))?;
        items.push(StarredQuestionSummary {
            question_id,
            question_title: validated_question_title(
                row.try_get("question_title").map_err(map_sqlx_error)?,
            )?,
        });
    }
    let next_cursor = if has_next {
        let row = visible_rows
            .last()
            .ok_or_else(|| StoreError::InvalidRecord("Starred Question page cursor".to_owned()))?;
        let position = StarCursor {
            page_size: page_size_value,
            starred_at_micros: row.try_get("starred_at_micros").map_err(map_sqlx_error)?,
            published_question_id: row
                .try_get("published_question_id")
                .map_err(map_sqlx_error)?,
        };
        Some(
            Cursor::parse(serde_json::to_string(&position).map_err(|_| {
                StoreError::InvalidRecord("Starred Question page cursor".to_owned())
            })?)
            .map_err(|_| StoreError::InvalidRecord("Starred Question page cursor".to_owned()))?,
        )
    } else {
        None
    };
    Ok(Page { items, next_cursor })
}

fn validated_display_name(value: String) -> Result<QuestionStarredInstructor, StoreError> {
    if value != value.trim()
        || value.is_empty()
        // C852's immutable vetting boundary owns the 1..=200 contract.
        // Keep this defensive decoder exactly aligned so a valid vetted name
        // cannot turn an otherwise authorized Star read into a server error.
        || value.chars().count() > 200
        || value.chars().any(char::is_control)
    {
        return Err(StoreError::InvalidRecord(
            "Question Star Instructor display identity".to_owned(),
        ));
    }
    Ok(QuestionStarredInstructor {
        display_name: value,
    })
}

#[async_trait]
impl QuestionStarStore for PostgresQuestionStarStore {
    async fn question_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
    ) -> Result<QuestionStarProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let projection = Self::read_in(&mut tx, question_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn set_current_question_star(
        &self,
        session_token_hash: SessionTokenHash,
        question_id: &PublishedQuestionId,
        starred: bool,
    ) -> Result<QuestionStarProjection, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        // The SQL procedure is the idempotent, database-authorized mutation;
        // the subsequent projection is from the same transaction snapshot.
        sqlx::query("SELECT ple_api.set_current_question_star($1, $2)")
            .bind(question_id.as_str())
            .bind(starred)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let projection = Self::read_in(&mut tx, question_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(projection)
    }

    async fn list_current_starred_questions(
        &self,
        session_token_hash: SessionTokenHash,
        page: DiscoveryPageRequest,
    ) -> Result<Page<StarredQuestionSummary>, StoreError> {
        let mut tx = self.begin(session_token_hash).await?;
        let position = page
            .after
            .as_ref()
            .map(|cursor| {
                if cursor.as_str().len() > 1024 {
                    return Err(StoreError::InvalidRecord(
                        "Starred Question cursor".to_owned(),
                    ));
                }
                serde_json::from_str::<StarCursor>(cursor.as_str())
                    .map_err(|_| StoreError::InvalidRecord("Starred Question cursor".to_owned()))
            })
            .transpose()?;
        if position
            .as_ref()
            .is_some_and(|cursor| cursor.page_size != page.size.get())
        {
            return Err(StoreError::InvalidRecord(
                "Starred Question cursor".to_owned(),
            ));
        }
        let rows = sqlx::query(
            "SELECT published_question_id, question_title, starred_at_micros \
             FROM ple_api.list_current_starred_questions($1, $2, $3)",
        )
        .bind(position.as_ref().map(|cursor| cursor.starred_at_micros))
        .bind(
            position
                .as_ref()
                .map(|cursor| cursor.published_question_id.as_str()),
        )
        .bind(i32::from(page.size.get()))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let result_page =
            starred_question_page(&rows, usize::from(page.size.get()), page.size.get())?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result_page)
    }
}
