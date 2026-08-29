//! PostgreSQL catalog publication, discovery, and protected source access.

mod search;

use async_trait::async_trait;
use question_model::taxonomy::TaxonomyTerm;
use question_model::{
    ActivityTimestamp, CatalogDiscoveryEvidence, CatalogLifecycle, CatalogOwnCourseUsage,
    CatalogProblemDetail, CatalogProblemSummary, CatalogSearchPage, CatalogSearchQuery,
    CatalogUsageDetail, DraftQuestionSource, MAX_CATALOG_OWN_COURSE_USAGES, ProblemVersionRef,
    PublicationScope, QuestionDefinition, UserId,
};
use serde_json::Value;
use sqlx::Row;
use sqlx::types::Uuid;

use super::connection::{map_sqlx_error, retry_transaction};
use super::{
    PostgresStore, catalog_lifecycle_parts, catalog_summary_page_from_rows,
    decode_catalog_discovery_evidence_row, decode_catalog_own_course_usage_row,
    decode_catalog_payload_row, decode_catalog_usage_summary_row, decode_payload_row,
    decode_payload_row_named, encode_payload, insert_catalog_asset_delivery,
    insert_problem_version, insert_published_source_artifact, publication_scope_name,
    question_backend_name, taxonomy_page_from_rows, validated_deprecation_reason,
};
use crate::{
    CatalogSourceStore, CatalogStore, CatalogTransition, DraftRecord, Page, PageRequest,
    PublishDraftCommand, PublishedProblemRecord, PublishedSourceArtifact, QtiImportRegistry,
    SessionTokenHash, StoreError, TenantContext, WorkspaceDraftRevision,
    WorkspaceFlatQuestionSource, ensure_tenant, validate_draft, validate_flat_question_publication,
    validate_publication_source, validate_published, validate_qti_publication_promotion,
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
                let has_pending_public_assets = command.scope == PublicationScope::Public
                    && (qti_promotion.is_some_and(|promotion| !promotion.assets.is_empty())
                        || flat_promotion.is_some_and(|promotion| !promotion.assets.is_empty()));

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
                let author_ids = vec![command.publisher];
                let derived_from = command.expected_draft.derived_from;
                if let Some(source) = derived_from {
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

                let question_id = {
                        let license =
                            serde_json::to_value(&command.expected_draft.question.metadata.license)
                                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                        let license = license
                            .get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or("other");
                        let reserved: Option<i64> = sqlx::query_scalar(
                            "UPDATE question_id_namespace SET issued_count = issued_count + 1 \
                             WHERE singleton AND issued_count < $1 RETURNING issued_count",
                        )
                        .bind(question_model::MAX_QUESTION_ID_COUNT as i64)
                        .fetch_optional(&mut *transaction)
                        .await
                        .map_err(map_sqlx_error)?;
                        if reserved.is_none() {
                            return Err(StoreError::Unavailable(
                                "Question ID product limit reached".to_string(),
                            ));
                        }
                        let mut issued_question_id = None;
                        for _ in 0..64 {
                            let question_id = self.question_ids.issue()?;
                            let inserted_problem: Option<Uuid> = sqlx::query_scalar(
                                "INSERT INTO problem \
                         (problem_id, question_id, owner_tenant_id, owner_user_id, visibility, license) \
                         VALUES ($1, $2, $3, $4, $5, $6) \
                         ON CONFLICT (question_id) DO NOTHING RETURNING problem_id",
                            )
                            .bind(publication.problem.as_uuid())
                            .bind(question_id.compact())
                            .bind(context.tenant_id().as_uuid())
                            .bind(command.publisher.as_uuid())
                            .bind(publication_scope_name(command.scope))
                            .bind(license)
                            .fetch_optional(&mut *transaction)
                            .await
                            .map_err(map_sqlx_error)?;
                            if inserted_problem == Some(publication.problem.as_uuid()) {
                                issued_question_id = Some(question_id);
                                break;
                            }
                        }
                        issued_question_id.ok_or_else(|| {
                            StoreError::Unavailable(
                                "Question ID collision retry exhausted".to_string(),
                            )
                        })?
                };

                let published_at_millis: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
                )
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
                let published_draft_question = command
                    .flat_question_promotion
                    .as_ref()
                    .map(|promotion| promotion.published_question.clone())
                    .unwrap_or_else(|| command.expected_draft.question.clone());
                let question = QuestionDefinition::from_draft(
                    published_draft_question,
                    publication.problem,
                    publication.version,
                    command.published_source.clone(),
                );
                let record = PublishedProblemRecord {
                    problem: publication.problem,
                    question_id,
                    version: publication.version,
                    question,
                    capabilities: command.capabilities,
                    scope: command.scope,
                    lifecycle: CatalogLifecycle::Published,
                    author_ids,
                    byline: command.byline.clone(),
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
                    for asset in &promotion.assets {
                        insert_catalog_asset_delivery(&mut transaction, asset).await?;
                    }
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
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
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
                    .bind(
                        grading::flat_question::public_binding_sha256_for_draft(
                            &promotion.published_question,
                        )
                        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?,
                    )
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    if !promoted {
                        return Err(StoreError::Conflict);
                    }
                }
                if has_pending_public_assets {
                    let job = crate::JobId::generate()?;
                    let payload = serde_json::to_value(crate::JobPayload::PublishPublicAssets {
                        reference: publication,
                    })
                    .map_err(|error| {
                        StoreError::InvalidRecord(format!(
                            "public-asset publisher payload serialization failed: {error}"
                        ))
                    })?;
                    sqlx::query(
                        "INSERT INTO worker_job (job_id, tenant_id, payload, state, max_attempts) \
                         VALUES ($1, $2, $3, 'ready', 20)",
                    )
                    .bind(job.as_uuid())
                    .bind(context.tenant_id().as_uuid())
                    .bind(payload)
                    .execute(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
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
            "SELECT pv.problem_id, p.question_id, pv.version_id, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason, pv.author_ids, pv.public_byline \
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
        if !self.question_ids.validates(&reference.question_id) {
            return Ok(None);
        }
        let question_id = reference.question_id.compact();
        let mut transaction = self.begin_tenant(context).await?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.question_id, pv.version_id, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason, pv.author_ids, pv.public_byline \
             FROM problem AS p \
             JOIN problem_version AS pv USING (problem_id) \
             JOIN problem_version_payload AS pvp USING (problem_id, version_id) \
             WHERE p.question_id = $1 \
               AND pv.lifecycle IN ('published', 'deprecated', 'archived')",
        )
        .bind(question_id)
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
            "SELECT document.question_id AS stable_key, document.question_id, \
                    document.backend, document.response_family, document.capabilities, \
                    document.metadata, document.publication_scope, document.lifecycle, \
                    document.lifecycle_reason, \
                    floor(extract(epoch FROM document.published_at) * 1000)::bigint \
                        AS published_at_millis \
             FROM catalog_search_document AS document \
             WHERE document.lifecycle IN ('published', 'deprecated', 'archived') \
               AND ($1::text IS NULL \
                    OR document.question_id > $1) \
             ORDER BY document.question_id LIMIT $2",
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
                     WHERE document.lifecycle IN ('published', 'deprecated', 'archived') \
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
        session: SessionTokenHash,
        query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        search::search_catalog(self, context, session, query).await
    }
    async fn get_catalog_detail(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        reference: ProblemVersionRef,
    ) -> Result<Option<CatalogProblemDetail>, StoreError> {
        retry_transaction(|| async move {
        let mut transaction = self.begin_tenant_snapshot(context).await?;
        // ASVS 8.2.2 and 8.3.3: the SQL capability resolves the presented
        // session instead of accepting a browser-provided actor identity.
        sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
            .bind(session.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = sqlx::query(
            "SELECT pv.problem_id, p.question_id, pv.version_id, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason, pv.author_ids, pv.public_byline \
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
        let Some(record) = record else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        let evidence_boundary: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(evidence_sequence), 0) \
             FROM catalog_discovery_evidence_revision",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let evidence_row = sqlx::query(
            "SELECT evidence_sequence IS NOT NULL AS evidence_visible, formula_version, \
                    course_count, first_attempt_count, difficulty_index, attempts_mean, \
                    time_median_seconds_estimate, discrimination_index, \
                    floor(extract(epoch FROM evidence_at) * 1000)::bigint AS evidence_at_millis \
             FROM ple_catalog_discovery_evidence_at($1, $2, $3)",
        )
        .bind(reference.problem.as_uuid())
        .bind(reference.version.as_uuid())
        .bind(evidence_boundary)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let evidence = match evidence_row.as_ref() {
            Some(row) => decode_catalog_discovery_evidence_row(row)?,
            None => CatalogDiscoveryEvidence::InsufficientEvidence,
        };
        let question_id = record.question_id.compact();
        // ASVS 1.2.4: typed query bindings keep tenant, session, and public
        // Question ID values out of SQL syntax and the returned DTO omits IDs.
        let usage_row = sqlx::query(
            "SELECT institution_course_count, institution_assignment_count, \
                    own_course_count, own_assignment_count \
             FROM ple_instructor_catalog_usage_summary($1, $2, $3)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .bind(&question_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let usage_summary = decode_catalog_usage_summary_row(&usage_row)?;
        let own_course_rows = sqlx::query(
            "SELECT course_reference, course_title, assignment_count \
             FROM ple_instructor_catalog_course_usage($1, $2, $3, $4, $5)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(session.to_string())
        .bind(&question_id)
        .bind(None::<i32>)
        .bind(i32::try_from(MAX_CATALOG_OWN_COURSE_USAGES).expect("catalog usage limit fits i32"))
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let own_courses = own_course_rows
            .iter()
            .map(decode_catalog_own_course_usage_row)
            .collect::<Result<Vec<CatalogOwnCourseUsage>, StoreError>>()?;
        let own_courses_truncated = usage_summary.own_course_count
            > u64::try_from(own_courses.len()).expect("catalog usage length fits u64");
        let prompt = crate::catalog_prompt::catalog_prompt_projection(&record.question)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(CatalogProblemDetail {
            summary: record.summary(),
            prompt,
            evidence,
            usage: CatalogUsageDetail {
                summary: usage_summary,
                own_courses,
                own_courses_truncated,
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
            "SELECT pv.problem_id, p.question_id, pv.version_id, \
                    pvp.payload, pvp.payload_sha256, pv.lifecycle, pv.lifecycle_reason, pv.author_ids, pv.public_byline \
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
        if !record.author_ids.contains(&actor) {
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
