#![cfg(feature = "postgres")]

//! Qualitative live evidence for the PostgreSQL catalog-search index operators.
//!
//! This is deliberately separate from Store behavior tests: on a tiny
//! disposable corpus the planner may correctly choose a sequential scan, so
//! these probes establish index/operator capability without coercing the plan.

use learning_data_access::postgres::{lazy_pool, verify_application_schema};

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_catalog_discovery_predicates_have_index_capability_evidence() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let mut transaction = pool.begin().await.expect("begin plan transaction");
    sqlx::query("SELECT set_config('pg_trgm.word_similarity_threshold', '0.30', true)")
        .execute(&mut *transaction)
        .await
        .expect("pin word similarity admission");
    for (query, index) in [
        (
            "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) SELECT problem_id FROM catalog_search_document WHERE search_text @@ websearch_to_tsquery('simple', 'molar')",
            "catalog_search_document_search_idx",
        ),
        (
            "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) SELECT problem_id FROM catalog_search_document WHERE lower('peptde') <% normalized_search_text",
            "catalog_search_document_trigram_text_idx",
        ),
    ] {
        let plan: serde_json::Value = sqlx::query_scalar(query)
            .fetch_one(&mut *transaction)
            .await
            .expect("representative discovery explain");
        let definition: String = sqlx::query_scalar(
            "SELECT pg_get_indexdef(indexrelid) FROM pg_index WHERE indexrelid = $1::regclass",
        )
        .bind(index)
        .fetch_one(&mut *transaction)
        .await
        .expect("declared discovery index");
        assert!(
            plan.is_array() && definition.contains(index),
            "EXPLAIN ran under normal planner settings and {index} is available for this operator"
        );
    }
}
