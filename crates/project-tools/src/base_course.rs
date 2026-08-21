//! Direct native-host CLI adapter for the product-owned Base Course installer.

use anyhow::{Context, Result, bail};
use base_course_installation::{
    BaseCourseInstallPhase, BaseCourseInstallRequest, BaseCourseParticipants,
};
use question_model::{TenantId, UserId};
use uuid::Uuid;

use crate::postgres_store;

const USAGE: &str = "usage: cargo tools base-course [--database-url <URL>] --tenant <UUID> --instructor <UUID> --mary <UUID> --jack <UUID> --approval-candidate <UUID> --sysadmin <UUID> --apply-migrations --lifecycle-phase <prepare|install> [--storage-receipt <canonical JSON>] (database URL also reads PLE_MIGRATION_DATABASE_URL)";

#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    database_url: String,
    participants: BaseCourseParticipants,
    phase: BaseCourseInstallPhase,
}

/// Parses host configuration, applies LDA migrations, and invokes one product call.
pub(crate) fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let arguments =
        parse_arguments_with_database_url(args, std::env::var("PLE_MIGRATION_DATABASE_URL").ok())?;
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
    let pool = learning_data_access::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for the Base Course installer")?;
    learning_data_access::postgres::apply_migrations(&pool)
        .await
        .context("applying embedded migrations for the Base Course installer")?;
    let store = postgres_store::configured_postgres_store(pool.clone())?;
    let request = BaseCourseInstallRequest::new(arguments.participants, arguments.phase);
    base_course_installation::install(&pool, &store, request)
        .await
        .context("installing the Base Course")
}

#[cfg(test)]
fn parse_arguments(args: &[String]) -> Result<Arguments> {
    parse_arguments_with_database_url(args, None)
}

fn parse_arguments_with_database_url(
    args: &[String],
    environment_database_url: Option<String>,
) -> Result<Arguments> {
    let mut database_url = None;
    let mut tenant = None;
    let mut instructor = None;
    let mut mary = None;
    let mut jack = None;
    let mut approval_candidate = None;
    let mut sysadmin = None;
    let mut apply_migrations = false;
    let mut lifecycle_phase = None;
    let mut storage_receipt = None;
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        index += 1;
        if flag == "--apply-migrations" && !apply_migrations {
            apply_migrations = true;
            continue;
        }
        let Some(value) = args.get(index) else {
            bail!("{flag} requires a value; {USAGE}");
        };
        index += 1;
        match flag.as_str() {
            "--database-url" if database_url.is_none() => database_url = Some(value.clone()),
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
    if !apply_migrations {
        bail!(
            "--apply-migrations is required to bring the schema current before Base Course installation; {USAGE}"
        );
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
        database_url: database_url
            .or(environment_database_url)
            .ok_or_else(|| anyhow::anyhow!("--database-url is required; {USAGE}"))?,
        participants,
        phase,
    })
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
            "--database-url".into(),
            "postgres://example".into(),
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
            "--apply-migrations".into(),
        ]
    }

    #[test]
    fn prepare_maps_to_the_receipt_free_typed_phase() {
        let mut args = common_args();
        args.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let parsed = parse_arguments(&args).unwrap();

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
        let parsed = parse_arguments(&args).unwrap();
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
        assert!(parse_arguments(&prepare_with_receipt).is_err());

        let mut collision = common_args();
        collision[7] = "00000000-0000-0000-0000-000000000002".into();
        collision.extend(["--lifecycle-phase".into(), "prepare".into()]);
        assert!(parse_arguments(&collision).is_err());
    }

    #[test]
    fn database_url_can_come_only_from_the_child_environment() {
        let mut args = common_args();
        args.drain(0..2);
        args.extend(["--lifecycle-phase".into(), "prepare".into()]);
        let parsed =
            parse_arguments_with_database_url(&args, Some("postgres://child-only".to_string()))
                .unwrap();
        assert_eq!(parsed.database_url, "postgres://child-only");
    }
}
