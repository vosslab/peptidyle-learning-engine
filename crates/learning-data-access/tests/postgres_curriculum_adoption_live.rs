#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for B2 curriculum-adoption persistence.
//!
//! This opt-in test exercises the session-bound Store path.  Direct SQL is
//! reserved for authority inspection and deliberate derived-row corruption.

mod postgres_curriculum_adoption_live {
    mod bridge_protocol;
    mod broker;
    mod fixture;
    mod imports;
    mod lifecycle;
    mod operations;

    use fixture::AdoptionFixture;

    #[tokio::test]
    #[ignore = "requires the disposable PostgreSQL acceptance database"]
    async fn postgres_curriculum_adoption_is_brokered_atomic_and_recoverable() {
        let fixture = AdoptionFixture::bootstrap().await;
        broker::assert_broker_boundary(&fixture).await;
        bridge_protocol::assert_public_bridge_protocol(&fixture).await;
        operations::assert_public_source_and_destination_write(&fixture).await;
        operations::assert_blueprint_replay_refusals_and_reload(&fixture).await;
        lifecycle::assert_rollover_and_unissued_term_shift(&fixture).await;
        imports::assert_import_updates_inspection_and_reconciliation(&fixture).await;
    }
}
