use super::*;

/// The draft-to-publication boundary is deliberately exercised against every
/// Store implementation. These are permanent behavior tests: a failed
/// publication must not consume tenant-owned authoring state, and only the
/// caller that owns a visible lineage may mint its next immutable version.
pub(super) async fn exercise_publication_identity_boundary<S>(store: &S)
where
    S: Store + CatalogStore + OwnerCorrectionStore + SessionStore,
{
    let tenant = TenantId::from_uuid(uuid(600));
    let foreign_tenant = TenantId::from_uuid(uuid(601));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(602));
    let correction_session = SessionTokenHash::compute(b"publication-correction-owner");
    store
        .create_session(
            correction_session,
            SessionSubject::new(
                tenant,
                publisher,
                "Correction owner",
                vec![UserRole::Instructor],
            )
            .expect("correction owner session subject should be valid"),
            SessionLifetime::from_seconds(3_600).expect("positive session lifetime"),
        )
        .await
        .expect("correction owner session should persist");
    let foreign_author = UserId::from_uuid(uuid(603));
    let capabilities = BackendCapabilities::from_iter([Capability::ServerGrading]);

    let stale_workspace = WorkspaceId::from_uuid(uuid(604));
    let stored_stale_draft = DraftRecord {
        tenant,
        question: draft_question(stale_workspace),
        revises: None,
        derived_from: None,
    };
    let stored_stale = store
        .upsert_draft(context, publisher, None, stored_stale_draft.clone())
        .await
        .expect("stale-publication fixture draft should save");
    let mut stale_expected_draft = stored_stale_draft.clone();
    stale_expected_draft.question.metadata.title = "Changed after validation".to_string();
    let stale_publication = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(605)),
        version: VersionId::from_uuid(uuid(606)),
    };
    assert_eq!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: stale_expected_draft,
                    expected_revision: stored_stale.revision,
                    publication: stale_publication,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stale expected draft must not publish"
    );
    assert_eq!(
        store
            .get_draft(context, publisher, stale_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(stored_stale_draft)),
        "a stale publication failure must preserve the exact stored draft"
    );
    assert_eq!(
        store.get_catalog_problem(context, stale_publication).await,
        Ok(None),
        "a stale publication failure must not leave an immutable version"
    );

    let base_workspace = WorkspaceId::from_uuid(uuid(607));
    let base_draft = DraftRecord {
        tenant,
        question: draft_question(base_workspace),
        revises: None,
        derived_from: None,
    };
    let base = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(608)),
        version: VersionId::from_uuid(uuid(609)),
    };
    let saved_base_draft = store
        .upsert_draft(context, publisher, None, base_draft.clone())
        .await
        .expect("base draft should save");
    let base_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: base_draft,
                expected_revision: saved_base_draft.revision,
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
        .expect("new work should mint a fresh published problem and version");
    assert_eq!(
        (base_record.problem, base_record.version),
        (base.problem, base.version)
    );
    assert_eq!(base_record.previous_version, None);
    assert_eq!(base_record.derived_from, None);

    let correction_course = CourseId::from_uuid(uuid(627));
    let correction_assignment = AssignmentId::from_uuid(uuid(628));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: correction_course,
                tenant,
                title: "Correction propagation".to_string(),
                members: vec![CourseMembership {
                    user: publisher,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("correction fixture course should save");
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: correction_assignment,
                tenant,
                course_id: correction_course,
                title: "Uses the current question".to_string(),
                items: fixed_items(vec![base]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("correction fixture assignment should save");

    let fork_workspace = WorkspaceId::from_uuid(uuid(610));
    let fork_draft = DraftRecord {
        tenant,
        question: draft_question(fork_workspace),
        revises: None,
        derived_from: Some(base),
    };
    let fork = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(611)),
        version: VersionId::from_uuid(uuid(612)),
    };
    let saved_fork_draft = store
        .upsert_draft(context, publisher, None, fork_draft.clone())
        .await
        .expect("fork draft should save");
    let fork_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: fork_draft,
                expected_revision: saved_fork_draft.revision,
                publication: fork,
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
        .expect("fork should mint a fresh problem and version");
    assert_ne!(fork_record.problem, base.problem);
    assert_ne!(fork_record.version, base.version);
    assert_eq!(fork_record.previous_version, None);
    assert_eq!(fork_record.derived_from, Some(base));

    let revision_workspace = WorkspaceId::from_uuid(uuid(613));
    let revision_draft = DraftRecord {
        tenant,
        question: draft_question(revision_workspace),
        revises: Some(base),
        derived_from: None,
    };
    let revision = ProblemVersionRef {
        problem: base.problem,
        version: VersionId::from_uuid(uuid(614)),
    };
    let saved_revision_draft = store
        .upsert_draft(context, publisher, None, revision_draft.clone())
        .await
        .expect("revision draft should save");
    let revision_record = store
        .publish_owner_correction(
            context,
            OwnerCorrectionAuthority {
                actor: publisher,
                session: correction_session,
            },
            PublishDraftCommand {
                expected_draft: revision_draft,
                expected_revision: saved_revision_draft.revision,
                publication: revision,
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
        .expect("owned revision should preserve its problem and mint a version");
    assert_eq!(revision_record.problem, base.problem);
    assert_ne!(revision_record.version, base.version);
    assert_eq!(revision_record.previous_version, Some(base.version));
    assert_eq!(revision_record.public_id, base_record.public_id);
    assert_eq!(revision_record.question_id, base_record.question_id);
    assert_eq!(base_record.version_number.value(), 1);
    assert_eq!(revision_record.version_number.value(), 2);
    assert_ne!(fork_record.public_id, base_record.public_id);
    assert_ne!(fork_record.question_id, base_record.question_id);
    assert_eq!(fork_record.version_number.value(), 1);
    let propagated = store
        .get_assignment_for_edit(context, correction_assignment)
        .await
        .expect("propagated assignment lookup should succeed")
        .expect("propagated assignment should remain present");
    assert_eq!(propagated.revision.value(), 2);
    assert_eq!(propagated.record.items[0].reference, revision);
    assert_eq!(
        store
            .resolve_catalog_problem(
                context,
                question_model::ProblemDisplayRef {
                    question_id: base_record.question_id.clone(),
                },
            )
            .await
            .expect("Question ID lookup should succeed")
            .map(|record| record.version),
        Some(revision.version),
        "one Question ID resolves the current owner-corrected question"
    );

    let foreign_author_workspace = WorkspaceId::from_uuid(uuid(615));
    let foreign_author_draft = DraftRecord {
        tenant,
        question: draft_question(foreign_author_workspace),
        revises: Some(revision),
        derived_from: None,
    };
    let saved_foreign_author_draft = store
        .upsert_draft(context, foreign_author, None, foreign_author_draft.clone())
        .await
        .expect("foreign-author draft should save before refusal");
    assert_eq!(
        store
            .publish_draft(
                context,
                foreign_author,
                PublishDraftCommand {
                    expected_draft: foreign_author_draft.clone(),
                    expected_revision: saved_foreign_author_draft.revision,
                    publication: ProblemVersionRef {
                        problem: base.problem,
                        version: VersionId::from_uuid(uuid(616)),
                    },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher: foreign_author,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::Forbidden),
        "a non-author must not extend an owned revision chain"
    );
    assert_eq!(
        store
            .get_draft(context, foreign_author, foreign_author_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(foreign_author_draft)),
        "a forbidden revision must retain its draft"
    );

    let mismatch_workspace = WorkspaceId::from_uuid(uuid(617));
    let mismatch_draft = DraftRecord {
        tenant,
        question: draft_question(mismatch_workspace),
        revises: Some(revision),
        derived_from: None,
    };
    let saved_mismatch_draft = store
        .upsert_draft(context, publisher, None, mismatch_draft.clone())
        .await
        .expect("reference-mismatch draft should save");
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: mismatch_draft.clone(),
                    expected_revision: saved_mismatch_draft.revision,
                    publication: ProblemVersionRef {
                        problem: ProblemId::from_uuid(uuid(618)),
                        version: VersionId::from_uuid(uuid(619)),
                    },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    assert_eq!(
        store
            .get_draft(context, publisher, mismatch_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(mismatch_draft)),
        "a reference mismatch must not consume a draft"
    );

    let foreign_tenant_workspace = WorkspaceId::from_uuid(uuid(620));
    let foreign_tenant_draft = DraftRecord {
        tenant,
        question: draft_question(foreign_tenant_workspace),
        revises: None,
        derived_from: None,
    };
    let saved_foreign_tenant_draft = store
        .upsert_draft(context, publisher, None, foreign_tenant_draft.clone())
        .await
        .expect("tenant-mismatch draft should save");
    assert_eq!(
        store
            .publish_draft(
                foreign_context,
                publisher,
                PublishDraftCommand {
                    expected_draft: foreign_tenant_draft.clone(),
                    expected_revision: saved_foreign_tenant_draft.revision,
                    publication: ProblemVersionRef {
                        problem: ProblemId::from_uuid(uuid(621)),
                        version: VersionId::from_uuid(uuid(622)),
                    },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::TenantMismatch),
        "a foreign tenant cannot publish another tenant's draft"
    );
    assert_eq!(
        store
            .get_draft(context, publisher, foreign_tenant_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(foreign_tenant_draft)),
        "a tenant mismatch must retain the owner's draft"
    );

    let imathas_workspace = WorkspaceId::from_uuid(uuid(623));
    let imathas_draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            source: DraftQuestionSource::Imathas {
                provider: "myopenmath".to_string(),
                item_ref: "4711".to_string(),
            },
            ..draft_question(imathas_workspace)
        },
        revises: None,
        derived_from: None,
    };
    let imathas_publication = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(624)),
        version: VersionId::from_uuid(uuid(625)),
    };
    let saved_imathas_draft = store
        .upsert_draft(context, publisher, None, imathas_draft.clone())
        .await
        .expect("iMathAS draft should save in the sandbox");
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: imathas_draft.clone(),
                    expected_revision: saved_imathas_draft.revision,
                    publication: imathas_publication,
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Public,
                    capabilities: capabilities.clone(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_draft(context, publisher, imathas_workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(imathas_draft.clone())),
        "an unprepared iMathAS source must not consume the sandbox draft"
    );
    let prepared_imathas_artifact = source_artifact(
        imathas_publication,
        QuestionBackend::Imathas,
        ObjectId::from_uuid(uuid(626)),
    );
    let prepared_imathas_source = QuestionSource::Imathas {
        provider: "myopenmath".to_string(),
        item_ref: "4711".to_string(),
        snapshot: ObjectId::from_uuid(uuid(626)),
        snapshot_sha256: prepared_imathas_artifact.object.sha256.to_string(),
        integration_profile: "lti-1.3".to_string(),
    };
    let imathas_record = store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: imathas_draft,
                expected_revision: saved_imathas_draft.revision,
                publication: imathas_publication,
                published_source: prepared_imathas_source,
                source_artifact: Some(prepared_imathas_artifact),
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities,
            },
        )
        .await
        .expect("a server-prepared iMathAS snapshot should persist");
    assert!(matches!(
        imathas_record.question.source,
        QuestionSource::Imathas { .. }
    ));
}

