//! Direct native-host CLI adapter for the product-owned Base Course installer.

use anyhow::{Context, Result, bail};
use base_course_installation::{
    AcceptedSubmissionSeedExecutor, AcceptedSubmissionSeedOutcome, AcceptedSubmissionSeedRequest,
    BaseCourseInstallPhase, BaseCourseInstallRequest, BaseCourseParticipants,
};
use question_model::AccountId;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::postgres_store;

const USAGE: &str = "usage: cargo tools base-course --instructor <UUID> --mary <UUID> --jack <UUID> --approval-candidate <UUID> --sysadmin <UUID> --lifecycle-phase <prepare|install> [--storage-receipt <canonical JSON>] (requires child-only installer, application, and accepted-submission fast-path database URLs; PLE_BASE_COURSE_DEPLOYMENT_MODE defaults to production and accepts local)";

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
    fast_path_database_url: String,
    participants: BaseCourseParticipants,
    phase: BaseCourseInstallPhase,
}

/// Parses child-only host configuration and invokes one product call.
pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let arguments = parse_arguments_with_database_urls_and_fast_path(
        args,
        std::env::var("PLE_BASE_COURSE_INSTALLER_DATABASE_URL").ok(),
        std::env::var("PLE_BASE_COURSE_APP_DATABASE_URL").ok(),
        std::env::var("PLE_BASE_COURSE_FAST_PATH_DATABASE_URL").ok(),
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
    let (installer_pool, application_pool, fast_path_pool) = database_pools(
        arguments.deployment_mode,
        &arguments.installer_database_url,
        &arguments.application_database_url,
        &arguments.fast_path_database_url,
    )
    .await?;
    let store = postgres_store::configured_postgres_store(application_pool)?;
    let request = BaseCourseInstallRequest::new(arguments.participants, arguments.phase);
    let executor = SeedExecutor::new(store.clone(), fast_path_pool)?;
    base_course_installation::install(&installer_pool, &store, &executor, request)
        .await
        .context("installing the Base Course")
}

#[cfg(test)]
fn parse_arguments(args: &[String]) -> Result<Arguments> {
    parse_arguments_with_database_urls(args, None, None, None, None)
}

#[cfg(test)]
fn parse_arguments_with_database_urls(
    args: &[String],
    installer_database_url: Option<String>,
    application_database_url: Option<String>,
    fast_path_database_url: Option<String>,
    deployment_mode: Option<String>,
) -> Result<Arguments> {
    parse_arguments_with_database_urls_and_fast_path(
        args,
        installer_database_url,
        application_database_url,
        fast_path_database_url,
        deployment_mode,
    )
}

fn parse_arguments_with_database_urls_and_fast_path(
    args: &[String],
    installer_database_url: Option<String>,
    application_database_url: Option<String>,
    fast_path_database_url: Option<String>,
    deployment_mode: Option<String>,
) -> Result<Arguments> {
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
            "--instructor" if instructor.is_none() => {
                instructor = Some(parse_account(value, "instructor")?);
            }
            "--mary" if mary.is_none() => mary = Some(parse_account(value, "Mary")?),
            "--jack" if jack.is_none() => jack = Some(parse_account(value, "Jack")?),
            "--approval-candidate" if approval_candidate.is_none() => {
                approval_candidate = Some(parse_account(value, "approval candidate")?);
            }
            "--sysadmin" if sysadmin.is_none() => {
                sysadmin = Some(parse_account(value, "Sysadmin")?)
            }
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
        fast_path_database_url: fast_path_database_url.ok_or_else(|| {
            anyhow::anyhow!("PLE_BASE_COURSE_FAST_PATH_DATABASE_URL is required; {USAGE}")
        })?,
        participants,
        phase,
    })
}

async fn database_pools(
    deployment_mode: DeploymentMode,
    installer_database_url: &str,
    application_database_url: &str,
    fast_path_database_url: &str,
) -> Result<(
    learning_data_access::postgres::BaseCourseInstallerPool,
    learning_data_access::postgres::Pool,
    learning_data_access::postgres::AcceptedSubmissionFastPathPool,
)> {
    match deployment_mode {
        DeploymentMode::Production => Ok((
            learning_data_access::postgres::base_course_installer_pool(installer_database_url)
                .context("invalid production Base Course installer database authority")?,
            learning_data_access::postgres::base_course_application_pool(application_database_url)
                .context("invalid production Base Course application database authority")?,
            learning_data_access::postgres::base_course_accepted_submission_fast_path_pool(
                fast_path_database_url,
            )
            .await
            .context("invalid production Base Course accepted-submission fast-path authority")?,
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
            learning_data_access::postgres::local_base_course_accepted_submission_fast_path_pool(
                fast_path_database_url,
            )
            .await
            .context("invalid local Base Course accepted-submission fast-path authority")?,
        )),
    }
}

