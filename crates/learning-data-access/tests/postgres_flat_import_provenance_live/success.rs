use super::*;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_flat_import_conversion_edit_and_publication_are_atomic_and_private() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let grader_url = std::env::var("PLE_TEST_GRADER_DATABASE_URL")
        .expect("PLE_TEST_GRADER_DATABASE_URL must name the disposable grader connection");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let grader = PostgresGraderStore::connect(&grader_url)
        .await
        .expect("dedicated grader credentials are accepted");

    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let actor = UserId::from_uuid(id());
    let workspace = WorkspaceId::from_uuid(id());
    let import = WorkspaceImportId::from_uuid(id());
    let import_reference = QtiImportRef {
        tenant,
        workspace,
        import,
    };
    let choice_map = FlatImportChoiceMapPayload::from_canonical_bytes(
        b"ple-flat-import-choice-map-v1:blue=canvas-blue:red=canvas-red".to_vec(),
    )
    .expect("bounded private choice-map fixture");
    let digests = integrity_digests(&choice_map);
    let archive = workspace_archive(tenant, workspace, import);
    let imported = flat_fixture(tenant, workspace, IMPORTED_FLAT_SOURCE);
    let initial_workspace = store
        .upsert_draft(context, actor, None, imported.draft.clone())
        .await
        .expect("save the author workspace before preparing its QTI import");
    let import_command = import_command(import_reference, archive, digests);
    commit_import(&store, &pool, context, &import_command, digests).await;

    let exact_origin = import_origin(
        &import_command,
        actor,
        imported.source.sha256,
        digests,
        choice_map.clone(),
    );
    let before_refusal = workspace_mutation_snapshot(&pool, tenant, workspace).await;
    let mut changed_digests = digests;
    changed_digests.warning_sha256 = Sha256Digest::compute(b"changed warning evidence");
    let changed_origin = import_origin(
        &import_command,
        actor,
        imported.source.sha256,
        changed_digests,
        choice_map,
    );
    assert_eq!(
        store
            .convert_qti_profile_item_to_flat(
                context,
                actor,
                conversion_command(&imported, initial_workspace.revision, changed_origin),
            )
            .await,
        Err(StoreError::Conflict),
        "changed committed evidence refuses the whole conversion"
    );
    assert_eq!(
        workspace_mutation_snapshot(&pool, tenant, workspace).await,
        before_refusal,
        "refused conversion leaves draft, source, grading, origin, and private map unchanged"
    );

    let converted = store
        .convert_qti_profile_item_to_flat(
            context,
            actor,
            conversion_command(&imported, initial_workspace.revision, exact_origin.clone()),
        )
        .await
        .expect("exact committed profile evidence converts atomically");
    let current_origin = store
        .workspace_flat_import_origin(context, actor, workspace)
        .await
        .expect("authorized current-origin read")
        .expect("converted workspace has current provenance");
    assert!(current_origin == exact_origin);
    let imported_grading = current_grading_snapshot(&pool, tenant, workspace)
        .await
        .expect("conversion atomically stages current private grading");
    assert_eq!(
        imported_grading,
        CurrentGradingSnapshot {
            draft_revision: i64::try_from(converted.workspace_revision.value())
                .expect("live revision fits PostgreSQL bigint"),
            source_object_id: imported.source.id.as_uuid(),
            canonical_source_sha256: imported.source.sha256.to_string(),
            public_binding_sha256: imported.grading.public_binding_sha256().to_string(),
            key_sha256: imported.grading.sha256().to_string(),
        }
    );
    assert!(
        store
            .workspace_flat_import_origin(foreign_context, actor, workspace)
            .await
            .expect("foreign origin read is non-enumerating")
            .is_none()
    );
    for role in ["ple_app", "ple_student", "ple_grader"] {
        assert_private_relation_denied(&pool, role, tenant, PrivateRelation::CurrentChoiceMap)
            .await;
    }
    for role in ["ple_app", "ple_student"] {
        assert_private_relation_denied(&pool, role, tenant, PrivateRelation::CurrentGrading).await;
    }

    let edited = flat_fixture(tenant, workspace, EDITED_FLAT_SOURCE);
    let edited_source = store
        .upsert_flat_question(
            context,
            actor,
            UpsertFlatQuestionCommand {
                expected_revision: Some(converted.workspace_revision),
                draft: edited.draft.clone(),
                source: edited.source.clone(),
                canonical_source_sha256: edited.source.sha256.to_string(),
                public_binding_sha256: edited.grading.public_binding_sha256().to_string(),
                grading: edited.grading.clone(),
            },
        )
        .await
        .expect("ordinary editor save replaces the flat source");
    assert_ne!(
        edited_source.canonical_source_sha256,
        current_origin.mapped_canonical_source_sha256().to_string()
    );
    let preserved_origin = store
        .workspace_flat_import_origin(context, actor, workspace)
        .await
        .expect("origin read after ordinary edit")
        .expect("ordinary edit preserves current origin");
    assert!(preserved_origin == current_origin);
    let edited_grading = current_grading_snapshot(&pool, tenant, workspace)
        .await
        .expect("ordinary edit atomically replaces current private grading");
    assert_eq!(
        edited_grading,
        CurrentGradingSnapshot {
            draft_revision: i64::try_from(edited_source.workspace_revision.value())
                .expect("live revision fits PostgreSQL bigint"),
            source_object_id: edited.source.id.as_uuid(),
            canonical_source_sha256: edited.source.sha256.to_string(),
            public_binding_sha256: edited.grading.public_binding_sha256().to_string(),
            key_sha256: edited.grading.sha256().to_string(),
        }
    );
    assert_ne!(edited_grading.key_sha256, imported_grading.key_sha256);

    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id()),
        version: VersionId::from_uuid(id()),
    };
    let published_archive = published_import_archive(reference, &preserved_origin);
    let import_promotion = FlatImportPublicationPromotion::new(
        &preserved_origin,
        reference,
        published_archive.clone(),
    )
    .expect("published archive exactly matches the current origin");
    let before_publication_refusals =
        workspace_publication_snapshot(&pool, tenant, workspace).await;
    for (command, reason) in [
        (
            publication_command(
                &edited,
                edited_source.workspace_revision,
                reference,
                None,
                actor,
            ),
            "caller cannot omit the locked flat source and stored-grading selector",
        ),
        (
            publication_command(
                &edited,
                edited_source.workspace_revision,
                reference,
                Some(FlatQuestionPublicationPromotion {
                    source: converted,
                    import_origin: Some(import_promotion.clone()),
                    published_question: edited.draft.question.clone(),
                    assets: Vec::new(),
                }),
                actor,
            ),
            "caller cannot select stale source metadata to invent a grading binding",
        ),
    ] {
        assert!(
            store.publish_draft(context, actor, command).await.is_err(),
            "{reason}"
        );
        assert_eq!(
            workspace_publication_snapshot(&pool, tenant, workspace).await,
            before_publication_refusals,
            "{reason}; refusal leaves draft, source, origin, and grading unchanged"
        );
    }
    let published = store
        .publish_draft(
            context,
            actor,
            publication_command(
                &edited,
                edited_source.workspace_revision,
                reference,
                Some(FlatQuestionPublicationPromotion {
                    source: edited_source,
                    import_origin: Some(import_promotion),
                    published_question: edited.draft.question.clone(),
                    assets: Vec::new(),
                }),
                actor,
            ),
        )
        .await
        .expect("matching flat publication copies immutable import provenance");
    assert_eq!(published.problem, reference.problem);
    assert!(
        store
            .get_catalog_problem(foreign_context, reference)
            .await
            .expect("foreign catalog lookup is non-enumerating")
            .is_none()
    );
    assert!(
        store
            .workspace_flat_import_origin(context, actor, workspace)
            .await
            .expect("current origin lookup after publication")
            .is_none(),
        "publication removes current-only provenance with the draft"
    );

    let published_row = sqlx::query(
        "SELECT source_import_id, source_archive_sha256, mapped_canonical_source_sha256, \
                published_archive_object_id \
           FROM public.published_flat_import_origin \
          WHERE owner_tenant_id = $1 AND problem_id = $2 AND version_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("migration owner verifies immutable published origin");
    assert_eq!(
        published_row.get::<Uuid, _>("source_import_id"),
        import.as_uuid()
    );
    assert_eq!(
        published_row
            .get::<String, _>("source_archive_sha256")
            .trim_end(),
        import_command.registry.source.sha256.to_string()
    );
    assert_eq!(
        published_row
            .get::<String, _>("mapped_canonical_source_sha256")
            .trim_end(),
        imported.source.sha256.to_string(),
        "published lineage retains the imported source digest after an ordinary edit"
    );
    assert_eq!(
        published_row.get::<Uuid, _>("published_archive_object_id"),
        published_archive.id.as_uuid()
    );
    let published_choice_map: Vec<u8> = sqlx::query_scalar(
        "SELECT payload FROM public.published_flat_import_choice_map \
          WHERE owner_tenant_id = $1 AND problem_id = $2 AND version_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(reference.problem.as_uuid())
    .bind(reference.version.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("migration owner verifies copied private choice map");
    assert_eq!(
        Sha256Digest::compute(&published_choice_map),
        digests.choice_map_sha256
    );
    assert_eq!(
        workspace_mutation_snapshot(&pool, tenant, workspace).await,
        WorkspaceMutationSnapshot {
            drafts: 0,
            access_bindings: 0,
            flat_sources: 0,
            current_gradings: 0,
            current_origins: 0,
            current_choice_maps: 0,
        },
        "publication removes only workspace staging"
    );

    let published_grading = grader
        .flat_question_published_grading(context, reference)
        .await
        .expect("dedicated grader reads edited flat grading")
        .expect("published flat grading exists");
    assert_eq!(published_grading.sha256(), edited.grading.sha256());
    assert!(
        grader
            .flat_question_published_grading(foreign_context, reference)
            .await
            .expect("foreign grader lookup is non-enumerating")
            .is_none()
    );
    for role in ["ple_app", "ple_student", "ple_grader"] {
        assert_private_relation_denied(&pool, role, tenant, PrivateRelation::PublishedChoiceMap)
            .await;
    }
    assert_published_origin_immutable(&pool, tenant, reference).await;
}
