//! Storage-topology behavior owned by the native composition root.

use super::settings::{
    ProcessRole, StorageRuntime, StorageSettings, StorageTopology, browser_boundary_for,
};

#[test]
fn storage_topology_defaults_to_aws_and_rejects_unrecognized_values() {
    assert_eq!(
        StorageTopology::from_value(None).expect("default topology"),
        StorageTopology::AwsWorkload
    );
    assert_eq!(
        StorageTopology::from_value(Some("aws-workload")).expect("explicit AWS topology"),
        StorageTopology::AwsWorkload
    );
    for value in ["", "aws", "local-file", "DisposableLocal"] {
        assert!(
            StorageTopology::from_value(Some(value)).is_err(),
            "{value:?}"
        );
    }
}

#[test]
fn disposable_local_topology_is_feature_gated() {
    let result = StorageTopology::from_value(Some("disposable-local"));
    #[cfg(feature = "local-disposable-storage")]
    assert_eq!(
        result.expect("enabled disposable storage"),
        StorageTopology::DisposableLocal
    );
    #[cfg(not(feature = "local-disposable-storage"))]
    assert!(result.is_err());
}

#[test]
fn disposable_local_storage_accepts_api_and_worker_without_changing_aws_defaults() {
    let local_api = StorageRuntime {
        role: ProcessRole::Api,
        topology: StorageTopology::DisposableLocal,
    };
    let local_worker = StorageRuntime {
        role: ProcessRole::Worker,
        topology: StorageTopology::DisposableLocal,
    };
    let aws_api = StorageRuntime {
        role: ProcessRole::Api,
        topology: StorageTopology::AwsWorkload,
    };
    let aws_worker = StorageRuntime {
        role: ProcessRole::Worker,
        topology: StorageTopology::AwsWorkload,
    };

    assert_eq!(local_api.database_variable(), "DATABASE_URL");
    assert_eq!(local_worker.database_variable(), "DATABASE_URL");
    assert!(local_api.uses_disposable_local_storage());
    assert!(local_worker.uses_disposable_local_storage());
    assert_eq!(aws_api.database_variable(), "DATABASE_URL");
    assert_eq!(aws_worker.database_variable(), "PLE_WORKER_DATABASE_URL");
    assert!(!aws_api.uses_disposable_local_storage());
}

#[test]
fn disposable_local_api_keeps_the_production_browser_boundary() {
    let runtime = StorageRuntime {
        role: ProcessRole::Api,
        topology: StorageTopology::DisposableLocal,
    };
    assert!(runtime.uses_disposable_local_storage());
    browser_boundary_for("https://learn.example.test").expect("valid production browser boundary");
}

#[test]
fn publisher_refuses_disposable_local_storage() {
    let publisher = StorageRuntime {
        role: ProcessRole::PublicAssetPublisher,
        topology: StorageTopology::DisposableLocal,
    };
    assert!(StorageSettings::from_env(publisher).is_err());
}
