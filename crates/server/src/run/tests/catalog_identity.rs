use super::*;

/// Deliberately violates the immutable catalog identity contract to prove
/// that run routes stop before any trusted backend can expose or grade it.
#[derive(Debug)]
struct MismatchedCatalogTestStore {
    record: PublishedProblemRecord,
}

#[async_trait]
impl CatalogStore for MismatchedCatalogTestStore {
    async fn publish_draft(
        &self,
        _context: TenantContext,
        _actor: UserId,
        _command: PublishDraftCommand,
    ) -> Result<PublishedProblemRecord, StoreError> {
        Err(StoreError::InvalidRecord(
            "test catalog is read-only".to_string(),
        ))
    }

    async fn get_catalog_problem(
        &self,
        _context: TenantContext,
        _reference: ProblemVersionRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        Ok(Some(self.record.clone()))
    }

    async fn resolve_catalog_problem(
        &self,
        _context: TenantContext,
        _reference: question_model::ProblemDisplayRef,
    ) -> Result<Option<PublishedProblemRecord>, StoreError> {
        Err(StoreError::InvalidRecord(
            "mismatched catalog test store only supports exact immutable lookups".to_string(),
        ))
    }

    async fn list_catalog(
        &self,
        _context: TenantContext,
        _page: PageRequest,
    ) -> Result<Page<CatalogProblemSummary>, StoreError> {
        Err(StoreError::InvalidRecord(
            "test catalog is read-only".to_string(),
        ))
    }

    async fn list_catalog_taxonomy(
        &self,
        _context: TenantContext,
        _page: PageRequest,
    ) -> Result<Page<question_model::taxonomy::TaxonomyTerm>, StoreError> {
        Err(StoreError::InvalidRecord(
            "test catalog is read-only".to_string(),
        ))
    }

    async fn search_catalog(
        &self,
        _context: TenantContext,
        _query: CatalogSearchQuery,
    ) -> Result<CatalogSearchPage, StoreError> {
        Err(StoreError::InvalidRecord(
            "mismatched catalog test store does not support catalog search".to_string(),
        ))
    }

    async fn transition_catalog_problem(
        &self,
        _context: TenantContext,
        _actor: UserId,
        _reference: ProblemVersionRef,
        _transition: CatalogTransition,
    ) -> Result<PublishedProblemRecord, StoreError> {
        Err(StoreError::InvalidRecord(
            "test catalog is read-only".to_string(),
        ))
    }
}

fn authenticated_for_test(context: TenantContext) -> AuthenticatedSession {
    let subject = SessionSubject::new(
        context.tenant_id(),
        UserId::from_uuid(id(2)),
        "Route test student",
        vec![UserRole::Student],
    )
    .expect("test session subject");
    AuthenticatedSession {
        record: SessionRecord {
            token_hash: SessionTokenHash::compute(b"run-route-test-session"),
            subject,
            created_at: ActivityTimestamp::from_unix_millis(10_000),
            expires_at: ActivityTimestamp::from_unix_millis(20_000),
        },
        tenant_context: context,
        session_hash: SessionTokenHash::compute(b"run-route-test-session"),
    }
}

#[tokio::test]
async fn mismatched_published_identity_never_reaches_envelope_or_grading() {
    let (store, _, _, _, _, _, _) = fixture().await;
    let context = TenantContext::from_authenticated_session(TenantId::from_uuid(id(1)));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(8)),
        version: VersionId::from_uuid(id(9)),
    };
    let mut record = store
        .get_catalog_problem(context, reference)
        .await
        .expect("catalog read")
        .expect("fixture published question");
    record.question.problem = ProblemId::from_uuid(id(99));
    let malformed_catalog = MismatchedCatalogTestStore { record };

    let response = load_run_question(
        &malformed_catalog,
        &authenticated_for_test(context),
        reference,
    )
    .await
    .expect_err("mismatched immutable question IDs must be refused");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
