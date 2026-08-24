//! Closed installer capability for the deterministic Base Course.
//!
//! This facade owns one separately attested installer connection. Every state
//! transition is a versioned PostgreSQL function; Rust never receives general
//! lifecycle, account, approval, or course-table write authority.

use std::fmt::Write as _;

use question_model::{
    AssignmentId, AssignmentItemId, CourseId, CourseMembershipId, EnrollmentId, ProblemId,
    QuestionAttemptId, QuestionId, RunId, StudentId, TenantId, VersionId,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::pool::PoolConnection;
use sqlx::{Connection, PgConnection, Postgres, Row, Transaction};

use crate::StoreError;

use super::{BaseCourseInstallerPool, map_sqlx_error};

const BASELINE_VERSION: &str = "base-course-v1";
const BASE_COURSE_SLOT: &str = "base_course";
const GENETICS_PRACTICE_SLOT: &str = "genetics_practice";
const UNCONSUMED_NAMESPACE_FAILURE: &str = "unconsumed_question_namespace";
const NONEMPTY_RELATION_FAILURE: &str = "nonempty_application_relation";
const COURSE_AGGREGATE_CONFLICT: &str = "course_aggregate_conflict";
const COMPLETION_AGGREGATE_INCOMPLETE: &str = "completion_aggregate_incomplete";
const COMPLETION_TRANSACTION_ATTEMPTS: usize = 3;
const MAX_COMPLETION_RECEIPT_BYTES: usize = 16 * 1024;

enum BaseCoursePrepareWitness {
    Accepted {
        state: String,
        installation_generation: uuid::Uuid,
        recipe_sha256: String,
    },
    Refused(BaseCourseFreshnessRefusal),
}

enum BaseCourseFreshnessRefusal {
    UnconsumedQuestionNamespace,
    NonemptyApplicationRelation(String),
}

impl BaseCourseFreshnessRefusal {
    fn into_store_error(self) -> StoreError {
        match self {
            Self::UnconsumedQuestionNamespace => StoreError::InvalidRecord(
                "live-demo baseline requires an unconsumed question ID namespace; regenerate both stores before Base Course installation".to_string(),
            ),
            Self::NonemptyApplicationRelation(relation) => StoreError::InvalidRecord(format!(
                "live-demo baseline requires an empty public application schema; table {relation} contains live rows; regenerate both stores before Base Course installation"
            )),
        }
    }
}

/// The durable state of the deployment's seeded Base Course.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseCourseInstallState {
    /// Installation has claimed a tenant and can resume only with identical inputs.
    Installing {
        tenant_id: TenantId,
        baseline_version: String,
        installation_generation: uuid::Uuid,
        object_manifest: Value,
        recipe_sha256: String,
    },
    /// The named tenant contains the completed baseline.
    Complete {
        tenant_id: TenantId,
        baseline_version: String,
        installation_generation: uuid::Uuid,
        object_manifest: Value,
        storage_receipt_sha256: String,
        completion_receipt_sha256: String,
        recipe_sha256: String,
    },
}

/// One verified deterministic course result from the installer broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseCourseInstallCourseReceipt {
    /// Whether this call created the strict aggregate or retained an exact interruption prefix.
    pub disposition: BaseCourseInstallCourseDisposition,
    /// The broker-created course identity.
    pub course_id: CourseId,
    /// The broker-created active Instructor membership identity.
    pub instructor_membership_id: question_model::CourseMembershipId,
}

/// Closed course-seed success returned by the database verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseCourseInstallCourseDisposition {
    /// The strict revision-1 course aggregate was created in this transaction.
    Created,
    /// An exact recipe interruption prefix was retained without a write.
    ExactPrefix,
}

/// The only two deterministic course slots accepted by the installer broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseCourseInstallCourseSlot {
    /// The visible Biochemistry Base Course.
    BaseCourse,
    /// The visible Genetics Practice Course.
    GeneticsPractice,
}

/// Host-owned identities that the sealed completion receipt must reproduce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCourseCompletionExpectation {
    tenant_id: TenantId,
    installation_generation: uuid::Uuid,
    recipe_sha256: String,
    course: BaseCourseCompletionCourseExpectation,
    content: BaseCourseCompletionContentExpectation,
    entitlement: BaseCourseCompletionEntitlementExpectation,
    activity: BaseCourseCompletionActivityExpectation,
}

