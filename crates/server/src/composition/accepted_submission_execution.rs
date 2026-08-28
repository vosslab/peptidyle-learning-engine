//! Accepted-submission execution configuration shared by API and recovery.

use anyhow::{Context, Result, bail};

/// Validated bounds shared by the API exact-claim path and recovery worker.
///
/// Polling belongs exclusively to the recovery process, but any accepted
/// submission must see the same lease and execution deadline whichever
/// process claims it.
#[derive(Clone, Copy)]
pub(super) struct AcceptedSubmissionExecutionSettings {
    worker: crate::worker::WorkerSettings,
}

impl AcceptedSubmissionExecutionSettings {
    pub(super) fn from_env() -> Result<Self> {
        Self::from_values(
            optional_env("PLE_WORKER_LEASE_SECONDS")?.as_deref(),
            optional_env("PLE_WORKER_PREPARATION_TIMEOUT_SECONDS")?.as_deref(),
        )
    }

    pub(super) fn from_values(
        lease_seconds: Option<&str>,
        preparation_timeout_seconds: Option<&str>,
    ) -> Result<Self> {
        let lease_seconds = bounded_value(
            "PLE_WORKER_LEASE_SECONDS",
            lease_seconds,
            120_u32,
            1_u32,
            900_u32,
        )?;
        let preparation_timeout_seconds = bounded_value(
            "PLE_WORKER_PREPARATION_TIMEOUT_SECONDS",
            preparation_timeout_seconds,
            90_u64,
            1_u64,
            899_u64,
        )?;
        let worker = crate::worker::WorkerSettings::new(
            lease_seconds,
            std::time::Duration::from_secs(preparation_timeout_seconds),
            1,
        )
        .context("accepted-submission execution settings are incompatible")?;
        Ok(Self { worker })
    }

    pub(super) fn worker_settings(self) -> crate::worker::WorkerSettings {
        self.worker
    }
}

pub(super) fn recovery_database_url_from_env() -> Result<String> {
    required_env("PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL")
}

fn optional_env(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => bail!("{name} must be valid UTF-8"),
    }
}

fn required_env(name: &str) -> Result<String> {
    optional_env(name)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} must be set"))
}

fn bounded_value<T>(
    name: &str,
    configured: Option<&str>,
    default: T,
    minimum: T,
    maximum: T,
) -> Result<T>
where
    T: Copy + Ord + std::str::FromStr,
{
    let value = configured
        .map(|value| {
            value
                .parse::<T>()
                .map_err(|_| anyhow::anyhow!("{name} must be an integer"))
        })
        .transpose()?
        .unwrap_or(default);
    if value < minimum || value > maximum {
        bail!("{name} is outside its supported range");
    }
    Ok(value)
}
