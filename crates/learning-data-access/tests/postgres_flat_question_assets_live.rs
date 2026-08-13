#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for private flat-question image descriptors.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    FlatQuestionAssetStore, StoreError, TenantContext, WorkspaceFlatQuestionAsset,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{ActivityTimestamp, AssetId, ObjectId, TenantId, WorkspaceId};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn descriptor(
    tenant: TenantId,
    workspace: WorkspaceId,
    asset: AssetId,
    object: ObjectId,
    bytes: &[u8],
) -> WorkspaceFlatQuestionAsset {
    let key = ObjectKey::WorkspaceQuestionAsset {
        tenant,
        workspace,
        asset,
        object,
    };
    WorkspaceFlatQuestionAsset::new(
        tenant,
        workspace,
        asset,
        ObjectRecord {
            id: object,
            bucket: key.bucket(),
            key,
            sha256: Sha256Digest::compute(bytes),
            size_bytes: u64::try_from(bytes.len()).expect("fixture bytes fit"),
            media_type: "image/png".to_string(),
            category: ObjectCategory::Asset,
            version: None,
            license: "CC BY 4.0".to_string(),
            provenance: "disposable PostgreSQL hotspot image".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1_000),
        },
        1_280,
        720,
        "Chromosome map".to_string(),
    )
    .expect("valid private workspace image descriptor")
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_flat_question_asset_registry_is_immutable_private_and_checksum_bound() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid disposable PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live database applies the asset registry migration");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let workspace = WorkspaceId::from_uuid(id());
    let foreign_workspace = WorkspaceId::from_uuid(id());
    let asset = AssetId::from_uuid(id());
    let record = descriptor(
        tenant,
        workspace,
        asset,
        ObjectId::from_uuid(id()),
        b"one verified private hotspot image",
    );
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);

    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, record.clone())
            .await,
        Ok(record.clone()),
        "first registration persists the exact descriptor"
    );
    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, record.clone())
            .await,
        Ok(record.clone()),
        "the same immutable descriptor is an idempotent retry"
    );
    let conflicting = descriptor(
        tenant,
        workspace,
        asset,
        ObjectId::from_uuid(id()),
        b"a different image cannot replace the asset",
    );
    assert_eq!(
        store
            .register_workspace_flat_question_asset(context, conflicting)
            .await,
        Err(StoreError::Conflict),
        "one logical asset identity cannot be overwritten"
    );
    assert_eq!(
        store
            .list_workspace_flat_question_assets(context, workspace)
            .await,
        Ok(vec![record.clone()]),
        "owner lists descriptors in deterministic logical-asset order"
    );
    assert_eq!(
        store
            .resolve_workspace_flat_question_asset(context, workspace, asset, record.checksum())
            .await,
        Ok(Some(record.clone())),
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
        "a mismatched claimed checksum is refused"
    );
    assert_eq!(
        store
            .list_workspace_flat_question_assets(foreign_context, workspace)
            .await,
        Ok(Vec::new()),
        "foreign tenant cannot enumerate a private workspace"
    );
    assert_eq!(
        store
            .resolve_workspace_flat_question_asset(
                foreign_context,
                workspace,
                asset,
                record.checksum(),
            )
            .await,
        Ok(None),
        "foreign tenant cannot resolve a known asset and checksum"
    );
    assert_eq!(
        store
            .list_workspace_flat_question_assets(context, foreign_workspace)
            .await,
        Ok(Vec::new()),
        "workspace namespaces remain independent"
    );

    let mut transaction = pool
        .begin()
        .await
        .expect("begin restricted-role transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("assume application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.as_uuid().to_string())
        .execute(&mut *transaction)
        .await
        .expect("set current tenant");
    let error = sqlx::query(
        "UPDATE public.workspace_flat_question_asset SET intrinsic_width = 1 \
         WHERE tenant_id = $1 AND workspace_id = $2 AND asset_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(workspace.as_uuid())
    .bind(asset.as_uuid())
    .execute(&mut *transaction)
    .await
    .expect_err("application role has no immutable-registry update path");
    assert_eq!(
        error
            .as_database_error()
            .and_then(|value| value.code())
            .as_deref(),
        Some("42501"),
        "least-privilege grants refuse direct mutation"
    );
    transaction
        .rollback()
        .await
        .expect("discard restricted-role probe");

    let row = sqlx::query(
        "SELECT relforcerowsecurity FROM pg_class \
         WHERE oid = 'public.workspace_flat_question_asset'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("read forced-RLS metadata");
    assert!(
        row.get::<bool, _>("relforcerowsecurity"),
        "owner paths cannot bypass tenant policy"
    );
}
