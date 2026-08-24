use super::catalog_search_tests::record;
use super::*;

#[tokio::test]
async fn ten_thousand_catalog_rows_return_one_bounded_page_with_server_facets() {
    let store = MemoryStore::default();
    {
        let mut state = store.write_state().expect("test state");
        for number in 1..=10_000 {
            let record = record(number);
            state
                .published
                .insert((record.problem, record.version), record);
        }
        let mut institution_only = record(10_001);
        institution_only.scope = PublicationScope::Institution;
        state.catalog_grants.insert((
            TenantId::from_uuid(Uuid::from_u128(50_001)),
            institution_only.problem,
            institution_only.version,
        ));
        state.published.insert(
            (institution_only.problem, institution_only.version),
            institution_only,
        );
        for number in 0..65_u128 {
            let mut distinct = record(11_000 + number);
            distinct.question.metadata.taxonomy = vec![TaxonomyTerm {
                scheme: "extra".to_string(),
                code: format!("{number:02}"),
                label: if number == 0 { "Zulu" } else { "Term" }.to_string(),
            }];
            state
                .published
                .insert((distinct.problem, distinct.version), distinct);
        }
        let mut duplicate_label = record(12_000);
        duplicate_label.question.metadata.taxonomy = vec![TaxonomyTerm {
            scheme: "extra".to_string(),
            code: "00".to_string(),
            label: "Alpha".to_string(),
        }];
        state.published.insert(
            (duplicate_label.problem, duplicate_label.version),
            duplicate_label,
        );
    }
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(50_000)));
    let first = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some(" peptide   catalog ".to_string()),
                page_size: Some(37),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("bounded search");
    assert_eq!(first.items.len(), 37);
    assert_eq!(first.facets.statistics.available, 0);
    assert_eq!(first.facets.statistics.unavailable, 10_066);
    assert_eq!(first.facets.taxonomy[0].count, 10_000);
    assert_eq!(first.facets.taxonomy.len(), MAX_CATALOG_TAXONOMY_FACETS);
    assert_eq!(first.facets.taxonomy[1].term.code, "00");
    assert_eq!(first.facets.taxonomy[1].term.label, "Alpha");
    assert_eq!(first.facets.taxonomy[1].count, 2);
    assert_eq!(
        first.facets.taxonomy[1..]
            .iter()
            .map(|facet| facet.term.code.clone())
            .collect::<Vec<_>>(),
        (0..=62)
            .map(|number| format!("{number:02}"))
            .collect::<Vec<_>>(),
    );
    assert!(
        first
            .facets
            .taxonomy
            .iter()
            .all(|facet| facet.term.code != "63" && facet.term.code != "64")
    );
    assert_eq!(first.facets.capabilities[0].count, 10_066);
    assert_eq!(first.facets.licenses[0].count, 10_066);
    assert!(
        first
            .items
            .iter()
            .all(|item| item.scope == PublicationScope::Public)
    );
    let cursor = first.next_cursor.clone().expect("next cursor");
    let mut tampered = cursor.clone().into_bytes();
    let last = tampered.len() - 1;
    tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
    assert!(matches!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("peptide catalog".to_string()),
                    cursor: Some(String::from_utf8(tampered).expect("url-safe cursor")),
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let second = store
        .search_catalog(
            context,
            CatalogSearchQuery {
                text: Some("peptide catalog".to_string()),
                cursor: Some(cursor),
                page_size: Some(37),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("second bounded search");
    assert_eq!(second.items.len(), 37);
    assert!(first.items.iter().all(|left| {
        second
            .items
            .iter()
            .all(|right| left.question_id != right.question_id)
    }));
    assert!(matches!(
        store
            .search_catalog(
                context,
                CatalogSearchQuery {
                    text: Some("different query".to_string()),
                    cursor: first.next_cursor,
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
