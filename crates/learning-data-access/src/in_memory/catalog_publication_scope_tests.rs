use super::catalog_search_tests::{record, seed_catalog};
use super::*;

#[tokio::test]
async fn catalog_search_includes_every_published_scope_in_one_deterministic_corpus() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(73_080));
    let context = TenantContext::from_authenticated_session(tenant);
    let public = record(73_081);
    let mut institution = record(73_082);
    institution.scope = PublicationScope::Institution;
    seed_catalog(&store, [public, institution.clone()]);
    store
        .write_state()
        .expect("catalog grant state")
        .catalog_grants
        .insert((tenant, institution.problem, institution.version));

    let complete_corpus = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                page_size: Some(10),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("complete shared catalog search");
    assert_eq!(complete_corpus.items.len(), 2);
    assert_eq!(complete_corpus.facets.evidence.unavailable, 2);

    let all_scopes_first_page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("first complete-corpus page");
    let cursor = all_scopes_first_page
        .next_cursor
        .expect("complete-corpus continuation");
    let second_page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                cursor: Some(cursor),
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("complete-corpus continuation");
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(second_page.facets, complete_corpus.facets);
    assert_ne!(
        all_scopes_first_page.items[0].summary.question_id,
        second_page.items[0].summary.question_id
    );
}
