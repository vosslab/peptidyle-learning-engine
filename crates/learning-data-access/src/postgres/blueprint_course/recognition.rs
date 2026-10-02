//! Current Question and Pool titles for Instructor recognition.

use std::collections::{BTreeMap, BTreeSet};

use question_model::{PublishedQuestionId, QuestionPoolId};
use sqlx::Row;

use super::{PostgresBlueprintCourseStore, invalid, map_sqlx_error};
use crate::{RecognitionTitles, SessionTokenHash, StoreError};

const MAX_RECOGNITION_TITLE_IDS: usize = 1000;

impl PostgresBlueprintCourseStore {
    pub(super) async fn load_recognition_titles_rows(
        &self,
        session: SessionTokenHash,
        question_ids: &[PublishedQuestionId],
        pool_ids: &[QuestionPoolId],
    ) -> Result<RecognitionTitles, StoreError> {
        let questions = bounded_ids(question_ids)?;
        let pools = bounded_ids(pool_ids)?;
        if questions.is_empty() && pools.is_empty() {
            return Err(invalid("recognition title selection"));
        }
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: bind the complete ID arrays. Missing lineages stay omitted.
        let rows = sqlx::query(
            "SELECT record_kind, public_id, title \
             FROM ple_api.load_recognition_titles($1, $2)",
        )
        .bind(&questions)
        .bind(&pools)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let titles = recognition_titles(&rows, &questions, &pools)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(titles)
    }
}

fn bounded_ids<T>(ids: &[T]) -> Result<Vec<String>, StoreError>
where
    T: Clone + Ord + AsRefPublicId,
{
    if ids.len() > MAX_RECOGNITION_TITLE_IDS {
        return Err(invalid("recognition title selection"));
    }
    let mut seen = BTreeSet::new();
    let mut values = Vec::with_capacity(ids.len());
    for id in ids {
        if !seen.insert(id.clone()) {
            return Err(invalid("recognition title selection"));
        }
        values.push(id.public_id().to_owned());
    }
    Ok(values)
}

trait AsRefPublicId {
    fn public_id(&self) -> &str;
}

impl AsRefPublicId for PublishedQuestionId {
    fn public_id(&self) -> &str {
        self.as_str()
    }
}

impl AsRefPublicId for QuestionPoolId {
    fn public_id(&self) -> &str {
        self.as_str()
    }
}

fn recognition_titles(
    rows: &[sqlx::postgres::PgRow],
    questions: &[String],
    pools: &[String],
) -> Result<RecognitionTitles, StoreError> {
    let requested_questions = questions.iter().cloned().collect::<BTreeSet<_>>();
    let requested_pools = pools.iter().cloned().collect::<BTreeSet<_>>();
    let mut question_titles = BTreeMap::new();
    let mut pool_titles = BTreeMap::new();
    for row in rows {
        let kind = row
            .try_get::<String, _>("record_kind")
            .map_err(map_sqlx_error)?;
        let public_id = row
            .try_get::<String, _>("public_id")
            .map_err(map_sqlx_error)?;
        let title = accepted_title(row.try_get::<String, _>("title").map_err(map_sqlx_error)?)?;
        match kind.as_str() {
            "question" if requested_questions.contains(&public_id) => {
                let id = public_id
                    .parse::<PublishedQuestionId>()
                    .map_err(|_| invalid("recognition title"))?;
                if question_titles.insert(id, title).is_some() {
                    return Err(invalid("recognition title"));
                }
            }
            "pool" if requested_pools.contains(&public_id) => {
                let id = public_id
                    .parse::<QuestionPoolId>()
                    .map_err(|_| invalid("recognition title"))?;
                if pool_titles.insert(id, title).is_some() {
                    return Err(invalid("recognition title"));
                }
            }
            _ => return Err(invalid("recognition title")),
        }
    }
    Ok(RecognitionTitles {
        questions: question_titles,
        pools: pool_titles,
    })
}

fn accepted_title(value: String) -> Result<String, StoreError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 512
        || value.chars().any(char::is_control)
    {
        return Err(invalid("recognition title"));
    }
    Ok(value)
}
