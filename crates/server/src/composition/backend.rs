//! Concrete production storage and grading backend assembly.

use super::router::{HealthState, compose_passwordless_router, verify_application_schema_bounded};
use super::settings::{
    GradingBackendSettings, LazyStorageDependencies, ProductionSettings, StorageRuntime,
    StorageTopology,
};
use super::*;

/// Concrete dependency construction for the future institution adapter.
/// It contains only replica-safe backends.
pub(super) struct PersistentDependencies {
    store: Arc<PostgresStore>,
    /// This capability is retained only for server-owned grading backends.
    /// It is never included in route state or browser-facing APIs.
    grader: Arc<PostgresGraderStore>,
    objects: Arc<objects::s3::S3ObjectStore>,
    accepted_submission_fast_path:
        Arc<dyn crate::accepted_submission_worker::AcceptedSubmissionFastPath>,
    public_assets: Arc<PublicAssetBaseUrl>,
    grading: GradingBackendSettings,
    imathas: Option<ConfiguredImathas>,
    invitation_issuer: crate::course::CourseInvitationIssuer,
    passwordless_email_delivery: Arc<dyn crate::auth::PasswordlessEmailDelivery>,
    passwordless_rate_limit_issuer: crate::auth::PasswordlessRateLimitIssuer,
    webauthn: crate::auth::PasswordlessWebauthn,
    browser_boundary: crate::auth::ProductionBrowserBoundary,
    client_address_policy: crate::auth::ClientAddressPolicy,
    live_demo_selector: Option<crate::auth::SeededAccountSelectorConfig>,
    health: Arc<HealthState>,
}

type ProductionImathasBackend = ImathasBackend<
    PostgresStore,
    objects::s3::S3ObjectStore,
    ContractedScoredEmbedProvider<HttpContractedScoredEmbedTransport>,
>;
pub(super) type ProductionGradingBackend =
    CompositeBackend<PostgresStore, objects::s3::S3ObjectStore, HttpWebworkRenderer>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ProductionBackendCapabilities {
    pub(super) native: bool,
    pub(super) webwork: bool,
    pub(super) qti: bool,
    pub(super) imathas: bool,
}

/// The shared grading builder always starts with native grading. API composition
/// may attach the external-tool capability only after this common boundary.
pub(super) fn production_grading_backend_capabilities(
    settings: &GradingBackendSettings,
) -> ProductionBackendCapabilities {
    ProductionBackendCapabilities {
        native: true,
        webwork: settings.webwork.is_some(),
        qti: settings.qti_runtime_enabled,
        imathas: false,
    }
}

/// Reports the API's superset only after the shared builder remains free of
/// authenticated external-tool state.
pub(super) fn api_backend_capabilities(
    settings: &GradingBackendSettings,
    imathas_attached: bool,
) -> ProductionBackendCapabilities {
    let mut capabilities = production_grading_backend_capabilities(settings);
    capabilities.imathas = imathas_attached;
    capabilities
}

/// Adds the authenticated external-tool capability only in API composition.
/// Worker composition receives the shared builder result directly.
pub(super) fn attach_api_imathas(
    backend: ProductionGradingBackend,
    imathas: Option<&ConfiguredImathas>,
) -> ProductionGradingBackend {
    match imathas {
        Some(imathas) => backend.with_imathas(imathas.backend.clone()),
        None => backend,
    }
}

/// Builds the deterministic grading families from already constructed
/// capabilities. This boundary deliberately reads neither environment settings
/// nor API-only external-tool configuration, so Worker composition can reuse it.
pub(super) fn build_production_grading_backend(
    store: Arc<PostgresStore>,
    objects: Arc<objects::s3::S3ObjectStore>,
    grader: Arc<PostgresGraderStore>,
    settings: &GradingBackendSettings,
) -> ProductionGradingBackend {
    let native_adapter = Arc::new(adapter_native::NativeAdapter::new());
    let flat_grader: Arc<dyn FlatQuestionGradingStore> = grader.clone();
    let native = NativeBackend::with_flat_grader(native_adapter, Arc::clone(&store), flat_grader);
    let capabilities = production_grading_backend_capabilities(settings);
    let mut backends = if capabilities.webwork {
        let renderer = settings
            .webwork
            .as_ref()
            .expect("WebWork capability requires validated renderer settings")
            .renderer()
            .expect("validated WebWork renderer settings must construct a renderer");
        let webwork_adapter = Arc::new(WebworkAdapter::new(objects.as_ref().clone(), renderer));
        let webwork =
            WebworkBackend::new(Arc::clone(&store), Arc::clone(&objects), webwork_adapter);
        CompositeBackend::new(native, webwork)
    } else {
        CompositeBackend::native_only(native)
    };
    if capabilities.qti {
        backends = backends.with_qti(Arc::new(QtiBackend::new(store, grader, objects)));
    }
    backends
}

