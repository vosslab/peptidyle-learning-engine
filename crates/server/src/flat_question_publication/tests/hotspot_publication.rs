//! HOTSPOT asset staging and versioned publication behavior.

use super::*;

const FORGED_HOTSPOT_SOURCE: &str = r#"{
  "format":"pleFlatQuestion", "version":2, "title":"Protein surface",
  "prompt":"Select the active-site region.",
  "response":{"kind":"hotspot","surface":{"asset":"00000000-0000-0000-0000-000000009999","checksum":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","description":"Protein structure"},"regions":[{"id":"site","label":"Active site","x":1000,"y":1000,"width":2000,"height":2000}],"correctRegions":["site"]},
  "points":1.0, "attemptPolicy":{"maxAttempts":null},
  "timingPolicy":{"kind":"untimed"}, "license":{"kind":"cc0"}, "language":"en-US"
}"#;

async fn register_hotspot_asset(fixture: &Fixture) -> (AssetId, Vec<u8>) {
    let asset = AssetId::from_uuid(id(9_001));
    let object = ObjectId::from_uuid(id(9_002));
    let bytes = b"verified private hotspot image bytes".to_vec();
    let record = fixture
        .objects
        .put(PutObject {
            key: ObjectKey::WorkspaceQuestionAsset {
                workspace: fixture.workspace,
                asset,
                object,
            },
            bytes: bytes.clone(),
            media_type: "image/png".to_string(),
            license: "cc0".to_string(),
            provenance: "verified test hotspot image".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(100),
        })
        .await
        .expect("private hotspot image stores");
    fixture
        .store
        .register_workspace_flat_question_asset(
            fixture.context(),
            WorkspaceFlatQuestionAsset::new(
                fixture.workspace,
                asset,
                record,
                400,
                300,
                "Protein surface".to_string(),
            )
            .expect("verified hotspot descriptor"),
        )
        .await
        .expect("private hotspot descriptor registers");
    (asset, bytes)
}

fn hotspot_source(asset: AssetId, checksum: Sha256Digest) -> String {
    format!(
        r#"{{"format":"pleFlatQuestion","version":2,"title":"Protein surface","prompt":"Select the active site.","response":{{"kind":"hotspot","surface":{{"asset":"{asset}","checksum":"{checksum}","description":"Protein structure"}},"regions":[{{"id":"site","label":"Active site","x":1000,"y":1000,"width":2000,"height":2000}}],"correctRegions":["site"]}},"points":1.0,"attemptPolicy":{{"maxAttempts":null}},"timingPolicy":{{"kind":"untimed"}},"license":{{"kind":"cc0"}},"language":"en-US"}}"#
    )
}

async fn restage_hotspot_revision(
    fixture: &Fixture,
    source: &str,
    derived_from: ProblemVersionRef,
) {
    let document = FlatQuestionDocument::parse(source.as_bytes()).expect("revision source parses");
    let canonical = document
        .canonical_bytes()
        .expect("revision source canonicalizes");
    let (question, private) = document
        .compile(fixture.workspace)
        .expect("revision source compiles")
        .into_parts();
    let object = ObjectId::from_uuid(id(9_003));
    let source_record = fixture
        .objects
        .put(PutObject {
            key: ObjectKey::WorkspaceQuestionSource {
                workspace: fixture.workspace,
                object,
            },
            bytes: canonical.clone(),
            media_type: FLAT_QUESTION_MEDIA_TYPE.to_string(),
            license: "cc0".to_string(),
            provenance: "revision hotspot source".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(101),
        })
        .await
        .expect("revision source stores");
    fixture
        .store
        .upsert_flat_question(
            fixture.context(),
            fixture.owner,
            UpsertFlatQuestionCommand {
                expected_revision: None,
                draft: DraftRecord {
                    question,
                    derived_from: Some(derived_from),
                },
                source: source_record,
                canonical_source_sha256: Sha256Digest::compute(&canonical).to_string(),
                public_binding_sha256: private.public_binding_sha256().to_string(),
                grading: FlatQuestionGradingPayload::from_private(&private)
                    .expect("revision grading payload"),
            },
        )
        .await
        .expect("revision staging succeeds");
}

#[tokio::test]
async fn forged_or_unregistered_hotspot_asset_never_stages_a_draft() {
    let fixture = fixture().await;
    let (status, _, body) =
        save(&fixture, &fixture.owner_cookie, FORGED_HOTSPOT_SOURCE, None).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(
        fixture
            .store
            .get_draft(fixture.context(), fixture.owner, fixture.workspace)
            .await
            .expect("draft lookup succeeds")
            .is_none()
    );
}

#[tokio::test]
async fn hotspot_publication_copies_a_verified_private_image_to_a_fresh_catalog_asset() {
    let fixture = fixture().await;
    let (workspace_asset, bytes) = register_hotspot_asset(&fixture).await;
    let checksum = Sha256Digest::compute(&bytes);
    let source = hotspot_source(workspace_asset, checksum);
    let (save_status, save_headers, _) =
        save(&fixture, &fixture.owner_cookie, source.clone(), None).await;
    assert_eq!(save_status, StatusCode::OK);
    let etag = save_headers
        .get("etag")
        .expect("saved hotspot has a revision")
        .to_str()
        .expect("revision is header text");

    let (publish_status, _, body) = publish(&fixture, etag).await;
    assert_eq!(
        publish_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let published = published_record(&fixture, &body).await;
    let question_model::ResponseDefinition::Hotspot { surface, .. } = &published.question.response
    else {
        panic!("publication retains hotspot response");
    };
    assert_ne!(
        surface.asset, workspace_asset,
        "catalog asset is version-scoped"
    );
    assert_eq!(surface.checksum, checksum.to_string());
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    let binding = fixture
        .store
        .catalog_asset_bindings(fixture.context(), reference)
        .await
        .expect("catalog asset binding lookup");
    assert_eq!(binding.len(), 1);
    assert_eq!(binding[0].asset, surface.asset);
    let delivery = fixture
        .store
        .authorize_asset_delivery(
            fixture.context(),
            fixture.owner,
            learning_data_access::AssetDeliveryId::from_asset(surface.asset),
        )
        .await
        .expect("institution catalog asset delivery authorizes for owner")
        .record;
    assert_eq!(delivery.object.id, binding[0].object);
    assert_eq!(delivery.object.sha256, checksum);
    assert_eq!(
        delivery.object.key.bucket(),
        objects::Bucket::PrivateContent
    );
    assert_eq!(delivery.object.key.version_id(), Some(reference.version));
    assert!(
        matches!(delivery.object.key, ObjectKey::RestrictedProblemAsset { problem, version, asset, .. }
        if problem == reference.problem && version == reference.version && asset == surface.asset)
    );
    assert_eq!(
        fixture
            .objects
            .get(&delivery.object.key)
            .await
            .expect("published asset object")
            .bytes,
        bytes
    );
    let published_source = fixture
        .store
        .catalog_source_artifact(fixture.context(), reference)
        .await
        .expect("published source binding")
        .expect("published hotspot retains canonical source");
    let source_bytes = fixture
        .objects
        .get(&published_source.object.key)
        .await
        .expect("published source object")
        .bytes;
    let source_text = String::from_utf8(source_bytes).expect("canonical JSON source is UTF-8");
    assert!(source_text.contains(&surface.asset.to_string()));
    assert!(!source_text.contains(&workspace_asset.to_string()));
    let private = fixture
        .store
        .resolve_workspace_flat_question_asset(
            fixture.context(),
            fixture.workspace,
            workspace_asset,
            checksum,
        )
        .await
        .expect("private asset resolution")
        .expect("private asset persists after publication");
    assert_eq!(
        fixture
            .objects
            .get(&private.object.key)
            .await
            .expect("private object")
            .bytes,
        bytes
    );
    assert!(
        fixture
            .store
            .get_catalog_problem(fixture.context(), reference)
            .await
            .expect("published catalog lookup")
            .is_some()
    );

    restage_hotspot_revision(&fixture, &source, reference).await;
    let (source_status, revision_headers, _) =
        read_source(&fixture, Some(&fixture.owner_cookie)).await;
    assert_eq!(source_status, StatusCode::OK);
    let revision_etag = revision_headers
        .get("etag")
        .expect("restaged revision has ETag")
        .to_str()
        .expect("revision ETag text");
    let (revision_status, _, revision_body) = publish(&fixture, revision_etag).await;
    assert_eq!(revision_status, StatusCode::CREATED);
    let revised = published_record(&fixture, &revision_body).await;
    let question_model::ResponseDefinition::Hotspot {
        surface: revised_surface,
        ..
    } = &revised.question.response
    else {
        panic!("revision retains hotspot response");
    };
    assert_ne!(revised_surface.asset, surface.asset);
    let revised_reference = ProblemVersionRef {
        problem: revised.problem,
        version: revised.version,
    };
    for (version, asset) in [
        (reference, surface.asset),
        (revised_reference, revised_surface.asset),
    ] {
        let bindings = fixture
            .store
            .catalog_asset_bindings(fixture.context(), version)
            .await
            .expect("version bindings");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].asset, asset);
        let delivery = fixture
            .store
            .authorize_asset_delivery(
                fixture.context(),
                fixture.owner,
                learning_data_access::AssetDeliveryId::from_asset(asset),
            )
            .await
            .expect("each immutable version remains deliverable");
        assert_eq!(
            fixture
                .objects
                .get(&delivery.record.object.key)
                .await
                .expect("asset bytes")
                .bytes,
            bytes
        );
    }
}
