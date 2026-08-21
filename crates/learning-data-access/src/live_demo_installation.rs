//! Read-only live-demo baseline lifecycle projection.

use async_trait::async_trait;
use uuid::Uuid;

use crate::StoreError;

/// Reads the generation of the one durable, completed live-demo baseline.
///
/// `None` deliberately combines an absent and an in-progress installation.
/// Callers receive no lifecycle metadata and cannot mutate the lifecycle.
#[async_trait]
pub trait LiveDemoInstallationStore: Send + Sync {
    async fn completed_live_demo_installation_generation(&self)
    -> Result<Option<Uuid>, StoreError>;
}
