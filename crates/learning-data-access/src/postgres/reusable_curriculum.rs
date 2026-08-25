//! PostgreSQL broker adapter for reusable Blueprint and Alpha aggregates.

use async_trait::async_trait;
use question_model::{
    AlphaCourseAccess, AlphaCourseModuleView, AlphaCourseReference, AlphaCourseRevision,
    AlphaCourseSummaryView, AlphaCourseView, BlueprintAccess, BlueprintReference,
    BlueprintRevision, BlueprintSummaryView, BlueprintView, CatalogDiscoveryEvidence,
    CatalogDiscoveryItem, CatalogProblemSummary, PublicAuthorName, PublicByline,
    ReusableAssignmentDefinitionInput, ReusableAssignmentDefinitionView,
    ReusableAssignmentEntryView, ReusablePoolCandidateView, ReusablePoolView, ReusableQuestionView,
    ReusableSelectionAvailability,
};
use sqlx::{Row, types::Json};

use super::{PostgresStore, map_sqlx_error};
use crate::{
    Cursor, Page, PageRequest, ReplaceAlphaCourseCommand, ReplaceBlueprintCommand,
    ReusableCurriculumCapability, ReusableCurriculumStore, SessionTokenHash, StoreError,
    TenantContext, decode_sealed_cursor_u32, encode_sealed_cursor_u32,
};

const CURSOR_BLUEPRINTS: &str = "reusable-curriculum-blueprints";
const CURSOR_ALPHA: &str = "reusable-curriculum-alpha";
const CURSOR_DOMAIN: &[u8] = b"peptidyle/reusable-curriculum-cursor/v1";