struct SeedExecutor {
    store: learning_data_access::postgres::PostgresStore,
    automated: Arc<learning_data_access::postgres::PostgresStore>,
    fast_path: Arc<dyn server_core::accepted_submission_worker::AcceptedSubmissionFastPath>,
}

impl SeedExecutor {
    fn new(
        store: learning_data_access::postgres::PostgresStore,
        fast_path_pool: learning_data_access::postgres::AcceptedSubmissionFastPathPool,
    ) -> Result<Self> {
        let automated = Arc::new(store.clone());
        let backend = server_core::native_backend::NativeBackend::new(
            Arc::new(adapter_native::NativeAdapter::new()),
            Arc::clone(&automated),
        );
        let settings = server_core::worker::WorkerSettings::new(120, Duration::from_secs(45), 1)
            .context("constructing Base Course accepted-submission worker settings")?;
        let worker = server_core::accepted_submission_worker::AcceptedSubmissionExecutionWorker::new(
            learning_data_access::postgres::PostgresAcceptedSubmissionFastPathStore::from_fast_path_pool(fast_path_pool),
            backend,
            learning_data_access::WorkerId::from_uuid(Uuid::new_v4()),
            settings,
        )
        .context("constructing Base Course accepted-submission fast path")?;
        Ok(Self {
            store,
            automated,
            fast_path: Arc::new(worker),
        })
    }
}

#[async_trait::async_trait]
impl AcceptedSubmissionSeedExecutor for SeedExecutor {
    async fn execute_seed_submission(
        &self,
        request: AcceptedSubmissionSeedRequest,
    ) -> Result<AcceptedSubmissionSeedOutcome, learning_data_access::StoreError> {
        let outcome = server_core::accepted_submission_service::accept_and_execute(
            &self.store,
            self.automated.as_ref(),
            self.fast_path.as_ref(),
            server_core::accepted_submission_service::AcceptedSubmissionRequest {
                student_account: request.student_account,
                binding: request.binding,
                attempt: request.attempt,
                response: request.response,
                idempotency_key: request.idempotency_key,
            },
        )
        .await?;
        match outcome {
            server_core::accepted_submission_service::AcceptedSubmissionApplicationOutcome::Executed {
                result:
                    server_core::accepted_submission_worker::AcceptedSubmissionHandlerResult::Committed,
                scoring_recalculation: Some(job),
                ..
            } => {
                let settings = server_core::worker::WorkerSettings::new(
                    120,
                    Duration::from_secs(45),
                    1,
                )?;
                match server_core::scoring_worker::execute_exact_assignment_scoring(
                    Arc::clone(&self.automated),
                    job,
                    settings,
                )
                .await?
                {
                    server_core::scoring_worker::ExactAssignmentScoringOutcome::Completed => {
                        Ok(AcceptedSubmissionSeedOutcome::Completed)
                    }
                    server_core::scoring_worker::ExactAssignmentScoringOutcome::Pending => {
                        Ok(AcceptedSubmissionSeedOutcome::PendingRecovery)
                    }
                }
            }
            server_core::accepted_submission_service::AcceptedSubmissionApplicationOutcome::Replay(_) => {
                Ok(AcceptedSubmissionSeedOutcome::Completed)
            }
            server_core::accepted_submission_service::AcceptedSubmissionApplicationOutcome::Pending {
                reason:
                    server_core::accepted_submission_service::AcceptedSubmissionPendingReason::AlreadyAccepted,
                ..
            } => {
                Ok(AcceptedSubmissionSeedOutcome::PendingRecovery)
            }
            server_core::accepted_submission_service::AcceptedSubmissionApplicationOutcome::Pending {
                reason:
                    server_core::accepted_submission_service::AcceptedSubmissionPendingReason::FastPathFailed(error),
                ..
            } => Err(error),
            server_core::accepted_submission_service::AcceptedSubmissionApplicationOutcome::Executed {
                result,
                ..
            } => Err(learning_data_access::StoreError::Unavailable(format!(
                "Base Course accepted-submission fast path returned {result:?}"
            ))),
        }
    }
}