impl BaseCourseCompletionExpectation {
    /// Creates the deterministic subset independently known by the Rust installer.
    pub fn new(
        tenant_id: TenantId,
        installation_generation: uuid::Uuid,
        recipe_sha256: String,
        course: BaseCourseCompletionCourseExpectation,
        content: BaseCourseCompletionContentExpectation,
        entitlement: BaseCourseCompletionEntitlementExpectation,
        activity: BaseCourseCompletionActivityExpectation,
    ) -> Self {
        Self {
            tenant_id,
            installation_generation,
            recipe_sha256,
            course,
            content,
            entitlement,
            activity,
        }
    }
}

/// Exact course and roster episodes independently observed before completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseCourseCompletionCourseExpectation {
    pub base_course_id: CourseId,
    pub practice_course_id: CourseId,
    pub base_instructor_membership_id: CourseMembershipId,
    pub mary_membership_id: CourseMembershipId,
    pub mary_student_id: StudentId,
    pub jack_membership_id: CourseMembershipId,
    pub jack_student_id: StudentId,
    pub practice_instructor_membership_id: CourseMembershipId,
    pub avery_membership_id: CourseMembershipId,
    pub avery_student_id: StudentId,
}

/// Exact publication and assignment identities observed before completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCourseCompletionContentExpectation {
    pub question_id: QuestionId,
    pub problem_id: ProblemId,
    pub version_id: VersionId,
    pub assignment_id: AssignmentId,
    pub assignment_item_id: AssignmentItemId,
}

/// Exact entitlement episodes observed before completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseCourseCompletionEntitlementExpectation {
    pub mary_enrollment_id: EnrollmentId,
    pub jack_enrollment_id: EnrollmentId,
}

/// Exact learner-work episode identities observed before completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseCourseCompletionActivityExpectation {
    pub mary_run_id: RunId,
    pub mary_attempt_id: QuestionAttemptId,
    pub mary_submission_id: uuid::Uuid,
    pub jack_run_id: RunId,
    pub jack_attempt_id: QuestionAttemptId,
}

/// Safe host projection of the immutable database completion receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCourseCompletionReceipt {
    receipt_sha256: String,
}

impl BaseCourseCompletionReceipt {
    /// Returns the digest naming the generation-bound canonical receipt.
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }
}

impl BaseCourseInstallCourseSlot {
    fn as_str(self) -> &'static str {
        match self {
            Self::BaseCourse => BASE_COURSE_SLOT,
            Self::GeneticsPractice => GENETICS_PRACTICE_SLOT,
        }
    }
}

/// Exclusive, linear installer capability backed by one physical connection.
pub struct BaseCourseInstallLock {
    connection: Option<PoolConnection<Postgres>>,
}

impl BaseCourseInstallLock {
    /// Reads the closed lifecycle projection without mutation.
    pub async fn read_state(&mut self) -> Result<Option<BaseCourseInstallState>, StoreError> {
        let mut transaction = self.begin_installer().await?;
        let row = sqlx::query(
            "SELECT state, tenant_id, baseline_version, installation_generation, object_manifest, \
             storage_receipt_sha256, completion_receipt_sha256, recipe_sha256 \
             FROM public.ple_base_course_install_read_v2()",
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        row.map(decode_state).transpose()
    }

    /// Claims or exactly resumes the one canonical installation recipe.
    pub async fn prepare(
        &mut self,
        tenant_id: TenantId,
        baseline_version: &str,
        object_manifest: &Value,
        recipe: &Value,
    ) -> Result<BaseCourseInstallState, StoreError> {
        validate_install_inputs(baseline_version, object_manifest, recipe)?;
        let mut transaction = self.begin_installer().await?;
        let row = sqlx::query(
            "SELECT state, installation_generation, recipe_sha256, \
                    freshness_failure_kind, freshness_relation_name \
             FROM public.ple_base_course_install_prepare_v2($1, $2, $3, $4)",
        )
        .bind(tenant_id.as_uuid())
        .bind(baseline_version)
        .bind(sqlx::types::Json(object_manifest))
        .bind(sqlx::types::Json(recipe))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let (state, installation_generation, recipe_sha256) = match decode_prepare_witness(&row)? {
            BaseCoursePrepareWitness::Accepted {
                state,
                installation_generation,
                recipe_sha256,
            } => (state, installation_generation, recipe_sha256),
            BaseCoursePrepareWitness::Refused(refusal) => {
                transaction.rollback().await.map_err(map_sqlx_error)?;
                return Err(refusal.into_store_error());
            }
        };
        validate_sha256(&recipe_sha256, "Base Course recipe")?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        match state.as_str() {
            "installing" => Ok(BaseCourseInstallState::Installing {
                tenant_id,
                baseline_version: baseline_version.to_owned(),
                installation_generation,
                object_manifest: object_manifest.clone(),
                recipe_sha256,
            }),
            "complete" => self.read_state().await?.ok_or_else(|| {
                StoreError::Unavailable("completed Base Course state disappeared".to_string())
            }),
            _ => Err(StoreError::Unavailable(
                "Base Course installer returned an invalid lifecycle state".to_string(),
            )),
        }
    }

    /// Creates or exactly verifies the recipe's account-and-approval aggregate.
    pub async fn seed_accounts(
        &mut self,
        installation_generation: uuid::Uuid,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_installer().await?;
        sqlx::query("SELECT public.ple_base_course_install_seed_accounts_v2($1)")
            .bind(installation_generation)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)
    }

