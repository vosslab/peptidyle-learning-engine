//! Live Demo Account mapping and local Sysadmin TOTP seed composition.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use learning_data_access::{
    SysadminTotpSeed, SysadminTotpStore,
    postgres::{
        Pool, PostgresSysadminTotpStore, ProductionLoginProfile, SysadminTotpSeedKeyId,
        SysadminTotpSeedKeyRing, local_development_pool, production_pool,
    },
};
use question_model::AccountId;
use zeroize::Zeroize;

use crate::auth::{SeededDemoAccount, SeededDemoConfig, SeededDemoPersona};

const LIVE_DEMO_ACCOUNT_ID_ENV: [(SeededDemoPersona, &str, &str); 6] = [
    (
        SeededDemoPersona::ElenaInstructor,
        "PLE_LIVE_DEMO_ELENA_INSTRUCTOR_ACCOUNT_ID",
        "Elena Rivera",
    ),
    (
        SeededDemoPersona::PriyaInstructor,
        "PLE_LIVE_DEMO_PRIYA_INSTRUCTOR_ACCOUNT_ID",
        "Priya Shah",
    ),
    (
        SeededDemoPersona::MaryStudent,
        "PLE_LIVE_DEMO_MARY_STUDENT_ACCOUNT_ID",
        "Mary Okafor",
    ),
    (
        SeededDemoPersona::JackStudent,
        "PLE_LIVE_DEMO_JACK_STUDENT_ACCOUNT_ID",
        "Jack Nguyen",
    ),
    (
        SeededDemoPersona::AveryStudent,
        "PLE_LIVE_DEMO_AVERY_STUDENT_ACCOUNT_ID",
        "Avery Thompson",
    ),
    (
        SeededDemoPersona::MorganSysadmin,
        "PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID",
        "Morgan Delgado",
    ),
];

pub(crate) fn live_demo_config_from_env() -> Result<Option<SeededDemoConfig>> {
    live_demo_config_from_environment_values(|name| std::env::var(name).ok())
}

pub(crate) fn live_demo_config_from_environment_values(
    mut value_for: impl FnMut(&str) -> Option<String>,
) -> Result<Option<SeededDemoConfig>> {
    // ASVS 2.2.1 and 16.5.2: each trusted deployment value is independently
    // allow-listed as one fixed persona and malformed values affect only it.
    let configured = LIVE_DEMO_ACCOUNT_ID_ENV
        .iter()
        .filter_map(|(persona, name, display_name)| {
            let value = value_for(name)?;
            let id = AccountId::new(value).ok()?;
            Some((*persona, id, *display_name))
        })
        .collect::<Vec<_>>();
    live_demo_config_from_account_mappings(configured)
}

pub(crate) fn live_demo_config_from_account_mappings(
    configured: Vec<(SeededDemoPersona, AccountId, &'static str)>,
) -> Result<Option<SeededDemoConfig>> {
    let duplicate_accounts = configured.iter().fold(
        std::collections::BTreeMap::<AccountId, usize>::new(),
        |mut counts, (_, account, _)| {
            *counts.entry(account.clone()).or_default() += 1;
            counts
        },
    );
    let accounts = configured
        .into_iter()
        // ASVS 8.2.1-8.2.3: never infer authority from a duplicate mapping.
        .filter(|(_, account, _)| duplicate_accounts[account] == 1)
        .map(|(persona, account, display_name)| {
            SeededDemoAccount::new(persona, account, display_name).map_err(anyhow::Error::msg)
        })
        .collect::<Result<Vec<_>>>()?;
    (!accounts.is_empty())
        .then(|| SeededDemoConfig::new(accounts).map_err(anyhow::Error::msg))
        .transpose()
}

pub(crate) fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value)
}

/// Creates the local-only store after the controller has supplied both
/// independent private files. No absent or malformed value falls back to a
/// fixed key or seed. Seed provisioning is deliberately a controller-owned
/// one-shot after installation data creates Morgan's Account. ASVS 2.2.1,
/// 2.3.1, 6.4.1, and 8.2.1.
pub(crate) fn local_sysadmin_totp_store_from_env(
    pool: Pool,
) -> Result<Option<Arc<PostgresSysadminTotpStore>>> {
    let seed_path = std::env::var("PLE_LOCAL_SYSADMIN_TOTP_SEED_FILE").ok();
    let key_path = std::env::var("PLE_LOCAL_SYSADMIN_TOTP_SEED_KEY_FILE").ok();
    let (Some(seed_path), Some(key_path)) = (seed_path, key_path) else {
        if std::env::var("PLE_LOCAL_SYSADMIN_TOTP_SEED_FILE").is_ok()
            || std::env::var("PLE_LOCAL_SYSADMIN_TOTP_SEED_KEY_FILE").is_ok()
        {
            bail!("local Sysadmin TOTP seed and wrapping-key files must be configured together");
        }
        return Ok(None);
    };
    if seed_path.is_empty() {
        bail!("local Sysadmin TOTP seed file must not be empty");
    }
    let mut key_bytes =
        std::fs::read(key_path).context("could not read the local Sysadmin TOTP wrapping key")?;
    let key: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("local Sysadmin TOTP wrapping key is invalid"))?;
    key_bytes.zeroize();
    let key_id = SysadminTotpSeedKeyId::parse("local-demo-v1")
        .map_err(|_| anyhow::anyhow!("local Sysadmin TOTP wrapping key ID is invalid"))?;
    let key_ring = Arc::new(
        SysadminTotpSeedKeyRing::new(key_id, key, [])
            .map_err(|_| anyhow::anyhow!("local Sysadmin TOTP wrapping key is invalid"))?,
    );
    Ok(Some(Arc::new(PostgresSysadminTotpStore::new(
        pool, key_ring,
    ))))
}

/// Performs the controller-owned local seed transition only after installation
/// data has created Morgan's active Sysadmin Account. The process mode has no
/// listener or browser surface. ASVS 2.3.1 and 6.4.1.
pub async fn provision_local_sysadmin_totp_from_env() -> Result<()> {
    let database_url = required_env("DATABASE_URL")?;
    let pool = if std::env::var("PLE_STORAGE_TOPOLOGY").ok().as_deref() == Some("disposable-local")
    {
        local_development_pool(&database_url, ProductionLoginProfile::Api)
    } else {
        production_pool(&database_url, ProductionLoginProfile::Api)
    }
    .context("could not construct the attested API database pool")?;
    pool.acquire()
        .await
        .context("the attested API database pool could not connect")?;
    let store = local_sysadmin_totp_store_from_env(pool)?
        .ok_or_else(|| anyhow::anyhow!("local Sysadmin TOTP material is not configured"))?;
    let seed_path = required_env("PLE_LOCAL_SYSADMIN_TOTP_SEED_FILE")?;
    let mut seed_bytes =
        std::fs::read(seed_path).context("could not read the local Sysadmin TOTP seed")?;
    let seed = SysadminTotpSeed::from_csprng_bytes(std::mem::take(&mut seed_bytes))
        .map_err(|_| anyhow::anyhow!("local Sysadmin TOTP seed is invalid"))?;
    seed_bytes.zeroize();
    let account = AccountId::new(required_env("PLE_LIVE_DEMO_MORGAN_SYSADMIN_ACCOUNT_ID")?)
        .map_err(|_| anyhow::anyhow!("Morgan local Sysadmin Account ID is invalid"))?;
    store
        .provision_sysadmin_totp_seed(account, seed)
        .await
        .map_err(|_| anyhow::anyhow!("local Sysadmin TOTP seed provisioning failed"))?;
    Ok(())
}
