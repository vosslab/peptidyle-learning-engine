//! Deployment configuration checks for the seeded Sysadmin ownership route.

use std::sync::Arc;

use question_model::UserId;

use super::super::settings::{
    live_demo_sysadmin_ownership_from_env, validate_live_demo_identity_config,
};

const LIVE_DEMO_SYSADMIN_USER_ID_ENV: &str = "PLE_LIVE_DEMO_SYSADMIN_USER_ID";
const LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV: &str = "PLE_LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE";
const LIVE_DEMO_CLAIM_PROOF: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE";

#[test]
fn live_demo_sysadmin_claim_configuration_requires_a_matched_private_context() {
    let configured_user = "00000000-0000-0000-0000-000000000005";
    let context_user = "00000000-0000-0000-0000-000000000006";
    let generation = "00000000-0000-0000-0000-000000000007";
    let path = live_demo_claim_context_file(generation, context_user);
    let path_value = path.to_str().expect("temporary claim context path");

    with_live_demo_sysadmin_environment(&[], || {
        assert!(
            live_demo_sysadmin_ownership_from_env("https://learn.example.test")
                .expect("disabled configuration")
                .is_none()
        );
    });
    with_live_demo_sysadmin_environment(
        &[(LIVE_DEMO_SYSADMIN_USER_ID_ENV, Some(configured_user))],
        || {
            let error = live_demo_sysadmin_ownership_from_env("https://learn.example.test")
                .expect_err("missing context file must fail closed");
            assert!(
                error
                    .to_string()
                    .contains(LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV)
            );
        },
    );
    with_live_demo_sysadmin_environment(
        &[(LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV, Some(path_value))],
        || {
            let error = live_demo_sysadmin_ownership_from_env("https://learn.example.test")
                .expect_err("missing configured Sysadmin must fail closed");
            assert!(error.to_string().contains(LIVE_DEMO_SYSADMIN_USER_ID_ENV));
        },
    );
    with_live_demo_sysadmin_environment(
        &[
            (LIVE_DEMO_SYSADMIN_USER_ID_ENV, Some("not-a-uuid")),
            (LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV, Some(path_value)),
        ],
        || {
            assert!(live_demo_sysadmin_ownership_from_env("https://learn.example.test").is_err());
        },
    );
    with_live_demo_sysadmin_environment(
        &[
            (
                LIVE_DEMO_SYSADMIN_USER_ID_ENV,
                Some("00000000-0000-0000-0000-00000000000A"),
            ),
            (LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV, Some(path_value)),
        ],
        || {
            assert!(live_demo_sysadmin_ownership_from_env("https://learn.example.test").is_err());
        },
    );
    with_live_demo_sysadmin_environment(
        &[
            (LIVE_DEMO_SYSADMIN_USER_ID_ENV, Some(configured_user)),
            (LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV, Some(path_value)),
        ],
        || {
            let error = live_demo_sysadmin_ownership_from_env("https://learn.example.test")
                .expect_err("context identity must match deployment identity");
            assert!(error.to_string().contains("must equal"));
        },
    );

    assert!(std::fs::remove_file(&path).is_ok());
    let path = live_demo_claim_context_file(generation, configured_user);
    let path_value = path.to_str().expect("temporary claim context path");
    let claim = with_live_demo_sysadmin_environment(
        &[
            (LIVE_DEMO_SYSADMIN_USER_ID_ENV, Some(configured_user)),
            (LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV, Some(path_value)),
        ],
        || {
            live_demo_sysadmin_ownership_from_env("https://learn.example.test")
                .expect("matched private context")
                .expect("enabled claim configuration")
        },
    );
    assert_eq!(claim.user().as_uuid().to_string(), configured_user);
    let debug = format!("{claim:?}");
    assert!(!debug.contains(generation));
    assert!(!debug.contains(LIVE_DEMO_CLAIM_PROOF));
    assert!(std::fs::remove_file(path).is_ok());
}

#[test]
fn live_demo_sysadmin_claim_identity_is_distinct_from_all_selector_accounts() {
    let selector_users = [
        UserId::from_uuid(uuid::Uuid::from_u128(1)),
        UserId::from_uuid(uuid::Uuid::from_u128(2)),
        UserId::from_uuid(uuid::Uuid::from_u128(3)),
        UserId::from_uuid(uuid::Uuid::from_u128(4)),
    ];
    let selector = crate::auth::SeededAccountSelectorConfig::new(
        Arc::from("https://learn.example.test"),
        selector_users,
    )
    .expect("four distinct selector accounts");
    let distinct = crate::auth::SeededSysadminOwnershipConfig::new(
        Arc::from("https://learn.example.test"),
        uuid::Uuid::from_u128(5),
        UserId::from_uuid(uuid::Uuid::from_u128(6)),
        [7; 32],
    )
    .expect("distinct Sysadmin claim account");
    assert!(validate_live_demo_identity_config(Some(&selector), Some(&distinct)).is_ok());

    let duplicate = crate::auth::SeededSysadminOwnershipConfig::new(
        Arc::from("https://learn.example.test"),
        uuid::Uuid::from_u128(5),
        selector_users[0],
        [7; 32],
    )
    .expect("well-formed duplicate only tests composition rejection");
    assert!(validate_live_demo_identity_config(Some(&selector), Some(&duplicate)).is_err());
}

fn live_demo_claim_context_file(generation: &str, user: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "ple-live-demo-claim-context-{}-{}.json",
        std::process::id(),
        NEXT_CONTEXT.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::write(
        &path,
        format!(
            r#"{{"installationGeneration":"{generation}","sysadminUserId":"{user}","ownershipProof":"{LIVE_DEMO_CLAIM_PROOF}"}}"#
        ),
    )
    .expect("write private claim context");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .expect("set private claim context mode");
    path
}

fn with_live_demo_sysadmin_environment<T>(
    replacements: &[(&str, Option<&str>)],
    action: impl FnOnce() -> T,
) -> T {
    static ENVIRONMENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = ENVIRONMENT_LOCK
        .lock()
        .expect("live-demo Sysadmin environment lock");
    let names = [
        LIVE_DEMO_SYSADMIN_USER_ID_ENV,
        LIVE_DEMO_SYSADMIN_CLAIM_CONTEXT_FILE_ENV,
    ];
    let saved = names
        .iter()
        .map(|name| (*name, std::env::var_os(name)))
        .collect::<Vec<_>>();
    for name in names {
        replace_live_demo_sysadmin_environment_variable(name, None);
    }
    for (name, value) in replacements {
        replace_live_demo_sysadmin_environment_variable(name, *value);
    }
    let output = action();
    for name in names {
        replace_live_demo_sysadmin_environment_variable(name, None);
    }
    for (name, value) in saved {
        // SAFETY: this helper holds its dedicated lock for its complete mutation interval.
        unsafe {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
    output
}

fn replace_live_demo_sysadmin_environment_variable(name: &str, value: Option<&str>) {
    // SAFETY: `with_live_demo_sysadmin_environment` serializes every mutation here.
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
