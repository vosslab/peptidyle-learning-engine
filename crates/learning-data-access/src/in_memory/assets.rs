//! In-memory immutable asset registration and protected delivery.

use async_trait::async_trait;
use question_model::{ProblemVersionRef, PublicationScope, UserId};

use super::{
    MemoryStore, catalog_record_visible, course_records_accessible,
    require_course_records_accessible,
};
use crate::{
    AssetAccessEvent, AssetDeliveryId, AssetDeliveryRecord, AssetDeliveryScope, AssetStore,
    AuthorizedAssetDelivery, CatalogAssetBinding, StoreError, TenantContext, ensure_tenant,
    validate_asset_delivery,
};

#[async_trait]
impl AssetStore for MemoryStore {
    async fn register_asset_delivery(
        &self,
        context: TenantContext,
        record: AssetDeliveryRecord,
    ) -> Result<(), StoreError> {
        validate_asset_delivery(&record)?;
        let mut state = self.write_state()?;
        match &record.scope {
            AssetDeliveryScope::Catalog { reference, .. } => {
                let published = state
                    .published
                    .get(&(reference.problem, reference.version))
                    .ok_or(StoreError::NotFound)?;
                if !catalog_record_visible(&state, context.tenant_id(), published) {
                    return Err(StoreError::NotFound);
                }
            }
            AssetDeliveryScope::StudentRecord { tenant, course, .. } => {
                ensure_tenant(context, *tenant)?;
                require_course_records_accessible(&state, *tenant, *course)?;
            }
            AssetDeliveryScope::CourseBanner { .. } => {
                return Err(StoreError::InvalidRecord(
                    "course-banner delivery registration is owned by appearance promotion"
                        .to_string(),
                ));
            }
        }
        if state.asset_deliveries.contains_key(&record.id) {
            return Err(StoreError::AlreadyExists);
        }
        state.asset_deliveries.insert(record.id, record);
        Ok(())
    }

    async fn get_public_asset_delivery(
        &self,
        delivery: AssetDeliveryId,
    ) -> Result<Option<AssetDeliveryRecord>, StoreError> {
        let state = self.read_state()?;
        let Some(record) = state.asset_deliveries.get(&delivery) else {
            return Ok(None);
        };
        let AssetDeliveryScope::Catalog { reference, .. } = record.scope else {
            return Ok(None);
        };
        Ok(state
            .published
            .get(&(reference.problem, reference.version))
            .filter(|published| published.scope == PublicationScope::Public)
            .map(|_| record.clone()))
    }

    async fn catalog_asset_bindings(
        &self,
        context: TenantContext,
        reference: ProblemVersionRef,
    ) -> Result<Vec<CatalogAssetBinding>, StoreError> {
        let state = self.read_state()?;
        let Some(published) = state.published.get(&(reference.problem, reference.version)) else {
            return Ok(Vec::new());
        };
        if !catalog_record_visible(&state, context.tenant_id(), published) {
            return Ok(Vec::new());
        }

        let mut bindings = state
            .asset_deliveries
            .values()
            .filter_map(|record| match record.scope {
                AssetDeliveryScope::Catalog {
                    asset,
                    reference: asset_reference,
                } if asset_reference == reference => Some(CatalogAssetBinding {
                    asset,
                    object: record.object.id,
                    rendition_checksum: record.object.sha256,
                    media_type: record.object.media_type.clone(),
                    intrinsic_width: record.intrinsic_width,
                    intrinsic_height: record.intrinsic_height,
                }),
                AssetDeliveryScope::Catalog { .. }
                | AssetDeliveryScope::StudentRecord { .. }
                | AssetDeliveryScope::CourseBanner { .. } => None,
            })
            .collect::<Vec<_>>();
        bindings.sort_unstable_by_key(|binding| binding.asset);
        Ok(bindings)
    }

    async fn authorize_asset_delivery(
        &self,
        context: TenantContext,
        actor: UserId,
        delivery: AssetDeliveryId,
    ) -> Result<AuthorizedAssetDelivery, StoreError> {
        let mut state = self.write_state()?;
        let record = state
            .asset_deliveries
            .get(&delivery)
            .cloned()
            .ok_or(StoreError::NotFound)?;
        let authorized = match &record.scope {
            AssetDeliveryScope::Catalog { reference, .. } => state
                .published
                .get(&(reference.problem, reference.version))
                .is_some_and(|published| {
                    catalog_record_visible(&state, context.tenant_id(), published)
                }),
            AssetDeliveryScope::StudentRecord {
                tenant,
                course,
                authorized_users,
            } => {
                *tenant == context.tenant_id()
                    && course_records_accessible(&state, *tenant, *course)
                    && authorized_users.contains(&actor)
            }
            AssetDeliveryScope::CourseBanner { .. } => false,
        };
        if !authorized {
            return Err(StoreError::NotFound);
        }
        let authorized_at = state.authoritative_time;
        state.asset_access_events.push(AssetAccessEvent {
            tenant: context.tenant_id(),
            actor,
            delivery,
            object: record.object.id,
            bucket: record.object.bucket,
            course: match record.scope {
                AssetDeliveryScope::Catalog { .. } => None,
                AssetDeliveryScope::StudentRecord { course, .. } => Some(course),
                AssetDeliveryScope::CourseBanner { course, .. } => Some(course),
            },
            occurred_at: authorized_at,
        });
        Ok(AuthorizedAssetDelivery {
            record,
            authorized_at,
        })
    }
}
