//! PostgreSQL broker adapter for the D2 problem-curation aggregate.

use async_trait::async_trait;
use question_model::{
    CatalogSearchFilter, ProblemCollectionAccess, ProblemCollectionKind,
    ProblemCollectionMemberView, ProblemCollectionReference, ProblemCollectionRevision,
    ProblemCollectionSelectionAvailability, ProblemCollectionSummaryView,
    ProblemCollectionVisibility, SavedProblemSearchReference, SavedProblemSearchRevision,
    SavedProblemSearchView, validate_problem_curation_title,
};
use serde_json::Value;
use sqlx::{Row, types::Json};

use super::{PostgresStore, decode_catalog_summary_row, map_sqlx_error};
use crate::{
    Cursor, Page, PageRequest, ProblemCollectionMembersPage, ProblemCollectionReplacementTarget,
    ProblemCurationCapability, ProblemCurationStore, ReplaceProblemCollectionCommand,
    ReplaceSavedProblemSearchCommand, SessionTokenHash, StoreError, TenantContext,
    decode_sealed_cursor_u32, encode_sealed_cursor_u32,
};

const CURSOR_COLLECTIONS: &str = "problem-curation-collections";
const CURSOR_MEMBERS: &str = "problem-curation-members";
const CURSOR_SEARCHES: &str = "problem-curation-searches";
const CURSOR_DOMAIN: &[u8] = b"peptidyle/problem-curation-cursor/v1";

