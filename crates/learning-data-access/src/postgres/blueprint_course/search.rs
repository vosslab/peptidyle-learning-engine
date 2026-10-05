//! Bound classification discovery and stable Blueprint continuation.

use super::{PostgresBlueprintCourseStore, decode_summary, invalid, map_sqlx_error};
use crate::{
    BlueprintCourseListCursorPosition, BlueprintCourseListSort, SessionTokenHash, StoreError,
    StoredBlueprintCourseSummary,
};

impl PostgresBlueprintCourseStore {
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
            sqlx::query("SELECT * FROM ple_api.list_blueprint_courses($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)")
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
            BlueprintCourseListSort::Name,
            BlueprintCourseListCursorPosition::Name { long_name, .. },
        ) => long_name.chars().count() <= 500,
        (
            BlueprintCourseListSort::Adoptions,
            BlueprintCourseListCursorPosition::Adoptions { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Students,
            BlueprintCourseListCursorPosition::Students { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Stars,
            BlueprintCourseListCursorPosition::Stars { long_name, .. },
        )
        | (
            BlueprintCourseListSort::Watches,
            BlueprintCourseListCursorPosition::Watches { long_name, .. },
        )
        | (
            BlueprintCourseListSort::RecentEdits,
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
        BlueprintCourseListSort::Name => BlueprintCourseListCursorPosition::Name {
            long_name: record.long_name.clone(),
            blueprint_course_id: record.id.clone(),
        },
        BlueprintCourseListSort::Adoptions => BlueprintCourseListCursorPosition::Adoptions {
            count: record.total_adoptions,
            long_name: record.long_name.clone(),
            blueprint_course_id: record.id.clone(),
        },
        BlueprintCourseListSort::Students => BlueprintCourseListCursorPosition::Students {
            count: record.total_students_ever_enrolled,
            long_name: record.long_name.clone(),
            blueprint_course_id: record.id.clone(),
        },
        BlueprintCourseListSort::Stars => BlueprintCourseListCursorPosition::Stars {
            count: record.star_count,
            long_name: record.long_name.clone(),
            blueprint_course_id: record.id.clone(),
        },
        BlueprintCourseListSort::Watches => BlueprintCourseListCursorPosition::Watches {
            count: record.watcher_count,
            long_name: record.long_name.clone(),
            blueprint_course_id: record.id.clone(),
        },
        BlueprintCourseListSort::RecentEdits => BlueprintCourseListCursorPosition::RecentEdits {
            edited_at_millis: record.last_edited_at_millis,
            long_name: record.long_name.clone(),
            blueprint_course_id: record.id.clone(),
        },
    }
}