    /// Creates or exactly verifies one named course slot from the retained recipe.
    pub async fn seed_course(
        &mut self,
        installation_generation: uuid::Uuid,
        slot: BaseCourseInstallCourseSlot,
    ) -> Result<BaseCourseInstallCourseReceipt, StoreError> {
        let mut transaction = self.begin_installer().await?;
        let row = sqlx::query(
            "SELECT seed_outcome, course_id, instructor_membership_id, failure_kind \
             FROM public.ple_base_course_install_seed_course_v2($1, $2)",
        )
        .bind(installation_generation)
        .bind(slot.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let receipt = decode_seed_course_witness(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    /// Makes an identical installing marker terminal after successful convergence.
    pub async fn mark_complete(
        &mut self,
        tenant_id: TenantId,
        baseline_version: &str,
        installation_generation: uuid::Uuid,
        object_manifest: &Value,
        storage_receipt_sha256: &str,
        expectation: &BaseCourseCompletionExpectation,
    ) -> Result<BaseCourseCompletionReceipt, StoreError> {
        validate_complete_inputs(baseline_version, object_manifest, storage_receipt_sha256)?;
        if expectation.tenant_id != tenant_id
            || expectation.installation_generation != installation_generation
        {
            return Err(StoreError::InvalidRecord(
                "Base Course completion expectation does not bind the active generation"
                    .to_string(),
            ));
        }
        for attempt in 1..=COMPLETION_TRANSACTION_ATTEMPTS {
            let mut transaction = self.begin_serializable_installer().await?;
            let result = sqlx::query(
                "SELECT failure_kind, canonical_receipt, canonical_receipt_text, receipt_sha256 \
                 FROM public.ple_base_course_install_complete_v2($1, $2, $3, $4, $5)",
            )
            .bind(tenant_id.as_uuid())
            .bind(installation_generation)
            .bind(baseline_version)
            .bind(sqlx::types::Json(object_manifest))
            .bind(storage_receipt_sha256)
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error);
            let row = match result {
                Err(StoreError::RetryableTransaction)
                    if attempt < COMPLETION_TRANSACTION_ATTEMPTS =>
                {
                    transaction.rollback().await.map_err(map_sqlx_error)?;
                    continue;
                }
                Err(error) => return Err(error),
                Ok(row) => row,
            };
            let receipt = match decode_completion_witness(&row, expectation) {
                Ok(receipt) => receipt,
                Err(error) => {
                    transaction.rollback().await.map_err(map_sqlx_error)?;
                    return Err(error);
                }
            };
            // A commit error is never replayed: its outcome can be ambiguous even
            // when a database code resembles a retryable transaction failure.
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(receipt);
        }
        unreachable!("the bounded completion retry loop always returns")
    }

    /// Releases the session lock through the installer broker.
    pub async fn release(mut self) -> Result<(), StoreError> {
        let mut transaction = self.begin_installer().await?;
        sqlx::query("SELECT public.ple_base_course_install_release_lock_v1()")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        self.connection.take();
        Ok(())
    }

    /// Closes the held connection after a failed installation attempt.
    pub async fn abort(mut self) -> Result<(), StoreError> {
        close_locked_connection(self.take_connection()?).await
    }

    async fn begin_installer(&mut self) -> Result<Transaction<'_, Postgres>, StoreError> {
        let connection = self.connection_mut()?;
        let mut transaction = connection.begin().await.map_err(map_sqlx_error)?;
        // ASVS 2.3.1, 2.3.3, 8.2.1, 8.3.1: enter the exact installer role
        // before a bounded broker call receives tenant or lifecycle data.
        sqlx::query("SET LOCAL ROLE ple_base_course_installer")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    async fn begin_serializable_installer(
        &mut self,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let connection = self.connection_mut()?;
        let mut transaction = connection.begin().await.map_err(map_sqlx_error)?;
        // ASVS 2.3.3, 15.4.2: isolation is selected before role entry or any
        // application read, so the verifier and marker share one exact snapshot.
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_base_course_installer")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    fn connection_mut(&mut self) -> Result<&mut PoolConnection<Postgres>, StoreError> {
        self.connection.as_mut().ok_or_else(|| {
            StoreError::Unavailable("Base Course installation lock is closed".to_string())
        })
    }

    fn take_connection(&mut self) -> Result<PoolConnection<Postgres>, StoreError> {
        self.connection.take().ok_or_else(|| {
            StoreError::Unavailable("Base Course installation lock is closed".to_string())
        })
    }
}

impl Drop for BaseCourseInstallLock {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            drop(connection.detach());
        }
    }
}