pub(super) async fn exercise_source_artifact_binding<S>(store: &S)
where
    S: Store + CatalogStore + CatalogSourceStore,
{
    let tenant = TenantId::from_uuid(uuid(6_500));
    let foreign_tenant = TenantId::from_uuid(uuid(6_501));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(6_502));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(6_503)),
        version: VersionId::from_uuid(uuid(6_504)),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            source: DraftQuestionSource::Qti {
                item_id: "item-1".to_string(),
                import_id: WorkspaceImportId::from_uuid(uuid(6_506)),
            },
            ..draft_question(WorkspaceId::from_uuid(uuid(6_505)))
        },
        revises: None,
        derived_from: None,
    };
    let saved_draft = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("source-backed draft should save");
    let artifact = source_artifact(
        reference,
        QuestionBackend::Qti,
        ObjectId::from_uuid(uuid(6_507)),
    );
    let source = QuestionSource::Qti {
        item_id: "item-1".to_string(),
        package_object: artifact.object.id,
        package_sha256: artifact.object.sha256.to_string(),
    };
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft.clone(),
                    expected_revision: saved_draft.revision,
                    publication: reference,
                    published_source: source.clone(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store
            .get_draft(context, publisher, draft.question.workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(draft.clone()))
    );
    assert_eq!(
        store.catalog_source_artifact(context, reference).await,
        Ok(None)
    );

    let mismatched_item = QuestionSource::Qti {
        item_id: "other-item".to_string(),
        package_object: artifact.object.id,
        package_sha256: artifact.object.sha256.to_string(),
    };
    let mismatched_object = QuestionSource::Qti {
        item_id: "item-1".to_string(),
        package_object: ObjectId::from_uuid(uuid(6_508)),
        package_sha256: artifact.object.sha256.to_string(),
    };
    let mismatched_checksum = QuestionSource::Qti {
        item_id: "item-1".to_string(),
        package_object: artifact.object.id,
        package_sha256: "a".repeat(64),
    };
    for invalid_source in [mismatched_item, mismatched_object, mismatched_checksum] {
        assert!(matches!(
            store
                .publish_draft(
                    context,
                    publisher,
                    PublishDraftCommand {
                        expected_draft: draft.clone(),
                        expected_revision: saved_draft.revision,
                        publication: reference,
                        published_source: invalid_source,
                        source_artifact: Some(artifact.clone()),
                        qti_promotion: None,
                        flat_question_promotion: None,
                        publisher,
                        scope: PublicationScope::Institution,
                        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    let mut wrong_backend = artifact.clone();
    wrong_backend.backend = QuestionBackend::Webwork;
    let mut wrong_reference = artifact.clone();
    wrong_reference.reference.version = VersionId::from_uuid(uuid(6_509));
    let mut wrong_category = artifact.clone();
    wrong_category.object.key = ObjectKey::ProblemAsset {
        problem: reference.problem,
        version: reference.version,
        asset: AssetId::from_uuid(uuid(6_510)),
        object: wrong_category.object.id,
    };
    wrong_category.object.category = objects::ObjectCategory::Asset;
    for invalid in [wrong_backend, wrong_reference, wrong_category] {
        assert!(matches!(
            store
                .publish_draft(
                    context,
                    publisher,
                    PublishDraftCommand {
                        expected_draft: draft.clone(),
                        expected_revision: saved_draft.revision,
                        publication: reference,
                        published_source: source.clone(),
                        source_artifact: Some(invalid),
                        qti_promotion: None,
                        flat_question_promotion: None,
                        publisher,
                        scope: PublicationScope::Institution,
                        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                    },
                )
                .await,
            Err(StoreError::InvalidRecord(_))
        ));
    }
    assert_eq!(
        store
            .get_draft(context, publisher, draft.question.workspace)
            .await
            .map(|draft| draft.map(|draft| draft.record)),
        Ok(Some(draft.clone()))
    );
    assert_eq!(
        store.catalog_source_artifact(context, reference).await,
        Ok(None)
    );
    assert_eq!(
        store.get_catalog_problem(context, reference).await,
        Ok(None),
        "a rejected source binding must not create a visible immutable version"
    );
    assert!(matches!(
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved_draft.revision,
                    publication: reference,
                    published_source: source,
                    source_artifact: Some(artifact.clone()),
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope: PublicationScope::Institution,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert_eq!(
        store.catalog_source_artifact(context, reference).await,
        Ok(None),
        "generic publication must not expose a QTI source binding"
    );
    assert_eq!(
        store
            .catalog_source_artifact(foreign_context, reference)
            .await,
        Ok(None),
        "foreign tenant must not learn a private source exists"
    );
}
