use super::*;

async fn exercise_catalog_store<S>(store: &S)
where
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(301));
    let foreign_tenant = TenantId::from_uuid(uuid(302));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(303));
    let other_user = UserId::from_uuid(uuid(304));
    let tenant_course = CourseId::from_uuid(uuid(317));
    let foreign_course = CourseId::from_uuid(uuid(318));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: tenant_course,
                tenant,
                title: "Tenant biochemistry".to_string(),
                members: vec![CourseMembership {
                    user: publisher,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("tenant course should save");
    store
        .upsert_course(
            foreign_context,
            CourseRecord {
                id: foreign_course,
                tenant: foreign_tenant,
                title: "Foreign biochemistry".to_string(),
                members: vec![CourseMembership {
                    user: other_user,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("foreign course should save");
    let institution_workspace = WorkspaceId::from_uuid(uuid(305));
    let institution_problem = ProblemId::from_uuid(uuid(306));
    let institution_version = VersionId::from_uuid(uuid(307));
    let mut institution_question = draft_question(institution_workspace);
    institution_question.metadata.taxonomy = vec![
        TaxonomyTerm {
            scheme: "discipline/core".to_string(),
            code: "BIOC".to_string(),
            label: "Biochemistry".to_string(),
        },
        TaxonomyTerm {
            scheme: "discipline".to_string(),
            code: "core/BIOC".to_string(),
            label: "Biochemistry integration".to_string(),
        },
    ];
    let institution_draft = DraftRecord {
        tenant,
        question: institution_question,
        revises: None,
        derived_from: None,
    };
    let saved_institution_draft = store
        .upsert_draft(context, publisher, None, institution_draft.clone())
        .await
        .expect("institution draft should save");
    let institution_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: institution_draft.clone(),
                expected_revision: saved_institution_draft.revision,
                publication: ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                },
                published_source: published_source(),
                publisher,
                scope: PublicationScope::Institution,
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("institution publication should succeed");

    assert_eq!(institution_record.question.problem, institution_problem);
    assert_eq!(
        store
            .get_draft(context, publisher, institution_workspace)
            .await
            .expect("published draft lookup"),
        None
    );
    assert_eq!(
        store
            .get_catalog_problem(
                foreign_context,
                ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                },
            )
            .await,
        Ok(None),
        "institution publication must not cross its visibility grant"
    );
    assert_eq!(
        store
            .get_published_problem(institution_problem, institution_version)
            .await,
        Ok(None),
        "the context-free public-content contract must not expose institution content"
    );
    let tenant_taxonomy = store
        .list_catalog_taxonomy(
            context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("tenant taxonomy should list");
    let foreign_taxonomy = store
        .list_catalog_taxonomy(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("foreign taxonomy should list");
    assert_eq!(
        tenant_taxonomy
            .items
            .iter()
            .map(|term| (term.scheme.as_str(), term.code.as_str()))
            .collect::<Vec<_>>(),
        vec![("discipline", "core/BIOC"), ("discipline/core", "BIOC"),],
        "taxonomy identity is the scheme/code pair, even when either contains a slash"
    );
    assert!(foreign_taxonomy.items.is_empty());
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(313)),
                tenant,
                course_id: tenant_course,
                title: "Institution content".to_string(),
                items: fixed_items(vec![ProblemVersionRef {
                    problem: institution_problem,
                    version: institution_version,
                }]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("publishing tenant should assign institution content");
    assert!(matches!(
        store
            .create_assignment(
                foreign_context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(314)),
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    title: "Hidden institution content".to_string(),
                    items: fixed_items(vec![ProblemVersionRef {
                        problem: institution_problem,
                        version: institution_version,
                    }]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));

    let public_workspace = WorkspaceId::from_uuid(uuid(308));
    let public_problem = ProblemId::from_uuid(uuid(309));
    let public_version = VersionId::from_uuid(uuid(310));
    let public_draft = DraftRecord {
        tenant,
        question: draft_question(public_workspace),
        revises: None,
        derived_from: None,
    };
    let saved_public_draft = store
        .upsert_draft(context, publisher, None, public_draft.clone())
        .await
        .expect("public draft should save");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: public_draft,
                expected_revision: saved_public_draft.revision,
                publication: ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
                published_source: published_source(),
                publisher,
                scope: PublicationScope::Public,
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("public publication should succeed");
    let foreign_catalog = store
        .list_catalog(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("foreign public catalog should list");
    assert_eq!(foreign_catalog.items.len(), 1);
    assert_eq!(foreign_catalog.items[0].problem, public_problem);

    assert_eq!(
        store
            .transition_catalog_problem(
                foreign_context,
                publisher,
                ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
                CatalogTransition::Deprecate {
                    reason: "Foreign tenant must not mutate".to_string(),
                },
            )
            .await,
        Err(StoreError::NotFound),
        "an identical tenant-local user ID does not own another tenant's public problem"
    );
    assert!(matches!(
        store
            .get_catalog_problem(
                context,
                ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
            )
            .await
            .expect("owner reads public problem after refused transition")
            .expect("public problem remains present")
            .lifecycle,
        CatalogLifecycle::Published
    ));

    let foreign_revision_version = VersionId::from_uuid(uuid(319));
    let foreign_revision_workspace = WorkspaceId::from_uuid(uuid(320));
    let foreign_revision_draft = DraftRecord {
        tenant: foreign_tenant,
        question: draft_question(foreign_revision_workspace),
        revises: Some(ProblemVersionRef {
            problem: public_problem,
            version: public_version,
        }),
        derived_from: None,
    };
    let saved_foreign_revision = store
        .upsert_draft(
            foreign_context,
            publisher,
            None,
            foreign_revision_draft.clone(),
        )
        .await
        .expect("same user ID may own an unrelated foreign-tenant draft");
    assert_eq!(
        store
            .publish_draft(
                foreign_context,
                publisher,
                PublishDraftCommand {
                    expected_draft: foreign_revision_draft,
                    expected_revision: saved_foreign_revision.revision,
                    publication: ProblemVersionRef {
                        problem: public_problem,
                        version: foreign_revision_version,
                    },
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
        Err(StoreError::NotFound),
        "a visible public version cannot be revised by the same user ID in a foreign tenant"
    );
    assert_eq!(
        store
            .get_catalog_problem(
                context,
                ProblemVersionRef {
                    problem: public_problem,
                    version: foreign_revision_version,
                },
            )
            .await,
        Ok(None),
        "refused foreign revision creates no published successor"
    );
    assert!(
        store
            .get_draft(foreign_context, publisher, foreign_revision_workspace)
            .await
            .expect("foreign draft remains readable after refused publication")
            .is_some(),
        "refused foreign publication leaves the source draft untouched"
    );

    let revision_version = VersionId::from_uuid(uuid(311));
    let revision_workspace = WorkspaceId::from_uuid(uuid(312));
    let revision_draft = DraftRecord {
        tenant,
        question: draft_question(revision_workspace),
        revises: Some(ProblemVersionRef {
            problem: public_problem,
            version: public_version,
        }),
        derived_from: None,
    };
    let saved_revision_draft = store
        .upsert_draft(context, publisher, None, revision_draft.clone())
        .await
        .expect("revision draft should save");
    let revision = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: revision_draft,
                expected_revision: saved_revision_draft.revision,
                publication: ProblemVersionRef {
                    problem: public_problem,
                    version: revision_version,
                },
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("owned linear revision should publish");
    assert_eq!(revision.previous_version, Some(public_version));
    assert_eq!(revision.authors, vec![publisher]);

    assert_eq!(
        store
            .transition_catalog_problem(
                context,
                other_user,
                ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                },
                CatalogTransition::Deprecate {
                    reason: "Correction available".to_string(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );
    let deprecated = store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
            CatalogTransition::Deprecate {
                reason: " Correction available ".to_string(),
            },
        )
        .await
        .expect("author should deprecate");
    assert!(matches!(
        deprecated.lifecycle,
        CatalogLifecycle::Deprecated { ref reason } if reason == "Correction available"
    ));
    let exact_deprecated = store
        .get_catalog_problem(
            foreign_context,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        )
        .await
        .expect("exact deprecated lookup should run");
    assert!(
        exact_deprecated.is_some(),
        "existing references remain resolvable"
    );
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(315)),
                tenant,
                course_id: tenant_course,
                title: "Deprecated exact reference".to_string(),
                items: fixed_items(vec![ProblemVersionRef {
                    problem: public_problem,
                    version: public_version,
                }]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("a deprecated version remains assignable by exact reference");
    let browse_after_deprecation = store
        .list_catalog(
            foreign_context,
            PageRequest::first(PageSize::new(10).expect("valid page size")),
        )
        .await
        .expect("catalog should list");
    assert_eq!(browse_after_deprecation.items.len(), 1);
    assert_eq!(browse_after_deprecation.items[0].version, revision_version);

    let archived = store
        .transition_catalog_problem(
            context,
            publisher,
            ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
            CatalogTransition::Archive,
        )
        .await
        .expect("deprecated version should archive");
    assert!(matches!(
        archived.lifecycle,
        CatalogLifecycle::Archived { .. }
    ));
    assert!(matches!(
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(316)),
                    tenant,
                    course_id: tenant_course,
                    title: "Archived exact reference".to_string(),
                    items: fixed_items(vec![ProblemVersionRef {
                        problem: public_problem,
                        version: public_version,
                    }]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn memory_catalog_store_conforms() {
    exercise_catalog_store(&MemoryStore::default()).await;
}
