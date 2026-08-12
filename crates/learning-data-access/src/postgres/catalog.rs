//! PostgreSQL catalog publication, discovery, and protected source access.

mod search;

use async_trait::async_trait;
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, CatalogLifecycle, CatalogProblemDetail, CatalogProblemSummary,
    CatalogSearchPage, CatalogSearchQuery, DraftQuestionSource, ProblemPublicId,
    ProblemVersionNumber, ProblemVersionRef, PublicationScope, QuestionDefinition,
    QuestionStatisticsDisclosure, UserId,
};
use serde_json::Value;
use sqlx::Row;

use super::connection::{map_sqlx_error, retry_transaction};
use super::{
    PostgresStore, catalog_lifecycle_parts, catalog_summary_page_from_rows,
    decode_catalog_payload_row, decode_payload_row, decode_payload_row_named, encode_payload,
    insert_catalog_asset_delivery, insert_problem_version, insert_published_source_artifact,
    publication_scope_name, question_backend_name, question_statistics_disclosure_from_row,
    taxonomy_page_from_rows, validated_deprecation_reason,
};
use crate::{
    CatalogSourceStore, CatalogStore, CatalogTransition, DraftRecord, Page, PageRequest,
    PublishDraftCommand, PublishedProblemRecord, PublishedSourceArtifact, QtiImportRegistry,
    StoreError, TenantContext, WorkspaceDraftRevision, WorkspaceFlatQuestionSource, ensure_tenant,
    validate_draft, validate_flat_question_publication, validate_publication_source,
    validate_published, validate_qti_publication_promotion,
    validate_source_artifact_for_publication, validate_source_artifact_identity,
};

