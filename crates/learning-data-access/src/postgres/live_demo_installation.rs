//! Restricted read of the completed live-demo baseline generation.

use async_trait::async_trait;
use sqlx::types::Uuid;

use super::{PostgresStore, map_sqlx_error};
use crate::{LiveDemoInstallationStore, StoreError};

#[async_trait]
impl LiveDemoInstallationStore for PostgresStore {
    async fn completed_live_demo_installation_generation(
        &self,
    ) -> Result<Option<Uuid>, StoreError> {
        let mut transaction = self.begin_auth().await?;
        let generation =
            sqlx::query_scalar("SELECT public.ple_completed_live_demo_installation_generation()")
                .fetch_one(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(generation)
    }
}
