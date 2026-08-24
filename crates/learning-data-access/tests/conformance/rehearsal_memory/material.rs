use super::*;

use learning_data_access::{
    RehearsalDeliveryMaterialStore, RehearsalRouteIdentity,
    VerifyRehearsalDeliveryMaterialRouteCommand,
};

#[tokio::test]
async fn immutable_material_verification_uses_frozen_siblings_and_returns_no_material() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    let route = RehearsalRouteIdentity {
        actor: fixture.instructor,
        course: fixture.course,
        assignment: locator.assignment,
        rehearsal: locator.rehearsal,
        expected_revision: locator.revision,
    };
    store
        .verify_rehearsal_delivery_material_from_route(
            fixture.context,
            VerifyRehearsalDeliveryMaterialRouteCommand { route },
        )
        .await
        .expect("immutable frozen material verifies without disclosure");
    let _ = frozen;
}
