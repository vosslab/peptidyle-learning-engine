//! LDA-lock-backed prepare, install, resume, and retained orchestration.

use learning_data_access::postgres::{
    BaseCourseCompletionActivityExpectation, BaseCourseCompletionContentExpectation,
    BaseCourseCompletionCourseExpectation, BaseCourseCompletionEntitlementExpectation,
    BaseCourseCompletionExpectation, BaseCourseInstallCourseSlot, BaseCourseInstallState,
    BaseCourseInstallerPool, PostgresStore,
};

use crate::publication;
use crate::receipt::{
    BaseCourseAction, BaseCourseInstallOutput, BaseCourseInstallStateOutput, output,
    validate_storage_receipt, verify_retained_storage_receipt_sha256,
};
use crate::records::{
    BASELINE_VERSION, BaseCourseIds, base_course, installation_recipe, practice_course,
};
use crate::{
    AcceptedSubmissionSeedExecutor, BaseCourseInstallError, BaseCourseInstallPhase,
    BaseCourseInstallRequest, BaseCourseParticipants,
};

struct PendingInstallation {
    participants: BaseCourseParticipants,
    ids: BaseCourseIds,
    resumed: bool,
    generation: uuid::Uuid,
    object_manifest: serde_json::Value,
    storage_receipt_sha256: String,
    recipe_sha256: String,
}

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
    installer_pool: &BaseCourseInstallerPool,
    store: &PostgresStore,
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    request: BaseCourseInstallRequest,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    // ASVS 15.4.3: LDA owns acquisition and release of the session advisory lock.
    let mut lock = learning_data_access::postgres::acquire_base_course_install_lock(installer_pool)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "acquiring the Base Course installation lock",
                source,
            )
        })?;
    let result = install_locked(&mut lock, store, seed_executor, request).await;
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
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    request: BaseCourseInstallRequest,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    let (participants, phase) = request.into_parts();
    let object_manifest = serde_json::json!([]);
    let ids = BaseCourseIds::for_installation();
    let recipe = installation_recipe(participants, ids)?;
    let state_before = lock.read_state().await.map_err(|source| {
        BaseCourseInstallError::persistence("reading the Base Course lifecycle state", source)
    })?;
    let state = lock
        .prepare(BASELINE_VERSION, &object_manifest, &recipe)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("claiming the Base Course lifecycle state", source)
        })?;
    let installation_generation = match state {
        BaseCourseInstallState::Complete {
            installation_generation,
            storage_receipt_sha256,
            completion_receipt_sha256,
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
                Some(completion_receipt_sha256),
                None,
            );
        }
        BaseCourseInstallState::Installing {
            installation_generation,
            recipe_sha256,
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
                    seed_executor,
                    PendingInstallation {
                        participants,
                        ids,
                        resumed: state_before.is_some(),
                        generation: installation_generation,
                        object_manifest,
                        storage_receipt_sha256,
                        recipe_sha256,
                    },
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
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    pending: PendingInstallation,
) -> Result<BaseCourseInstallOutput, BaseCourseInstallError> {
    lock.seed_accounts(pending.generation)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("seeding the Base Course accounts", source)
        })?;
    let base_course_receipt = lock
        .seed_course(pending.generation, BaseCourseInstallCourseSlot::BaseCourse)
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("seeding the Base Course course slot", source)
        })?;
    let practice_course_receipt = lock
        .seed_course(
            pending.generation,
            BaseCourseInstallCourseSlot::GeneticsPractice,
        )
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence("seeding the Genetics Practice course slot", source)
        })?;
    if base_course_receipt.course_id != pending.ids.base_course
        || practice_course_receipt.course_id != pending.ids.practice_course
    {
        return Err(BaseCourseInstallError::baseline(
            "the installer broker returned a course identity outside the deterministic recipe",
        ));
    }
    publication::verify_installer_courses(
        store,
        pending.participants,
        base_course(pending.ids.base_course)?,
        practice_course(pending.ids.practice_course)?,
    )
    .await?;
    let verified = publication::converge(store, seed_executor, pending.participants).await?;
    let expectation = BaseCourseCompletionExpectation::new(
        pending.generation,
        pending.recipe_sha256,
        BaseCourseCompletionCourseExpectation {
            base_course_id: pending.ids.base_course,
            practice_course_id: pending.ids.practice_course,
            base_instructor_membership_id: base_course_receipt.instructor_membership_id,
            mary_membership_id: question_model::CourseMembershipId::from_uuid(
                verified.mary_membership.id.as_uuid(),
            ),
            mary_student_id: verified.mary_membership.student,
            jack_membership_id: question_model::CourseMembershipId::from_uuid(
                verified.jack_membership.id.as_uuid(),
            ),
            jack_student_id: verified.jack_membership.student,
            practice_instructor_membership_id: practice_course_receipt.instructor_membership_id,
            avery_membership_id: question_model::CourseMembershipId::from_uuid(
                verified.avery_membership.id.as_uuid(),
            ),
            avery_student_id: verified.avery_membership.student,
        },
        BaseCourseCompletionContentExpectation {
            question_id: verified.manifest.question_id().clone(),
            problem_id: pending.ids.problem,
            version_id: pending.ids.version,
            assignment_id: pending.ids.assignment,
            assignment_item_id: pending.ids.assignment_item,
        },
        BaseCourseCompletionEntitlementExpectation {
            mary_enrollment_id: verified.mary_enrollment.id,
            jack_enrollment_id: verified.jack_enrollment.id,
        },
        BaseCourseCompletionActivityExpectation {
            mary_run_id: pending.ids.mary_run,
            mary_attempt_id: pending.ids.mary_attempt,
            mary_submission_id: pending.ids.mary_attempt.as_uuid(),
            jack_run_id: pending.ids.jack_run,
            jack_attempt_id: pending.ids.jack_attempt,
        },
    );
    let completion_receipt = lock
        .mark_complete(
            BASELINE_VERSION,
            pending.generation,
            &pending.object_manifest,
            &pending.storage_receipt_sha256,
            &expectation,
        )
        .await
        .map_err(|source| {
            BaseCourseInstallError::persistence(
                "completing the Base Course lifecycle state",
                source,
            )
        })?;
    output(
        if pending.resumed {
            BaseCourseAction::Resumed
        } else {
            BaseCourseAction::Installed
        },
        BaseCourseInstallStateOutput::Complete,
        pending.generation,
        Some(pending.storage_receipt_sha256),
        Some(completion_receipt.receipt_sha256().to_string()),
        Some(verified.manifest),
    )
}
