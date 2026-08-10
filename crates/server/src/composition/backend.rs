//! Concrete production storage and grading backend assembly.

use super::router::{HealthState, compose_router, verify_application_schema_bounded};
use super::settings::{LazyStorageDependencies, ProductionSettings};
use super::*;

/// Concrete dependency construction for the future institution adapter.
/// It contains only replica-safe backends.
// Until an institution adapter implements `IdentityProvider`, the normal
// binary intentionally fails before it can call `router`. These fields remain
// available for that adapter and the route-level integration boundary.
#[allow(dead_code)]
pub(super) struct PersistentDependencies {
    store: Arc<PostgresStore>,
    /// This capability is retained only for server-owned grading backends.
    /// It is never included in route state or browser-facing APIs.
    grader: Arc<PostgresGraderStore>,
    objects: Arc<objects::s3::S3ObjectStore>,
    public_assets: Arc<PublicAssetBaseUrl>,
    webwork_renderer: Option<HttpWebworkRenderer>,
    imathas: Option<ConfiguredImathas>,
    qti: Option<Arc<ProductionQtiBackend>>,
    health: Arc<HealthState>,
}

type ProductionImathasBackend = ImathasBackend<
    PostgresStore,
    objects::s3::S3ObjectStore,
    ContractedScoredEmbedProvider<HttpContractedScoredEmbedTransport>,
>;
type ProductionQtiBackend =
    QtiBackend<PostgresStore, PostgresGraderStore, objects::s3::S3ObjectStore>;
pub(super) struct ConfiguredImathas {
    pub(super) backend: Arc<ProductionImathasBackend>,
    pub(super) aead: Arc<LaunchStateAead>,
}

#[allow(dead_code)]
impl PersistentDependencies {
    pub(super) async fn from_env() -> Result<Self> {
        let settings = ProductionSettings::from_env()?;
        let dependencies = Self::from_settings(&settings).await?;
        dependencies.verify_startup_schema().await?;
        Ok(dependencies)
    }

    async fn verify_startup_schema(&self) -> Result<()> {
        match verify_application_schema_bounded(&self.health.postgres).await {
            Ok(()) => Ok(()),
            Err(SchemaCompatibilityError::Unavailable) => {
                eprintln!("database schema check unavailable; API starting degraded");
                Ok(())
            }
            Err(SchemaCompatibilityError::Incompatible(reason)) => {
                bail!("database schema is incompatible: {reason}")
            }
        }
    }

    async fn from_settings(settings: &ProductionSettings) -> Result<Self> {
        let LazyStorageDependencies {
            store,
            objects,
            pool,
            object_client,
        } = LazyStorageDependencies::from_settings(&settings.storage)?;
        let public_assets = Arc::new(
            PublicAssetBaseUrl::new(settings.public_asset_base_url.clone())
                .context("PLE_PUBLIC_ASSET_BASE_URL rejected")?,
        );
        // This only validates and constructs a private HTTP client.  It makes
        // no renderer request: renderer availability must not gate API startup,
        // native questions, or the API health endpoint.
        let webwork_renderer = settings.webwork_renderer()?;
        let imathas = settings.imathas(&store, &objects)?;
        let qti_runtime_enabled = settings.qti_runtime_enabled()?;
        let grader_database_url = settings.grader_database_url()?;
        let grader = Arc::new(
            PostgresGraderStore::connect(grader_database_url)
                .await
                .map_err(|_| anyhow::anyhow!("PLE grader connection could not be established"))?,
        );
        let qti = if qti_runtime_enabled {
            Some(Arc::new(QtiBackend::new(
                Arc::clone(&store),
                Arc::clone(&grader),
                Arc::clone(&objects),
            )))
        } else {
            None
        };
        Ok(Self {
            store,
            grader,
            objects,
            public_assets,
            webwork_renderer,
            imathas,
            qti,
            health: Arc::new(HealthState {
                postgres: pool,
                object_client,
                content_bucket: settings.storage.content_bucket.clone(),
            }),
        })
    }

    /// Composes every production route group after trusted institution-owned
    /// dependencies have been supplied.  Neither generic route state nor this
    /// root stores educational records in the process.
    fn router<P, R>(
        &self,
        identity_provider: Arc<P>,
        review_gate: Arc<R>,
        session_config: SessionConfig,
    ) -> Router
    where
        P: IdentityProvider + 'static,
        P::Presentation: serde::de::DeserializeOwned + Send + Sync + 'static,
        R: PublicReviewGate + 'static,
    {
        let native_adapter = Arc::new(adapter_native::NativeAdapter::new());
        let flat_grader: Arc<dyn FlatQuestionGradingStore> = self.grader.clone();
        let native = NativeBackend::with_flat_grader(
            Arc::clone(&native_adapter),
            Arc::clone(&self.store),
            flat_grader,
        );
        let mut backends = if let Some(renderer) = &self.webwork_renderer {
            let webwork_adapter = Arc::new(WebworkAdapter::new(
                self.objects.as_ref().clone(),
                renderer.clone(),
            ));
            let webwork = WebworkBackend::new(
                Arc::clone(&self.store),
                Arc::clone(&self.objects),
                webwork_adapter,
            );
            CompositeBackend::new(native, webwork)
        } else {
            CompositeBackend::<PostgresStore, objects::s3::S3ObjectStore, HttpWebworkRenderer>::native_only(
                native,
            )
        };
        if let Some(imathas) = &self.imathas {
            backends = backends.with_imathas(imathas.backend.clone());
        }
        if let Some(qti) = &self.qti {
            backends = backends.with_qti(qti.clone());
        }
        let backends = Arc::new(backends);
        let mut router = compose_router(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            Arc::clone(&self.public_assets),
            Arc::clone(&backends),
            native_adapter,
            identity_provider,
            review_gate,
            session_config,
            Arc::clone(&self.health),
        );
        if let Some(imathas) = &self.imathas {
            router = router.merge(external_tool_router(
                Arc::clone(&self.store),
                Arc::clone(&backends),
                Arc::clone(&imathas.aead),
            ));
        }
        router
    }

    pub(super) fn local_development_router<R>(
        &self,
        local_authentication: LocalDevelopmentAuthentication,
        review_gate: Arc<R>,
    ) -> Router
    where
        R: PublicReviewGate + 'static,
    {
        self.router(
            local_authentication.provider,
            review_gate,
            local_authentication.session_config,
        )
    }
}
