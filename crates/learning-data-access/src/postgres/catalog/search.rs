//! Tenant-scoped ranked catalog search and snapshot facets.

use question_model::{
    Capability, CatalogLicenseValue, CatalogSearchFacets, CatalogSearchPage, CatalogSearchQuery,
    CatalogStatisticsAvailability, CatalogStatisticsFacet, CatalogTaxonomyFilter,
};
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, Row};

use super::super::connection::{map_sqlx_error, retry_transaction};
use super::super::{
    PostgresStore, catalog_summary_page_from_rows, decode_catalog_capability_facet,
    decode_catalog_license_facet, decode_catalog_taxonomy_facet,
    postgres_catalog_search_fingerprint, postgres_search_page_request,
};
use crate::{
    StoreError, TenantContext, decode_catalog_search_cursor, encode_catalog_search_cursor,
};

const TRIGRAM_THRESHOLD: &str = "0.30";

// Keep the predicate/rank projection literally shared by page rows and every
// facet family.  `$10` is the page-one event boundary; `$6..$9` are only used
// by the row keyset, so facets intentionally observe the complete snapshot.
const RANKED_CTE: &str = "WITH ranked AS ( \
    SELECT document.*, \
      (document.statistics_available AND COALESCE(document.statistics_disclosed_sequence <= $10, false)) AS statistics_visible, \
      CASE WHEN $11::text IS NOT NULL THEN 9223372036854775807::bigint \
           WHEN $1::text IS NULL THEN 0::bigint \
           ELSE floor(ts_rank_cd(document.search_text, websearch_to_tsquery('simple', $1)) * 1000000)::bigint END AS full_text_rank, \
      CASE WHEN $11::text IS NOT NULL THEN 9223372036854775807::bigint \
           WHEN $1::text IS NULL THEN 0::bigint \
           ELSE floor(word_similarity(lower($1), document.normalized_search_text) * 1000000)::bigint END AS similarity_score \
    FROM catalog_search_view AS document \
    WHERE document.lifecycle = 'published' \
      AND document.catalog_sequence <= $10 \
      AND (($11::text IS NOT NULL AND document.question_id = $11::text) \
        OR ($11::text IS NULL AND ($1::text IS NULL \
          OR document.search_text @@ websearch_to_tsquery('simple', $1) \
          OR lower($1) <% document.normalized_search_text))) \
      AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($2::jsonb) AS wanted \
        WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored \
          WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
      AND document.capabilities @> $3::jsonb \
      AND (jsonb_array_length($4::jsonb) = 0 OR document.license IN (SELECT jsonb_array_elements_text($4::jsonb))) \
      AND ($5::smallint <> 1 OR document.statistics_available AND COALESCE(document.statistics_disclosed_sequence <= $10, false)) \
      AND ($5::smallint <> 2 OR NOT (document.statistics_available AND COALESCE(document.statistics_disclosed_sequence <= $10, false))) \
) ";