/// Opens the dedicated grader connection selected by parsed shared settings.
/// The caller chooses topology, while the builder above receives only the
/// finished least-authority capability.
pub(super) async fn connect_production_grader(
    settings: &GradingBackendSettings,
    topology: StorageTopology,
) -> Result<Arc<PostgresGraderStore>> {
    let grader = match topology {
        StorageTopology::DisposableLocal => {
            PostgresGraderStore::connect_local_development(&settings.grader_database_url).await
        }
        StorageTopology::AwsWorkload => {
            PostgresGraderStore::connect(&settings.grader_database_url).await
        }
    }
    .map_err(|_| anyhow::anyhow!("PLE grader connection could not be established"))?;
    Ok(Arc::new(grader))
}
pub(super) struct ConfiguredImathas {
    pub(super) backend: Arc<ProductionImathasBackend>,
    pub(super) aead: Arc<LaunchStateAead>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum StartupSchemaPolicy {
    Ready,
    StartDegraded,
    Refuse(SchemaCompatibilityError),
}

/// Selects API startup behavior for the configured storage topology.
/// AWS workloads retain degraded startup; disposable local requires schema availability.
pub(super) fn startup_schema_policy(
    topology: StorageTopology,
    verification: Result<(), SchemaCompatibilityError>,
) -> StartupSchemaPolicy {
    match verification {
        Ok(()) => StartupSchemaPolicy::Ready,
        Err(SchemaCompatibilityError::Unavailable) if topology == StorageTopology::AwsWorkload => {
            StartupSchemaPolicy::StartDegraded
        }
        Err(error) => StartupSchemaPolicy::Refuse(error),
    }
}

impl PersistentDependencies {
    pub(super) async fn from_env(runtime: StorageRuntime) -> Result<Self> {
        let settings = ProductionSettings::from_env(runtime)?;
        let topology = settings.storage.runtime.topology;
        let dependencies = Self::from_settings(&settings).await?;
        dependencies.verify_startup_schema(topology).await?;
        Ok(dependencies)
    }

