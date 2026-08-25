//! Tenant-scoped ranked catalog search and immutable facet snapshots.

use std::fmt::Write;

use question_model::{
    Capability, CatalogAuthorship, CatalogBackendFacet, CatalogBylineFacet,
    CatalogEvidenceAvailability, CatalogEvidenceFacet, CatalogLicenseValue, CatalogResponseFamily,
    CatalogResponseFamilyFacet, CatalogSearchFacets, CatalogSearchPage, CatalogSearchQuery,
    CatalogTagFacet, CatalogTaxonomyFilter, CatalogUsedInMyCourses, CatalogUsedInMyCoursesFacet,
    MAX_CATALOG_BACKEND_FACETS, MAX_CATALOG_BYLINE_FACETS, MAX_CATALOG_RESPONSE_FAMILY_FACETS,
    MAX_CATALOG_TAG_FACETS, QuestionBackend,
};
use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, Row};
use uuid::Uuid;

use super::super::connection::{map_sqlx_error, retry_transaction};
use super::super::{
    PostgresStore, catalog_discovery_item_page_from_rows, decode_catalog_capability_facet,
    decode_catalog_license_facet, decode_catalog_taxonomy_facet, parse_question_backend,
    postgres_catalog_search_fingerprint, postgres_search_page_request,
};
use crate::{
    CatalogSearchCursorKey, SessionTokenHash, StoreError, TenantContext,
    decode_catalog_search_cursor, encode_catalog_search_cursor,
};

const TRIGRAM_THRESHOLD: &str = "0.30";
const USAGE_SNAPSHOT_TTL_SECONDS: i32 = 900;
const MAX_USAGE_SNAPSHOT_ROWS: i32 = 5_000;

// `used_publications` is an opaque, actor-authorized reverse index loaded
// from the broker snapshot. The document query never learns a course identity.
// `$16` fixes publication/evidence history, `$18` supplies the actor snapshot,
// `$19` is the closed authorship scope, `$20` is the authenticated actor, and
// `$11..$15` are only the row keyset values.
const RANKED_CTE: &str = "WITH used_publications AS ( \
    SELECT DISTINCT usage.problem_id, usage.version_id \
      FROM jsonb_to_recordset($18::jsonb) AS usage(problem_id uuid, version_id uuid) \
), ranked AS ( \
    SELECT document.*, \
      evidence.evidence_sequence IS NOT NULL AS evidence_visible, \
      COALESCE(floor(evidence.quality_signal * 1000000)::bigint, 0) AS quality_fixed_point, \
      evidence.formula_version, evidence.course_count, evidence.first_attempt_count, \
      evidence.difficulty_index, evidence.attempts_mean, \
      evidence.time_median_seconds_estimate, evidence.discrimination_index, \
      floor(extract(epoch FROM evidence.evidence_at) * 1000)::bigint AS evidence_at_millis, \
      CASE WHEN $17::text IS NOT NULL THEN 9223372036854775807::bigint \
           WHEN $1::text IS NULL THEN 0::bigint \
           ELSE floor(ts_rank_cd(document.search_text, websearch_to_tsquery('simple', $1)) * 1000000)::bigint END AS full_text_rank, \
      CASE WHEN $17::text IS NOT NULL THEN 9223372036854775807::bigint \
           WHEN $1::text IS NULL THEN 0::bigint \
           ELSE floor(word_similarity(lower($1), document.normalized_search_text) * 1000000)::bigint END AS similarity_score \
    FROM catalog_search_document AS document \
    JOIN problem_version AS version \
      ON version.problem_id = document.problem_id \
     AND version.version_id = document.version_id \
    LEFT JOIN LATERAL public.ple_catalog_discovery_evidence_at( \
        document.problem_id, document.version_id, $16 \
    ) AS evidence ON true \
    WHERE document.lifecycle = 'published' \
      AND document.catalog_sequence <= $16 \
      AND (($17::text IS NOT NULL AND document.question_id = $17::text) \
        OR ($17::text IS NULL AND ($1::text IS NULL \
          OR document.search_text @@ websearch_to_tsquery('simple', $1) \
          OR lower($1) <% document.normalized_search_text))) \
      AND (cardinality($2::text[]) = 0 OR EXISTS ( \
          SELECT 1 FROM unnest(document.public_byline) AS byline \
           WHERE lower(byline) = ANY($2::text[]) \
      )) \
      AND (jsonb_array_length($3::jsonb) = 0 \
          OR document.backend IN (SELECT jsonb_array_elements_text($3::jsonb))) \
      AND (cardinality($4::text[]) = 0 OR EXISTS ( \
          SELECT 1 FROM jsonb_array_elements_text(document.keywords) AS tag \
           WHERE lower(tag) = ANY($4::text[]) \
      )) \
      AND (jsonb_array_length($5::jsonb) = 0 \
          OR document.response_family IN (SELECT jsonb_array_elements_text($5::jsonb))) \
      AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements($6::jsonb) AS wanted \
        WHERE NOT EXISTS (SELECT 1 FROM jsonb_array_elements(document.taxonomy) AS stored \
          WHERE stored->>'scheme' = wanted->>'scheme' AND stored->>'code' = wanted->>'code')) \
      AND document.capabilities @> $7::jsonb \
      AND (jsonb_array_length($8::jsonb) = 0 \
          OR document.license IN (SELECT jsonb_array_elements_text($8::jsonb))) \
      AND ($9::smallint <> 1 OR evidence.evidence_sequence IS NOT NULL) \
      AND ($9::smallint <> 2 OR evidence.evidence_sequence IS NULL) \
      AND ($10::smallint <> 1 OR EXISTS (SELECT 1 FROM used_publications AS usage \
          WHERE usage.problem_id = document.problem_id AND usage.version_id = document.version_id)) \
      AND ($19::smallint <> 1 OR version.author_ids @> jsonb_build_array($20::uuid::text)) \
) ";

