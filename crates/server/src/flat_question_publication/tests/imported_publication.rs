use super::*;

#[tokio::test]
async fn imported_flat_publish_copies_verified_archive_and_promotes_origin() {
    let fixture = fixture().await;
    let imported = install_import_origin(&fixture).await;

    let (status, headers, body) = publish(&fixture, &imported.etag).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_no_store(&headers);
    assert_no_private_tokens(&body);
    let published = published_record(&fixture, &body).await;
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    let archive_object = published_import_archive_object_id(
        reference.problem,
        reference.version,
        imported.origin.import().import,
        imported.origin.source_archive().sha256,
    );
    let published_archive_key = ObjectKey::PublishedImportArchive {
        problem: reference.problem,
        version: reference.version,
        import: imported.origin.import().import,
        object: archive_object,
    };
    let copied = fixture
        .objects
        .get(&published_archive_key)
        .await
        .expect("published imported-flat archive is retained");
    assert_eq!(copied.bytes, imported.archive_bytes);
    assert_eq!(copied.record.id, archive_object);
    assert_eq!(
        copied.record.sha256,
        imported.origin.source_archive().sha256
    );
    assert_eq!(
        copied.record.size_bytes,
        imported.origin.source_archive().size_bytes
    );
    assert_eq!(copied.record.media_type, QTI_PROFILE_ARCHIVE_MEDIA_TYPE);
    assert_eq!(
        copied.record.license,
        imported.origin.source_archive().license
    );
    assert!(
        copied.record.provenance.contains("verified QTI"),
        "published provenance identifies the server-verified archive"
    );
    assert!(matches!(
        fixture
            .objects
            .signed_url(&published_archive_key, copied.record.created_at)
            .await,
        Err(ObjectStoreError::NotSignable)
    ));
    assert!(
        fixture
            .store
            .get_draft(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("imported-flat draft lookup after publish")
            .is_none()
    );
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("imported-flat source lookup after publish")
            .is_none()
    );
    assert!(
        fixture
            .store
            .workspace_flat_import_origin(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("imported-flat origin lookup after publish")
            .is_none(),
        "atomic publication consumes the current workspace origin"
    );
}

#[tokio::test]
async fn imported_archive_candidate_replay_accepts_only_the_exact_existing_object() {
    let exact = fixture().await;
    let exact_imported = install_import_origin(&exact).await;
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(id(20_001)),
        version: VersionId::from_uuid(id(20_002)),
    };
    let archive_object = published_import_archive_object_id(
        reference.problem,
        reference.version,
        exact_imported.origin.import().import,
        exact_imported.origin.source_archive().sha256,
    );
    let candidate_key = ObjectKey::PublishedImportArchive {
        problem: reference.problem,
        version: reference.version,
        import: exact_imported.origin.import().import,
        object: archive_object,
    };
    let existing = exact
        .objects
        .put(PutObject {
            key: candidate_key,
            bytes: exact_imported.archive_bytes.clone(),
            media_type: QTI_PROFILE_ARCHIVE_MEDIA_TYPE.to_string(),
            license: exact_imported.origin.source_archive().license.clone(),
            provenance: "published from verified QTI workspace import archive".to_string(),
            created_at: exact_imported.origin.source_archive().created_at,
        })
        .await
        .expect("seed the exact immutable archive candidate");
    let replay =
        prepare_flat_import_promotion(exact.objects.as_ref(), &exact_imported.origin, reference)
            .await
            .expect("an exact deterministic archive replay is accepted");
    assert_eq!(replay.published_archive(), &existing);

    let divergent = fixture().await;
    let divergent_imported = install_import_origin(&divergent).await;
    let divergent_object = published_import_archive_object_id(
        reference.problem,
        reference.version,
        divergent_imported.origin.import().import,
        divergent_imported.origin.source_archive().sha256,
    );
    divergent
        .objects
        .put(PutObject {
            key: ObjectKey::PublishedImportArchive {
                problem: reference.problem,
                version: reference.version,
                import: divergent_imported.origin.import().import,
                object: divergent_object,
            },
            bytes: divergent_imported.archive_bytes.clone(),
            media_type: QTI_PROFILE_ARCHIVE_MEDIA_TYPE.to_string(),
            license: divergent_imported.origin.source_archive().license.clone(),
            provenance: "divergent but otherwise valid archive provenance".to_string(),
            created_at: divergent_imported.origin.source_archive().created_at,
        })
        .await
        .expect("seed a divergent immutable archive collision");
    let response = prepare_flat_import_promotion(
        divergent.objects.as_ref(),
        &divergent_imported.origin,
        reference,
    )
    .await
    .expect_err("a divergent deterministic candidate must refuse");
    let (status, headers, body) = response_parts(response.into_response()).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);
    assert_no_private_tokens(&body);
    assert!(
        divergent
            .store
            .get_draft(divergent.context(), divergent.owner, divergent.workspace,)
            .await
            .expect("draft lookup after divergent replay refusal")
            .is_some(),
        "a replay refusal occurs before Store publication"
    );
}

#[tokio::test]
async fn missing_import_archive_refuses_before_publication_mutation_without_leaking_details() {
    let fixture = fixture().await;
    let imported = install_import_origin(&fixture).await;
    fixture
        .objects
        .delete(&imported.archive_key)
        .await
        .expect("remove imported archive to inject a broken reference");

    let (status, headers, body) = publish(&fixture, &imported.etag).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_no_store(&headers);
    assert_no_private_tokens(&body);
    assert_eq!(
        body,
        br#"{"error":"flat-question source changed; reload it"}"#
    );
    assert!(
        fixture
            .store
            .get_draft(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("draft lookup after missing archive refusal")
            .is_some(),
        "archive refusal occurs before the atomic Store publication"
    );
    assert!(
        fixture
            .store
            .flat_question_source(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("source lookup after missing archive refusal")
            .is_some()
    );
    assert!(
        fixture
            .store
            .workspace_flat_import_origin(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("origin lookup after missing archive refusal")
            .is_some()
    );
}
