use super::catalog_search_tests::{record, seed_catalog};
use super::*;

#[tokio::test]
async fn catalog_search_publication_scope_filter_controls_rows_facets_and_cursors() {
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

    let all_scopes = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                page_size: Some(10),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("all publication scopes search");
    assert_eq!(all_scopes.items.len(), 2);
    assert_eq!(all_scopes.facets.evidence.unavailable, 2);

    let all_scopes_first_page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("first all-scopes page");
    let cursor = all_scopes_first_page
        .next_cursor
        .expect("all-scopes continuation");

    let public_only = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                publication_scopes: vec![PublicationScope::Public],
                page_size: Some(10),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("public publication scope search");
    assert_eq!(public_only.items.len(), 1);
    assert_eq!(public_only.items[0].summary.scope, PublicationScope::Public);
    assert_eq!(public_only.facets.evidence.unavailable, 1);
    assert!(matches!(
        store
            .search_catalog_as_instructor(
                context,
                CatalogSearchQuery {
                    publication_scopes: vec![PublicationScope::Public],
                    cursor: Some(cursor),
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