fn parse_account(value: &str, name: &str) -> Result<AccountId> {
    Ok(AccountId::from_uuid(
        Uuid::parse_str(value).with_context(|| format!("{name} must be a UUID"))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn common_args() -> Vec<String> {
        vec![
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

    fn child_urls() -> (Option<String>, Option<String>, Option<String>) {
        (
            Some("postgres://ple_base_course_installer_login:secret@db/ple".to_string()),
            Some("postgres://ple_base_course_app_login:secret@db/ple".to_string()),
            Some("not-a-database-url".to_string()),
        )
    }

    fn production_urls() -> (String, String, String) {
        (
            "postgres://ple_base_course_installer_login:secret@db/ple?sslmode=verify-full"
                .to_string(),
            "postgres://ple_base_course_app_login:secret@db/ple?sslmode=verify-full".to_string(),
            "not-a-database-url".to_string(),
        )
    }

    #[test]
    fn prepare_maps_to_the_receipt_free_typed_phase() {
        let mut args = common_args();
        args.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let (installer, application, fast_path) = child_urls();
        let parsed =
            parse_arguments_with_database_urls(&args, installer, application, fast_path, None)
                .unwrap();

        assert!(matches!(parsed.phase, BaseCourseInstallPhase::Prepare));
        assert_eq!(
            parsed.participants,
            BaseCourseParticipants::try_new(
                AccountId::from_uuid(Uuid::from_u128(2)),
                AccountId::from_uuid(Uuid::from_u128(3)),
                AccountId::from_uuid(Uuid::from_u128(4)),
                AccountId::from_uuid(Uuid::from_u128(5)),
                AccountId::from_uuid(Uuid::from_u128(6)),
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
        let (installer, application, fast_path) = child_urls();
        let parsed =
            parse_arguments_with_database_urls(&args, installer, application, fast_path, None)
                .unwrap();
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
        let (installer, application, fast_path) = child_urls();
        assert!(
            parse_arguments_with_database_urls(
                &prepare_with_receipt,
                installer,
                application,
                fast_path,
                None
            )
            .is_err()
        );

        let mut collision = common_args();
        collision[5] = "00000000-0000-0000-0000-000000000002".into();
        collision.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let (installer, application, fast_path) = child_urls();
        assert!(
            parse_arguments_with_database_urls(&collision, installer, application, fast_path, None)
                .is_err()
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
            Some("postgres://fast-path-child-only".to_string()),
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
        assert_eq!(
            parsed.fast_path_database_url,
            "postgres://fast-path-child-only"
        );
        assert!(
            parse_arguments_with_database_urls(
                &args,
                None,
                Some("postgres://app".to_string()),
                Some("postgres://fast-path".to_string()),
                None
            )
            .is_err()
        );
        assert!(
            parse_arguments_with_database_urls(
                &args,
                Some("postgres://installer".to_string()),
                None,
                Some("postgres://fast-path".to_string()),
                None,
            )
            .is_err()
        );
        assert!(
            parse_arguments_with_database_urls(
                &args,
                Some("postgres://installer".to_string()),
                Some("postgres://app".to_string()),
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
        let (installer, application, fast_path) = child_urls();
        let production =
            parse_arguments_with_database_urls(&args, installer, application, fast_path, None)
                .expect("production is the safe default");
        assert_eq!(production.deployment_mode, DeploymentMode::Production);

        let (installer, application, fast_path) = child_urls();
        let local = parse_arguments_with_database_urls(
            &args,
            installer,
            application,
            fast_path,
            Some("local".to_string()),
        )
        .expect("local mode is explicit");
        assert_eq!(local.deployment_mode, DeploymentMode::Local);

        let (installer, application, fast_path) = child_urls();
        assert!(
            parse_arguments_with_database_urls(
                &args,
                installer,
                application,
                fast_path,
                Some("development".to_string()),
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn database_pools_require_the_three_closed_database_capabilities() {
        let (installer, application, fast_path) = child_urls();
        assert!(
            database_pools(
                DeploymentMode::Production,
                installer.as_deref().expect("installer URL"),
                application.as_deref().expect("application URL"),
                fast_path.as_deref().expect("fast-path URL"),
            )
            .await
            .is_err()
        );

        let (installer, application, fast_path) = production_urls();
        assert!(
            database_pools(
                DeploymentMode::Production,
                &installer,
                &application,
                &fast_path
            )
            .await
            .is_err()
        );
        let (installer, application, fast_path) = child_urls();
        assert!(
            database_pools(
                DeploymentMode::Local,
                installer.as_deref().expect("installer URL"),
                application.as_deref().expect("application URL"),
                fast_path.as_deref().expect("fast-path URL"),
            )
            .await
            .is_err()
        );
        assert!(
            database_pools(
                DeploymentMode::Local,
                application.as_deref().expect("application URL"),
                installer.as_deref().expect("installer URL"),
                fast_path.as_deref().expect("fast-path URL"),
            )
            .await
            .is_err()
        );
    }
}
