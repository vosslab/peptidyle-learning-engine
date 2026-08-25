use super::fixtures::{Fixture, OTHER_TENANT, tenant, token, user};

use crate::{
    PageRequest, PageSize, ProblemCurationCapability, ProblemCurationStore, SessionLifetime,
    SessionStore, SessionSubject, StoreError,
};
use question_model::{ProblemCollectionAccess, ProblemCollectionVisibility, UserRole};

#[tokio::test]
async fn role_preflight_distinguishes_catalog_reads_from_personal_curation() {
    let fixture = Fixture::new(0).await;
    for session in [fixture.elena, fixture.ada, fixture.morgan] {
        fixture
            .store
            .preflight_problem_curation(
                fixture.context,
                session,
                ProblemCurationCapability::CatalogInstitutionRead,
            )
            .await
            .expect("Instructor and Sysadmin catalog readers");
    }
    for session in [fixture.elena, fixture.ada] {
        fixture
            .store
            .preflight_problem_curation(
                fixture.context,
                session,
                ProblemCurationCapability::PersonalMutation,
            )
            .await
            .expect("approved Instructor personal curation");
    }
    assert_eq!(
        fixture
            .store
            .preflight_problem_curation(
                fixture.context,
                fixture.morgan,
                ProblemCurationCapability::PersonalMutation,
            )
            .await,
        Err(StoreError::Forbidden)
    );

    let student = token("d2-student-preflight");
    fixture
        .store
        .create_session(
            student,
            SessionSubject::new(
                fixture.context.tenant_id(),
                user(930_099),
                "Student",
                vec![UserRole::Student],
            )
            .expect("student session"),
            SessionLifetime::from_seconds(600).expect("positive lifetime"),
        )
        .await
        .expect("stored student session");
    for capability in [
        ProblemCurationCapability::CatalogInstitutionRead,
        ProblemCurationCapability::PersonalMutation,
    ] {
        assert_eq!(
            fixture
                .store
                .preflight_problem_curation(fixture.context, student, capability)
                .await,
            Err(StoreError::Forbidden)
        );
    }
}

#[tokio::test]
async fn private_collections_are_owner_only_and_tenant_scoped() {
    let fixture = Fixture::new(2).await;
    let private = fixture
        .named(
            fixture.elena,
            "Elena private picks",
            ProblemCollectionVisibility::Private,
            vec![fixture.question_ids[0].clone()],
        )
        .await
        .expect("owner collection");

    assert!(
        fixture
            .store
            .get_problem_collection_summary(fixture.context, fixture.ada, private.reference)
            .await
            .expect("cross-owner lookup")
            .is_none()
    );
    assert!(
        fixture
            .store
            .get_problem_collection_summary(fixture.context, fixture.morgan, private.reference)
            .await
            .expect("sysadmin lookup")
            .is_none()
    );

    let other_context = fixture.context_for(tenant(OTHER_TENANT));
    let other_session = token("d2-other-tenant");
    fixture
        .store
        .create_session(
            other_session,
            SessionSubject::new(
                tenant(OTHER_TENANT),
                user(OTHER_TENANT),
                "Other tenant instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid session"),
            SessionLifetime::from_seconds(600).expect("valid lifetime"),
        )
        .await
        .expect("other tenant session");
    assert!(matches!(
        fixture
            .store
            .get_problem_collection_summary(other_context, other_session, private.reference)
            .await,
        Err(StoreError::Forbidden) | Err(StoreError::NotFound)
    ));
}

#[tokio::test]
async fn institution_collections_are_readable_by_same_tenant_instructor_and_sysadmin() {
    let fixture = Fixture::new(1).await;
    let institution = fixture
        .named(
            fixture.elena,
            "Shared peptide questions",
            ProblemCollectionVisibility::Institution,
            fixture.question_ids.clone(),
        )
        .await
        .expect("institution collection");

    for session in [fixture.ada, fixture.morgan] {
        let view = fixture
            .store
            .get_problem_collection_summary(fixture.context, session, institution.reference)
            .await
            .expect("same-tenant reader")
            .expect("institution projection");
        assert_eq!(view.access, ProblemCollectionAccess::InstitutionReader);
        assert_eq!(view.title, "Shared peptide questions");
    }
    let page = fixture
        .store
        .list_problem_collections(
            fixture.context,
            fixture.morgan,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("Morgan collection list");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].reference, institution.reference);
}
