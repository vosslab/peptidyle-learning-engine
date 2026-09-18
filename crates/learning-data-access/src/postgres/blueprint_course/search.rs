//! Bound classification discovery and stable Blueprint continuation.

use question_model::BlueprintCourseId;

use super::{PostgresBlueprintCourseStore, decode_summary, invalid, map_sqlx_error};
use crate::{SessionTokenHash, StoreError, StoredBlueprintCourseSummary};

impl PostgresBlueprintCourseStore {
    pub(super) async fn list_discovery_page(
        &self,
        session: SessionTokenHash,
        request: crate::BlueprintCourseListRequest,
    ) -> Result<crate::Page<StoredBlueprintCourseSummary>, StoreError> {
        let after: Option<(String, BlueprintCourseId)> = request
            .page
            .after
            .as_ref()
            .map(|cursor| {
                if cursor.as_str().len() > 8192 {
                    return Err(invalid("Blueprint page cursor"));
                }
                serde_json::from_str(cursor.as_str()).map_err(|_| invalid("Blueprint page cursor"))
            })
            .transpose()?;
        if request.query.chars().count() > 256
            || request.query.chars().any(char::is_control)
            || (request.public_only && request.include_archived)
            || after
                .as_ref()
                .is_some_and(|key| key.0.chars().count() > 500)
        {
            return Err(invalid("Blueprint discovery page"));
        }
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: bind literal names, visibility, public keys and bounded limit.
        let rows =
            sqlx::query("SELECT * FROM ple_api.list_blueprint_courses($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)")
                .bind(request.include_archived)
                .bind(request.public_only)
                .bind(request.promoted_only)
                .bind(&request.query)
                .bind(after.as_ref().map(|key| key.0.as_str()))
                .bind(after.as_ref().map(|key| key.1.as_string()))
                .bind(i32::from(request.page.size.get()) + 1)
                .bind(request.discipline_uuid)
                .bind(request.subject_uuid)
                .bind(request.topic_uuid)
                .bind(request.subtopic_uuid)
                .bind(request.cross_discipline)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let limit = usize::from(request.page.size.get());
        let records = rows
            .iter()
            .take(limit)
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if rows.len() > limit {
            let last = records.last().ok_or_else(|| invalid("Blueprint page"))?;
            let key = serde_json::to_string(&(&last.long_name, &last.reference))
                .map_err(|_| invalid("Blueprint page cursor"))?;
            Some(crate::Cursor::parse(key).map_err(|_| invalid("Blueprint page cursor"))?)
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::Page {
            items: records,
            next_cursor,
        })
    }
}
