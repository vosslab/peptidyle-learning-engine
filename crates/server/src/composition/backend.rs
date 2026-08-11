//! Concrete production storage and grading backend assembly.

use super::router::{HealthState, compose_passwordless_router, verify_application_schema_bounded};
use super::settings::{LazyStorageDependencies, ProductionSettings};
use super::*;

/// Concrete dependency construction for the future institution adapter.
/// It contains only replica-safe backends.
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
    invitation_issuer: crate::course::CourseInvitationIssuer,
    invitation_delivery: Arc<dyn crate::course::CourseInvitationDelivery>,
    passwordless_email_delivery: Arc<dyn crate::auth::PasswordlessEmailDelivery>,
    passwordless_rate_limit_issuer: crate::auth::PasswordlessRateLimitIssuer,
    webauthn: crate::auth::PasswordlessWebauthn,
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
        let (invitation_issuer, passwordless_rate_limit_issuer): (
            crate::course::CourseInvitationIssuer,
            crate::auth::PasswordlessRateLimitIssuer,
        ) = match &settings.enrollment_secret {
            Some(settings) => settings.issuers()?,
            None => (
                crate::course::CourseInvitationIssuer::unavailable(),
                crate::auth::PasswordlessRateLimitIssuer::unavailable(),
            ),
        };
        let (invitation_delivery, passwordless_email_delivery): (
            Arc<dyn crate::course::CourseInvitationDelivery>,
            Arc<dyn crate::auth::PasswordlessEmailDelivery>,
        ) = match &settings.enrollment_email {
            Some(email_settings) => {
                if settings.enrollment_secret.is_none() {
                    bail!(
                        "PLE_INVITATION_TOKEN_SECRET_FILE must be set when PLE SMTP is configured"
                    );
                }
                let delivery = email_settings.delivery()?;
                (
                    delivery.clone() as Arc<dyn crate::course::CourseInvitationDelivery>,
                    delivery as Arc<dyn crate::auth::PasswordlessEmailDelivery>,
                )
            }
            None => (
                Arc::new(crate::course::UnavailableCourseInvitationDelivery)
                    as Arc<dyn crate::course::CourseInvitationDelivery>,
                Arc::new(crate::auth::UnavailablePasswordlessEmailDelivery)
                    as Arc<dyn crate::auth::PasswordlessEmailDelivery>,
            ),
        };
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
            invitation_issuer,
            invitation_delivery,
            passwordless_email_delivery,
            passwordless_rate_limit_issuer,
            webauthn: settings.webauthn.clone(),
            health: Arc::new(HealthState {
                postgres: pool,
                object_client,
                content_bucket: settings.storage.content_bucket.clone(),
            }),
        })
    }

    /// Composes the production route graph with PLE-owned account identity.
    ///
    /// The direct passwordless routes own account sessions and tenant-session
    /// selection, so this graph intentionally has no provider-backed legacy
    /// `/api/auth/login` route and does not inspect local-development setup.
    pub(super) fn production_router(&self) -> Router {
        self.passwordless_router(
            Arc::new(crate::catalog::ReviewNotRequired),
            production_session_config(),
        )
    }

    fn passwordless_router<R>(&self, review_gate: Arc<R>, session_config: SessionConfig) -> Router
    where
        R: PublicReviewGate + 'static,
    {
        self.passwordless_router_with_local(review_gate, session_config, None)
    }

    fn passwordless_router_with_local<R>(
        &self,
        review_gate: Arc<R>,
        session_config: SessionConfig,
        local_development_roster: Option<Arc<crate::course::LocalDevelopmentRosterDirectory>>,
    ) -> Router
    where
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
        let mut router = compose_passwordless_router(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            Arc::clone(&self.public_assets),
            Arc::clone(&backends),
            native_adapter,
            review_gate,
            session_config,
            self.invitation_issuer.clone(),
            Arc::clone(&self.invitation_delivery),
            Arc::clone(&self.passwordless_email_delivery),
            self.passwordless_rate_limit_issuer.clone(),
            Some(self.webauthn.clone()),
            local_development_roster,
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
        self.passwordless_router_with_local(
            review_gate,
            local_authentication.session_config,
            Some(local_authentication.roster_directory),
        )
        .merge(crate::auth::router(
            local_authentication.provider,
            Arc::clone(&self.store),
            local_authentication.session_config,
        ))
    }
}

pub(super) fn production_session_config() -> SessionConfig {
    SessionConfig::new(
        learning_data_access::SessionLifetime::from_seconds(8 * 60 * 60)
            .expect("eight-hour production session lifetime is positive"),
        crate::auth::CookieTransport::FirstPartyHttps,
    )
}