/// Acquires the deployment-wide lock using only a separately attested installer pool.
pub async fn acquire_base_course_install_lock(
    pool: &BaseCourseInstallerPool,
) -> Result<BaseCourseInstallLock, StoreError> {
    let mut connection = pool
        .acquire_pool()
        .acquire()
        .await
        .map_err(map_sqlx_error)?;
    let mut transaction = connection.begin().await.map_err(map_sqlx_error)?;
    sqlx::query("SET LOCAL ROLE ple_base_course_installer")
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
    sqlx::query("SELECT public.ple_base_course_install_acquire_lock_v1()")
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
    transaction.commit().await.map_err(map_sqlx_error)?;
    Ok(BaseCourseInstallLock {
        connection: Some(connection),
    })
}

async fn close_locked_connection(connection: PoolConnection<Postgres>) -> Result<(), StoreError> {
    let connection: PgConnection = connection.detach();
    connection.close().await.map_err(map_sqlx_error)
}

fn decode_prepare_witness(
    row: &sqlx::postgres::PgRow,
) -> Result<BaseCoursePrepareWitness, StoreError> {
    let state: Option<String> = row.try_get("state").map_err(map_sqlx_error)?;
    let installation_generation: Option<uuid::Uuid> = row
        .try_get("installation_generation")
        .map_err(map_sqlx_error)?;
    let recipe_sha256: Option<String> = row.try_get("recipe_sha256").map_err(map_sqlx_error)?;
    let failure_kind: Option<String> = row
        .try_get("freshness_failure_kind")
        .map_err(map_sqlx_error)?;
    let relation_name: Option<String> = row
        .try_get("freshness_relation_name")
        .map_err(map_sqlx_error)?;
    let refusal = decode_freshness_refusal(failure_kind.as_deref(), relation_name.as_deref())?;
    match (state, installation_generation, recipe_sha256, refusal) {
        (Some(state), Some(installation_generation), Some(recipe_sha256), None) => {
            Ok(BaseCoursePrepareWitness::Accepted {
                state,
                installation_generation,
                recipe_sha256,
            })
        }
        (None, None, None, Some(refusal)) => Ok(BaseCoursePrepareWitness::Refused(refusal)),
        _ => Err(StoreError::Unavailable(
            "Base Course installer returned an invalid prepare witness".to_string(),
        )),
    }
}

fn decode_seed_course_witness(
    row: &sqlx::postgres::PgRow,
) -> Result<BaseCourseInstallCourseReceipt, StoreError> {
    let outcome: String = row.try_get("seed_outcome").map_err(map_sqlx_error)?;
    let course_id: Option<uuid::Uuid> = row.try_get("course_id").map_err(map_sqlx_error)?;
    let instructor_membership_id: Option<uuid::Uuid> = row
        .try_get("instructor_membership_id")
        .map_err(map_sqlx_error)?;
    let failure_kind: Option<String> = row.try_get("failure_kind").map_err(map_sqlx_error)?;
    decode_seed_course_values(
        &outcome,
        course_id,
        instructor_membership_id,
        failure_kind.as_deref(),
    )
}