#[derive(Clone, Serialize)]
struct UsageSnapshotPublication {
    problem_id: Uuid,
    version_id: Uuid,
}

struct UsageSnapshot {
    token: String,
    expires_at_millis: u64,
}

#[derive(Clone)]
struct CatalogSearchBindings {
    text: Option<String>,
    bylines: Vec<String>,
    backends: Json<Vec<QuestionBackend>>,
    tags: Vec<String>,
    response_families: Json<Vec<CatalogResponseFamily>>,
    taxonomy: Json<Vec<CatalogTaxonomyFilter>>,
    capabilities: Json<Vec<Capability>>,
    licenses: Json<Vec<CatalogLicenseValue>>,
    evidence_filter: i16,
    used_in_my_courses_filter: i16,
    authorship_filter: i16,
    rank: Option<i64>,
    similarity: Option<i64>,
    quality: Option<i64>,
    problem: Option<Uuid>,
    version: Option<Uuid>,
    boundary: i64,
    exact_question_id: Option<String>,
    usage_publications: Json<Vec<UsageSnapshotPublication>>,
    actor: Uuid,
    limit: i64,
}

impl CatalogSearchBindings {
    fn new(
        query: &CatalogSearchQuery,
        after: Option<CatalogSearchCursorKey>,
        boundary: i64,
        exact_question_id: Option<String>,
        usage_publications: Vec<UsageSnapshotPublication>,
        actor: Uuid,
        limit: i64,
    ) -> Self {
        Self {
            text: query.text.clone(),
            bylines: query.bylines.clone(),
            backends: Json(query.backends.clone()),
            tags: query.tags.clone(),
            response_families: Json(query.response_families.clone()),
            taxonomy: Json(query.taxonomy.clone()),
            capabilities: Json(query.capabilities.clone()),
            licenses: Json(query.licenses.clone()),
            evidence_filter: match query.evidence {
                CatalogEvidenceAvailability::Any => 0,
                CatalogEvidenceAvailability::Available => 1,
                CatalogEvidenceAvailability::Unavailable => 2,
            },
            used_in_my_courses_filter: match query.used_in_my_courses {
                CatalogUsedInMyCourses::Any => 0,
                CatalogUsedInMyCourses::Used => 1,
            },
            authorship_filter: match query.authorship {
                CatalogAuthorship::Any => 0,
                CatalogAuthorship::AuthoredByCurrentActor => 1,
            },
            rank: after.map(|key| key.full_text_rank),
            similarity: after.map(|key| key.similarity),
            quality: after.map(|key| key.quality),
            problem: after.map(|key| key.problem),
            version: after.map(|key| key.version),
            boundary,
            exact_question_id,
            usage_publications: Json(usage_publications),
            actor,
            limit,
        }
    }

