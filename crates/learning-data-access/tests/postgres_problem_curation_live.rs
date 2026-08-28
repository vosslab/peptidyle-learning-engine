#![cfg(feature = "postgres")]

//! Disposable PostgreSQL 17 oracle for the WP-PROF-D2 curation capability.
//!
//! The single ignored entry point deliberately exercises the application Store
//! and then inspects the migration-owned broker/RLS boundary directly.  Its
//! fixture creates ordinary accounts, sessions, and published questions in the
//! disposable acceptance database; each assertion owns only its tenant-scoped
//! records.

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
#[path = "postgres_problem_curation_live/authority.rs"]
mod authority;
#[path = "postgres_problem_curation_live/behavior.rs"]
mod behavior;
#[path = "postgres_problem_curation_live/fixture.rs"]
mod fixture;
#[path = "postgres_problem_curation_live/pagination.rs"]
mod pagination;

use acceptance_runtime::load as load_acceptance_runtime;
use learning_data_access::postgres::{lazy_pool, verify_application_schema};

/// Covers the production-shaped D2 aggregate, including its sealed PostgreSQL
/// broker capability.  Run only against the disposable acceptance database.
#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_problem_curation_live_oracle_is_sealed_and_atomic() {
    let runtime = load_acceptance_runtime();
    let url = runtime.admin_url().expose();
    authority::pre_d2_broker_drift_converges(url).await;
    let pool = lazy_pool(url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("fully migrated D2 schema");
    authority::broker_role_and_forced_rls_are_sealed(
        &pool,
        authority::BrokerAuthorityStage::FullyMigrated,
    )
    .await;

    let fixture = fixture::Fixture::new(pool.clone()).await;
    authority::actor_authority_privacy_and_tenant_isolation(&fixture).await;
    behavior::favorites_replacement_and_retention_are_atomic(&fixture).await;
    behavior::saved_searches_are_normalized_revisioned_and_personal(&fixture).await;
    pagination::sealed_cursors_bind_actor_scope_and_member_revision(&fixture).await;
    behavior::aggregate_limits_title_conflicts_and_broker_input_validation(&fixture).await;
    fixture.cleanup().await;
}
