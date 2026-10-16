//! Bound classification discovery and stable Blueprint continuation.

use super::{PostgresBlueprintCourseStore, decode_summary, invalid, map_sqlx_error};
use crate::{
    BlueprintCourseListCursorPosition, BlueprintCourseListSort, SessionTokenHash, StoreError,
    StoredBlueprintCourseSummary,
};
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
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
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
            .bind(search.public_only)
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
    ) -> Result<crate::Page<StoredBlueprintCourseSummary>, StoreError> {
        let after: Option<BlueprintCourseListCursorPosition> = request
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
            || (request.public_only && request.include_archived)
            || after
                .as_ref()
                .is_some_and(|key| !valid_cursor(key, request.sort))
        {
            return Err(invalid("Blueprint discovery page"));
        }
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4: bind literal names, visibility, public keys and bounded limit.
        let rows =
            sqlx::query("SELECT * FROM ple_api.list_blueprint_courses($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)")
                .bind(request.include_archived)
                .bind(request.public_only)
                .bind(request.promoted_only)
                .bind(&request.query)
                .bind(request.sort.as_sql())
                .bind(after_count(after.as_ref()))
                .bind(after_name(after.as_ref()))
                .bind(after_id(after.as_ref()))
                .bind(i32::from(request.page.size.get()) + 1)
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
        let limit = usize::from(request.page.size.get());
        let records = rows
            .iter()
            .take(limit)
            .map(decode_summary)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if rows.len() > limit {
            let last = records.last().ok_or_else(|| invalid("Blueprint page"))?;
            let key = serde_json::to_string(&cursor_position(last, request.sort))
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

fn acceptable_blueprint_tag(tag: &str) -> bool {
    tag.chars().count() <= 120 && !tag.chars().any(char::is_control) && tag == tag.trim_matches(' ')
}

fn valid_cursor(key: &BlueprintCourseListCursorPosition, sort: BlueprintCourseListSort) -> bool {
    match (sort, key) {
        (
            BlueprintCourseListSort::Name | BlueprintCourseListSort::NameDescending,
            BlueprintCourseListCursorPosition::Name { long_name, .. },
        ) => long_name.chars().count() <= 500,
        (
            BlueprintCourseListSort::Adoptions | BlueprintCourseListSort::AdoptionsAscending,
            BlueprintCourseListCursorPosition::Adoptions { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Students | BlueprintCourseListSort::StudentsAscending,
            BlueprintCourseListCursorPosition::Students { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Stars | BlueprintCourseListSort::StarsAscending,
            BlueprintCourseListCursorPosition::Stars { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Watches | BlueprintCourseListSort::WatchesAscending,
            BlueprintCourseListCursorPosition::Watches { long_name, .. },
        )
        | (
            BlueprintCourseListSort::RecentEdits | BlueprintCourseListSort::RecentEditsAscending,
            BlueprintCourseListCursorPosition::RecentEdits { long_name, .. },
        ) => long_name.chars().count() <= 500 && count_fits_postgres(key),
        _ => false,
    }
}

fn count_fits_postgres(key: &BlueprintCourseListCursorPosition) -> bool {
    match key {
        BlueprintCourseListCursorPosition::Adoptions { count, .. }
        | BlueprintCourseListCursorPosition::Students { count, .. }
        | BlueprintCourseListCursorPosition::Stars { count, .. }
        | BlueprintCourseListCursorPosition::Watches { count, .. } => *count <= i64::MAX as u64,
        BlueprintCourseListCursorPosition::RecentEdits { .. } => true,
        BlueprintCourseListCursorPosition::Name { .. } => true,
    }
}

fn after_count(after: Option<&BlueprintCourseListCursorPosition>) -> Option<i64> {
    match after {
        Some(BlueprintCourseListCursorPosition::Adoptions { count, .. })
        | Some(BlueprintCourseListCursorPosition::Students { count, .. })
        | Some(BlueprintCourseListCursorPosition::Stars { count, .. })
        | Some(BlueprintCourseListCursorPosition::Watches { count, .. }) => {
            i64::try_from(*count).ok()
        }
        Some(BlueprintCourseListCursorPosition::RecentEdits {
            edited_at_millis, ..
        }) => Some(*edited_at_millis),
        Some(BlueprintCourseListCursorPosition::Name { .. }) | None => None,
    }
}

fn after_name(after: Option<&BlueprintCourseListCursorPosition>) -> Option<&str> {
    match after {
        Some(BlueprintCourseListCursorPosition::Name { long_name, .. })
        | Some(BlueprintCourseListCursorPosition::Adoptions { long_name, .. })
        | Some(BlueprintCourseListCursorPosition::Students { long_name, .. })
        | Some(BlueprintCourseListCursorPosition::Stars { long_name, .. })
        | Some(BlueprintCourseListCursorPosition::Watches { long_name, .. })
        | Some(BlueprintCourseListCursorPosition::RecentEdits { long_name, .. }) => Some(long_name),
        None => None,
    }
}

fn after_id(after: Option<&BlueprintCourseListCursorPosition>) -> Option<String> {
    match after {
        Some(BlueprintCourseListCursorPosition::Name {
            blueprint_course_id,
            ..
        })
        | Some(BlueprintCourseListCursorPosition::Adoptions {
            blueprint_course_id,
            ..
        })
        | Some(BlueprintCourseListCursorPosition::Students {
            blueprint_course_id,
            ..
        })
        | Some(BlueprintCourseListCursorPosition::Stars {
            blueprint_course_id,
            ..
        })
        | Some(BlueprintCourseListCursorPosition::Watches {
            blueprint_course_id,
            ..
        })
        | Some(BlueprintCourseListCursorPosition::RecentEdits {
            blueprint_course_id,
            ..
        }) => Some(blueprint_course_id.as_string()),
        None => None,
    }
}

fn cursor_position(
    record: &StoredBlueprintCourseSummary,
    sort: BlueprintCourseListSort,
) -> BlueprintCourseListCursorPosition {
    match sort {
        BlueprintCourseListSort::Name | BlueprintCourseListSort::NameDescending => {
            BlueprintCourseListCursorPosition::Name {
                long_name: record.long_name.clone(),
                blueprint_course_id: record.id.clone(),
            }
        }
        BlueprintCourseListSort::Adoptions | BlueprintCourseListSort::AdoptionsAscending => {
            BlueprintCourseListCursorPosition::Adoptions {
                count: record.total_adoptions,
                long_name: record.long_name.clone(),
                blueprint_course_id: record.id.clone(),
            }
        }
        BlueprintCourseListSort::Students | BlueprintCourseListSort::StudentsAscending => {
            BlueprintCourseListCursorPosition::Students {
                count: record.total_students_ever_enrolled,
                long_name: record.long_name.clone(),
                blueprint_course_id: record.id.clone(),
            }
        }
        BlueprintCourseListSort::Stars | BlueprintCourseListSort::StarsAscending => {
            BlueprintCourseListCursorPosition::Stars {
                count: record.star_count,
                long_name: record.long_name.clone(),
                blueprint_course_id: record.id.clone(),
            }
        }
        BlueprintCourseListSort::Watches | BlueprintCourseListSort::WatchesAscending => {
            BlueprintCourseListCursorPosition::Watches {
                count: record.watcher_count,
                long_name: record.long_name.clone(),
                blueprint_course_id: record.id.clone(),
            }
        }
        BlueprintCourseListSort::RecentEdits | BlueprintCourseListSort::RecentEditsAscending => {
            BlueprintCourseListCursorPosition::RecentEdits {
                edited_at_millis: record.last_edited_at_millis,
                long_name: record.long_name.clone(),
                blueprint_course_id: record.id.clone(),
            }
        }
    }
}