#[async_trait]
impl CatalogStore for PostgresStore {
    async fn publish_draft(
        &self,
        context: TenantContext,
        actor: UserId,
        command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        retry_transaction(|| {
            let command = command.clone();
            async move {
                ensure_tenant(context, command.expected_draft.tenant)?;
                validate_draft(&command.expected_draft)?;
                validate_publication_source(&command.expected_draft, &command.published_source)?;
                let flat_promotion = match (
                    &command.expected_draft.question.source,
                    command.flat_question_promotion.as_ref(),
                ) {
                    (DraftQuestionSource::Native { .. }, Some(promotion)) => Some(promotion),
                    (DraftQuestionSource::Native { .. }, None) => None,
                    (_, Some(_)) => {
                        return Err(StoreError::InvalidRecord(
                            "flat-question promotion requires a native draft source".to_string(),
                        ));
                    }
                    (_, None) => None,
                };
                validate_source_artifact_for_publication(
                    command.publication,
                    &command.published_source,
                    command.source_artifact.as_ref(),
                    flat_promotion.is_some(),
                )?;
                let qti_promotion = match (
                    &command.expected_draft.question.source,
                    command.qti_promotion.as_ref(),
                ) {
                    (question_model::DraftQuestionSource::Qti { .. }, Some(promotion)) => {
                        Some(promotion)
                    }
                    (question_model::DraftQuestionSource::Qti { .. }, None) | (_, Some(_)) => {
                        return Err(StoreError::InvalidRecord(
                            "QTI publication requires dedicated committed staging evidence"
                                .to_string(),
                        ));
                    }
                    (_, None) => None,
                };
                if qti_promotion.is_some() && flat_promotion.is_some() {
                    return Err(StoreError::InvalidRecord(
                        "publication cannot contain both QTI and flat-question promotion"
                            .to_string(),
                    ));
                }

                let mut transaction = self.begin_tenant(context).await?;
                if command.publisher != actor {
                    return Err(StoreError::Forbidden);
                }
                let workspace_role: Option<String> = sqlx::query_scalar(
                    "SELECT role FROM workspace_draft_access \
             WHERE tenant_id = $1 AND workspace_id = $2 AND user_id = $3",
                )
                .bind(context.tenant_id().as_uuid())
                .bind(command.expected_draft.question.workspace.as_uuid())
                .bind(actor.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if workspace_role.as_deref() != Some("owner") {
                    return Err(StoreError::Forbidden);
                }
                let draft_row = sqlx::query(
                    "SELECT payload, payload_sha256, revision FROM workspace_draft \
             WHERE tenant_id = $1 AND workspace_id = $2 FOR UPDATE",
                )
                .bind(context.tenant_id().as_uuid())
                .bind(command.expected_draft.question.workspace.as_uuid())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
                let stored_draft: DraftRecord = decode_payload_row(&draft_row)?;
                let stored_revision = WorkspaceDraftRevision::from_stored(
                    draft_row.try_get("revision").map_err(map_sqlx_error)?,
                )?;
                if stored_draft != command.expected_draft
                    || stored_revision != command.expected_revision
                {
                    return Err(StoreError::Conflict);
                }
                let stored_draft_checksum: String = draft_row
                    .try_get("payload_sha256")
                    .map_err(map_sqlx_error)?;
                let current_flat_import_origin =
                    super::flat_import_provenance::read_workspace_flat_import_origin(
                        &mut transaction,
                        context,
                        actor,
                        command.expected_draft.question.workspace,
                    )
                    .await?;
                let requested_flat_import_promotion =
                    flat_promotion.and_then(|promotion| promotion.import_origin.as_ref());
                match (
                    current_flat_import_origin.as_ref(),
                    requested_flat_import_promotion,
                ) {
                    (Some(_), None) | (None, Some(_)) => return Err(StoreError::Conflict),
                    (Some(current), Some(requested)) => {
                        let exact = crate::FlatImportPublicationPromotion::new(
                            current,
                            command.publication,
                            requested.published_archive().clone(),
                        )?;
                        if exact != *requested {
                            return Err(StoreError::Conflict);
                        }
                    }
                    (None, None) => {}
                }
                let qti_item = if let Some(promotion) = qti_promotion {
                    let row = sqlx::query(
                "SELECT payload, payload_sha256 FROM ple_read_committed_qti_import($1, $2, $3)",
            )
            .bind(context.tenant_id().as_uuid())
            .bind(promotion.staging.workspace.as_uuid())
            .bind(promotion.staging.import.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
                    let registry: QtiImportRegistry = decode_payload_row(&row)?;
                    validate_qti_publication_promotion(context, &command, promotion, &registry)?;
                    let question_model::DraftQuestionSource::Qti { item_id, .. } =
                        &command.expected_draft.question.source
                    else {
                        unreachable!("QTI promotion was matched against a QTI draft");
                    };
                    Some(item_id.clone())
                } else {
                    None
                };
                let flat_source = if flat_promotion.is_some() {
                    let row = sqlx::query(
                        "SELECT source_payload, source_payload_sha256, canonical_source_sha256, \
                        public_binding_sha256 \
                 FROM workspace_flat_question_source \
                 WHERE tenant_id = $1 AND workspace_id = $2",
                    )
                    .bind(context.tenant_id().as_uuid())
                    .bind(command.expected_draft.question.workspace.as_uuid())
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?
                    .ok_or(StoreError::NotFound)?;
                    let source_record =
                        decode_payload_row_named(&row, "source_payload", "source_payload_sha256")?;
                    let source_payload_checksum: String = row
                        .try_get("source_payload_sha256")
                        .map_err(map_sqlx_error)?;
                    let DraftQuestionSource::Native { family } = &stored_draft.question.source
                    else {
                        return Err(StoreError::Conflict);
                    };
                    let staged = WorkspaceFlatQuestionSource::new(
                        context.tenant_id(),
                        command.expected_draft.question.workspace,
                        stored_revision,
                        family.to_string(),
                        source_record,
                        row.try_get("canonical_source_sha256")
                            .map_err(map_sqlx_error)?,
                        row.try_get("public_binding_sha256")
                            .map_err(map_sqlx_error)?,
                    )?;
                    validate_flat_question_publication(context, &command, &staged)?;
                    Some((staged, source_payload_checksum))
                } else {
                    None
                };

                let publication = command.publication;
                let (authors, previous_version, derived_from, existing_display_identity) =
                    if let Some(revises) = command.expected_draft.revises {
                        if publication.problem != revises.problem {
                            return Err(StoreError::InvalidRecord(
                                "revision must remain in its existing problem chain".to_string(),
                            ));
                        }
                        let base_row = sqlx::query(
                            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                            pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
                     FROM problem_version AS pv \
                     JOIN problem AS p USING (problem_id) \
                     JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
                     WHERE pv.problem_id = $1 AND pv.version_id = $2 \
                       AND p.owner_tenant_id = $3 \
                     FOR UPDATE OF pv",
                        )
                        .bind(revises.problem.as_uuid())
                        .bind(revises.version.as_uuid())
                        .bind(context.tenant_id().as_uuid())
                        .fetch_optional(&mut *transaction)
                        .await
                        .map_err(map_sqlx_error)?
                        .ok_or(StoreError::NotFound)?;
                        let base = decode_catalog_payload_row(&base_row)?;
                        if !base.authors.contains(&command.publisher) {
                            return Err(StoreError::Forbidden);
                        }
                        let has_successor: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM problem_version \
                     WHERE problem_id = $1 AND previous_version_id = $2)",
                        )
                        .bind(revises.problem.as_uuid())
                        .bind(revises.version.as_uuid())
                        .fetch_one(&mut *transaction)
                        .await
                        .map_err(map_sqlx_error)?;
                        if has_successor {
                            return Err(StoreError::Conflict);
                        }
                        (
                            base.authors,
                            Some(revises.version),
                            base.derived_from,
                            Some((
                                base.public_id,
                                base.version_number
                                    .value()
                                    .checked_add(1)
                                    .and_then(|value| ProblemVersionNumber::new(u64::from(value)))
                                    .ok_or_else(|| {
                                        StoreError::Unavailable(
                                            "problem version number limit reached".to_string(),
                                        )
                                    })?,
                            )),
                        )
                    } else {
                        if let Some(source) = command.expected_draft.derived_from {
                            let source_visible: bool = sqlx::query_scalar(
                                "SELECT EXISTS(SELECT 1 FROM problem_version \
                         WHERE problem_id = $1 AND version_id = $2)",
                            )
                            .bind(source.problem.as_uuid())
                            .bind(source.version.as_uuid())
                            .fetch_one(&mut *transaction)
                            .await
                            .map_err(map_sqlx_error)?;
                            if !source_visible {
                                return Err(StoreError::NotFound);
                            }
                        }
                        (
                            vec![command.publisher],
                            None,
                            command.expected_draft.derived_from,
                            None,
                        )
                    };

                let duplicate_version: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM problem_version \
             WHERE problem_id = $1 AND version_id = $2)",
                )
                .bind(publication.problem.as_uuid())
                .bind(publication.version.as_uuid())
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if duplicate_version {
                    return Err(StoreError::AlreadyExists);
                }

                let (public_id, version_number) = match existing_display_identity {
                    Some(identity) => identity,
                    None => {
                        let license =
                            serde_json::to_value(&command.expected_draft.question.metadata.license)
                                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                        let license = license
                            .get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or("other");
                        let value: i64 = sqlx::query_scalar(
                            "INSERT INTO problem \
                     (problem_id, owner_tenant_id, owner_user_id, visibility, license) \
                     VALUES ($1, $2, $3, $4, $5) RETURNING public_id",
                        )
                        .bind(publication.problem.as_uuid())
                        .bind(context.tenant_id().as_uuid())
                        .bind(command.publisher.as_uuid())
                        .bind(publication_scope_name(command.scope))
                        .bind(license)
                        .fetch_one(&mut *transaction)
                        .await
                        .map_err(map_sqlx_error)?;
                        let value = u64::try_from(value).map_err(|_| {
                            StoreError::Unavailable(
                                "stored problem public ID is invalid".to_string(),
                            )
                        })?;
                        (
                            ProblemPublicId::new(value).ok_or_else(|| {
                                StoreError::Unavailable(
                                    "stored problem public ID is invalid".to_string(),
                                )
                            })?,
                            ProblemVersionNumber::new(1).expect("one is positive"),
                        )
                    }
                };

                let published_at_millis: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
                )
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                let question = QuestionDefinition::from_draft(
                    command.expected_draft.question.clone(),
                    publication.problem,
                    publication.version,
                    command.published_source.clone(),
                );
                let record = PublishedProblemRecord {
                    problem: publication.problem,
                    public_id,
                    version: publication.version,
                    version_number,
                    question,
                    capabilities: command.capabilities,
                    scope: command.scope,
                    lifecycle: CatalogLifecycle::Published,
                    authors,
                    previous_version,
                    derived_from,
                    published_at: ActivityTimestamp::from_unix_millis(published_at_millis),
                };
                validate_published(&record)?;
                let (payload, checksum) = encode_payload(&record)?;

                if record.scope == PublicationScope::Institution {
                    sqlx::query(
                        "INSERT INTO catalog_tenant_grant (tenant_id, problem_id, version_id) \
                 VALUES ($1, $2, $3)",
                    )
                    .bind(context.tenant_id().as_uuid())
                    .bind(record.problem.as_uuid())
                    .bind(record.version.as_uuid())
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                }
                insert_problem_version(&mut transaction, &record, &checksum).await?;
                sqlx::query(
                    "INSERT INTO problem_version_payload \
             (problem_id, version_id, payload, payload_sha256) VALUES ($1, $2, $3, $4)",
                )
                .bind(record.problem.as_uuid())
                .bind(record.version.as_uuid())
                .bind(payload)
                .bind(checksum)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                if let Some(artifact) = command.source_artifact {
                    insert_published_source_artifact(&mut transaction, &artifact).await?;
                }
                if let Some(promotion) = command.qti_promotion {
                    for asset in &promotion.assets {
                        insert_catalog_asset_delivery(&mut transaction, asset).await?;
                    }
                    let item_id = qti_item.expect("QTI promotion has an exact staged item");
                    let promoted: bool = sqlx::query_scalar(
                        "SELECT ple_promote_qti_grading($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(context.tenant_id().as_uuid())
                    .bind(promotion.staging.workspace.as_uuid())
                    .bind(promotion.staging.import.as_uuid())
                    .bind(record.problem.as_uuid())
                    .bind(record.version.as_uuid())
                    .bind(item_id)
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if !promoted {
                        return Err(StoreError::Conflict);
                    }
                }
                if let Some(promotion) = command.flat_question_promotion {
                    let (staged, source_payload_checksum) =
                        flat_source.expect("flat promotion has staged source evidence");
                    // Imported publication must take committed import and
                    // current-origin locks before either capability takes the
                    // current-source lock. A later grading refusal rolls this
                    // immutable origin copy back with the whole transaction.
                    if let (Some(current), Some(import_promotion)) = (
                        current_flat_import_origin.as_ref(),
                        promotion.import_origin.as_ref(),
                    ) {
                        let promoted = super::flat_import_provenance::promote_flat_import_origin(
                            &mut transaction,
                            context,
                            actor,
                            staged.workspace,
                            publication,
                            current,
                            import_promotion,
                        )
                        .await?;
                        if !promoted {
                            return Err(StoreError::Conflict);
                        }
                    }
                    let promoted: bool = sqlx::query_scalar(
                        "SELECT ple_promote_flat_question_grading(\
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                    )
                    .bind(context.tenant_id().as_uuid())
                    .bind(staged.workspace.as_uuid())
                    .bind(
                        i64::try_from(staged.workspace_revision.value()).map_err(|_| {
                            StoreError::Unavailable(
                                "workspace draft revision does not fit database integer"
                                    .to_string(),
                            )
                        })?,
                    )
                    .bind(stored_draft_checksum)
                    .bind(staged.source_record.id.as_uuid())
                    .bind(source_payload_checksum)
                    .bind(staged.canonical_source_sha256)
                    .bind(staged.public_binding_sha256)
                    .bind(record.problem.as_uuid())
                    .bind(record.version.as_uuid())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if !promoted {
                        return Err(StoreError::Conflict);
                    }
                }
                sqlx::query(
                    "DELETE FROM workspace_draft WHERE tenant_id = $1 AND workspace_id = $2",
                )
                .bind(context.tenant_id().as_uuid())
                .bind(record.question.workspace.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(record)
            }
        })
        .await
    }

