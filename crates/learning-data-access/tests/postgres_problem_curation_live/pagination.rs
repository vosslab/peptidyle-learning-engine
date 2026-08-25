use learning_data_access::{
    Cursor, PageRequest, PageSize, ProblemCollectionReplacementTarget, ProblemCurationStore,
    ReplaceProblemCollectionCommand, ReplaceSavedProblemSearchCommand, StoreError,
};
use question_model::{CatalogSearchFilter, CatalogSearchQuery, ProblemCollectionVisibility};

use crate::fixture::Fixture;

pub(super) async fn sealed_cursors_bind_actor_scope_and_member_revision(fixture: &Fixture) {
    let first = fixture
        .store
        .list_problem_collections(
            fixture.context,
            fixture.elena_session,
            PageRequest::first(PageSize::new(1).expect("page")),
        )
        .await
        .expect("first collection page");
    let cursor = first
        .next_cursor
        .expect("multiple collections create a cursor");
    let tampered = Cursor::parse(format!("{}x", cursor.as_str())).expect("nonempty wire cursor");
    assert!(
        matches!(
            fixture
                .store
                .list_problem_collections(
                    fixture.context,
                    fixture.elena_session,
                    PageRequest::after(tampered, PageSize::new(1).expect("page")),
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "AEAD cursor rejects tampering"
    );
    assert!(
        matches!(
            fixture
                .store
                .list_problem_collections(
                    fixture.context,
                    fixture.morgan_session,
                    PageRequest::after(cursor.clone(), PageSize::new(1).expect("page")),
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "collection cursor binds the session actor"
    );
    assert!(
        matches!(
            fixture
                .store
                .list_problem_collections(
                    fixture.other_context,
                    fixture.elena_session,
                    PageRequest::after(cursor, PageSize::new(1).expect("page")),
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "collection cursor binds tenant scope"
    );

    let favorites = fixture
        .store
        .get_or_create_favorites(fixture.context, fixture.elena_session)
        .await
        .expect("Favorites for member cursor proof");
    let page = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena_session,
            favorites.reference,
            PageRequest::first(PageSize::new(1).expect("page")),
        )
        .await
        .expect("member page")
        .expect("Favorites exists");
    let cursor = page
        .members
        .next_cursor
        .expect("two Favorites members create cursor");
    let changed = fixture
        .store
        .replace_problem_collection(
            fixture.context,
            fixture.elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::Existing(favorites.reference),
                expected_revision: Some(page.collection.revision),
                title: Some("Favorites".into()),
                visibility: Some(ProblemCollectionVisibility::Private),
                question_ids: fixture.public_questions.iter().cloned().rev().collect(),
            },
        )
        .await
        .expect("member order replacement");
    assert!(changed.revision > page.collection.revision);
    assert!(
        matches!(
            fixture
                .store
                .list_problem_collection_members(
                    fixture.context,
                    fixture.elena_session,
                    favorites.reference,
                    PageRequest::after(cursor, PageSize::new(1).expect("page")),
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "member cursor binds collection revision"
    );

    let filter =
        CatalogSearchFilter::from_query(CatalogSearchQuery::default()).expect("fresh saved filter");
    for title in ["D2 saved cursor one", "D2 saved cursor two"] {
        fixture
            .store
            .replace_saved_problem_search(
                fixture.context,
                fixture.elena_session,
                ReplaceSavedProblemSearchCommand {
                    reference: None,
                    expected_revision: None,
                    title: title.into(),
                    filter: filter.clone(),
                },
            )
            .await
            .expect("saved-search cursor fixture");
    }
    let saved = fixture
        .store
        .list_saved_problem_searches(
            fixture.context,
            fixture.elena_session,
            PageRequest::first(PageSize::new(1).expect("page")),
        )
        .await
        .expect("first saved search page");
    let cursor = saved.next_cursor.expect("two saved searches create cursor");
    assert!(
        matches!(
            fixture
                .store
                .list_saved_problem_searches(
                    fixture.context,
                    fixture.ada_session,
                    PageRequest::after(cursor, PageSize::new(1).expect("page")),
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ),
        "saved-search cursor binds owner session"
    );
}