    async fn verify_startup_schema(&self, topology: StorageTopology) -> Result<()> {
        match startup_schema_policy(
            topology,
            verify_application_schema_bounded(&self.health.postgres).await,
        ) {
            StartupSchemaPolicy::Ready => Ok(()),
            StartupSchemaPolicy::StartDegraded => {
                eprintln!("database schema check unavailable; API starting degraded");
                Ok(())
            }
            StartupSchemaPolicy::Refuse(SchemaCompatibilityError::Unavailable) => {
                bail!("database schema check unavailable; disposable local API cannot start")
            }
            StartupSchemaPolicy::Refuse(SchemaCompatibilityError::Incompatible(reason)) => {
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
        } = LazyStorageDependencies::from_settings(&settings.storage).await?;
        let public_assets = Arc::new(settings.public_asset_base_url.clone());
        let imathas = settings.imathas(&store, &objects)?;
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
        let passwordless_email_delivery: Arc<dyn crate::auth::PasswordlessEmailDelivery> =
            match &settings.enrollment_email {
                Some(email_settings) => {
                    if settings.enrollment_secret.is_none() {
                        bail!(
                            "PLE_INVITATION_TOKEN_SECRET_FILE must be set when PLE SMTP is configured"
                        );
                    }
                    email_settings.delivery()? as Arc<dyn crate::auth::PasswordlessEmailDelivery>
                }
                None => Arc::new(crate::auth::UnavailablePasswordlessEmailDelivery)
                    as Arc<dyn crate::auth::PasswordlessEmailDelivery>,
            };
        let grader =
            connect_production_grader(&settings.grading, settings.storage.runtime.topology).await?;
        #[cfg(not(feature = "e2e-grader-fault"))]
        let fast_path_pool = match settings.storage.runtime.topology {
            StorageTopology::DisposableLocal => {
                super::local_accepted_submission_fast_path_pool(
                    &settings.accepted_submission_fast_path_database_url,
                )
                .await
            }
            StorageTopology::AwsWorkload => {
                super::accepted_submission_fast_path_pool(
                    &settings.accepted_submission_fast_path_database_url,
                )
                .await
            }
        }
        .map_err(|_| {
            anyhow::anyhow!(
                "accepted-submission fast-path database connection configuration was rejected"
            )
        })?;
        let accepted_submission_fast_path: Arc<
            dyn crate::accepted_submission_worker::AcceptedSubmissionFastPath,
        > = {
            #[cfg(feature = "e2e-grader-fault")]
            {
                // The feature-only fault profile keeps the accepted response
                // in durable recovery.  Its separate process exercises the
                // common handler and real failure persistence exactly once.
                Arc::new(crate::accepted_submission_worker::RecoveryOnlyAcceptedSubmissionFastPath)
            }
            #[cfg(not(feature = "e2e-grader-fault"))]
            {
                Arc::new(
                    crate::accepted_submission_worker::AcceptedSubmissionExecutionWorker::new(
                        super::PostgresAcceptedSubmissionFastPathStore::from_fast_path_pool(
                            fast_path_pool,
                        ),
                        build_production_grading_backend(
                            Arc::clone(&store),
                            Arc::clone(&objects),
                            Arc::clone(&grader),
                            &settings.grading,
                        ),
                        learning_data_access::WorkerId::from_uuid(uuid::Uuid::new_v4()),
                        settings.accepted_submission_execution.worker_settings(),
                    )
                    .context("accepted-submission fast-path settings are incompatible")?,
                )
            }
        };
        Ok(Self {
            store,
            grader,
            objects,
            accepted_submission_fast_path,
            public_assets,
            grading: settings.grading.clone(),
            imathas,
            invitation_issuer,
            passwordless_email_delivery,
            passwordless_rate_limit_issuer,
            webauthn: settings.webauthn.clone(),
            browser_boundary: settings.browser_boundary.clone(),
            client_address_policy: settings.client_address_policy.clone(),
            live_demo_selector: settings.live_demo_selector.clone(),
            health: Arc::new(HealthState {
                postgres: pool,
                object_client,
                public_assets_bucket: settings.storage.public_assets_bucket.clone(),
                private_content_bucket: settings.storage.private_content_bucket.clone(),
                student_records_bucket: settings.storage.student_records_bucket.clone(),
                temp_processing_bucket: settings.storage.temp_processing_bucket.clone(),
            }),
        })
    }

    /// Composes the production route graph with PLE-owned account identity.
    ///
    /// The direct passwordless routes own account sessions and tenant-session
    /// selection. The optional deployment-gated selector enters that same
    /// account/session graph.
    pub(super) fn production_router(&self) -> Result<Router> {
        Ok(complete_production_router(
            self.passwordless_router(
                Arc::new(crate::catalog::ReviewNotRequired),
                production_session_config(),
            ),
            self.browser_boundary.clone(),
        ))
    }

    fn passwordless_router<R>(&self, review_gate: Arc<R>, session_config: SessionConfig) -> Router
    where
        R: PublicReviewGate + 'static,
    {
        let native_adapter = Arc::new(adapter_native::NativeAdapter::new());
        let mut backends = build_production_grading_backend(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            Arc::clone(&self.grader),
            &self.grading,
        );
        backends = attach_api_imathas(backends, self.imathas.as_ref());
        debug_assert_eq!(
            backends.has_imathas(),
            api_backend_capabilities(&self.grading, self.imathas.is_some()).imathas,
        );
        let backends = Arc::new(backends);
        let sealed_execution: Arc<dyn learning_data_access::SealedPrivateExecutionStore> =
            self.grader.clone();
        let mut router = compose_passwordless_router(
            Arc::clone(&self.store),
            Arc::clone(&self.objects),
            Arc::clone(&self.public_assets),
            Arc::clone(&backends),
            sealed_execution,
            native_adapter,
            review_gate,
            session_config,
            self.invitation_issuer.clone(),
            Arc::clone(&self.passwordless_email_delivery),
            self.passwordless_rate_limit_issuer.clone(),
            self.client_address_policy.clone(),
            self.live_demo_selector.clone(),
            Some(self.webauthn.clone()),
            Arc::clone(&self.health),
            Arc::clone(&self.accepted_submission_fast_path),
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
}

/// Applies the production-only browser boundary to an already composed route
/// graph. Persistent dependency construction stays above this seam, so the
/// composition contract can be exercised with deterministic injected routes.
pub(super) fn complete_production_router(
    router: Router,
    browser_boundary: crate::auth::ProductionBrowserBoundary,
) -> Router {
    crate::http_security::apply_api_security_headers(
        router
            // The host/origin boundary is inside the universal response
            // boundary so even an early 403/421 remains non-cacheable and
            // gets the same browser hardening headers as a routed response.
            .layer(axum::middleware::from_fn_with_state(
                browser_boundary,
                crate::auth::production_cookie_boundary,
            )),
    )
}

pub(super) fn production_session_config() -> SessionConfig {
    SessionConfig::new(
        learning_data_access::SessionLifetime::from_seconds(8 * 60 * 60)
            .expect("eight-hour production session lifetime is positive"),
        crate::auth::CookieTransport::FirstPartyHttps,
    )
}