#[async_trait]
impl ReusableCurriculumStore for PostgresStore {
    async fn preflight_reusable_curriculum(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        capability: ReusableCurriculumCapability,
    ) -> Result<(), StoreError> {
        let capability = match capability {
            ReusableCurriculumCapability::BlueprintPersonal => "blueprintPersonal",
            ReusableCurriculumCapability::AlphaRead => "alphaRead",
            ReusableCurriculumCapability::AlphaCreatorWrite => "alphaCreatorWrite",
        };
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let authorized: bool =
            sqlx::query_scalar("SELECT public.ple_reusable_curriculum_preflight_v1($1, $2, $3)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(capability)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_reusable_curriculum_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        authorized.then_some(()).ok_or(StoreError::Forbidden)
    }

    async fn list_blueprints(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<BlueprintSummaryView>, StoreError> {
        let scope = cursor_scope(CURSOR_BLUEPRINTS, context, session);
        let after = decode_cursor(&self.catalog_cursors, page.after.as_ref(), &scope)?;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let rows =
            sqlx::query("SELECT * FROM public.ple_list_curriculum_blueprints_v1($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(after)
                .bind(i32::from(page.size.get()) + 1)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_reusable_curriculum_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        blueprint_page(&self.catalog_cursors, rows, page.size.get(), &scope)
    }

    async fn get_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
    ) -> Result<Option<BlueprintView>, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let detail: Option<Json<serde_json::Value>> =
            sqlx::query_scalar("SELECT public.ple_get_curriculum_blueprint_v1($1, $2, $3)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(route_reference(reference.number())?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_reusable_curriculum_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        detail
            .map(|Json(detail)| decode_blueprint_detail(detail))
            .transpose()
    }

    async fn replace_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceBlueprintCommand,
    ) -> Result<BlueprintView, StoreError> {
        command
            .definition
            .validate()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        validate_blueprint_replacement_target(&command)?;
        let body = serde_json::to_value(&command.definition.definition)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_replace_curriculum_blueprint_v1($1, $2, $3, $4, $5)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .bind(
            command
                .reference
                .map(|reference| route_reference(reference.number()))
                .transpose()?,
        )
        .bind(
            command
                .expected_revision
                .map(|revision| revision_value(revision.value()))
                .transpose()?,
        )
        .bind(Json(body))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_reusable_curriculum_error)?
        .ok_or(StoreError::NotFound)?;
        let reference = decode_blueprint_reference(
            row.try_get("blueprint_reference").map_err(map_sqlx_error)?,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.get_blueprint(context, session, reference)
            .await?
            .ok_or(StoreError::NotFound)
    }

    async fn delete_blueprint(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: BlueprintReference,
        expected_revision: BlueprintRevision,
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let deleted: bool =
            sqlx::query_scalar("SELECT public.ple_delete_curriculum_blueprint_v1($1, $2, $3, $4)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(route_reference(reference.number())?)
                .bind(revision_value(expected_revision.value())?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_reusable_curriculum_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(deleted)
    }

    async fn list_alpha_courses(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        page: PageRequest,
    ) -> Result<Page<AlphaCourseSummaryView>, StoreError> {
        let scope = cursor_scope(CURSOR_ALPHA, context, session);
        let after = decode_cursor(&self.catalog_cursors, page.after.as_ref(), &scope)?;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let rows = sqlx::query("SELECT * FROM public.ple_list_curriculum_alpha_v1($1, $2, $3, $4)")
            .bind(context.tenant_id().as_uuid())
            .bind(session.to_string())
            .bind(after)
            .bind(i32::from(page.size.get()) + 1)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_reusable_curriculum_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        alpha_page(&self.catalog_cursors, rows, page.size.get(), &scope)
    }

    async fn get_alpha_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: AlphaCourseReference,
    ) -> Result<Option<AlphaCourseView>, StoreError> {
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let detail: Option<Json<serde_json::Value>> =
            sqlx::query_scalar("SELECT public.ple_get_curriculum_alpha_v1($1, $2, $3)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(route_reference(reference.number())?)
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_reusable_curriculum_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        detail
            .map(|Json(detail)| decode_alpha_detail(detail))
            .transpose()
    }

    async fn replace_alpha_course(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceAlphaCourseCommand,
    ) -> Result<AlphaCourseView, StoreError> {
        command
            .definition
            .validate()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        validate_alpha_replacement_target(&command)?;
        let body = serde_json::to_value(&command.definition)
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self.begin_tenant_session(context, session).await?;
        let row =
            sqlx::query("SELECT * FROM public.ple_replace_curriculum_alpha_v1($1, $2, $3, $4, $5)")
                .bind(context.tenant_id().as_uuid())
                .bind(session.to_string())
                .bind(
                    command
                        .reference
                        .map(|reference| route_reference(reference.number()))
                        .transpose()?,
                )
                .bind(
                    command
                        .expected_revision
                        .map(|revision| revision_value(revision.value()))
                        .transpose()?,
                )
                .bind(Json(body))
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_reusable_curriculum_error)?
                .ok_or(StoreError::NotFound)?;
        let reference = decode_alpha_reference(
            row.try_get("alpha_course_reference")
                .map_err(map_sqlx_error)?,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.get_alpha_course(context, session, reference)
            .await?
            .ok_or(StoreError::NotFound)
    }
}

fn decode_blueprint_detail(detail: serde_json::Value) -> Result<BlueprintView, StoreError> {
    let object = detail_object(&detail, "Blueprint detail")?;
    if detail_string(object, "access")? != "owner" {
        return Err(unavailable("stored Blueprint access is invalid"));
    }
    Ok(BlueprintView {
        reference: decode_blueprint_reference(detail_i32(object, "reference")?)?,
        revision: decode_blueprint_revision(detail_i64(object, "revision")?)?,
        access: BlueprintAccess::Owner,
        definition: decode_definition(detail_value(object, "definition")?)?,
    })
}

fn decode_alpha_detail(detail: serde_json::Value) -> Result<AlphaCourseView, StoreError> {
    let object = detail_object(&detail, "Alpha detail")?;
    let modules = detail_array(object, "modules")?
        .iter()
        .enumerate()
        .map(|(module_position, module)| {
            let module = detail_object(module, "Alpha module")?;
            detail_position(module, "position", module_position)?;
            let definitions = detail_array(module, "definitions")?
                .iter()
                .enumerate()
                .map(|(definition_position, definition)| {
                    let definition = detail_object(definition, "Alpha definition")?;
                    detail_position(definition, "position", definition_position)?;
                    decode_definition(detail_value(definition, "definition")?)
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            Ok(AlphaCourseModuleView {
                label: detail_string(module, "label")?.to_string(),
                definitions,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(AlphaCourseView {
        reference: decode_alpha_reference(detail_i32(object, "reference")?)?,
        title: detail_string(object, "title")?.to_string(),
        revision: decode_alpha_revision(detail_i64(object, "revision")?)?,
        creator_byline: decode_byline(detail_string_array(object, "creatorByline")?)?,
        access: decode_alpha_access(detail_string(object, "access")?.to_string())?,
        modules,
    })
}

fn decode_definition(
    value: &serde_json::Value,
) -> Result<ReusableAssignmentDefinitionView, StoreError> {
    let object = detail_object(value, "reusable definition")?;
    let entries = detail_array(object, "entries")?
        .iter()
        .map(decode_entry)
        .collect::<Result<Vec<_>, StoreError>>()?;
    let mut input_value = value.clone();
    input_value
        .as_object_mut()
        .ok_or_else(|| unavailable("reusable definition is invalid"))?
        .insert("entries".to_string(), serde_json::Value::Array(Vec::new()));
    let input: ReusableAssignmentDefinitionInput = serde_json::from_value(input_value)
        .map_err(|_| unavailable("reusable definition meaning is invalid"))?;
    Ok(ReusableAssignmentDefinitionView {
        title: input.title,
        instructions: input.instructions,
        entries,
        defaults: input.defaults,
        schedule: input.schedule,
    })
}

fn decode_entry(value: &serde_json::Value) -> Result<ReusableAssignmentEntryView, StoreError> {
    let object = detail_object(value, "reusable entry")?;
    match detail_string(object, "kind")? {
        "fixed" => Ok(ReusableAssignmentEntryView::Fixed {
            question: Box::new(decode_question(detail_value(object, "catalog")?)?),
            points_possible: serde_json::from_value(
                detail_value(object, "pointsPossible")?.clone(),
            )
            .map_err(|_| unavailable("stored fixed points are invalid"))?,
            scoring_mode: serde_json::from_value(detail_value(object, "scoringMode")?.clone())
                .map_err(|_| unavailable("stored fixed scoring mode is invalid"))?,
        }),
        "pool" => Ok(ReusableAssignmentEntryView::Pool(ReusablePoolView {
            candidates: detail_array(object, "candidates")?
                .iter()
                .map(|candidate| {
                    let candidate = detail_object(candidate, "pool candidate")?;
                    let question = decode_question(detail_value(candidate, "catalog")?)?;
                    Ok(ReusablePoolCandidateView {
                        catalog: question.catalog,
                        selection_availability: question.selection_availability,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?,
            draw_count: serde_json::from_value(detail_value(object, "drawCount")?.clone())
                .map_err(|_| unavailable("stored pool draw count is invalid"))?,
            points_per_item: serde_json::from_value(detail_value(object, "pointsPerItem")?.clone())
                .map_err(|_| unavailable("stored pool points are invalid"))?,
            ordering: serde_json::from_value(detail_value(object, "ordering")?.clone())
                .map_err(|_| unavailable("stored pool ordering is invalid"))?,
            algorithm: serde_json::from_value(detail_value(object, "algorithm")?.clone())
                .map_err(|_| unavailable("stored pool algorithm is invalid"))?,
        })),
        _ => Err(unavailable("stored reusable entry kind is invalid")),
    }
}

fn decode_question(value: &serde_json::Value) -> Result<ReusableQuestionView, StoreError> {
    let mut catalog_value = value.clone();
    let catalog = catalog_value
        .as_object_mut()
        .ok_or_else(|| unavailable("stored reusable catalog projection is invalid"))?;
    let available = catalog
        .remove("selectionAvailable")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| unavailable("stored reusable selection availability is invalid"))?;
    let evidence = catalog
        .remove("evidence")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| unavailable("stored reusable catalog evidence is invalid"))?
        .unwrap_or(CatalogDiscoveryEvidence::InsufficientEvidence);
    let summary: CatalogProblemSummary = serde_json::from_value(catalog_value)
        .map_err(|_| unavailable("stored reusable catalog projection is invalid"))?;
    Ok(ReusableQuestionView {
        catalog: CatalogDiscoveryItem { summary, evidence },
        selection_availability: if available {
            ReusableSelectionAvailability::Available
        } else {
            ReusableSelectionAvailability::Retained
        },
    })
}

fn blueprint_page(
    codec: &crate::CatalogCursorCodec,
    rows: Vec<sqlx::postgres::PgRow>,
    size: u16,
    scope: &str,
) -> Result<Page<BlueprintSummaryView>, StoreError> {
    let has_more = rows.len() > usize::from(size);
    let items = rows
        .into_iter()
        .take(usize::from(size))
        .map(|row| {
            Ok(BlueprintSummaryView {
                reference: decode_blueprint_reference(
                    row.try_get("blueprint_reference").map_err(map_sqlx_error)?,
                )?,
                title: row.try_get("title").map_err(map_sqlx_error)?,
                revision: decode_blueprint_revision(
                    row.try_get("revision").map_err(map_sqlx_error)?,
                )?,
                access: BlueprintAccess::Owner,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Page {
        next_cursor: has_more.then(|| {
            next_cursor(
                codec,
                scope,
                items.last().expect("page has item").reference.number(),
            )
        }),
        items,
    })
}

fn alpha_page(
    codec: &crate::CatalogCursorCodec,
    rows: Vec<sqlx::postgres::PgRow>,
    size: u16,
    scope: &str,
) -> Result<Page<AlphaCourseSummaryView>, StoreError> {
    let has_more = rows.len() > usize::from(size);
    let items = rows
        .into_iter()
        .take(usize::from(size))
        .map(|row| {
            Ok(AlphaCourseSummaryView {
                reference: decode_alpha_reference(
                    row.try_get("alpha_course_reference")
                        .map_err(map_sqlx_error)?,
                )?,
                title: row.try_get("title").map_err(map_sqlx_error)?,
                revision: decode_alpha_revision(row.try_get("revision").map_err(map_sqlx_error)?)?,
                creator_byline: decode_byline(
                    row.try_get("creator_public_byline")
                        .map_err(map_sqlx_error)?,
                )?,
                access: decode_alpha_access(row.try_get("access").map_err(map_sqlx_error)?)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(Page {
        next_cursor: has_more.then(|| {
            next_cursor(
                codec,
                scope,
                items.last().expect("page has item").reference.number(),
            )
        }),
        items,
    })
}

fn next_cursor(codec: &crate::CatalogCursorCodec, scope: &str, value: u32) -> Cursor {
    Cursor::from_stable_key(
        encode_sealed_cursor_u32(codec, CURSOR_DOMAIN, scope.as_bytes(), value)
            .expect("configured cursor codec"),
    )
}

fn cursor_scope(kind: &str, context: TenantContext, session: SessionTokenHash) -> String {
    format!("{kind}:{}:{session}", context.tenant_id())
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
                        StoreError::InvalidRecord(
                            "reusable curriculum cursor is invalid".to_string(),
                        )
                    })
                })
                .map(Some)
        })
        .transpose()
        .map(Option::flatten)
}

fn detail_object<'a>(
    value: &'a serde_json::Value,
    label: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, StoreError> {
    value
        .as_object()
        .ok_or_else(|| unavailable(&format!("stored {label} is invalid")))
}

fn detail_value<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<&'a serde_json::Value, StoreError> {
    object
        .get(name)
        .ok_or_else(|| unavailable(&format!("stored reusable detail lacks {name}")))
}

fn detail_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<&'a Vec<serde_json::Value>, StoreError> {
    detail_value(object, name)?
        .as_array()
        .ok_or_else(|| unavailable(&format!("stored reusable {name} is invalid")))
}

fn detail_string<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<&'a str, StoreError> {
    detail_value(object, name)?
        .as_str()
        .ok_or_else(|| unavailable(&format!("stored reusable {name} is invalid")))
}

fn detail_string_array(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Vec<String>, StoreError> {
    detail_array(object, name)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| unavailable("stored reusable byline is invalid"))
        })
        .collect()
}

fn detail_i32(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<i32, StoreError> {
    serde_json::from_value(detail_value(object, name)?.clone())
        .map_err(|_| unavailable(&format!("stored reusable {name} is invalid")))
}

fn detail_i64(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<i64, StoreError> {
    serde_json::from_value(detail_value(object, name)?.clone())
        .map_err(|_| unavailable(&format!("stored reusable {name} is invalid")))
}

fn detail_position(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    expected: usize,
) -> Result<(), StoreError> {
    let value = detail_i64(object, name)?;
    (usize::try_from(value).ok() == Some(expected))
        .then_some(())
        .ok_or_else(|| unavailable("stored reusable detail ordering is invalid"))
}

fn unavailable(message: &str) -> StoreError {
    StoreError::Unavailable(message.to_string())
}

fn route_reference(value: u32) -> Result<i32, StoreError> {
    i32::try_from(value).map_err(|_| {
        StoreError::Unavailable("curriculum route reference is out of range".to_string())
    })
}
fn revision_value(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value)
        .map_err(|_| StoreError::Unavailable("curriculum revision is out of range".to_string()))
}
fn decode_blueprint_reference(value: i32) -> Result<BlueprintReference, StoreError> {
    BlueprintReference::new(u64::try_from(value).map_err(|_| {
        StoreError::Unavailable("stored Blueprint reference is invalid".to_string())
    })?)
    .ok_or_else(|| StoreError::Unavailable("stored Blueprint reference is invalid".to_string()))
}
fn decode_alpha_reference(value: i32) -> Result<AlphaCourseReference, StoreError> {
    AlphaCourseReference::new(
        u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored Alpha reference is invalid".to_string())
        })?,
    )
    .ok_or_else(|| StoreError::Unavailable("stored Alpha reference is invalid".to_string()))
}
fn decode_blueprint_revision(value: i64) -> Result<BlueprintRevision, StoreError> {
    BlueprintRevision::new(
        u64::try_from(value).map_err(|_| {
            StoreError::Unavailable("stored Blueprint revision is invalid".to_string())
        })?,
    )
    .ok_or_else(|| StoreError::Unavailable("stored Blueprint revision is invalid".to_string()))
}
fn decode_alpha_revision(value: i64) -> Result<AlphaCourseRevision, StoreError> {
    AlphaCourseRevision::new(
        u64::try_from(value)
            .map_err(|_| StoreError::Unavailable("stored Alpha revision is invalid".to_string()))?,
    )
    .ok_or_else(|| StoreError::Unavailable("stored Alpha revision is invalid".to_string()))
}
fn decode_byline(values: Vec<String>) -> Result<PublicByline, StoreError> {
    PublicByline::new(
        values
            .into_iter()
            .map(PublicAuthorName::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable("stored Alpha byline is invalid".to_string()))?,
    )
    .map_err(|_| StoreError::Unavailable("stored Alpha byline is invalid".to_string()))
}
fn decode_alpha_access(value: String) -> Result<AlphaCourseAccess, StoreError> {
    match value.as_str() {
        "creator" => Ok(AlphaCourseAccess::Creator),
        "approvedInstructor" => Ok(AlphaCourseAccess::ApprovedInstructor),
        _ => Err(StoreError::Unavailable(
            "stored Alpha access is invalid".to_string(),
        )),
    }
}

fn map_reusable_curriculum_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database) = &error {
        if database.code().as_deref() == Some("55000") && database.message().contains("revision") {
            return StoreError::Conflict;
        }
        if database.code().as_deref() == Some("42501") {
            return StoreError::Forbidden;
        }
    }
    map_sqlx_error(error)
}

fn validate_blueprint_replacement_target(
    command: &ReplaceBlueprintCommand,
) -> Result<(), StoreError> {
    match (command.reference, command.expected_revision) {
        (None, None) | (Some(_), Some(_)) => Ok(()),
        (None, Some(_)) => Err(StoreError::InvalidRecord(
            "new Blueprint cannot carry an observed revision".to_string(),
        )),
        (Some(_), None) => Err(StoreError::InvalidRecord(
            "Blueprint replacement requires its observed revision".to_string(),
        )),
    }
}

fn validate_alpha_replacement_target(
    command: &ReplaceAlphaCourseCommand,
) -> Result<(), StoreError> {
    match (command.reference, command.expected_revision) {
        (None, None) | (Some(_), Some(_)) => Ok(()),
        (None, Some(_)) => Err(StoreError::InvalidRecord(
            "new Alpha curriculum cannot carry an observed revision".to_string(),
        )),
        (Some(_), None) => Err(StoreError::InvalidRecord(
            "Alpha replacement requires its observed revision".to_string(),
        )),
    }
}