/// Searches tenant-visible published content.  The cursor binds the first page
/// event boundary; lifecycle/RLS still evaluate at each request deliberately.
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
            let after = page.after.as_ref()
                .map(|cursor| decode_catalog_search_cursor(&store.catalog_cursors, cursor.as_str(), &fingerprint))
                .transpose()?;
            let (cursor_boundary, after_rank, after_similarity, after_problem, after_version) = after
                .map(|(boundary, rank, similarity, problem, version)| (Some(boundary), Some(rank), Some(similarity), Some(problem), Some(version)))
                .unwrap_or((None, None, None, None, None));
            let text = query.text.clone();
            let taxonomy = Json(query.taxonomy.clone());
            let capabilities = Json(query.capabilities.clone());
            let licenses = Json(query.licenses.clone());
            let statistics = match query.statistics {
                CatalogStatisticsAvailability::Any => 0_i16,
                CatalogStatisticsAvailability::Available => 1_i16,
                CatalogStatisticsAvailability::Unavailable => 2_i16,
            };
            let mut transaction = store.begin_tenant_snapshot(context).await?;
            // `<%` is the index-supported word-similarity admission operator.
            // Its GUC default is session mutable, so pin the product rule.
            sqlx::query("SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)")
                .bind(TRIGRAM_THRESHOLD).execute(&mut *transaction).await.map_err(map_sqlx_error)?;
            let boundary = match cursor_boundary {
                Some(boundary) => i64::try_from(boundary).map_err(|_| StoreError::InvalidRecord("catalog cursor has an invalid snapshot boundary".to_string()))?,
                None => sqlx::query_scalar::<_, i64>(
                    "SELECT GREATEST(COALESCE(max(catalog_sequence), 0), COALESCE(max(statistics_disclosed_sequence), 0)) FROM catalog_search_view",
                ).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?,
            };
            let limit = i64::from(page.size.get()) + 1;
            let rows_sql = format!("{RANKED_CTE} SELECT full_text_rank::text || '/' || similarity_score::text || '/' || problem_id::text || '/' || version_id::text AS stable_key, question_id, backend, capabilities, metadata, publication_scope, lifecycle, lifecycle_reason, public_byline, floor(extract(epoch FROM published_at) * 1000)::bigint AS published_at_millis FROM ranked WHERE ($6::bigint IS NULL OR full_text_rank < $6 OR (full_text_rank = $6 AND (similarity_score < $7 OR (similarity_score = $7 AND (problem_id, version_id) > ($8, $9))))) ORDER BY full_text_rank DESC, similarity_score DESC, problem_id, version_id LIMIT $12");
            let rows = bind_catalog(sqlx::query(AssertSqlSafe(rows_sql)), text.clone(), taxonomy.clone(), capabilities.clone(), licenses.clone(), statistics, after_rank, after_similarity, after_problem, after_version, boundary, exact_question_id.clone(), limit)
                .fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
            let page_result = catalog_summary_page_from_rows(rows, page.size.get())?;

            let taxonomy_sql = format!("{RANKED_CTE} SELECT jsonb_build_object('scheme', term->>'scheme', 'code', term->>'code', 'label', min(term->>'label')) AS taxonomy_term, count(*)::bigint AS facet_count FROM ranked CROSS JOIN LATERAL jsonb_array_elements(CASE WHEN jsonb_typeof(metadata->'taxonomy') = 'array' THEN metadata->'taxonomy' ELSE '[]'::jsonb END) AS term GROUP BY term->>'scheme', term->>'code' ORDER BY count(*) DESC, term->>'scheme', term->>'code' LIMIT 64");
            let taxonomy_rows = bind_catalog(sqlx::query(AssertSqlSafe(taxonomy_sql)), text.clone(), taxonomy.clone(), capabilities.clone(), licenses.clone(), statistics, after_rank, after_similarity, after_problem, after_version, boundary, exact_question_id.clone(), limit).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
            let capability_sql = format!("{RANKED_CTE} SELECT capability, count(*)::bigint AS facet_count FROM ranked CROSS JOIN LATERAL jsonb_array_elements_text(capabilities) AS capability GROUP BY capability ORDER BY capability");
            let capability_rows = bind_catalog(sqlx::query(AssertSqlSafe(capability_sql)), text.clone(), taxonomy.clone(), capabilities.clone(), licenses.clone(), statistics, after_rank, after_similarity, after_problem, after_version, boundary, exact_question_id.clone(), limit).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
            let license_sql = format!("{RANKED_CTE} SELECT license, count(*)::bigint AS facet_count FROM ranked GROUP BY license ORDER BY license");
            let license_rows = bind_catalog(sqlx::query(AssertSqlSafe(license_sql)), text.clone(), taxonomy.clone(), capabilities.clone(), licenses.clone(), statistics, after_rank, after_similarity, after_problem, after_version, boundary, exact_question_id.clone(), limit).fetch_all(&mut *transaction).await.map_err(map_sqlx_error)?;
            let statistics_sql = format!("{RANKED_CTE} SELECT count(*) FILTER (WHERE statistics_visible)::bigint AS available, count(*) FILTER (WHERE NOT statistics_visible)::bigint AS unavailable FROM ranked");
            let statistics_facet = bind_catalog(sqlx::query(AssertSqlSafe(statistics_sql)), text, taxonomy, capabilities, licenses, statistics, after_rank, after_similarity, after_problem, after_version, boundary, exact_question_id, limit).fetch_one(&mut *transaction).await.map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(CatalogSearchPage {
                items: page_result.items,
                next_cursor: page_result.next_cursor.map(|cursor| {
                    let mut parts = cursor.as_str().split('/');
                    let rank = parts.next().expect("catalog rank").parse().expect("catalog rank integer");
                    let similarity = parts.next().expect("catalog similarity").parse().expect("catalog similarity integer");
                    let problem = parts.next().expect("catalog problem UUID").parse().expect("catalog problem UUID");
                    let version = parts.next().expect("catalog version UUID").parse().expect("catalog version UUID");
                    encode_catalog_search_cursor(&store.catalog_cursors, &fingerprint, u64::try_from(boundary).expect("nonnegative catalog boundary"), rank, similarity, problem, version)
                }),
                facets: CatalogSearchFacets {
                    taxonomy: taxonomy_rows.into_iter().map(decode_catalog_taxonomy_facet).collect::<Result<_, _>>()?,
                    capabilities: capability_rows.into_iter().map(decode_catalog_capability_facet).collect::<Result<_, _>>()?,
                    licenses: license_rows.into_iter().map(decode_catalog_license_facet).collect::<Result<_, _>>()?,
                    statistics: CatalogStatisticsFacet {
                        available: u64::try_from(statistics_facet.try_get::<i64, _>("available").map_err(map_sqlx_error)?).map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
                        unavailable: u64::try_from(statistics_facet.try_get::<i64, _>("unavailable").map_err(map_sqlx_error)?).map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
                    },
                },
            })
        }
    }).await
}

#[allow(clippy::too_many_arguments)]
fn bind_catalog<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    text: Option<String>,
    taxonomy: Json<Vec<CatalogTaxonomyFilter>>,
    capabilities: Json<Vec<Capability>>,
    licenses: Json<Vec<CatalogLicenseValue>>,
    statistics: i16,
    rank: Option<i64>,
    similarity: Option<i64>,
    problem: Option<uuid::Uuid>,
    version: Option<uuid::Uuid>,
    boundary: i64,
    exact_question_id: Option<String>,
    limit: i64,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    query
        .bind(text)
        .bind(taxonomy)
        .bind(capabilities)
        .bind(licenses)
        .bind(statistics)
        .bind(rank)
        .bind(similarity)
        .bind(problem)
        .bind(version)
        .bind(boundary)
        .bind(exact_question_id)
        .bind(limit)
}
