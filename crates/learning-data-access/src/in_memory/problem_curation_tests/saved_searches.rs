use super::fixtures::Fixture;

use crate::{
    PageRequest, PageSize, ProblemCurationStore, ReplaceSavedProblemSearchCommand, StoreError,
};
use question_model::{CatalogSearchFilter, CatalogSearchQuery};

fn saved(title: &str, query: CatalogSearchQuery) -> ReplaceSavedProblemSearchCommand {
    ReplaceSavedProblemSearchCommand {
        reference: None,
        expected_revision: None,
        title: title.to_string(),
        filter: CatalogSearchFilter::from_query(query).expect("fresh normalized filter"),
    }
}

#[tokio::test]
async fn saved_filters_normalize_without_continuation_and_are_owner_only() {
    let fixture = Fixture::new(1).await;
    let created = fixture
        .store
        .replace_saved_problem_search(
            fixture.context,
            fixture.elena,
            saved(
                "Peptide search",
                CatalogSearchQuery {
                    text: Some("  peptide   bond ".to_string()),
                    cursor: Some("a prior D1 cursor".to_string()),
                    page_size: Some(1),
                    ..CatalogSearchQuery::default()
                },
            ),
        )
        .await
        .expect("saved search");
    assert_eq!(created.filter.text.as_deref(), Some("peptide bond"));
    assert_eq!(created.filter.fresh_query().cursor, None);
    assert_eq!(created.filter.fresh_query().page_size, None);
    assert!(
        fixture
            .store
            .get_saved_problem_search(fixture.context, fixture.ada, created.reference)
            .await
            .expect("foreign lookup")
            .is_none()
    );
    assert!(
        fixture
            .store
            .get_saved_problem_search(fixture.context, fixture.morgan, created.reference)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn saved_search_titles_are_case_insensitive_and_revisions_are_strong() {
    let fixture = Fixture::new(1).await;
    let first = fixture
        .store
        .replace_saved_problem_search(
            fixture.context,
            fixture.elena,
            saved("Exam candidates", CatalogSearchQuery::default()),
        )
        .await
        .expect("first saved search");
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena,
                saved("exam CANDIDATES", CatalogSearchQuery::default()),
            )
            .await,
        Err(StoreError::AlreadyExists)
    ));
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena,
                ReplaceSavedProblemSearchCommand {
                    reference: Some(first.reference),
                    expected_revision: Some(first.revision.checked_next().expect("next revision")),
                    title: "Exam candidates".to_string(),
                    filter: CatalogSearchFilter::from_query(CatalogSearchQuery::default())
                        .expect("filter"),
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    let no_op = fixture
        .store
        .replace_saved_problem_search(
            fixture.context,
            fixture.elena,
            ReplaceSavedProblemSearchCommand {
                reference: Some(first.reference),
                expected_revision: Some(first.revision),
                title: "Exam candidates".to_string(),
                filter: CatalogSearchFilter::from_query(CatalogSearchQuery::default())
                    .expect("filter"),
            },
        )
        .await
        .expect("no-op saved search");
    assert_eq!(no_op.revision, first.revision);
    assert!(matches!(
        fixture
            .store
            .delete_saved_problem_search(
                fixture.context,
                fixture.elena,
                first.reference,
                first.revision.checked_next().expect("next revision"),
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(
        fixture
            .store
            .delete_saved_problem_search(
                fixture.context,
                fixture.elena,
                first.reference,
                first.revision,
            )
            .await
            .expect("delete saved search")
    );
}

#[tokio::test]
async fn saved_search_titles_reject_unicode_boundary_whitespace_without_persisting() {
    let fixture = Fixture::new(1).await;
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena,
                saved("\u{00a0}Peptide search", CatalogSearchQuery::default()),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let page = fixture
        .store
        .list_saved_problem_searches(
            fixture.context,
            fixture.elena,
            PageRequest::first(PageSize::new(100).expect("page size")),
        )
        .await
        .expect("owner list after rejected title");
    assert!(page.items.is_empty());
}

#[tokio::test]
async fn saved_searches_enforce_the_personal_object_bound() {
    let fixture = Fixture::new(1).await;
    for number in 0..100 {
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena,
                saved(
                    &format!("Saved search {number}"),
                    CatalogSearchQuery::default(),
                ),
            )
            .await
            .expect("bounded saved search");
    }
    assert!(matches!(
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena,
                saved("One more", CatalogSearchQuery::default()),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let page = fixture
        .store
        .list_saved_problem_searches(
            fixture.context,
            fixture.elena,
            PageRequest::first(PageSize::new(100).expect("page size")),
        )
        .await
        .expect("owner list");
    assert_eq!(page.items.len(), 100);
}
