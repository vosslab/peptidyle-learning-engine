use super::fixtures::Fixture;

use crate::{PageRequest, PageSize, ProblemCurationStore, StoreError};
use question_model::ProblemCollectionVisibility;

#[tokio::test]
async fn collection_continuations_are_actor_bound_and_survive_unrelated_row_changes() {
    let fixture = Fixture::new(1).await;
    let mut collections = Vec::new();
    for title in ["A", "B", "C"] {
        collections.push(
            fixture
                .named(
                    fixture.elena,
                    title,
                    ProblemCollectionVisibility::Private,
                    Vec::new(),
                )
                .await
                .expect("named collection"),
        );
    }
    let first = fixture
        .store
        .list_problem_collections(
            fixture.context,
            fixture.elena,
            PageRequest::first(PageSize::new(1).expect("page size")),
        )
        .await
        .expect("first page");
    let continuation = first.next_cursor.clone().expect("continuation");
    assert!(matches!(
        fixture
            .store
            .list_problem_collections(
                fixture.context,
                fixture.ada,
                PageRequest::after(continuation.clone(), PageSize::new(1).expect("page size")),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    fixture
        .named(
            fixture.elena,
            "D",
            ProblemCollectionVisibility::Private,
            Vec::new(),
        )
        .await
        .expect("later insertion");
    assert!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena,
                collections[2].reference,
                collections[2].revision,
            )
            .await
            .expect("unseen-row deletion")
    );
    let second = fixture
        .store
        .list_problem_collections(
            fixture.context,
            fixture.elena,
            PageRequest::after(continuation, PageSize::new(1).expect("page size")),
        )
        .await
        .expect("owner continuation remains usable");
    assert_eq!(second.items.len(), 1);
    assert_ne!(second.items[0].reference, first.items[0].reference);
}

#[tokio::test]
async fn member_continuations_bind_to_collection_revision_and_scope() {
    let fixture = Fixture::new(3).await;
    let collection = fixture
        .named(
            fixture.elena,
            "Paging members",
            ProblemCollectionVisibility::Private,
            fixture.question_ids.clone(),
        )
        .await
        .expect("collection");
    let first = fixture
        .store
        .list_problem_collection_members(
            fixture.context,
            fixture.elena,
            collection.reference,
            PageRequest::first(PageSize::new(1).expect("page size")),
        )
        .await
        .expect("first members")
        .expect("visible collection");
    let continuation = first.members.next_cursor.clone().expect("continuation");
    assert!(matches!(
        fixture
            .store
            .list_problem_collections(
                fixture.context,
                fixture.elena,
                PageRequest::after(continuation.clone(), PageSize::new(1).expect("page size")),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let updated = fixture
        .replace_named(
            fixture.elena,
            collection.reference,
            collection.revision,
            "Paging members",
            ProblemCollectionVisibility::Private,
            vec![
                fixture.question_ids[0].clone(),
                fixture.question_ids[1].clone(),
            ],
        )
        .await
        .expect("revision-changing replacement");
    assert_ne!(updated.revision, collection.revision);
    assert!(matches!(
        fixture
            .store
            .list_problem_collection_members(
                fixture.context,
                fixture.elena,
                collection.reference,
                PageRequest::after(continuation, PageSize::new(1).expect("page size")),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn named_collection_limit_and_case_folded_title_uniqueness_are_enforced() {
    let fixture = Fixture::new(1).await;
    fixture
        .named(
            fixture.elena,
            "Exam review",
            ProblemCollectionVisibility::Private,
            Vec::new(),
        )
        .await
        .expect("first collection");
    assert!(matches!(
        fixture
            .named(
                fixture.elena,
                "EXAM REVIEW",
                ProblemCollectionVisibility::Private,
                Vec::new(),
            )
            .await,
        Err(StoreError::AlreadyExists)
    ));
    for number in 1..100 {
        fixture
            .named(
                fixture.elena,
                &format!("Collection {number}"),
                ProblemCollectionVisibility::Private,
                Vec::new(),
            )
            .await
            .expect("bounded named collection");
    }
    assert!(matches!(
        fixture
            .named(
                fixture.elena,
                "Beyond the limit",
                ProblemCollectionVisibility::Private,
                Vec::new(),
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
