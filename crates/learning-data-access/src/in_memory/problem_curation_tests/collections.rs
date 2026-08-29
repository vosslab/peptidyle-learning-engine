use super::fixtures::Fixture;

use crate::{
    PageRequest, PageSize, ProblemCollectionReplacementTarget, ProblemCurationStore,
    ReplaceProblemCollectionCommand, StoreError,
};
use question_model::{
    CatalogLifecycle, ProblemCollectionKind, ProblemCollectionSelectionAvailability,
    ProblemCollectionVisibility,
};

#[tokio::test]
async fn replacement_retains_exact_immutable_member_after_lifecycle_changes() {
    for lifecycle in [
        CatalogLifecycle::Deprecated {
            reason: "A successor replaces this item.".to_string(),
        },
        CatalogLifecycle::Archived {
            reason: "Historical teaching evidence.".to_string(),
        },
    ] {
        let fixture = Fixture::new(1).await;
        let collection = fixture
            .named(
                fixture.elena,
                "Replacement candidates",
                ProblemCollectionVisibility::Private,
                fixture.question_ids.clone(),
            )
            .await
            .expect("collection");
        {
            let mut state = fixture.store.write_state().expect("fixture state");
            let record = state.published.values_mut().next().expect("publication");
            record.lifecycle = lifecycle;
        }

        let members = fixture
            .store
            .list_problem_collection_members(
                fixture.context,
                fixture.elena,
                collection.reference,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await
            .expect("member lookup")
            .expect("owner collection");
        assert_eq!(members.members.items.len(), 1);
        assert_eq!(
            members.members.items[0].question_id,
            fixture.question_ids[0]
        );
        assert_eq!(
            members.members.items[0].selection_availability,
            ProblemCollectionSelectionAvailability::Retained
        );
    }
}

#[tokio::test]
async fn destination_rechecks_current_visibility_before_atomic_replacement() {
    for lifecycle in [
        CatalogLifecycle::Deprecated {
            reason: "No longer assigned.".to_string(),
        },
        CatalogLifecycle::Archived {
            reason: "Historical teaching evidence.".to_string(),
        },
    ] {
        let fixture = Fixture::new(2).await;
        let collection = fixture
            .named(
                fixture.elena,
                "Current selection",
                ProblemCollectionVisibility::Private,
                vec![fixture.question_ids[0].clone()],
            )
            .await
            .expect("collection");
        {
            let mut state = fixture.store.write_state().expect("fixture state");
            let record = state
                .published
                .values_mut()
                .find(|record| record.question_id == fixture.question_ids[1])
                .expect("publication");
            record.lifecycle = lifecycle;
        }
        assert!(matches!(
            fixture
                .replace_named(
                    fixture.elena,
                    collection.reference,
                    collection.revision,
                    "Current selection",
                    ProblemCollectionVisibility::Private,
                    vec![fixture.question_ids[1].clone()],
                )
                .await,
            Err(StoreError::NotFound)
        ));
        let unchanged = fixture
            .store
            .list_problem_collection_members(
                fixture.context,
                fixture.elena,
                collection.reference,
                PageRequest::first(PageSize::new(10).expect("page size")),
            )
            .await
            .expect("member lookup")
            .expect("collection remains");
        assert_eq!(unchanged.collection.revision, collection.revision);
        assert_eq!(
            unchanged.members.items[0].question_id,
            fixture.question_ids[0]
        );
    }
}

#[tokio::test]
async fn collection_replacement_has_duplicate_size_stale_noop_and_delete_semantics() {
    let fixture = Fixture::new(201).await;
    let collection = fixture
        .named(
            fixture.elena,
            "Review set",
            ProblemCollectionVisibility::Private,
            vec![fixture.question_ids[0].clone()],
        )
        .await
        .expect("collection");
    for ids in [
        vec![
            fixture.question_ids[0].clone(),
            fixture.question_ids[0].clone(),
        ],
        fixture.question_ids.clone(),
    ] {
        assert!(matches!(
            fixture
                .replace_named(
                    fixture.elena,
                    collection.reference,
                    collection.revision,
                    "Review set",
                    ProblemCollectionVisibility::Private,
                    ids,
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    let unchanged = fixture
        .replace_named(
            fixture.elena,
            collection.reference,
            collection.revision,
            "Review set",
            ProblemCollectionVisibility::Private,
            vec![fixture.question_ids[0].clone()],
        )
        .await
        .expect("no-op replacement");
    assert_eq!(unchanged.revision, collection.revision);
    assert!(matches!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena,
                collection.reference,
                collection.revision.checked_next().expect("next revision"),
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena,
                collection.reference,
                collection.revision,
            )
            .await
            .expect("delete named collection")
    );
    assert!(
        !fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena,
                collection.reference,
                collection.revision,
            )
            .await
            .expect("idempotent deletion")
    );
}

#[tokio::test]
async fn favorites_is_singleton_with_fixed_metadata_and_cannot_be_deleted() {
    let fixture = Fixture::new(1).await;
    let favorites = fixture
        .store
        .get_or_create_favorites(fixture.context, fixture.elena)
        .await
        .expect("favorites");
    assert_eq!(favorites.kind, ProblemCollectionKind::Favorites);
    assert_eq!(favorites.title, "Favorites");
    assert_eq!(favorites.visibility, ProblemCollectionVisibility::Private);
    assert!(matches!(
        fixture
            .store
            .replace_problem_collection(
                fixture.context,
                fixture.elena,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::Favorites,
                    expected_revision: Some(favorites.revision),
                    title: Some("Renamed".to_string()),
                    visibility: Some(ProblemCollectionVisibility::Institution),
                    question_ids: Vec::new(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        fixture
            .store
            .delete_problem_collection(
                fixture.context,
                fixture.elena,
                favorites.reference,
                favorites.revision,
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
