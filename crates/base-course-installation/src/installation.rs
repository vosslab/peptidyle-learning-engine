//! LDA-lock-backed prepare, install, resume, and retained orchestration.

use learning_data_access::postgres::{BaseCourseInstallState, Pool, PostgresStore};

use crate::accounts::ensure_accounts;
use crate::publication;
use crate::receipt::{
    BaseCourseAction, BaseCourseInstallOutput, BaseCourseInstallStateOutput, output,
    validate_storage_receipt, verify_retained_storage_receipt_sha256,
};
use crate::records::BASELINE_VERSION;
use crate::{
    BaseCourseInstallError, BaseCourseInstallPhase, BaseCourseInstallRequest,
    BaseCourseParticipants,
};

/// Installs or observes the Base Course through the sole LDA lifecycle lock and Store.
///
/// The pool and Store are borrowed for this call. The returned output owns every
/// value needed for serialization by a CLI or future deployment initializer.
///
/// # Errors
///
/// Returns a concrete [`BaseCourseInstallError`] for invalid receipts, retained
/// baseline mismatches, native presentation failures, or LDA persistence failures.
pub async fn install(
    pool: &Pool,
    store: &PostgresStore,
    request: BaseCourseInstallRequest,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    // ASVS 15.4.3: LDA owns acquisition and release of the session advisory lock.
    let mut lock = learning_data_access::postgres::acquire_base_course_install_lock(pool)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "acquiring the Base Course installation lock",
                source,
            )
        })?;
    let result = install_locked(&mut lock, store, request).await;
    match result {
        Ok(value) => {
            lock.release().await.map_err(|source| {
                BaseCourseInstallError::persistence(
                    "releasing the Base Course installation lock",
                    source,
                )
            })?;
            Ok(value)
        }
        Err(install) => match lock.abort().await {
            Ok(()) => Err(install),
            Err(cleanup) => Err(BaseCourseInstallError::LockCleanup {
                install: Box::new(install),
                cleanup,
            }),
        },
    }
}

async fn install_locked(
    lock: &mut learning_data_access::postgres::BaseCourseInstallLock,
    store: &PostgresStore,
    request: BaseCourseInstallRequest,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    let (participants, phase) = request.into_parts();
    let object_manifest = serde_json::json!([]);
    let state_before = lock.read_state().await.map_err(|source| {
        BaseCourseInstallError::persistence("reading the Base Course lifecycle state", source)
    })?;
    let state = lock
        .prepare(participants.tenant(), BASELINE_VERSION, &object_manifest)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("claiming the Base Course lifecycle state", source)
        })?;
    let installation_generation = match state {
        BaseCourseInstallState::Complete {
            installation_generation,
            storage_receipt_sha256,
            ..
        } => {
            // ASVS 2.3.1: a complete generation is terminal and performs no seed or storage read.
            verify_retained_storage_receipt_sha256(
                installation_generation,
                &storage_receipt_sha256,
            )?;
            return output(
                BaseCourseAction::Retained,
                BaseCourseInstallStateOutput::Complete,
                installation_generation,
                Some(storage_receipt_sha256),
                None,
            );
        }
        BaseCourseInstallState::Installing {
            installation_generation,
            ..
        } => match phase {
            BaseCourseInstallPhase::Prepare => {
                return output(
                    if state_before.is_some() {
                        BaseCourseAction::Resumed
                    } else {
                        BaseCourseAction::Prepared
                    },
                    BaseCourseInstallStateOutput::Installing,
                    installation_generation,
                    None,
                    None,
                );
            }
            BaseCourseInstallPhase::Install {
                storage_receipt_json,
            } => {
                let storage_receipt_sha256 =
                    validate_storage_receipt(&storage_receipt_json, installation_generation)?;
                complete_installation(
                    lock,
                    store,
                    participants,
                    state_before.is_some(),
                    installation_generation,
                    &object_manifest,
                    storage_receipt_sha256,
                )
                .await?
            }
        },
    };
    Ok(installation_generation)
}

async fn complete_installation(
    lock: &mut learning_data_access::postgres::BaseCourseInstallLock,
    store: &PostgresStore,
    participants: BaseCourseParticipants,
    resumed: bool,
    installation_generation: uuid::Uuid,
    object_manifest: &serde_json::Value,
    storage_receipt_sha256: String,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    ensure_accounts(lock, participants).await?;
    let manifest = publication::converge(store, participants).await?;
    lock.mark_complete(
        participants.tenant(),
        BASELINE_VERSION,
        installation_generation,
        object_manifest,
        &storage_receipt_sha256,
    )
    .await
    .map_err(|source| {
        BaseCourseInstallError::persistence("completing the Base Course lifecycle state", source)
    })?;
    output(
        if resumed {
            BaseCourseAction::Resumed
        } else {
            BaseCourseAction::Installed
        },
        BaseCourseInstallStateOutput::Complete,
        installation_generation,
        Some(storage_receipt_sha256),
        Some(manifest),
    )
}
