//! Bound classification discovery and stable Blueprint continuation.

use super::{PostgresBlueprintCourseStore, decode_summary, invalid, map_sqlx_error};
use crate::postgres::connection::begin_authenticated_transaction;
use crate::{SessionTokenHash, StoreError};
use sqlx::Row;

impl PostgresBlueprintCourseStore {
    pub(super) async fn list_facet_choices(
        &self,
        session: SessionTokenHash,
        request: crate::BlueprintCourseFacetChoiceRequest,
    ) -> Result<crate::BlueprintCourseFacetChoiceResult, StoreError> {
        if request.choice_search.chars().count() > 128
            || request.choice_search.chars().any(char::is_control)
            || request.search.query.chars().count() > 256
            || !acceptable_blueprint_tag(&request.search.tag)
            || request.after_position.as_ref().is_some_and(|(label, key)| {
                label.chars().count() > 500 || key.chars().count() > 120
            })
        {
            return Err(invalid("Blueprint facet page"));
        }
        let mut transaction = begin_authenticated_transaction(&self.pool, session).await?;
        let (after_label, after_key) = request
            .after_position
            .as_ref()
            .map_or((None, None), |(label, key)| {
                (Some(label.as_str()), Some(key.as_str()))
            });
        let search = &request.search;
        let rows = sqlx::query("SELECT * FROM ple_api.blueprint_course_facet_choices($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)")
            .bind(request.field.as_sql())
            .bind(search.include_archived)
            .bind(search.active_only)
            .bind(search.promoted_only)
            .bind(&search.query)
            .bind(search.discipline_uuid)
            .bind(search.subject_uuid)
            .bind(search.topic_uuid)
            .bind(search.subtopic_uuid)
            .bind(search.cross_discipline)
            .bind(&search.tag)
            .bind(&search.discipline_uuids)
            .bind(&search.subject_uuids)
            .bind(&search.topic_uuids)
            .bind(&search.subtopic_uuids)
            .bind(&search.tags)
            .bind(&request.choice_search)
            .bind(after_label)
            .bind(after_key)
            .bind(101_i32)
            .fetch_all(&mut *transaction)
            .await.map_err(map_sqlx_error)?;
        let mut choices = Vec::new();
        let mut selected = Vec::new();
        for row in rows {
            let count: i64 = row.try_get("choice_count").map_err(map_sqlx_error)?;
            let choice = crate::BlueprintCourseFacetChoice {
                key: row.try_get("choice_key").map_err(map_sqlx_error)?,
                label: row.try_get("choice_label").map_err(map_sqlx_error)?,
                count: u64::try_from(count).map_err(|_| invalid("Blueprint facet count"))?,
            };
            if row.try_get::<bool, _>("selected").map_err(map_sqlx_error)? {
                selected.push(choice);
            } else {
                choices.push(choice);
            }
        }
        choices.sort_by(|a, b| a.label.cmp(&b.label).then(a.key.cmp(&b.key)));
        selected.sort_by(|a, b| a.label.cmp(&b.label).then(a.key.cmp(&b.key)));
        let next_position = (choices.len() > 100).then(|| {
            let last = &choices[99];
            (last.label.clone(), last.key.clone())
        });
        choices.truncate(100);
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::BlueprintCourseFacetChoiceResult {
            choices,
            selected,
            next_position,
        })
    }

    pub(super) async fn list_discovery_page(
        &self,
        session: SessionTokenHash,
        request: crate::BlueprintCourseListRequest,
    ) -> Result<
        crate::NumberedPage<crate::StoredBlueprintCourseSummary, crate::DiscoveryPageSize>,
        StoreError,
    > {
        if request.query.chars().count() > 256
            || request.query.chars().any(char::is_control)
            || !acceptable_blueprint_tag(&request.tag)
            || request.discipline_uuids.len() > 20
            || request.subject_uuids.len() > 20
            || request.topic_uuids.len() > 20
            || request.subtopic_uuids.len() > 20
            || request.tags.len() > 20
            || request
                .tags
                .iter()
                .any(|tag| tag.is_empty() || !acceptable_blueprint_tag(tag))
            || (request.active_only && request.include_archived)
        {
            return Err(invalid("Blueprint discovery page"));
        }
        let mut transaction = begin_authenticated_transaction(&self.pool, session).await?;
        // The SQL function counts and slices the same complete authorized result.
        let rows = sqlx::query("SELECT * FROM ple_api.list_blueprint_courses($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)")
            .bind(request.include_archived)
            .bind(request.active_only)
            .bind(request.promoted_only)
            .bind(&request.query)
            .bind(request.sort.as_sql())
            .bind(i64::from(request.page.page_number.get()))
            .bind(i32::from(request.page.page_size.get()))
            .bind(request.discipline_uuid)
            .bind(request.subject_uuid)
            .bind(request.topic_uuid)
            .bind(request.subtopic_uuid)
            .bind(request.cross_discipline)
            .bind(&request.tag)
            .bind(&request.discipline_uuids)
            .bind(&request.subject_uuids)
            .bind(&request.topic_uuids)
            .bind(&request.subtopic_uuids)
            .bind(&request.tags)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let first = rows.first().ok_or_else(|| invalid("Blueprint page"))?;
        let total_count = u64::try_from(
            first
                .try_get::<i64, _>("total_count")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint result count"))?;
        let page_number = crate::PageNumber::new(
            u32::try_from(
                first
                    .try_get::<i64, _>("page_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| invalid("Blueprint page number"))?,
        )
        .map_err(|_| invalid("Blueprint page number"))?;
        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            if row
                .try_get::<Option<String>, _>("blueprint_course_id")
                .map_err(map_sqlx_error)?
                .is_some()
            {
                records.push(decode_summary(row)?);
            }
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::NumberedPage {
            items: records,
            page_number,
            page_size: request.page.page_size,
            total_count,
        })
    }
}

fn acceptable_blueprint_tag(tag: &str) -> bool {
    tag.chars().count() <= 120 && !tag.chars().any(char::is_control) && tag == tag.trim_matches(' ')
}
