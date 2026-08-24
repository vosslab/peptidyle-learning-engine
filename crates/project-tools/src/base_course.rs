//! Direct native-host CLI adapter for the product-owned Base Course installer.

use anyhow::{Context, Result, bail};
use base_course_installation::{
    BaseCourseInstallPhase, BaseCourseInstallRequest, BaseCourseParticipants,
};
use question_model::{TenantId, UserId};
use uuid::Uuid;

use crate::postgres_store;

const USAGE: &str = "usage: cargo tools base-course --tenant <UUID> --instructor <UUID> --mary <UUID> --jack <UUID> --approval-candidate <UUID> --sysadmin <UUID> --lifecycle-phase <prepare|install> [--storage-receipt <canonical JSON>] (requires child-only PLE_BASE_COURSE_INSTALLER_DATABASE_URL and PLE_BASE_COURSE_APP_DATABASE_URL; PLE_BASE_COURSE_DEPLOYMENT_MODE defaults to production and accepts local)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentMode {
    Production,
    Local,
}

impl DeploymentMode {
    fn parse(value: Option<&str>) -> Result<Self> {
        match value {
            None | Some("production") => Ok(Self::Production),
            Some("local") => Ok(Self::Local),
            Some(_) => {
                bail!("PLE_BASE_COURSE_DEPLOYMENT_MODE must be production or local; {USAGE}")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    deployment_mode: DeploymentMode,
    installer_database_url: String,
    application_database_url: String,
    participants: BaseCourseParticipants,
    phase: BaseCourseInstallPhase,
}

/// Parses child-only host configuration and invokes one product call.
pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let arguments = parse_arguments_with_database_urls(
        args,
        std::env::var("PLE_BASE_COURSE_INSTALLER_DATABASE_URL").ok(),
        std::env::var("PLE_BASE_COURSE_APP_DATABASE_URL").ok(),
        std::env::var("PLE_BASE_COURSE_DEPLOYMENT_MODE").ok(),
    )?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the Base Course installer runtime")?;
    let output = runtime.block_on(invoke(arguments))?;
    println!(
        "{}",
        serde_json::to_string(&output).context("serializing the Base Course installer output")?
    );
    Ok(())
}

async fn invoke(arguments: Arguments) -> Result<base_course_installation::BaseCourseInstallOutput> {
    let (installer_pool, application_pool) = database_pools(
        arguments.deployment_mode,
        &arguments.installer_database_url,
        &arguments.application_database_url,
    )?;
    let store = postgres_store::configured_postgres_store(application_pool)?;
    let request = BaseCourseInstallRequest::new(arguments.participants, arguments.phase);
    base_course_installation::install(&installer_pool, &store, request)
        .await
        .context("installing the Base Course")
}

#[cfg(test)]
fn parse_arguments(args: &[String]) -> Result<Arguments> {
    parse_arguments_with_database_urls(args, None, None, None)
}

fn parse_arguments_with_database_urls(
    args: &[String],
    installer_database_url: Option<String>,
    application_database_url: Option<String>,
    deployment_mode: Option<String>,
) -> Result<Arguments> {
    let mut tenant = None;
    let mut instructor = None;
    let mut mary = None;
    let mut jack = None;
    let mut approval_candidate = None;
    let mut sysadmin = None;
    let mut lifecycle_phase = None;
    let mut storage_receipt = None;
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        index += 1;
        let Some(value) = args.get(index) else {
            bail!("{flag} requires a value; {USAGE}");
        };
        index += 1;
        match flag.as_str() {
            "--tenant" if tenant.is_none() => tenant = Some(parse_tenant(value)?),
            "--instructor" if instructor.is_none() => {
                instructor = Some(parse_user(value, "instructor")?);
            }
            "--mary" if mary.is_none() => mary = Some(parse_user(value, "Mary")?),
            "--jack" if jack.is_none() => jack = Some(parse_user(value, "Jack")?),
            "--approval-candidate" if approval_candidate.is_none() => {
                approval_candidate = Some(parse_user(value, "approval candidate")?);
            }
            "--sysadmin" if sysadmin.is_none() => sysadmin = Some(parse_user(value, "Sysadmin")?),
            "--lifecycle-phase" if lifecycle_phase.is_none() => {
                lifecycle_phase = Some(value.as_str());
            }
            "--storage-receipt" if storage_receipt.is_none() => {
                storage_receipt = Some(value.clone());
            }
            _ => bail!("unknown, duplicate, or misplaced argument {flag}; {USAGE}"),
        }
    }
    let phase = match (lifecycle_phase, storage_receipt) {
        (Some("prepare"), None) => BaseCourseInstallPhase::Prepare,
        (Some("install"), Some(storage_receipt_json)) => BaseCourseInstallPhase::Install {
            storage_receipt_json,
        },
        (Some("prepare"), Some(_)) => {
            bail!("base-course prepare does not accept --storage-receipt; {USAGE}")
        }
        (Some("install"), None) => {
            bail!("base-course install requires --storage-receipt; {USAGE}")
        }
        (Some(_), _) => bail!("--lifecycle-phase must be prepare or install; {USAGE}"),
        (None, _) => bail!("base-course requires --lifecycle-phase; {USAGE}"),
    };
    let participants = BaseCourseParticipants::try_new(
        tenant.ok_or_else(|| anyhow::anyhow!("--tenant is required; {USAGE}"))?,
        instructor.ok_or_else(|| anyhow::anyhow!("--instructor is required; {USAGE}"))?,
        mary.ok_or_else(|| anyhow::anyhow!("--mary is required; {USAGE}"))?,
        jack.ok_or_else(|| anyhow::anyhow!("--jack is required; {USAGE}"))?,
        approval_candidate
            .ok_or_else(|| anyhow::anyhow!("--approval-candidate is required; {USAGE}"))?,
        sysadmin.ok_or_else(|| anyhow::anyhow!("--sysadmin is required; {USAGE}"))?,
    )?;
    Ok(Arguments {
        deployment_mode: DeploymentMode::parse(deployment_mode.as_deref())?,
        installer_database_url: installer_database_url.ok_or_else(|| {
            anyhow::anyhow!("PLE_BASE_COURSE_INSTALLER_DATABASE_URL is required; {USAGE}")
        })?,
        application_database_url: application_database_url.ok_or_else(|| {
            anyhow::anyhow!("PLE_BASE_COURSE_APP_DATABASE_URL is required; {USAGE}")
        })?,
        participants,
        phase,
    })
}

fn database_pools(
    deployment_mode: DeploymentMode,
    installer_database_url: &str,
    application_database_url: &str,
) -> Result<(
    learning_data_access::postgres::BaseCourseInstallerPool,
    learning_data_access::postgres::Pool,
)> {
    match deployment_mode {
        DeploymentMode::Production => Ok((
            learning_data_access::postgres::base_course_installer_pool(installer_database_url)
                .context("invalid production Base Course installer database authority")?,
            learning_data_access::postgres::base_course_application_pool(application_database_url)
                .context("invalid production Base Course application database authority")?,
        )),
        DeploymentMode::Local => Ok((
            learning_data_access::postgres::local_base_course_installer_pool(
                installer_database_url,
            )
            .context("invalid local Base Course installer database authority")?,
            learning_data_access::postgres::local_base_course_application_pool(
                application_database_url,
            )
            .context("invalid local Base Course application database authority")?,
        )),
    }
}

fn parse_tenant(value: &str) -> Result<TenantId> {
    Ok(TenantId::from_uuid(
        Uuid::parse_str(value).context("tenant must be a UUID")?,
    ))
}

fn parse_user(value: &str, name: &str) -> Result<UserId> {
    Ok(UserId::from_uuid(
        Uuid::parse_str(value).with_context(|| format!("{name} must be a UUID"))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn common_args() -> Vec<String> {
        vec![
            "--tenant".into(),
            "00000000-0000-0000-0000-000000000001".into(),
            "--instructor".into(),
            "00000000-0000-0000-0000-000000000002".into(),
            "--mary".into(),
            "00000000-0000-0000-0000-000000000003".into(),
            "--jack".into(),
            "00000000-0000-0000-0000-000000000004".into(),
            "--approval-candidate".into(),
            "00000000-0000-0000-0000-000000000005".into(),
            "--sysadmin".into(),
            "00000000-0000-0000-0000-000000000006".into(),
        ]
    }

    fn child_urls() -> (Option<String>, Option<String>) {
        (
            Some("postgres://ple_base_course_installer_login:secret@db/ple".to_string()),
            Some("postgres://ple_base_course_app_login:secret@db/ple".to_string()),
        )
    }

    fn production_urls() -> (String, String) {
        (
            "postgres://ple_base_course_installer_login:secret@db/ple?sslmode=verify-full"
                .to_string(),
            "postgres://ple_base_course_app_login:secret@db/ple?sslmode=verify-full".to_string(),
        )
    }

    #[test]
    fn prepare_maps_to_the_receipt_free_typed_phase() {
        let mut args = common_args();
        args.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let (installer, application) = child_urls();
        let parsed =
            parse_arguments_with_database_urls(&args, installer, application, None).unwrap();

        assert!(matches!(parsed.phase, BaseCourseInstallPhase::Prepare));
        assert_eq!(
            parsed.participants,
            BaseCourseParticipants::try_new(
                TenantId::from_uuid(Uuid::from_u128(1)),
                UserId::from_uuid(Uuid::from_u128(2)),
                UserId::from_uuid(Uuid::from_u128(3)),
                UserId::from_uuid(Uuid::from_u128(4)),
                UserId::from_uuid(Uuid::from_u128(5)),
                UserId::from_uuid(Uuid::from_u128(6)),
            )
            .unwrap()
        );
    }

    #[test]
    fn install_maps_to_the_receipt_carrying_typed_phase() {
        let mut args = common_args();
        args.extend([
            "--lifecycle-phase".into(),
            "install".into(),
            "--storage-receipt".into(),
            "receipt".into(),
        ]);
        let (installer, application) = child_urls();
        let parsed =
            parse_arguments_with_database_urls(&args, installer, application, None).unwrap();
        assert!(matches!(
            parsed.phase,
            BaseCourseInstallPhase::Install { storage_receipt_json } if storage_receipt_json == "receipt"
        ));
    }

    #[test]
    fn invalid_phase_and_participant_combinations_are_refused() {
        let mut prepare_with_receipt = common_args();
        prepare_with_receipt.extend([
            "--lifecycle-phase".into(),
            "prepare".into(),
            "--storage-receipt".into(),
            "receipt".into(),
        ]);
        let (installer, application) = child_urls();
        assert!(
            parse_arguments_with_database_urls(&prepare_with_receipt, installer, application, None)
                .is_err()
        );

        let mut collision = common_args();
        collision[5] = "00000000-0000-0000-0000-000000000002".into();
        collision.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let (installer, application) = child_urls();
        assert!(
            parse_arguments_with_database_urls(&collision, installer, application, None).is_err()
        );
    }

    #[test]
    fn separate_database_urls_come_only_from_the_child_environment() {
        let mut args = common_args();
        args.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let parsed = parse_arguments_with_database_urls(
            &args,
            Some("postgres://installer-child-only".to_string()),
            Some("postgres://application-child-only".to_string()),
            Some("local".to_string()),
        )
        .unwrap();
        assert_eq!(
            parsed.installer_database_url,
            "postgres://installer-child-only"
        );
        assert_eq!(
            parsed.application_database_url,
            "postgres://application-child-only"
        );
        assert!(
            parse_arguments_with_database_urls(
                &args,
                None,
                Some("postgres://app".to_string()),
                None
            )
            .is_err()
        );
        assert!(
            parse_arguments_with_database_urls(
                &args,
                Some("postgres://installer".to_string()),
                None,
                None,
            )
            .is_err()
        );
        assert!(parse_arguments(&args).is_err());
    }

    #[test]
    fn deployment_mode_defaults_to_production_and_local_is_explicit() {
        let mut args = common_args();
        args.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let (installer, application) = child_urls();
        let production = parse_arguments_with_database_urls(&args, installer, application, None)
            .expect("production is the safe default");
        assert_eq!(production.deployment_mode, DeploymentMode::Production);

        let (installer, application) = child_urls();
        let local = parse_arguments_with_database_urls(
            &args,
            installer,
            application,
            Some("local".to_string()),
        )
        .expect("local mode is explicit");
        assert_eq!(local.deployment_mode, DeploymentMode::Local);

        let (installer, application) = child_urls();
        assert!(
            parse_arguments_with_database_urls(
                &args,
                installer,
                application,
                Some("development".to_string()),
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn database_pools_select_dedicated_application_factories_for_each_mode() {
        let (installer, application) = child_urls();
        assert!(
            database_pools(
                DeploymentMode::Production,
                installer.as_deref().expect("installer URL"),
                application.as_deref().expect("application URL"),
            )
            .is_err()
        );

        let (installer, application) = production_urls();
        let (_, production_application) =
            database_pools(DeploymentMode::Production, &installer, &application).unwrap();
        let (installer, application) = child_urls();
        let (_, local_application) = database_pools(
            DeploymentMode::Local,
            installer.as_deref().expect("installer URL"),
            application.as_deref().expect("application URL"),
        )
        .unwrap();
        assert_eq!(production_application.options().get_max_connections(), 1);
        assert_eq!(local_application.options().get_max_connections(), 1);
        assert!(
            database_pools(
                DeploymentMode::Local,
                application.as_deref().expect("application URL"),
                installer.as_deref().expect("installer URL"),
            )
            .is_err()
        );
    }
}
