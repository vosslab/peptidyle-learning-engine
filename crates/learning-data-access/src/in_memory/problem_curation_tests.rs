//! Focused behavioral checks for the in-memory problem-curation capability.

mod access;
mod collections;
mod fixtures;
mod pagination;
mod saved_searches;

use super::*;
use crate::{
    PageRequest, PageSize, ProblemCollectionReplacementTarget, ProblemCurationStore,
    ReplaceProblemCollectionCommand, SessionLifetime, SessionStore, SessionSubject,
    SessionTokenHash, StoreError,
};
use question_model::{
    CatalogSearchFilter, CatalogSearchQuery, ProblemCollectionAccess, ProblemCollectionReference,
    ProblemCollectionVisibility, SavedProblemSearchReference, UserRole,
};

#[test]
fn curation_references_and_saved_filter_use_browser_safe_contracts() {
    assert_eq!(
        ProblemCollectionReference::new(8)
            .expect("reference")
            .to_string(),
        "PC-8"
    );
    assert_eq!(
        SavedProblemSearchReference::new(9)
            .expect("reference")
            .to_string(),
        "PS-9"
    );
    let filter = CatalogSearchFilter::from_query(CatalogSearchQuery::default())
        .expect("fresh filter normalizes");
    assert_eq!(filter.fresh_query().cursor, None);
    assert_eq!(filter.fresh_query().page_size, None);
}

#[tokio::test]
async fn approved_dual_role_mutates_and_unapproved_dual_role_reads_institution_only() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(91_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let elena = UserId::from_uuid(Uuid::from_u128(91_002));
    let morgan = UserId::from_uuid(Uuid::from_u128(91_003));
    let elena_session = SessionTokenHash::compute(b"curation-elena");
    let morgan_session = SessionTokenHash::compute(b"curation-morgan");
    let publication = super::catalog_search_tests::record(91_004);
    let question_id = publication.question_id.clone();
    {
        let mut state = store.write_state().expect("fixture state");
        state
            .published
            .insert((publication.problem, publication.version), publication);
        state.instructor_approvals.insert(
            elena,
            crate::StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: elena,
                    approved_by: elena,
                    approved_at: ActivityTimestamp::from_unix_millis(0),
                    revoked_at: None,
                },
                revision: crate::InstructorApprovalRevision::INITIAL,
            },
        );
    }
    for (token, user, label) in [
        (elena_session, elena, "Elena"),
        (morgan_session, morgan, "Morgan"),
    ] {
        store
            .create_session(
                token,
                SessionSubject::new(
                    tenant,
                    user,
                    label,
                    vec![UserRole::Instructor, UserRole::Sysadmin],
                )
                .expect("role session"),
                SessionLifetime::from_seconds(60).expect("session lifetime"),
            )
            .await
            .expect("session stored");
    }
    let collection = store
        .replace_problem_collection(
            context,
            elena_session,
            ReplaceProblemCollectionCommand {
                target: ProblemCollectionReplacementTarget::NewNamed,
                expected_revision: None,
                title: Some("Biochemistry picks".to_string()),
                visibility: Some(ProblemCollectionVisibility::Institution),
                question_ids: vec![question_id],
            },
        )
        .await
        .expect("Elena mutates through active Instructor authority");
    let morgan_view = store
        .get_problem_collection_summary(context, morgan_session, collection.reference)
        .await
        .expect("Morgan read")
        .expect("institution collection visible");
    assert_eq!(
        morgan_view.access,
        ProblemCollectionAccess::InstitutionReader
    );
    assert!(matches!(
        store
            .replace_problem_collection(
                context,
                morgan_session,
                ReplaceProblemCollectionCommand {
                    target: ProblemCollectionReplacementTarget::Existing(collection.reference),
                    expected_revision: Some(collection.revision),
                    title: Some("Changed".to_string()),
                    visibility: Some(ProblemCollectionVisibility::Institution),
                    question_ids: Vec::new(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    let page = store
        .list_problem_collections(
            context,
            morgan_session,
            PageRequest::first(PageSize::new(50).expect("bounded page")),
        )
        .await
        .expect("Morgan institution listing");
    assert_eq!(page.items, vec![morgan_view]);
}

#[tokio::test]
async fn favorites_materializes_once_for_the_approved_instructor() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(92_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let elena = UserId::from_uuid(Uuid::from_u128(92_002));
    let session = SessionTokenHash::compute(b"favorites-elena");
    {
        let mut state = store.write_state().expect("fixture state");
        state.instructor_approvals.insert(
            elena,
            crate::StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: elena,
                    approved_by: elena,
                    approved_at: ActivityTimestamp::from_unix_millis(0),
                    revoked_at: None,
                },
                revision: crate::InstructorApprovalRevision::INITIAL,
            },
        );
    }
    store
        .create_session(
            session,
            SessionSubject::new(tenant, elena, "Elena", vec![UserRole::Instructor])
                .expect("instructor session"),
            SessionLifetime::from_seconds(60).expect("session lifetime"),
        )
        .await
        .expect("session stored");
    let first = store
        .get_or_create_favorites(context, session)
        .await
        .expect("first Favorites resolution");
    let second = store
        .get_or_create_favorites(context, session)
        .await
        .expect("idempotent Favorites resolution");
    assert_eq!(first, second);
    assert_eq!(first.title, "Favorites");
    assert_eq!(first.visibility, ProblemCollectionVisibility::Private);
    assert_eq!(first.access, ProblemCollectionAccess::Owner);
}