    async fn get_catalog_problem(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn resolve_catalog_problem(
        &self,
        context: TenantContext,
        reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        let requested_version = reference.version.map(|version| i64::from(version.value()));
        let public_id = i64::from(reference.problem.value());
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem AS p \
             JOIN problem_version AS pv USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE p.public_id = $1 \
               AND ($2::bigint IS NULL OR pv.version_number = $2) \
               AND pv.lifecycle IN ('published', 'deprecated') \
             ORDER BY pv.version_number DESC LIMIT 1",
        )
        .bind(public_id)
        .bind(requested_version)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn list_catalog(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT document.problem_id::text || '/' || document.version_id::text AS stable_key, \
                    document.problem_id, document.public_id, document.version_id, \
                    document.version_number, document.backend, document.capabilities, \
                    document.metadata, document.publication_scope, document.lifecycle, \
                    document.lifecycle_reason, document.authors, document.previous_version_id, \
                    document.derived_from_problem_id, document.derived_from_version_id, \
                    floor(extract(epoch FROM document.published_at) * 1000)::bigint \
                        AS published_at_millis \
             FROM catalog_search_document AS document \
             WHERE document.lifecycle = 'published' \
               AND ($1::text IS NULL \
                    OR document.problem_id::text || '/' || document.version_id::text > $1) \
             ORDER BY document.problem_id::text, document.version_id::text LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = catalog_summary_page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn list_catalog_taxonomy(
        &self,
        context: TenantContext,
        page: PageRequest,
    ) -> Result<Page<TaxonomyTerm>, StoreError> {
        let cursor = page.after.as_ref().map(|value| value.as_str().to_string());
        let limit = i64::from(page.size.get()) + 1;
        let mut transaction = self.begin_tenant(context).await?;
        let rows = sqlx::query(
            "SELECT stable_key, taxonomy_term \
             FROM ( \
                 SELECT DISTINCT ON (term_row.stable_key) \
                        term_row.stable_key, term_row.taxonomy_term \
                 FROM ( \
                     SELECT document.problem_id, document.version_id, \
                            encode(convert_to(term->>'scheme', 'UTF8'), 'hex') || '/' || \
                            encode(convert_to(term->>'code', 'UTF8'), 'hex') AS stable_key, \
                            term AS taxonomy_term \
                     FROM catalog_search_document AS document \
                     CROSS JOIN LATERAL jsonb_array_elements(document.taxonomy) AS term \
                     WHERE document.lifecycle = 'published' \
                 ) AS term_row \
                 ORDER BY term_row.stable_key, term_row.problem_id::text, \
                          term_row.version_id::text \
             ) AS distinct_term \
             WHERE $1::text IS NULL OR stable_key > $1 \
             ORDER BY stable_key LIMIT $2",
        )
        .bind(cursor)
        .bind(limit)
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = taxonomy_page_from_rows(rows, page.size.get())?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn search_catalog(
        &self,
        context: TenantContext,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        search::search_catalog(self, context, query).await
    }
    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        retry_transaction(|| async move {
        // Keep the authored prompt and its safe aggregate projection in one
        // tenant-scoped snapshot. The statistics statement calls only the
        // k-gated reader; it never joins catalog payload or learner history.
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let record = row.as_ref().map(decode_catalog_payload_row).transpose()?;
        let statistics = if record.is_some() {
            let row = sqlx::query(
                "SELECT cohort_size, difficulty_index, attempts_mean, time_median_seconds_estimate, \
                        discrimination_index \
                 FROM ple_question_statistics_view($1, $2)",
            )
            .bind(reference.problem.as_uuid())
            .bind(reference.version.as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            question_statistics_disclosure_from_row(row.as_ref())?
        } else {
            QuestionStatisticsDisclosure::Suppressed
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record.map(|record| CatalogProblemDetail {
            summary: record.summary(),
            prompt: record.question.prompt,
            statistics: match statistics {
                QuestionStatisticsDisclosure::Suppressed => {
                    question_model::CatalogStatisticsStatus::Unavailable
                }
                QuestionStatisticsDisclosure::Available(view) => {
                    question_model::CatalogStatisticsStatus::Available(view)
                }
            },
        }))
        })
        .await
    }

    async fn transition_catalog_problem(
        &self,
        context: TenantContext,
        actor: UserId,
        reference: ProblemVersionRef,
        transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.public_id, pv.version_id, pv.version_number, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason \
             FROM problem_version AS pv \
             JOIN problem AS p USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE pv.problem_id = $1 AND pv.version_id = $2 \
               AND p.owner_tenant_id = $3 FOR UPDATE OF pv",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(context.tenant_id().as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let mut record = decode_catalog_payload_row(&row)?;
        if !record.authors.contains(&actor) {
            return Err(StoreError::Forbidden);
        }
        record.lifecycle = match (&record.lifecycle, transition) {
            (CatalogLifecycle::Published, CatalogTransition::Deprecate { reason }) => {
                CatalogLifecycle::Deprecated {
                    reason: validated_deprecation_reason(reason)?,
                }
            }
            (CatalogLifecycle::Deprecated { reason }, CatalogTransition::Archive) => {
                CatalogLifecycle::Archived {
                    reason: reason.clone(),
                }
            }
            _ => {
                return Err(StoreError::InvalidRecord(
                    "catalog lifecycle transition is not allowed".to_string(),
                ));
            }
        };
        let (lifecycle, lifecycle_reason) = catalog_lifecycle_parts(&record.lifecycle);
        let updated = sqlx::query(
            "UPDATE problem_version SET lifecycle = $3, lifecycle_reason = $4 \
             WHERE problem_id = $1 AND version_id = $2 \
               AND EXISTS ( \
                   SELECT 1 FROM problem AS owner_problem \
                    WHERE owner_problem.problem_id = problem_version.problem_id \
                      AND owner_problem.owner_tenant_id = $5 \
               )",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(lifecycle)
        .bind(lifecycle_reason)
        .bind(context.tenant_id().as_uuid())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::NotFound);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }
}

#[async_trait]
impl CatalogSourceStore for PostgresStore {
    async fn catalog_source_artifact(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Option<PublishedSourceArtifact>, StoreError> {
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT backend, payload, payload_sha256 FROM published_source_artifact \
             WHERE problem_id = $1 AND version_id = $2",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let artifact: Option<PublishedSourceArtifact> =
            row.as_ref().map(decode_payload_row).transpose()?;
        if let Some(ref artifact) = artifact {
            let stored_backend: String = row
                .as_ref()
                .expect("artifact row exists when payload decoded")
                .get("backend");
            if stored_backend != question_backend_name(artifact.backend) {
                return Err(StoreError::InvalidRecord(
                    "stored source artifact backend does not match its payload".to_string(),
                ));
            }
            validate_source_artifact_identity(reference, artifact.backend, artifact)?;
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(artifact)
    }
}
