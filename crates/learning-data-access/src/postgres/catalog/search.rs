//! Tenant-scoped catalog search and facet queries.

use question_model::{
    CatalogSearchFacets, CatalogSearchPage, CatalogSearchQuery, CatalogStatisticsAvailability,
    CatalogStatisticsFacet,
};
use sqlx::Row;
use sqlx::types::Json;

use super::super::connection::{map_sqlx_error, retry_transaction};
use super::super::{
    PostgresStore, catalog_summary_page_from_rows, decode_catalog_capability_facet,
    decode_catalog_license_facet, decode_catalog_taxonomy_facet,
    postgres_catalog_search_fingerprint, postgres_search_page_request,
};
use crate::{
    StoreError, TenantContext, decode_catalog_search_cursor, encode_catalog_search_cursor,
};

/// Searches the tenant-visible published catalog and derives all facets from the same snapshot.
pub(super) async fn search_catalog(
    store: &PostgresStore,
    context: TenantContext,
    query: CatalogSearchQuery,
) -> Result<CatalogSearchPage, StoreError> {
    let query = query
        .normalized()
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let exact_question_id = query
        .exact_question_id()
        .filter(|question_id| store.question_ids.validates(question_id))
        .map(|question_id| question_id.compact());
    retry_transaction(|| {
        let query = query.clone();
        let exact_question_id = exact_question_id.clone();
        async move {
            let page = postgres_search_page_request(&query)?;
            let fingerprint = postgres_catalog_search_fingerprint(&query);
            let after = page
                .after
                .as_ref()
                .map(|cursor| decode_catalog_search_cursor(cursor.as_str(), &fingerprint))
                .transpose()?;
            let (after_problem, after_version) = after
                .map(|(problem, version)| (Some(problem), Some(version)))
                .unwrap_or((None, None));
            let text = query.text.clone();
            let taxonomy = Json(query.taxonomy.clone());
            let capabilities = Json(query.capabilities.clone());
            let licenses = Json(query.licenses.clone());
            let statistics = match query.statistics {
                CatalogStatisticsAvailability::Any => 0_i16,
                CatalogStatisticsAvailability::Available => 1_i16,
                CatalogStatisticsAvailability::Unavailable => 2_i16,
            };
            let limit = i64::from(page.size.get()) + 1;
            let mut transaction = store.begin_tenant_snapshot(context).await?;
            // All statements below remain in this one tenant-scoped transaction.
            // PostgreSQL's RLS visibility applies before these predicates; no
            // caller-provided tenant ID or payload join can widen the result.
            let rows = sqlx::query(
                "SELECT document.problem_id::text || '/' || document.version_id::text AS stable_key, \
                        document.problem_id, document.question_id, document.version_id, \
                        document.version_number, document.backend, document.capabilities, \
                        document.metadata, document.publication_scope, document.lifecycle, \
                        document.lifecycle_reason, document.authors, document.previous_version_id, \
                        document.derived_from_problem_id, document.derived_from_version_id, \
                        floor(extract(epoch FROM document.published_at) * 1000)::bigint \
                            AS published_at_millis \
                 FROM catalog_search_view AS document \
                 WHERE document.lifecycle = 'published' \
                   AND ( \
                       ($9::text IS NOT NULL AND document.question_id = $9::text) \
                       OR ($9::text IS NULL AND ($1::text IS NULL \
                           OR document.search_text @@ websearch_to_tsquery('simple', $1))) \
                   ) \
                   AND NOT EXISTS ( \
                       SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
                       WHERE NOT EXISTS ( \
                           SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored \
                           WHERE stored->>'scheme' = wanted->>'scheme' \
                           AND stored->>'code' = wanted->>'code' \
                       ) \
                   ) \
                   AND document.capabilities @> $3::jsonb \
                   AND (jsonb_array_length($4::jsonb) = 0 OR document.license \
                        IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                   AND ($5::smallint <> 1 OR document.statistics_available) \
                   AND ($5::smallint <> 2 OR NOT document.statistics_available) \
                   AND ($6::uuid IS NULL OR (document.problem_id, document.version_id) > ($6, $7)) \
                 ORDER BY document.problem_id, document.version_id LIMIT $8",
            )
            .bind(text.clone())
            .bind(taxonomy.clone())
            .bind(capabilities.clone())
            .bind(licenses.clone())
            .bind(statistics)
            .bind(after_problem)
            .bind(after_version)
            .bind(limit)
            .bind(exact_question_id.clone())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let page_result = catalog_summary_page_from_rows(rows, page.size.get())?;
            let taxonomy_rows = sqlx::query(
                "WITH filtered AS ( \
                     SELECT document.metadata FROM catalog_search_view AS document \
                     WHERE document.lifecycle = 'published' \
                       AND (($6::text IS NOT NULL AND document.question_id = $6::text) \
                            OR ($6::text IS NULL AND ($1::text IS NULL OR document.search_text \
                             @@ websearch_to_tsquery('simple', $1)))) \
                       AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
                           WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements( \
                               document.taxonomy) AS stored \
                               WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
                       AND document.capabilities @> $3::jsonb \
                       AND (jsonb_array_length($4::jsonb) = 0 OR document.license \
                            IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                       AND ($5::smallint <> 1 OR document.statistics_available) \
                       AND ($5::smallint <> 2 OR NOT document.statistics_available) \
                 ) SELECT jsonb_build_object('scheme', term->>'scheme', 'code', term->>'code', \
                             'label', min(term->>'label')) AS taxonomy_term, count(*)::bigint AS facet_count \
                   FROM filtered CROSS JOIN LATERAL jsonb_array_elements( \
                       CASE WHEN jsonb_typeof(metadata->'taxonomy') = 'array' \
                            THEN metadata->'taxonomy' ELSE '[]'::jsonb END) AS term \
                   GROUP BY term->>'scheme', term->>'code' \
                   ORDER BY count(*) DESC, term->>'scheme', term->>'code' LIMIT 64",
            )
            .bind(text.clone())
            .bind(taxonomy.clone())
            .bind(capabilities.clone())
            .bind(licenses.clone())
            .bind(statistics)
            .bind(exact_question_id.clone())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let capability_rows = sqlx::query(
                "WITH filtered AS (SELECT document.capabilities FROM catalog_search_view AS document \
                   WHERE document.lifecycle = 'published' \
                   AND (($6::text IS NOT NULL AND document.question_id = $6::text) \
                        OR ($6::text IS NULL AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1))) ) \
                   AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
                   AND document.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                   AND ($5::smallint <> 1 OR document.statistics_available) \
                   AND ($5::smallint <> 2 OR NOT document.statistics_available)) \
                 SELECT capability, count(*)::bigint AS facet_count FROM filtered CROSS JOIN LATERAL jsonb_array_elements_text(capabilities) AS capability GROUP BY capability ORDER BY capability",
            )
            .bind(text.clone())
            .bind(taxonomy.clone())
            .bind(capabilities.clone())
            .bind(licenses.clone())
            .bind(statistics)
            .bind(exact_question_id.clone())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let license_rows = sqlx::query(
                "WITH filtered AS (SELECT document.license FROM catalog_search_view AS document \
                   WHERE document.lifecycle = 'published' \
                   AND (($6::text IS NOT NULL AND document.question_id = $6::text) \
                        OR ($6::text IS NULL AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1))) ) \
                   AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
                   AND document.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                   AND ($5::smallint <> 1 OR document.statistics_available) \
                   AND ($5::smallint <> 2 OR NOT document.statistics_available)) \
                 SELECT license, count(*)::bigint AS facet_count FROM filtered GROUP BY license ORDER BY license",
            )
            .bind(text.clone())
            .bind(taxonomy.clone())
            .bind(capabilities.clone())
            .bind(licenses.clone())
            .bind(statistics)
            .bind(exact_question_id.clone())
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            let statistics_facet = sqlx::query(
                "SELECT count(*) FILTER (WHERE document.statistics_available)::bigint AS available, \
                        count(*) FILTER (WHERE NOT document.statistics_available)::bigint AS unavailable \
                 FROM catalog_search_view AS document \
                 WHERE document.lifecycle = 'published' \
                 AND (($6::text IS NOT NULL AND document.question_id = $6::text) \
                      OR ($6::text IS NULL AND ($1::text IS NULL OR document.search_text @@ websearch_to_tsquery('simple', $1))) ) \
                 AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
                 AND document.capabilities @> $3::jsonb AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
                 AND ($5::smallint <> 1 OR document.statistics_available) \
                 AND ($5::smallint <> 2 OR NOT document.statistics_available)",
            )
            .bind(text)
            .bind(taxonomy)
            .bind(capabilities)
            .bind(licenses)
            .bind(statistics)
            .bind(exact_question_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(CatalogSearchPage {
                items: page_result.items,
                next_cursor: page_result.next_cursor.map(|cursor| {
                    let (problem, version) = cursor
                        .as_str()
                        .split_once('/')
                        .expect("catalog stable key");
                    encode_catalog_search_cursor(
                        &fingerprint,
                        problem.parse().expect("catalog problem UUID"),
                        version.parse().expect("catalog version UUID"),
                    )
                }),
                facets: CatalogSearchFacets {
                    taxonomy: taxonomy_rows
                        .into_iter()
                        .map(decode_catalog_taxonomy_facet)
                        .collect::<Result<_, _>>()?,
                    capabilities: capability_rows
                        .into_iter()
                        .map(decode_catalog_capability_facet)
                        .collect::<Result<_, _>>()?,
                    licenses: license_rows
                        .into_iter()
                        .map(decode_catalog_license_facet)
                        .collect::<Result<_, _>>()?,
                    statistics: CatalogStatisticsFacet {
                        available: u64::try_from(
                            statistics_facet
                                .try_get::<i64, _>("available")
                                .map_err(map_sqlx_error)?,
                        )
                        .map_err(|_| {
                            StoreError::Unavailable("catalog count overflow".to_string())
                        })?,
                        unavailable: u64::try_from(
                            statistics_facet
                                .try_get::<i64, _>("unavailable")
                                .map_err(map_sqlx_error)?,
                        )
                        .map_err(|_| {
                            StoreError::Unavailable("catalog count overflow".to_string())
                        })?,
                    },
                },
            })
        }
    })
    .await
}
