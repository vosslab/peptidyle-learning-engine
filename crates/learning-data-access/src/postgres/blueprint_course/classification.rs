//! Course-classification encoding and PostgreSQL row decoding for Blueprints.

use sqlx::Row;

use super::super::connection::map_sqlx_error;
use super::{StoreError, invalid};

pub(in crate::postgres) fn classification_tags(
    value: &question_model::CourseClassification,
) -> Vec<String> {
    value
        .tags
        .iter()
        .map(|tag| tag.as_str().to_owned())
        .collect()
}

pub(in crate::postgres) fn decode_classification(
    row: &sqlx::postgres::PgRow,
) -> Result<question_model::CourseClassification, StoreError> {
    let tags: Vec<String> = row.try_get("tags").map_err(map_sqlx_error)?;
    let value = question_model::CourseClassification {
        discipline_uuid: row.try_get("discipline_uuid").map_err(map_sqlx_error)?,
        subject_uuid: row.try_get("subject_uuid").map_err(map_sqlx_error)?,
        topic_uuid: row.try_get("topic_uuid").map_err(map_sqlx_error)?,
        subtopic_uuid: row.try_get("subtopic_uuid").map_err(map_sqlx_error)?,
        tags: tags.into_iter().map(question_model::Tag::new).collect(),
    };
    value
        .validate()
        .map_err(|_| invalid("Course classification"))?;
    Ok(value)
}