fn decode_seed_course_values(
    outcome: &str,
    course_id: Option<uuid::Uuid>,
    instructor_membership_id: Option<uuid::Uuid>,
    failure_kind: Option<&str>,
) -> Result<BaseCourseInstallCourseReceipt, StoreError> {
    match (outcome, course_id, instructor_membership_id, failure_kind) {
        ("created" | "exact_prefix", Some(course_id), Some(membership_id), None)
            if !course_id.is_nil() && !membership_id.is_nil() =>
        {
            Ok(BaseCourseInstallCourseReceipt {
                disposition: if outcome == "created" {
                    BaseCourseInstallCourseDisposition::Created
                } else {
                    BaseCourseInstallCourseDisposition::ExactPrefix
                },
                course_id: CourseId::from_uuid(course_id),
                instructor_membership_id: question_model::CourseMembershipId::from_uuid(
                    membership_id,
                ),
            })
        }
        ("refused", None, None, Some(COURSE_AGGREGATE_CONFLICT)) => Err(StoreError::InvalidRecord(
            "Base Course course aggregate conflicts with the versioned recipe".to_string(),
        )),
        _ => Err(StoreError::Unavailable(
            "Base Course installer returned an invalid course-seed witness".to_string(),
        )),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletionReceiptWire {
    schema_version: u8,
    baseline_version: String,
    installation_generation: uuid::Uuid,
    tenant_id: uuid::Uuid,
    recipe_sha256: String,
    course_graph: CompletionCourseGraphWire,
    content_graph: CompletionContentGraphWire,
    entitlement_graph: CompletionEntitlementGraphWire,
    activity_graph: CompletionActivityGraphWire,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletionCourseGraphWire {
    base_course_id: uuid::Uuid,
    practice_course_id: uuid::Uuid,
    base_instructor_membership_id: uuid::Uuid,
    mary_membership_id: uuid::Uuid,
    mary_student_id: uuid::Uuid,
    jack_membership_id: uuid::Uuid,
    jack_student_id: uuid::Uuid,
    practice_instructor_membership_id: uuid::Uuid,
    avery_membership_id: uuid::Uuid,
    avery_student_id: uuid::Uuid,
    base_roster_revision: i64,
    practice_roster_revision: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletionContentGraphWire {
    question_id: String,
    problem_id: uuid::Uuid,
    version_id: uuid::Uuid,
    assignment_id: uuid::Uuid,
    assignment_item_id: uuid::Uuid,
    content_sha256: String,
    payload_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletionEntitlementGraphWire {
    mary_enrollment_id: uuid::Uuid,
    jack_enrollment_id: uuid::Uuid,
    mary_basis_sha256: String,
    jack_basis_sha256: String,
    applicable_scope_sha256: String,
    mary_summary_sha256: String,
    jack_summary_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletionActivityGraphWire {
    mary_run_id: uuid::Uuid,
    mary_attempt_id: uuid::Uuid,
    mary_submission_id: uuid::Uuid,
    jack_run_id: uuid::Uuid,
    jack_attempt_id: uuid::Uuid,
    mary_run_sha256: String,
    jack_run_sha256: String,
    mary_attempt_sha256: String,
    jack_attempt_sha256: String,
    mary_presentation_sha256: String,
    jack_presentation_sha256: String,
    mary_grading_sha256: String,
    jack_grading_sha256: String,
    submission_sha256: String,
    idempotency_request_sha256: String,
    idempotency_payload_sha256: String,
    evaluation_sha256: String,
    feedback_sha256: String,
    snapshot_run_sha256: String,
    snapshot_summary_sha256: String,
    snapshot_presentation_sha256: String,
}

fn decode_completion_witness(
    row: &sqlx::postgres::PgRow,
    expected: &BaseCourseCompletionExpectation,
) -> Result<BaseCourseCompletionReceipt, StoreError> {
    let failure_kind: Option<String> = row.try_get("failure_kind").map_err(map_sqlx_error)?;
    let canonical_receipt: Option<sqlx::types::Json<Value>> =
        row.try_get("canonical_receipt").map_err(map_sqlx_error)?;
    let canonical_receipt_text: Option<String> = row
        .try_get("canonical_receipt_text")
        .map_err(map_sqlx_error)?;
    let receipt_sha256: Option<String> = row.try_get("receipt_sha256").map_err(map_sqlx_error)?;
    decode_completion_values(
        failure_kind.as_deref(),
        canonical_receipt.map(|value| value.0),
        canonical_receipt_text.as_deref(),
        receipt_sha256.as_deref(),
        expected,
    )
}

fn decode_completion_values(
    failure_kind: Option<&str>,
    canonical_receipt: Option<Value>,
    canonical_receipt_text: Option<&str>,
    receipt_sha256: Option<&str>,
    expected: &BaseCourseCompletionExpectation,
) -> Result<BaseCourseCompletionReceipt, StoreError> {
    match (
        failure_kind,
        canonical_receipt,
        canonical_receipt_text,
        receipt_sha256,
    ) {
        (Some(COMPLETION_AGGREGATE_INCOMPLETE), None, None, None) => {
            Err(StoreError::InvalidRecord(
                "Base Course completion aggregate does not exactly match the versioned recipe"
                    .to_string(),
            ))
        }
        (None, Some(value), Some(receipt_text), Some(receipt_sha256)) => {
            validate_sha256(receipt_sha256, "Base Course completion receipt")?;
            if receipt_text.len() > MAX_COMPLETION_RECEIPT_BYTES
                || sha256_hex(receipt_text.as_bytes()) != receipt_sha256
            {
                return Err(StoreError::Unavailable(
                    "Base Course completion broker returned inconsistent receipt evidence"
                        .to_string(),
                ));
            }
            let parsed_value: Value = serde_json::from_str(receipt_text).map_err(|_| {
                StoreError::Unavailable(
                    "Base Course completion broker returned an invalid typed receipt".to_string(),
                )
            })?;
            if parsed_value != value {
                return Err(StoreError::Unavailable(
                    "Base Course completion broker returned inconsistent receipt projections"
                        .to_string(),
                ));
            }
            let receipt: CompletionReceiptWire =
                serde_json::from_value(parsed_value).map_err(|_| {
                    StoreError::Unavailable(
                        "Base Course completion broker returned an invalid typed receipt"
                            .to_string(),
                    )
                })?;
            validate_completion_receipt(&receipt, expected)?;
            Ok(BaseCourseCompletionReceipt {
                receipt_sha256: receipt_sha256.to_owned(),
            })
        }
        _ => Err(StoreError::Unavailable(
            "Base Course completion broker returned an invalid witness".to_string(),
        )),
    }
}

fn validate_completion_receipt(
    receipt: &CompletionReceiptWire,
    expected: &BaseCourseCompletionExpectation,
) -> Result<(), StoreError> {
    let graph_matches = receipt.schema_version == 1
        && receipt.baseline_version == BASELINE_VERSION
        && receipt.installation_generation == expected.installation_generation
        && receipt.tenant_id == expected.tenant_id.as_uuid()
        && receipt.recipe_sha256 == expected.recipe_sha256
        && receipt.course_graph.base_course_id == expected.course.base_course_id.as_uuid()
        && receipt.course_graph.practice_course_id == expected.course.practice_course_id.as_uuid()
        && receipt.course_graph.base_instructor_membership_id
            == expected.course.base_instructor_membership_id.as_uuid()
        && receipt.course_graph.mary_membership_id == expected.course.mary_membership_id.as_uuid()
        && receipt.course_graph.mary_student_id == expected.course.mary_student_id.as_uuid()
        && receipt.course_graph.jack_membership_id == expected.course.jack_membership_id.as_uuid()
        && receipt.course_graph.jack_student_id == expected.course.jack_student_id.as_uuid()
        && receipt.course_graph.practice_instructor_membership_id
            == expected.course.practice_instructor_membership_id.as_uuid()
        && receipt.course_graph.avery_membership_id
            == expected.course.avery_membership_id.as_uuid()
        && receipt.course_graph.avery_student_id == expected.course.avery_student_id.as_uuid()
        && receipt.course_graph.base_roster_revision == 3
        && receipt.course_graph.practice_roster_revision == 2
        && receipt.content_graph.question_id == expected.content.question_id.to_string()
        && receipt.content_graph.problem_id == expected.content.problem_id.as_uuid()
        && receipt.content_graph.version_id == expected.content.version_id.as_uuid()
        && receipt.content_graph.assignment_id == expected.content.assignment_id.as_uuid()
        && receipt.content_graph.assignment_item_id
            == expected.content.assignment_item_id.as_uuid()
        && receipt.entitlement_graph.mary_enrollment_id
            == expected.entitlement.mary_enrollment_id.as_uuid()
        && receipt.entitlement_graph.jack_enrollment_id
            == expected.entitlement.jack_enrollment_id.as_uuid()
        && receipt.activity_graph.mary_run_id == expected.activity.mary_run_id.as_uuid()
        && receipt.activity_graph.mary_attempt_id == expected.activity.mary_attempt_id.as_uuid()
        && receipt.activity_graph.mary_submission_id == expected.activity.mary_submission_id
        && receipt.activity_graph.jack_run_id == expected.activity.jack_run_id.as_uuid()
        && receipt.activity_graph.jack_attempt_id == expected.activity.jack_attempt_id.as_uuid();
    if !graph_matches {
        return Err(StoreError::InvalidRecord(
            "Base Course completion receipt differs from the converged deterministic graph"
                .to_string(),
        ));
    }
    validate_sha256(&receipt.recipe_sha256, "Base Course completion recipe")?;
    for hash in completion_receipt_hashes(receipt) {
        validate_sha256(hash, "Base Course completion evidence")?;
    }
    if receipt.content_graph.content_sha256 != receipt.content_graph.payload_sha256 {
        return Err(StoreError::InvalidRecord(
            "Base Course completion receipt contains inconsistent publication hashes".to_string(),
        ));
    }
    Ok(())
}

fn completion_receipt_hashes(receipt: &CompletionReceiptWire) -> [&str; 24] {
    [
        &receipt.content_graph.content_sha256,
        &receipt.content_graph.payload_sha256,
        &receipt.entitlement_graph.mary_basis_sha256,
        &receipt.entitlement_graph.jack_basis_sha256,
        &receipt.entitlement_graph.applicable_scope_sha256,
        &receipt.entitlement_graph.mary_summary_sha256,
        &receipt.entitlement_graph.jack_summary_sha256,
        &receipt.activity_graph.mary_run_sha256,
        &receipt.activity_graph.jack_run_sha256,
        &receipt.activity_graph.mary_attempt_sha256,
        &receipt.activity_graph.jack_attempt_sha256,
        &receipt.activity_graph.mary_presentation_sha256,
        &receipt.activity_graph.jack_presentation_sha256,
        &receipt.activity_graph.mary_grading_sha256,
        &receipt.activity_graph.jack_grading_sha256,
        &receipt.activity_graph.submission_sha256,
        &receipt.activity_graph.idempotency_request_sha256,
        &receipt.activity_graph.idempotency_payload_sha256,
        &receipt.activity_graph.evaluation_sha256,
        &receipt.activity_graph.feedback_sha256,
        &receipt.activity_graph.snapshot_run_sha256,
        &receipt.activity_graph.snapshot_summary_sha256,
        &receipt.activity_graph.snapshot_presentation_sha256,
        &receipt.recipe_sha256,
    ]
}

fn decode_freshness_refusal(
    failure_kind: Option<&str>,
    relation_name: Option<&str>,
) -> Result<Option<BaseCourseFreshnessRefusal>, StoreError> {
    match (failure_kind, relation_name) {
        (None, None) => Ok(None),
        (Some(UNCONSUMED_NAMESPACE_FAILURE), None) => Ok(Some(
            BaseCourseFreshnessRefusal::UnconsumedQuestionNamespace,
        )),
        (Some(NONEMPTY_RELATION_FAILURE), Some(relation))
            if is_safe_public_relation_witness(relation) =>
        {
            Ok(Some(
                BaseCourseFreshnessRefusal::NonemptyApplicationRelation(relation.to_owned()),
            ))
        }
        _ => Err(StoreError::Unavailable(
            "Base Course installer returned an invalid freshness refusal".to_string(),
        )),
    }
}

fn is_safe_public_relation_witness(relation: &str) -> bool {
    relation.strip_prefix("public.").is_some_and(|name| {
        !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    })
}

fn decode_state(row: sqlx::postgres::PgRow) -> Result<BaseCourseInstallState, StoreError> {
    let state: String = row.try_get("state").map_err(map_sqlx_error)?;
    let tenant_id: uuid::Uuid = row.try_get("tenant_id").map_err(map_sqlx_error)?;
    let baseline_version: String = row.try_get("baseline_version").map_err(map_sqlx_error)?;
    let installation_generation: uuid::Uuid = row
        .try_get("installation_generation")
        .map_err(map_sqlx_error)?;
    let object_manifest: sqlx::types::Json<Value> =
        row.try_get("object_manifest").map_err(map_sqlx_error)?;
    let recipe_sha256: String = row.try_get("recipe_sha256").map_err(map_sqlx_error)?;
    validate_sha256(&recipe_sha256, "stored Base Course recipe")?;
    let storage_receipt_sha256: Option<String> = row
        .try_get("storage_receipt_sha256")
        .map_err(map_sqlx_error)?;
    let completion_receipt_sha256: Option<String> = row
        .try_get("completion_receipt_sha256")
        .map_err(map_sqlx_error)?;
    match (
        state.as_str(),
        storage_receipt_sha256,
        completion_receipt_sha256,
    ) {
        ("installing", None, None) => Ok(BaseCourseInstallState::Installing {
            tenant_id: TenantId::from_uuid(tenant_id),
            baseline_version,
            installation_generation,
            object_manifest: object_manifest.0,
            recipe_sha256,
        }),
        ("complete", Some(storage_receipt_sha256), Some(completion_receipt_sha256)) => {
            validate_sha256(&storage_receipt_sha256, "stored Base Course receipt")?;
            validate_sha256(
                &completion_receipt_sha256,
                "stored Base Course completion receipt",
            )?;
            Ok(BaseCourseInstallState::Complete {
                tenant_id: TenantId::from_uuid(tenant_id),
                baseline_version,
                installation_generation,
                object_manifest: object_manifest.0,
                storage_receipt_sha256,
                completion_receipt_sha256,
                recipe_sha256,
            })
        }
        _ => Err(StoreError::Unavailable(
            "live-demo install state violates its lifecycle invariant".to_string(),
        )),
    }
}

fn validate_install_inputs(
    baseline_version: &str,
    object_manifest: &Value,
    recipe: &Value,
) -> Result<(), StoreError> {
    if baseline_version != BASELINE_VERSION || object_manifest != &Value::Array(Vec::new()) {
        return Err(StoreError::InvalidRecord(
            "live-demo install inputs do not match the supported baseline".to_string(),
        ));
    }
    if recipe.get("schemaVersion") != Some(&Value::from(1))
        || !recipe.get("participants").is_some_and(Value::is_object)
        || !recipe.get("courses").is_some_and(Value::is_object)
    {
        return Err(StoreError::InvalidRecord(
            "Base Course recipe does not have the supported canonical shape".to_string(),
        ));
    }
    Ok(())
}

fn validate_complete_inputs(
    baseline_version: &str,
    object_manifest: &Value,
    storage_receipt_sha256: &str,
) -> Result<(), StoreError> {
    if baseline_version != BASELINE_VERSION || object_manifest != &Value::Array(Vec::new()) {
        return Err(StoreError::InvalidRecord(
            "live-demo install inputs do not match the supported baseline".to_string(),
        ));
    }
    validate_sha256(storage_receipt_sha256, "live-demo storage receipt")
}

fn validate_sha256(value: &str, label: &str) -> Result<(), StoreError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(StoreError::Unavailable(format!(
            "{label} hash violates its lowercase SHA-256 invariant"
        )));
    }
    Ok(())
}

fn sha256_hex(value: &[u8]) -> String {
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut hash, "{byte:02x}").expect("writing a SHA-256 hash to String cannot fail");
    }
    hash
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_recipe_shape_requires_versioned_participants_and_courses() {
        let valid = json!({"schemaVersion": 1, "participants": {}, "courses": {}});
        assert!(validate_install_inputs(BASELINE_VERSION, &json!([]), &valid).is_ok());
        assert!(validate_install_inputs(BASELINE_VERSION, &json!([]), &json!({})).is_err());
    }

    #[test]
    fn installer_slot_is_closed_to_the_two_recipe_courses() {
        assert_eq!(
            BaseCourseInstallCourseSlot::BaseCourse.as_str(),
            BASE_COURSE_SLOT
        );
        assert_eq!(
            BaseCourseInstallCourseSlot::GeneticsPractice.as_str(),
            GENETICS_PRACTICE_SLOT
        );
    }

    #[test]
    fn receipt_hashes_are_lowercase_sha256() {
        assert!(validate_sha256(&"a".repeat(64), "test").is_ok());
        assert!(validate_sha256(&"A".repeat(64), "test").is_err());
        assert!(validate_sha256("short", "test").is_err());
    }

    #[path = "completion_tests.rs"]
    mod completion_tests;
}
