use super::*;
use learning_data_access::{FlatQuestionAssetStore, WorkspaceFlatQuestionAsset};

fn workspace_image(
    workspace: WorkspaceId,
    asset: AssetId,
    object: ObjectId,
    bytes: &[u8],
) -> WorkspaceFlatQuestionAsset {
    let key = ObjectKey::WorkspaceQuestionAsset {
        workspace,
        asset,
        object,
    };
    WorkspaceFlatQuestionAsset::new(
        workspace,
        asset,
        ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(bytes),
            size_bytes: u64::try_from(bytes.len()).expect("fixture size fits"),
            media_type: "image/png".to_string(),
            category: ObjectCategory::Asset,
            version: None,
            license: "CC BY 4.0".to_string(),
            provenance: "Instructor-uploaded cellular diagram".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        },
        1_280,
        720,
        "Cell membrane diagram".to_string(),
    )
    .expect("valid workspace image fixture")
}

async fn exercise_flat_question_asset_store<S>(store: &S)
where
    S: FlatQuestionAssetStore,
{
    let tenant = TenantId::from_uuid(uuid(81_001));
    let workspace = WorkspaceId::from_uuid(uuid(81_003));
    let foreign_workspace = WorkspaceId::from_uuid(uuid(81_004));
    let asset = AssetId::from_uuid(uuid(81_005));
    let descriptor = workspace_image(
        workspace,
        asset,
        ObjectId::from_uuid(uuid(81_006)),
        b"first immutable image",
    );
    let context = TenantContext::from_authenticated_session(tenant);

    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, descriptor.clone())
            .await,
        Ok(descriptor.clone()),
        "first registration retains the exact verified descriptor"
    );
    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, descriptor.clone())
            .await,
        Ok(descriptor.clone()),
        "an exact immutable registration retry is idempotent"
    );
    let conflicting = workspace_image(
        workspace,
        asset,
        ObjectId::from_uuid(uuid(81_007)),
        b"different immutable image",
    );
    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, conflicting)
            .await,
        Err(StoreError::Conflict),
        "one logical asset identity cannot be overwritten"
    );
    let mut changed_dimensions = descriptor.clone();
    changed_dimensions.intrinsic_width = 1_024;
    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, changed_dimensions)
            .await,
        Err(StoreError::Conflict),
        "intrinsic dimensions are bound to the immutable asset descriptor"
    );
    assert_eq!(
        store
            .resolve_workspace_flat_question_asset(
                context,
                workspace,
                asset,
                descriptor.checksum(),
            )
            .await,
        Ok(Some(descriptor.clone())),
        "only the exact persisted checksum resolves"
    );
    assert_eq!(
        store
            .resolve_workspace_flat_question_asset(
                context,
                workspace,
                asset,
                Sha256Digest::compute(b"claimed browser checksum"),
            )
            .await,
        Ok(None),
        "a caller-supplied checksum cannot select a different object"
    );
    assert_eq!(
        store
            .list_workspace_flat_question_assets(context, foreign_workspace)
            .await,
        Ok(Vec::new()),
        "a different workspace has an independent asset namespace"
    );
    assert!(
        !descriptor.object.key.may_issue_signed_url(),
        "workspace-question images are private and cannot be signed directly"
    );
}

#[tokio::test]
async fn memory_flat_question_assets_are_immutable_private_and_checksum_bound() {
    exercise_flat_question_asset_store(&MemoryStore::default()).await;
}

#[tokio::test]
async fn memory_flat_question_asset_lists_are_deterministic() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(81_101));
    let workspace = WorkspaceId::from_uuid(uuid(81_102));
    let context = TenantContext::from_authenticated_session(tenant);
    let high = AssetId::from_uuid(uuid(81_104));
    let low = AssetId::from_uuid(uuid(81_103));
    for (asset, object) in [
        (high, ObjectId::from_uuid(uuid(81_105))),
        (low, ObjectId::from_uuid(uuid(81_106))),
    ] {
        store
            .register_workspace_flat_question_asset(
                context,
                workspace_image(workspace, asset, object, asset.as_uuid().as_bytes()),
            )
            .await
            .expect("image registration");
    }
    let listed = store
        .list_workspace_flat_question_assets(context, workspace)
        .await
        .expect("ordered workspace assets");
    assert_eq!(
        listed
            .iter()
            .map(|descriptor| descriptor.asset)
            .collect::<Vec<_>>(),
        vec![low, high],
        "BTree-backed workspace descriptors have stable logical-asset order"
    );
}

#[test]
fn flat_question_asset_descriptor_refuses_untrusted_metadata() {
    let tenant = TenantId::from_uuid(uuid(81_201));
    let workspace = WorkspaceId::from_uuid(uuid(81_202));
    let asset = AssetId::from_uuid(uuid(81_203));
    let object = ObjectId::from_uuid(uuid(81_204));
    let mut invalid = workspace_image(workspace, asset, object, b"valid bytes");
    invalid.object.media_type = "image/svg+xml".to_string();
    assert!(
        invalid.validate().is_err(),
        "active SVG is not a hotspot surface"
    );
    let mut invalid_dimensions = workspace_image(workspace, asset, object, b"valid bytes");
    invalid_dimensions.intrinsic_width = 0;
    assert!(
        invalid_dimensions.validate().is_err(),
        "zero dimensions are refused"
    );
    let mut invalid_provenance = workspace_image(workspace, asset, object, b"valid bytes");
    invalid_provenance.object.provenance = "untrusted\nline break".to_string();
    assert!(
        invalid_provenance.validate().is_err(),
        "unsafe provenance is refused"
    );
}
