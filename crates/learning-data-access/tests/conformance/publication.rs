use super::*;

/// Exercises publication as one immutable, independently identified question.
/// The caller provides fresh opaque storage references; the store assigns the
/// only durable human-facing identity, the Question ID.
pub(super) async fn exercise_publication_identity_boundary<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(600));
    let foreign_tenant = TenantId::from_uuid(uuid(601));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(602));
    let capabilities = BackendCapabilities::from_iter([Capability::ServerGrading]);

    let publish = |workspace, derived_from: Option<ProblemVersionRef>| DraftRecord {
        tenant,
        question: draft_question(WorkspaceId::from_uuid(uuid(workspace))),
        derived_from,
    };
    let base = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(608)),
        version: VersionId::from_uuid(uuid(609)),
    };
    let base_draft = publish(607, None);
    let saved_base = store
        .upsert_draft(context, publisher, None, base_draft.clone())
        .await
        .expect("base draft saves");
    let base_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: base_draft,
                expected_revision: saved_base.revision,
                publication: base,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: capabilities.clone(),
            },
        )
        .await
        .expect("new content publishes as a question");
    assert_eq!(
        store
            .resolve_catalog_problem(
                context,
                ProblemDisplayRef {
                    question_id: base_record.question_id.clone(),
                },
            )
            .await,
        Ok(Some(base_record.clone())),
        "a Question ID resolves the one exact visible publication"
    );

    let derivative = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(610)),
        version: VersionId::from_uuid(uuid(611)),
    };
    let derivative_draft = publish(612, Some(base));
    let saved_derivative = store
        .upsert_draft(context, publisher, None, derivative_draft.clone())
        .await
        .expect("derivative draft saves");
    let derivative_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: derivative_draft,
                expected_revision: saved_derivative.revision,
                publication: derivative,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities,
            },
        )
        .await
        .expect("changed content gets a separate immutable publication");
    assert_ne!(derivative_record.question_id, base_record.question_id);
    assert_eq!(derivative_record.derived_from, Some(base));
    assert_eq!(
        store.get_catalog_problem(context, base).await,
        Ok(Some(base_record.clone())),
        "source content stays exact and unchanged"
    );

    let duplicate_problem = ProblemVersionRef {
        problem: base.problem,
        version: VersionId::from_uuid(uuid(613)),
    };
    let duplicate_draft = publish(614, None);
    let saved_duplicate = store
        .upsert_draft(context, publisher, None, duplicate_draft.clone())
        .await
        .expect("duplicate-reference draft saves before publication validation");
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: duplicate_draft,
                    expected_revision: saved_duplicate.revision,
                    publication: duplicate_problem,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::AlreadyExists),
        "each content publication uses a fresh opaque problem/version pair"
    );
    let duplicate_version = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(615)),
        version: base.version,
    };
    let duplicate_version_draft = publish(616, None);
    let saved_duplicate_version = store
        .upsert_draft(context, publisher, None, duplicate_version_draft.clone())
        .await
        .expect("duplicate-version draft saves before publication validation");
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: duplicate_version_draft,
                    expected_revision: saved_duplicate_version.revision,
                    publication: duplicate_version,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::AlreadyExists),
        "a fresh problem identity still requires a globally fresh version identity"
    );
    assert_eq!(
        store
            .resolve_catalog_problem(
                foreign_context,
                ProblemDisplayRef {
                    question_id: derivative_record.question_id.clone(),
                },
            )
            .await,
        Ok(Some(derivative_record)),
        "public Question ID visibility remains exact across tenants"
    );
}