    fn bind<'q>(
        self,
        query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
        query
            .bind(self.text)
            .bind(self.bylines)
            .bind(self.backends)
            .bind(self.tags)
            .bind(self.response_families)
            .bind(self.taxonomy)
            .bind(self.capabilities)
            .bind(self.licenses)
            .bind(self.evidence_filter)
            .bind(self.used_in_my_courses_filter)
            .bind(self.rank)
            .bind(self.similarity)
            .bind(self.quality)
            .bind(self.problem)
            .bind(self.version)
            .bind(self.boundary)
            .bind(self.exact_question_id)
            .bind(self.usage_publications)
            .bind(self.authorship_filter)
            .bind(self.actor)
            .bind(self.limit)
    }
}

/// Searches hot catalog metadata with an immutable evidence boundary and an
/// actor-bound course-use snapshot. Lifecycle RLS remains live by design.
pub(super) async fn search_catalog(
    store: &PostgresStore,
    context: TenantContext,
    session: SessionTokenHash,
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
            let session_hash = session.to_string();
            // Starting an actor-usage snapshot performs bounded expiry cleanup
            // and persists the cursor-bound rows, so the whole page uses one
            // writable repeatable-read transaction.
            let mut transaction = store.begin_tenant_writable_snapshot(context).await?;
            // The browser never supplies an actor ID. Both cursor binding and
            // the broker snapshot resolve this presented active session.
            sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
                .bind(&session_hash)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let actor: Uuid = sqlx::query_scalar(
                "SELECT user_id FROM public.ple_target_session_subject($1, $2)",
            )
            .bind(&session_hash)
            .bind(context.tenant_id().as_uuid())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(StoreError::NotFound)?;
            let fingerprint = postgres_catalog_search_fingerprint(&query, actor);
            let after = page
                .after
                .as_ref()
                .map(|cursor| {
                    decode_catalog_search_cursor(
                        &store.catalog_cursors,
                        cursor.as_str(),
                        &fingerprint,
                    )
                })
                .transpose()?;
            let boundary = match after {
                Some(key) => i64::try_from(key.snapshot_boundary).map_err(|_| {
                    StoreError::InvalidRecord(
                        "catalog cursor has an invalid snapshot boundary".to_string(),
                    )
                })?,
                None => sqlx::query_scalar::<_, i64>(
                    "SELECT GREATEST( \
                        COALESCE((SELECT max(catalog_sequence) FROM catalog_search_document), 0), \
                        COALESCE((SELECT max(evidence_sequence) FROM catalog_discovery_evidence_revision), 0) \
                    )",
                )
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?,
            };
            let usage_snapshot = match after {
                Some(key) => {
                    let now_millis: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM transaction_timestamp()) * 1000)::bigint",
                    )
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_sqlx_error)?;
                    let now_millis = u64::try_from(now_millis).map_err(|_| {
                        StoreError::Unavailable("database clock is invalid".to_string())
                    })?;
                    if now_millis >= key.actor_usage_snapshot_expires_at_millis {
                        return Err(StoreError::InvalidRecord(
                            "catalog cursor actor-usage snapshot expired; restart search"
                                .to_string(),
                        ));
                    }
                    UsageSnapshot {
                        token: usage_snapshot_token_text(key.actor_usage_snapshot),
                        expires_at_millis: key.actor_usage_snapshot_expires_at_millis,
                    }
                }
                None => begin_usage_snapshot(&mut transaction, context, &session_hash).await?,
            };
            let usage_publications = usage_snapshot_publications(
                &mut transaction,
                context,
                &session_hash,
                &usage_snapshot.token,
            )
            .await
            .map_err(|error| {
                if after.is_some() {
                    StoreError::InvalidRecord(format!(
                        "catalog cursor actor-usage snapshot must be restarted: {error}"
                    ))
                } else {
                    error
                }
            })?;
            // `<%` is the index-supported word-similarity admission operator.
            // Its GUC default is session mutable, so pin the product rule.
            sqlx::query("SELECT set_config('pg_trgm.word_similarity_threshold', $1, true)")
                .bind(TRIGRAM_THRESHOLD)
                .execute(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let limit = i64::from(page.size.get()) + 1;
            let bindings = CatalogSearchBindings::new(
                &query,
                after,
                boundary,
                exact_question_id,
                usage_publications,
                actor,
                limit,
            );

            // ASVS 1.2.4: every request value stays bound. The reviewed CTE
            // has one typed binder so row and facet queries cannot drift.
            let rows_sql = format!(
                "{RANKED_CTE} SELECT full_text_rank::text || '/' || similarity_score::text || '/' || quality_fixed_point::text || '/' || problem_id::text || '/' || version_id::text AS stable_key, question_id, backend, response_family, capabilities, metadata, publication_scope, lifecycle, lifecycle_reason, public_byline, floor(extract(epoch FROM published_at) * 1000)::bigint AS published_at_millis, evidence_visible, formula_version, course_count, first_attempt_count, difficulty_index, attempts_mean, time_median_seconds_estimate, discrimination_index, evidence_at_millis FROM ranked WHERE ($11::bigint IS NULL OR full_text_rank < $11 OR (full_text_rank = $11 AND (similarity_score < $12 OR (similarity_score = $12 AND (quality_fixed_point < $13 OR (quality_fixed_point = $13 AND (problem_id, version_id) > ($14, $15))))))) ORDER BY full_text_rank DESC, similarity_score DESC, quality_fixed_point DESC, problem_id, version_id LIMIT $21"
            );
            let rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(rows_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let page_result = catalog_discovery_item_page_from_rows(rows, page.size.get())?;

            let byline_sql = format!(
                "{RANKED_CTE} SELECT byline, count(*)::bigint AS facet_count FROM ranked CROSS JOIN LATERAL unnest(public_byline) AS byline GROUP BY byline ORDER BY count(*) DESC, lower(byline), byline LIMIT {MAX_CATALOG_BYLINE_FACETS}"
            );
            let byline_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(byline_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let backend_sql = format!(
                "{RANKED_CTE} SELECT backend, count(*)::bigint AS facet_count FROM ranked GROUP BY backend ORDER BY backend LIMIT {MAX_CATALOG_BACKEND_FACETS}"
            );
            let backend_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(backend_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let tag_sql = format!(
                "{RANKED_CTE} SELECT tag, count(*)::bigint AS facet_count FROM ranked CROSS JOIN LATERAL jsonb_array_elements_text(keywords) AS tag GROUP BY tag ORDER BY count(*) DESC, lower(tag), tag LIMIT {MAX_CATALOG_TAG_FACETS}"
            );
            let tag_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(tag_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let response_family_sql = format!(
                "{RANKED_CTE} SELECT response_family, count(*)::bigint AS facet_count FROM ranked GROUP BY response_family ORDER BY response_family LIMIT {MAX_CATALOG_RESPONSE_FAMILY_FACETS}"
            );
            let response_family_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(response_family_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let taxonomy_sql = format!(
                "{RANKED_CTE} SELECT jsonb_build_object('scheme', term->>'scheme', 'code', term->>'code', 'label', min(term->>'label')) AS taxonomy_term, count(*)::bigint AS facet_count FROM ranked CROSS JOIN LATERAL jsonb_array_elements(taxonomy) AS term GROUP BY term->>'scheme', term->>'code' ORDER BY count(*) DESC, term->>'scheme', term->>'code' LIMIT 64"
            );
            let taxonomy_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(taxonomy_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let capability_sql = format!(
                "{RANKED_CTE} SELECT capability, count(*)::bigint AS facet_count FROM ranked CROSS JOIN LATERAL jsonb_array_elements_text(capabilities) AS capability GROUP BY capability ORDER BY capability"
            );
            let capability_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(capability_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let license_sql = format!(
                "{RANKED_CTE} SELECT license, count(*)::bigint AS facet_count FROM ranked GROUP BY license ORDER BY license"
            );
            let license_rows = bindings
                .clone()
                .bind(sqlx::query(AssertSqlSafe(license_sql)))
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            let evidence_sql = format!(
                "{RANKED_CTE} SELECT count(*) FILTER (WHERE evidence_visible)::bigint AS available, count(*) FILTER (WHERE NOT evidence_visible)::bigint AS unavailable, count(*) FILTER (WHERE EXISTS (SELECT 1 FROM used_publications AS usage WHERE usage.problem_id = ranked.problem_id AND usage.version_id = ranked.version_id))::bigint AS used FROM ranked"
            );
            let evidence_facet = bindings
                .bind(sqlx::query(AssertSqlSafe(evidence_sql)))
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            Ok(CatalogSearchPage {
                items: page_result.items,
                next_cursor: page_result.next_cursor.map(|cursor| {
                    let (rank, similarity, quality, problem, version) = page_cursor_key(&cursor);
                    encode_catalog_search_cursor(
                        &store.catalog_cursors,
                        &fingerprint,
                        CatalogSearchCursorKey {
                            snapshot_boundary: u64::try_from(boundary)
                                .expect("nonnegative catalog boundary"),
                            full_text_rank: rank,
                            similarity,
                            quality,
                            actor_usage_snapshot: usage_snapshot_token_bytes(&usage_snapshot.token)
                                .expect("database snapshot token is hexadecimal"),
                            actor_usage_snapshot_expires_at_millis: usage_snapshot.expires_at_millis,
                            problem,
                            version,
                        },
                    )
                }).transpose()?,
                facets: CatalogSearchFacets {
                    bylines: byline_rows
                        .into_iter()
                        .map(decode_byline_facet)
                        .collect::<Result<_, _>>()?,
                    backends: backend_rows
                        .into_iter()
                        .map(decode_backend_facet)
                        .collect::<Result<_, _>>()?,
                    tags: tag_rows
                        .into_iter()
                        .map(decode_tag_facet)
                        .collect::<Result<_, _>>()?,
                    response_families: response_family_rows
                        .into_iter()
                        .map(decode_response_family_facet)
                        .collect::<Result<_, _>>()?,
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
                    evidence: CatalogEvidenceFacet {
                        available: nonnegative_facet_count(&evidence_facet, "available")?,
                        unavailable: nonnegative_facet_count(&evidence_facet, "unavailable")?,
                    },
                    used_in_my_courses: CatalogUsedInMyCoursesFacet {
                        used: nonnegative_facet_count(&evidence_facet, "used")?,
                    },
                },
            })
        }
    })
    .await
}

async fn begin_usage_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: TenantContext,
    session_hash: &str,
) -> Result<UsageSnapshot, StoreError> {
    let row = sqlx::query(
        "SELECT snapshot_token::text AS snapshot_token, row_count, \
                floor(extract(epoch FROM expires_at) * 1000)::bigint AS expires_at_millis \
         FROM public.ple_begin_instructor_catalog_usage_snapshot($1, $2, $3, $4)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(session_hash)
    .bind(USAGE_SNAPSHOT_TTL_SECONDS)
    .bind(MAX_USAGE_SNAPSHOT_ROWS)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| StoreError::Unavailable("catalog usage snapshot was not created".to_string()))?;
    let row_count: i32 = row.try_get("row_count").map_err(map_sqlx_error)?;
    if !(0..=MAX_USAGE_SNAPSHOT_ROWS).contains(&row_count) {
        return Err(StoreError::Unavailable(
            "catalog usage snapshot row count is invalid".to_string(),
        ));
    }
    let token: String = row.try_get("snapshot_token").map_err(map_sqlx_error)?;
    usage_snapshot_token_bytes(&token).ok_or_else(|| {
        StoreError::Unavailable("catalog usage snapshot token is invalid".to_string())
    })?;
    let expires_at_millis: i64 = row.try_get("expires_at_millis").map_err(map_sqlx_error)?;
    let expires_at_millis = u64::try_from(expires_at_millis).map_err(|_| {
        StoreError::Unavailable("catalog usage snapshot expiry is invalid".to_string())
    })?;
    Ok(UsageSnapshot {
        token,
        expires_at_millis,
    })
}

async fn usage_snapshot_publications(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: TenantContext,
    session_hash: &str,
    snapshot_token: &str,
) -> Result<Vec<UsageSnapshotPublication>, StoreError> {
    let rows = sqlx::query(
        "SELECT problem_id, version_id \
         FROM public.ple_instructor_catalog_usage_snapshot_rows($1, $2, $3)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(session_hash)
    .bind(snapshot_token)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    if rows.len() > usize::try_from(MAX_USAGE_SNAPSHOT_ROWS).expect("positive row bound") {
        return Err(StoreError::Unavailable(
            "catalog usage snapshot exceeds its declared bound".to_string(),
        ));
    }
    rows.into_iter()
        .map(|row| {
            Ok(UsageSnapshotPublication {
                problem_id: row.try_get("problem_id").map_err(map_sqlx_error)?,
                version_id: row.try_get("version_id").map_err(map_sqlx_error)?,
            })
        })
        .collect()
}

fn usage_snapshot_token_bytes(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut token = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        token[index] = (high << 4) | low;
    }
    Some(token)
}

fn usage_snapshot_token_text(token: [u8; 32]) -> String {
    let mut value = String::with_capacity(64);
    for byte in token {
        write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
    }
    value
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn page_cursor_key(cursor: &crate::Cursor) -> (i64, i64, i64, Uuid, Uuid) {
    let mut parts = cursor.as_str().split('/');
    let rank = parts
        .next()
        .expect("catalog rank")
        .parse()
        .expect("catalog rank integer");
    let similarity = parts
        .next()
        .expect("catalog similarity")
        .parse()
        .expect("catalog similarity integer");
    let quality = parts
        .next()
        .expect("catalog quality")
        .parse()
        .expect("catalog quality integer");
    let problem = parts
        .next()
        .expect("catalog problem UUID")
        .parse()
        .expect("catalog problem UUID");
    let version = parts
        .next()
        .expect("catalog version UUID")
        .parse()
        .expect("catalog version UUID");
    debug_assert!(parts.next().is_none());
    (rank, similarity, quality, problem, version)
}

fn nonnegative_facet_count(row: &PgRow, column: &str) -> Result<u64, StoreError> {
    let count: i64 = row.try_get(column).map_err(map_sqlx_error)?;
    u64::try_from(count).map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))
}

fn decode_byline_facet(row: PgRow) -> Result<CatalogBylineFacet, StoreError> {
    let byline: String = row.try_get("byline").map_err(map_sqlx_error)?;
    if byline.trim().is_empty() {
        return Err(StoreError::Unavailable(
            "stored catalog byline facet is invalid".to_string(),
        ));
    }
    Ok(CatalogBylineFacet {
        byline,
        count: nonnegative_facet_count(&row, "facet_count")?,
    })
}

fn decode_backend_facet(row: PgRow) -> Result<CatalogBackendFacet, StoreError> {
    let backend: String = row.try_get("backend").map_err(map_sqlx_error)?;
    Ok(CatalogBackendFacet {
        backend: parse_question_backend(&backend)?,
        count: nonnegative_facet_count(&row, "facet_count")?,
    })
}

fn decode_tag_facet(row: PgRow) -> Result<CatalogTagFacet, StoreError> {
    let tag: String = row.try_get("tag").map_err(map_sqlx_error)?;
    if tag.trim().is_empty() {
        return Err(StoreError::Unavailable(
            "stored catalog tag facet is invalid".to_string(),
        ));
    }
    Ok(CatalogTagFacet {
        tag,
        count: nonnegative_facet_count(&row, "facet_count")?,
    })
}

fn decode_response_family_facet(row: PgRow) -> Result<CatalogResponseFamilyFacet, StoreError> {
    let response_family: String = row.try_get("response_family").map_err(map_sqlx_error)?;
    let response_family =
        serde_json::from_value::<CatalogResponseFamily>(Value::String(response_family)).map_err(
            |_| StoreError::Unavailable("stored catalog response family is invalid".to_string()),
        )?;
    Ok(CatalogResponseFamilyFacet {
        response_family,
        count: nonnegative_facet_count(&row, "facet_count")?,
    })
}