#[async_trait]
impl ProblemCurationStore for PostgresStore {
    async fn preflight_problem_curation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        capability: ProblemCurationCapability,
    ) -> Result<(), StoreError> {
        let capability = match capability {
            ProblemCurationCapability::CatalogInstitutionRead => "catalogInstitutionRead",
            ProblemCurationCapability::PersonalMutation => "personalMutation",
        };
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let authorized: bool =
            sqlx::query_scalar("SELECT public.ple_problem_curation_preflight_v1($1, $2, $3)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(capability)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_curation_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        authorized.then_some(()).ok_or(StoreError::Forbidden)
    }

    async fn get_or_create_favorites(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<ProblemCollectionSummaryView, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query(
            "SELECT *, 'owner'::text AS access FROM public.ple_ensure_problem_favorites_v1($1, $2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_curation_error)?
        .ok_or(StoreError::Forbidden)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        decode_collection_summary(&row)
    }

    async fn list_problem_collections(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<ProblemCollectionSummaryView>, StoreError> {
        let scope = cursor_scope(CURSOR_COLLECTIONS, context, session, None, None);
        let after = decode_cursor(&self.catalog_cursors, page.after.as_ref(), &scope)?;
        let limit = i32::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let rows =
            sqlx::query("SELECT * FROM public.ple_list_problem_collections_v1($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(after)
                .bind(limit)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        collection_page(&self.catalog_cursors, rows, page.size.get(), &scope)
    }

    async fn get_problem_collection_summary(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
    ) -> Result<Option<ProblemCollectionSummaryView>, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query("SELECT * FROM public.ple_problem_collection_summary_v1($1, $2, $3)")
            .bind(context.tenant_id().as_uuid())
            .bind(session.to_string())
            .bind(i32::try_from(reference.number()).expect("route reference fits i32"))
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.as_ref().map(decode_collection_summary).transpose()
    }

    async fn list_problem_collection_members(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        page: PageRequest,
    ) -> Result<Option<ProblemCollectionMembersPage>, StoreError> {
        let mut transaction = self.begin_tenant_session_snapshot(context, session).await?;
        let summary_row =
            sqlx::query("SELECT * FROM public.ple_problem_collection_summary_v1($1, $2, $3)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(i32::try_from(reference.number()).expect("route reference fits i32"))
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let Some(summary_row) = summary_row else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let collection = decode_collection_summary(&summary_row)?;
        let scope = cursor_scope(
            CURSOR_MEMBERS,
            context,
            session,
            Some(reference.number()),
            Some(collection.revision.value()),
        );
        let after = decode_cursor(&self.catalog_cursors, page.after.as_ref(), &scope)?;
        let limit = i32::from(page.size.get()) + 1;
        let rows = sqlx::query(
            "SELECT * FROM public.ple_problem_collection_members_v1($1, $2, $3, $4, $5)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .bind(i32::try_from(reference.number()).expect("route reference fits i32"))
        .bind(after)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        member_page(&self.catalog_cursors, rows, page.size.get(), &scope).map(|members| {
            Some(ProblemCollectionMembersPage {
                collection,
                members,
            })
        })
    }

    async fn replace_problem_collection(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceProblemCollectionCommand,
    ) -> Result<ProblemCollectionSummaryView, StoreError> {
        let (reference, expected, title, visibility) = match command.target {
            ProblemCollectionReplacementTarget::Favorites => {
                let summary = self.get_or_create_favorites(context, session).await?;
                (
                    Some(summary.reference),
                    summary.revision.value(),
                    "Favorites".to_string(),
                    "private",
                )
            }
            ProblemCollectionReplacementTarget::NewNamed => (
                None,
                command
                    .expected_revision
                    .map_or(0, |revision| revision.value()),
                command.title.clone().ok_or_else(|| {
                    StoreError::InvalidRecord("named collection requires a title".into())
                })?,
                visibility_name(command.visibility.ok_or_else(|| {
                    StoreError::InvalidRecord("named collection requires a visibility".into())
                })?),
            ),
            ProblemCollectionReplacementTarget::Existing(reference) => (
                Some(reference),
                command
                    .expected_revision
                    .ok_or(StoreError::Conflict)?
                    .value(),
                command.title.clone().ok_or_else(|| {
                    StoreError::InvalidRecord("named collection requires a title".into())
                })?,
                visibility_name(command.visibility.ok_or_else(|| {
                    StoreError::InvalidRecord("named collection requires a visibility".into())
                })?),
            ),
        };
        let expected = match command.target {
            ProblemCollectionReplacementTarget::Favorites => command
                .expected_revision
                .map_or(expected, |revision| revision.value()),
            _ => expected,
        };
        validate_problem_curation_title(&title)
            .map_err(|_| StoreError::InvalidRecord("invalid problem curation title".to_string()))?;
        let question_ids: Vec<String> = command
            .question_ids
            .iter()
            .map(|question_id| question_id.compact())
            .collect();
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query(
            "SELECT *, 'owner'::text AS access FROM public.ple_replace_problem_collection_v1($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .bind(reference.map(|value| i32::try_from(value.number()).expect("route reference fits i32")))
        .bind(i64::try_from(expected).map_err(|_| StoreError::Unavailable("problem collection revision is out of range".into()))?)
        .bind(title)
        .bind(visibility)
        .bind(question_ids)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_curation_error)?
        .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        decode_collection_summary(&row)
    }

    async fn delete_problem_collection(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: ProblemCollectionReference,
        expected_revision: ProblemCollectionRevision,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let result =
            sqlx::query_scalar("SELECT public.ple_delete_problem_collection_v1($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(i32::try_from(reference.number()).expect("route reference fits i32"))
                .bind(i64::try_from(expected_revision.value()).map_err(|_| {
                    StoreError::Unavailable("problem collection revision is out of range".into())
                })?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_curation_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_saved_problem_searches(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<SavedProblemSearchView>, StoreError> {
        let scope = cursor_scope(CURSOR_SEARCHES, context, session, None, None);
        let after = decode_cursor(&self.catalog_cursors, page.after.as_ref(), &scope)?;
        let limit = i32::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let rows =
            sqlx::query("SELECT * FROM public.ple_list_saved_problem_searches_v1($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(after)
                .bind(limit)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        saved_search_page(&self.catalog_cursors, rows, page.size.get(), &scope)
    }

    async fn get_saved_problem_search(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: SavedProblemSearchReference,
    ) -> Result<Option<SavedProblemSearchView>, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query("SELECT * FROM public.ple_saved_problem_search_v1($1, $2, $3)")
            .bind(context.tenant_id().as_uuid())
            .bind(session.to_string())
            .bind(i32::try_from(reference.number()).expect("route reference fits i32"))
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.as_ref().map(decode_saved_search).transpose()
    }

    async fn replace_saved_problem_search(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceSavedProblemSearchCommand,
    ) -> Result<SavedProblemSearchView, StoreError> {
        validate_problem_curation_title(&command.title)
            .map_err(|_| StoreError::InvalidRecord("invalid problem curation title".to_string()))?;
        let filter = command
            .filter
            .normalized()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let json = serde_json::to_value(filter)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_replace_saved_problem_search_v1($1, $2, $3, $4, $5, $6)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .bind(
            command
                .reference
                .map(|value| i32::try_from(value.number()).expect("route reference fits i32")),
        )
        .bind(
            i64::try_from(
                command
                    .expected_revision
                    .map_or(0, |revision| revision.value()),
            )
            .map_err(|_| StoreError::Unavailable("saved search revision is out of range".into()))?,
        )
        .bind(command.title)
        .bind(Json(json))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_curation_error)?
        .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        decode_saved_search(&row)
    }

    async fn delete_saved_problem_search(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: SavedProblemSearchReference,
        expected_revision: SavedProblemSearchRevision,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let result =
            sqlx::query_scalar("SELECT public.ple_delete_saved_problem_search_v1($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(i32::try_from(reference.number()).expect("route reference fits i32"))
                .bind(i64::try_from(expected_revision.value()).map_err(|_| {
                    StoreError::Unavailable("saved problem search revision is out of range".into())
                })?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_curation_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

fn collection_page(
    codec: &crate::CatalogCursorCodec,
    rows: Vec<sqlx::postgres::PgRow>,
    size: u16,
    scope: &str,
) -> Result<Page<ProblemCollectionSummaryView>, StoreError> {
    let has_more = rows.len() > usize::from(size);
    let items = rows
        .into_iter()
        .take(usize::from(size))
        .map(|row| decode_collection_summary(&row))
        .collect::<Result<Vec<_>, _>>()?;
    let next_cursor = has_more.then(|| {
        Cursor::from_stable_key(
            encode_sealed_cursor_u32(
                codec,
                CURSOR_DOMAIN,
                scope.as_bytes(),
                items
                    .last()
                    .expect("nonempty when next cursor")
                    .reference
                    .number(),
            )
            .expect("configured cursor codec"),
        )
    });
    Ok(Page { items, next_cursor })
}

fn member_page(
    codec: &crate::CatalogCursorCodec,
    rows: Vec<sqlx::postgres::PgRow>,
    size: u16,
    scope: &str,
) -> Result<Page<ProblemCollectionMemberView>, StoreError> {
    let has_more = rows.len() > usize::from(size);
    let mut last_position = None;
    let items = rows
        .into_iter()
        .take(usize::from(size))
        .map(|row| {
            last_position = Some(row.try_get::<i32, _>("position").map_err(map_sqlx_error)?);
            let summary = decode_catalog_summary_row(&row)?;
            let available = row
                .try_get::<bool, _>("selection_available")
                .map_err(map_sqlx_error)?;
            Ok(ProblemCollectionMemberView {
                question_id: summary.question_id.clone(),
                summary,
                selection_availability: if available {
                    ProblemCollectionSelectionAvailability::Available
                } else {
                    ProblemCollectionSelectionAvailability::Retained
                },
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Page {
        items,
        next_cursor: has_more.then(|| {
            Cursor::from_stable_key(
                encode_sealed_cursor_u32(
                    codec,
                    CURSOR_DOMAIN,
                    scope.as_bytes(),
                    u32::try_from(last_position.expect("nonempty when next cursor"))
                        .expect("position nonnegative")
                        + 1,
                )
                .expect("configured cursor codec"),
            )
        }),
    })
}

fn saved_search_page(
    codec: &crate::CatalogCursorCodec,
    rows: Vec<sqlx::postgres::PgRow>,
    size: u16,
    scope: &str,
) -> Result<Page<SavedProblemSearchView>, StoreError> {
    let has_more = rows.len() > usize::from(size);
    let items = rows
        .into_iter()
        .take(usize::from(size))
        .map(|row| decode_saved_search(&row))
        .collect::<Result<Vec<_>, _>>()?;
    let next_cursor = has_more.then(|| {
        Cursor::from_stable_key(
            encode_sealed_cursor_u32(
                codec,
                CURSOR_DOMAIN,
                scope.as_bytes(),
                items
                    .last()
                    .expect("nonempty when next cursor")
                    .reference
                    .number(),
            )
            .expect("configured cursor codec"),
        )
    });
    Ok(Page { items, next_cursor })
}

fn decode_collection_summary(
    row: &sqlx::postgres::PgRow,
) -> Result<ProblemCollectionSummaryView, StoreError> {
    let reference = positive_reference(
        row.try_get::<i32, _>("collection_reference")
            .map_err(map_sqlx_error)?,
        ProblemCollectionReference::new,
        "problem collection reference",
    )?;
    let revision = positive_revision(
        row.try_get::<i64, _>("revision").map_err(map_sqlx_error)?,
        ProblemCollectionRevision::new,
        "problem collection revision",
    )?;
    let kind = match row
        .try_get::<String, _>("kind")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "favorites" => ProblemCollectionKind::Favorites,
        "named" => ProblemCollectionKind::Named,
        _ => {
            return Err(StoreError::Unavailable(
                "stored problem collection kind is invalid".into(),
            ));
        }
    };
    let visibility = decode_visibility(row.try_get("visibility").map_err(map_sqlx_error)?)?;
    let access = match row
        .try_get::<String, _>("access")
        .map_err(map_sqlx_error)?
        .as_str()
    {
        "owner" => ProblemCollectionAccess::Owner,
        "institutionReader" => ProblemCollectionAccess::InstitutionReader,
        _ => {
            return Err(StoreError::Unavailable(
                "stored problem collection access is invalid".into(),
            ));
        }
    };
    Ok(ProblemCollectionSummaryView {
        reference,
        kind,
        title: row.try_get("title").map_err(map_sqlx_error)?,
        visibility,
        revision,
        access,
    })
}

fn decode_saved_search(row: &sqlx::postgres::PgRow) -> Result<SavedProblemSearchView, StoreError> {
    let reference = positive_reference(
        row.try_get::<i32, _>("search_reference")
            .map_err(map_sqlx_error)?,
        SavedProblemSearchReference::new,
        "saved problem search reference",
    )?;
    let revision = positive_revision(
        row.try_get::<i64, _>("revision").map_err(map_sqlx_error)?,
        SavedProblemSearchRevision::new,
        "saved problem search revision",
    )?;
    let schema_version: i16 = row
        .try_get("query_schema_version")
        .map_err(map_sqlx_error)?;
    let stored_digest: Vec<u8> = row
        .try_get("normalized_query_sha256")
        .map_err(map_sqlx_error)?;
    let canonical_digest: Vec<u8> = row
        .try_get("canonical_query_sha256")
        .map_err(map_sqlx_error)?;
    if schema_version != 1 || stored_digest.len() != 32 || stored_digest != canonical_digest {
        return Err(StoreError::Unavailable(
            "stored saved problem search integrity is invalid".into(),
        ));
    }
    let Json(json): Json<Value> = row.try_get("normalized_query").map_err(map_sqlx_error)?;
    let stored_filter: CatalogSearchFilter = serde_json::from_value(json)
        .map_err(|_| StoreError::Unavailable("stored saved problem search is invalid".into()))?;
    let filter = stored_filter.clone().normalized().map_err(|_| {
        StoreError::Unavailable("stored saved problem search is not normalized".into())
    })?;
    if stored_filter != filter {
        return Err(StoreError::Unavailable(
            "stored saved problem search is not canonical".into(),
        ));
    }
    Ok(SavedProblemSearchView {
        reference,
        title: row.try_get("title").map_err(map_sqlx_error)?,
        filter,
        revision,
    })
}

fn positive_reference<T>(
    value: i32,
    build: impl FnOnce(u64) -> Option<T>,
    label: &str,
) -> Result<T, StoreError> {
    build(u64::try_from(value).ok().unwrap_or_default())
        .ok_or_else(|| StoreError::Unavailable(format!("stored {label} is invalid")))
}
fn positive_revision<T>(
    value: i64,
    build: impl FnOnce(u64) -> Option<T>,
    label: &str,
) -> Result<T, StoreError> {
    build(u64::try_from(value).ok().unwrap_or_default())
        .ok_or_else(|| StoreError::Unavailable(format!("stored {label} is invalid")))
}
fn decode_visibility(value: String) -> Result<ProblemCollectionVisibility, StoreError> {
    match value.as_str() {
        "private" => Ok(ProblemCollectionVisibility::Private),
        "institution" => Ok(ProblemCollectionVisibility::Institution),
        _ => Err(StoreError::Unavailable(
            "stored problem collection visibility is invalid".into(),
        )),
    }
}
fn visibility_name(value: ProblemCollectionVisibility) -> &'static str {
    match value {
        ProblemCollectionVisibility::Private => "private",
        ProblemCollectionVisibility::Institution => "institution",
    }
}
fn cursor_scope(
    kind: &str,
    context: TenantContext,
    session: SessionTokenHash,
    collection: Option<u32>,
    revision: Option<u64>,
) -> String {
    format!(
        "{kind}:{}:{session}:{}:{}",
        context.tenant_id(),
        collection.unwrap_or_default(),
        revision.unwrap_or_default()
    )
}

fn decode_cursor(
    codec: &crate::CatalogCursorCodec,
    cursor: Option<&Cursor>,
    scope: &str,
) -> Result<Option<i32>, StoreError> {
    cursor
        .map(|cursor| {
            decode_sealed_cursor_u32(codec, CURSOR_DOMAIN, scope.as_bytes(), cursor.as_str())
                .and_then(|value| {
                    i32::try_from(value).map_err(|_| {
                        StoreError::InvalidRecord("problem curation cursor is invalid".into())
                    })
                })
                .map(Some)
        })
        .transpose()
        .map(Option::flatten)
}
fn map_curation_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database) = &error {
        if database.code().as_deref() == Some("55000")
            && database.message().contains("revision conflict")
        {
            return StoreError::Conflict;
        }
        if database.code().as_deref() == Some("42501") {
            return StoreError::Forbidden;
        }
    }
    map_sqlx_error(error)
}
