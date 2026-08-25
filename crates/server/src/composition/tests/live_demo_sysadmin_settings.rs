//! Deployment configuration checks for direct seeded Sysadmin selection.

use super::super::settings::live_demo_selector_from_env;

const LIVE_DEMO_SELECTOR_ENV: [&str; 5] = [
    "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_USER_ID",
    "PLE_LIVE_DEMO_MARY_STUDENT_USER_ID",
    "PLE_LIVE_DEMO_JACK_STUDENT_USER_ID",
    "PLE_LIVE_DEMO_AVERY_STUDENT_USER_ID",
    "PLE_LIVE_DEMO_SYSADMIN_USER_ID",
];

#[test]
fn selector_configuration_requires_all_five_personas() {
    with_live_demo_selector_environment(&[], || {
        assert!(
            live_demo_selector_from_env("https://learn.example.test")
                .expect("disabled selector")
                .is_none()
        );
    });
    with_live_demo_selector_environment(
        &[(
            LIVE_DEMO_SELECTOR_ENV[0],
            Some("00000000-0000-0000-0000-000000000001"),
        )],
        || {
            assert!(live_demo_selector_from_env("https://learn.example.test").is_err());
        },
    );
}

#[test]
fn selector_configuration_includes_the_configured_sysadmin_account() {
    let users = [
        "00000000-0000-0000-0000-000000000001",
        "00000000-0000-0000-0000-000000000002",
        "00000000-0000-0000-0000-000000000003",
        "00000000-0000-0000-0000-000000000004",
        "00000000-0000-0000-0000-000000000005",
    ];
    let replacements = LIVE_DEMO_SELECTOR_ENV
        .iter()
        .zip(users)
        .map(|(name, user)| (*name, Some(user)))
        .collect::<Vec<_>>();
    with_live_demo_selector_environment(&replacements, || {
        let selector = live_demo_selector_from_env("https://learn.example.test")
            .expect("complete selector configuration")
            .expect("enabled selector");
        assert!(selector.contains_user(question_model::UserId::from_uuid(
            uuid::Uuid::parse_str(users[4]).expect("Sysadmin user UUID"),
        )));
    });
}

fn with_live_demo_selector_environment<T>(
    replacements: &[(&str, Option<&str>)],
    action: impl FnOnce() -> T,
) -> T {
    static ENVIRONMENT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = ENVIRONMENT_LOCK
        .lock()
        .expect("live-demo selector environment lock");
    let saved = LIVE_DEMO_SELECTOR_ENV
        .iter()
        .map(|name| (*name, std::env::var_os(name)))
        .collect::<Vec<_>>();
    for name in LIVE_DEMO_SELECTOR_ENV {
        replace_live_demo_selector_environment_variable(name, None);
    }
    for (name, value) in replacements {
        replace_live_demo_selector_environment_variable(name, *value);
    }
    let output = action();
    for name in LIVE_DEMO_SELECTOR_ENV {
        replace_live_demo_selector_environment_variable(name, None);
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

fn replace_live_demo_selector_environment_variable(name: &str, value: Option<&str>) {
    // SAFETY: `with_live_demo_selector_environment` serializes every mutation here.
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
