//! PostgreSQL broker adapter for B2 curriculum-adoption operations.
//!
//! The migration-owned broker is the authority for source resolution, teaching
//! mutations, receipts, immutable evidence, and derived-current repair.  This
//! module carries only closed qmodel projections across that boundary and
//! verifies every returned binding before committing an operation.

use async_trait::async_trait;
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationCompleted, AlphaInstantiationPreviewRequest,
    AlphaInstantiationPreviewView, AssignmentFastForwardCommand, AssignmentFastForwardCompleted,
    AssignmentFastForwardPreviewRequest, AssignmentFastForwardPreviewView,
    BlueprintInstantiationCommand, BlueprintInstantiationCompleted,
    BlueprintInstantiationPreviewRequest, BlueprintInstantiationPreviewView, CourseReference,
    CourseRolloverCommand, CourseRolloverCompleted, CourseRolloverPreviewRequest,
    CourseRolloverPreviewView, CourseTermShiftCommand, CourseTermShiftCompleted,
    CourseTermShiftPreviewOutcome, CourseTermShiftPreviewRequest,
    CreateSourceDerivedAssignmentCommand, CurriculumAdoptionReconciliationResult,
    CurriculumCourseImportView, ForkAlphaCommand, ForkAlphaCompleted, ForkAlphaPreviewRequest,
    ForkAlphaPreviewView, ReconcileCurriculumAdoptionCommand, SourceDerivedAssignmentCompleted,
    SourceDerivedAssignmentPreviewRequest, SourceDerivedAssignmentPreviewView,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::types::Json;

mod bridge;
use bridge::{
    CurriculumAdoptionBridgeOperationV1, MaterializationBindingV1, MaterializationPlanV1,
    OperationFactsV1, PreparedMaterializationV1, PreparedReconciliationV1, SnapshotFactsV1,
    SqlAdoptionResultV1, complete_alpha, complete_blueprint, complete_fast_forward, complete_fork,
    complete_rollover, complete_source_derived, complete_term_shift, prepare_alpha,
    prepare_blueprint, prepare_fast_forward, prepare_fork, prepare_reconciliation,
    prepare_rollover, prepare_source_derived, prepare_term_shift, project_alpha, project_blueprint,
    project_fast_forward, project_fork, project_import_inspection, project_reconciliation_result,
    project_rollover, project_source_derived, project_term_shift, reconciliation_result,
};

use super::{PostgresStore, map_sqlx_error, retry_transaction};
use crate::{CurriculumAdoptionStore, SessionTokenHash, StoreError, TenantContext};

const MAX_BROKER_JSON_BYTES: usize = 512 * 1024;

const PREFLIGHT_SQL: &str = "SELECT public.ple_curriculum_adoption_preflight_v1($1, $2)";
const SNAPSHOT_SQL: &str = "SELECT public.ple_snapshot_curriculum_adoption_v1($1, $2, $3)";
const MATERIALIZATION_ACTOR_SQL: &str =
    "SELECT public.ple_curriculum_adoption_materialization_actor_v1($1, $2)";
const MATERIALIZE_SQL: &str =
    "SELECT public.ple_materialize_curriculum_adoption_v1($1, $2, $3, $4)";

#[async_trait]
impl CurriculumAdoptionStore for PostgresStore {
    async fn preflight_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_tenant_session_snapshot(context, session).await?;
        let authorized: bool = sqlx::query_scalar(PREFLIGHT_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(session.to_string())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_curriculum_adoption_error)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        authorized.then_some(()).ok_or(StoreError::Forbidden)
    }

    async fn preview_fork_alpha(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: ForkAlphaPreviewRequest,
    ) -> Result<ForkAlphaPreviewView, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewForkAlpha,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::ForkAlpha { source } => project_fork(&request, source),
                _ => Err(StoreError::Unavailable(
                    "fork preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn apply_fork_alpha(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ForkAlphaCommand,
    ) -> Result<ForkAlphaCompleted, StoreError> {
        let request = fork_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::ApplyForkAlpha,
                command: command.clone(),
                request,
            },
            |command, preparation_id, facts| match facts {
                OperationFactsV1::ForkAlpha { source } => Ok(MaterializationPlanV1::ForkAlpha {
                    plan: prepare_fork(preparation_id, command, source)?,
                }),
                _ => Err(StoreError::Unavailable(
                    "fork preparation returned the wrong facts".into(),
                )),
            },
            complete_fork,
        )
        .await
    }

    async fn preview_blueprint_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: BlueprintInstantiationPreviewRequest,
    ) -> Result<BlueprintInstantiationPreviewView, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewBlueprintInstantiation,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::BlueprintInstantiation {
                    source,
                    destination,
                } => project_blueprint(&request, source, destination),
                _ => Err(StoreError::Unavailable(
                    "Blueprint preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn apply_blueprint_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: BlueprintInstantiationCommand,
    ) -> Result<BlueprintInstantiationCompleted, StoreError> {
        let request = blueprint_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::ApplyBlueprintInstantiation,
                command: command.clone(),
                request,
            },
            |command, preparation_id, facts| match facts {
                OperationFactsV1::BlueprintInstantiation {
                    source,
                    destination,
                } => Ok(MaterializationPlanV1::BlueprintInstantiation {
                    plan: prepare_blueprint(preparation_id, command, source, destination)?,
                }),
                _ => Err(StoreError::Unavailable(
                    "Blueprint preparation returned the wrong facts".into(),
                )),
            },
            complete_blueprint,
        )
        .await
    }

    async fn preview_alpha_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: AlphaInstantiationPreviewRequest,
    ) -> Result<AlphaInstantiationPreviewView, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewAlphaInstantiation,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::AlphaInstantiation { source } => project_alpha(&request, source),
                _ => Err(StoreError::Unavailable(
                    "Alpha preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn apply_alpha_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: AlphaInstantiationCommand,
    ) -> Result<AlphaInstantiationCompleted, StoreError> {
        let request = alpha_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::ApplyAlphaInstantiation,
                command: command.clone(),
                request,
            },
            |command, preparation_id, facts| match facts {
                OperationFactsV1::AlphaInstantiation { source } => {
                    Ok(MaterializationPlanV1::AlphaInstantiation {
                        plan: prepare_alpha(preparation_id, command, source)?,
                    })
                }
                _ => Err(StoreError::Unavailable(
                    "Alpha preparation returned the wrong facts".into(),
                )),
            },
            complete_alpha,
        )
        .await
    }

    async fn preview_course_rollover(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CourseRolloverPreviewRequest,
    ) -> Result<CourseRolloverPreviewView, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewCourseRollover,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::CourseRollover { source } => project_rollover(&request, source),
                _ => Err(StoreError::Unavailable(
                    "rollover preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn apply_course_rollover(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CourseRolloverCommand,
    ) -> Result<CourseRolloverCompleted, StoreError> {
        let request = rollover_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::ApplyCourseRollover,
                command: command.clone(),
                request,
            },
            |command, _, facts| match facts {
                OperationFactsV1::CourseRollover { source } => {
                    Ok(MaterializationPlanV1::CourseRollover {
                        plan: prepare_rollover(command, source)?,
                    })
                }
                _ => Err(StoreError::Unavailable(
                    "rollover preparation returned the wrong facts".into(),
                )),
            },
            complete_rollover,
        )
        .await
    }

    async fn preview_course_term_shift(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CourseTermShiftPreviewRequest,
    ) -> Result<CourseTermShiftPreviewOutcome, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewCourseTermShift,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::CourseTermShift { course } => {
                    project_term_shift(&request, course)
                }
                _ => Err(StoreError::Unavailable(
                    "term-shift preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn apply_course_term_shift(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CourseTermShiftCommand,
    ) -> Result<CourseTermShiftCompleted, StoreError> {
        let request = term_shift_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::ApplyCourseTermShift,
                command: command.clone(),
                request,
            },
            |command, _, facts| match facts {
                OperationFactsV1::CourseTermShift { course } => {
                    Ok(MaterializationPlanV1::CourseTermShift {
                        plan: prepare_term_shift(command, course)?,
                    })
                }
                _ => Err(StoreError::Unavailable(
                    "term-shift preparation returned the wrong facts".into(),
                )),
            },
            complete_term_shift,
        )
        .await
    }

    async fn preview_assignment_fast_forward(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: AssignmentFastForwardPreviewRequest,
    ) -> Result<AssignmentFastForwardPreviewView, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewAssignmentFastForward,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::AssignmentFastForward { import } => {
                    project_fast_forward(&request, import)
                }
                _ => Err(StoreError::Unavailable(
                    "fast-forward preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn apply_assignment_fast_forward(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: AssignmentFastForwardCommand,
    ) -> Result<AssignmentFastForwardCompleted, StoreError> {
        let request = fast_forward_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::ApplyAssignmentFastForward,
                command: command.clone(),
                request,
            },
            |command, _, facts| match facts {
                OperationFactsV1::AssignmentFastForward { import } => {
                    let request = AssignmentFastForwardPreviewRequest {
                        course: command.course(),
                        assignment: command.assignment(),
                        import_revision: command.import_revision(),
                        source: command.source(),
                    };
                    Ok(MaterializationPlanV1::AssignmentFastForward {
                        plan: prepare_fast_forward(&request, import)?,
                    })
                }
                _ => Err(StoreError::Unavailable(
                    "fast-forward preparation returned the wrong facts".into(),
                )),
            },
            complete_fast_forward,
        )
        .await
    }

    async fn preview_source_derived_assignment(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: SourceDerivedAssignmentPreviewRequest,
    ) -> Result<SourceDerivedAssignmentPreviewView, StoreError> {
        self.read_snapshot(
            context,
            session,
            CurriculumAdoptionBridgeOperationV1::PreviewSourceDerivedAssignment,
            encode(&request)?,
            |facts| match facts {
                OperationFactsV1::SourceDerivedAssignment {
                    source,
                    destination,
                } => project_source_derived(&request, source, destination),
                _ => Err(StoreError::Unavailable(
                    "source-derived preview returned the wrong facts".into(),
                )),
            },
        )
        .await
    }

    async fn create_source_derived_assignment(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateSourceDerivedAssignmentCommand,
    ) -> Result<SourceDerivedAssignmentCompleted, StoreError> {
        let request = source_derived_command_wire(&command)?;
        self.write_materialized(
            MaterializedWriteContext {
                context,
                session,
                operation: CurriculumAdoptionBridgeOperationV1::CreateSourceDerivedAssignment,
                command: command.clone(),
                request,
            },
            |command, preparation_id, facts| match facts {
                OperationFactsV1::SourceDerivedAssignment {
                    source,
                    destination,
                } => Ok(MaterializationPlanV1::SourceDerivedAssignment {
                    plan: prepare_source_derived(preparation_id, command, source, destination)?,
                }),
                _ => Err(StoreError::Unavailable(
                    "source-derived preparation returned the wrong facts".into(),
                )),
            },
            complete_source_derived,
        )
        .await
    }

    async fn inspect_curriculum_imports(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseReference,
    ) -> Result<Option<CurriculumCourseImportView>, StoreError> {
        Ok(self
            .read_snapshot_optional(
                context,
                session,
                CurriculumAdoptionBridgeOperationV1::InspectImports,
                encode(&course)?,
                |facts| match facts {
                    OperationFactsV1::Inspection { inspection } => {
                        project_import_inspection(inspection)
                    }
                    _ => Err(StoreError::Unavailable(
                        "import inspection returned the wrong facts".into(),
                    )),
                },
            )
            .await?
            .flatten())
    }

    async fn reconcile_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReconcileCurriculumAdoptionCommand,
    ) -> Result<CurriculumAdoptionReconciliationResult, StoreError> {
        self.write_reconciliation(context, session, command).await
    }
}

struct MaterializedWriteContext<C> {
    context: TenantContext,
    session: SessionTokenHash,
    operation: CurriculumAdoptionBridgeOperationV1,
    command: C,
    request: Value,
}

impl PostgresStore {
    async fn read_snapshot<T, D>(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        operation: CurriculumAdoptionBridgeOperationV1,
        request: Value,
        project: D,
    ) -> Result<T, StoreError>
    where
        D: Fn(&OperationFactsV1) -> Result<T, StoreError>,
    {
        let mut transaction = self
            .begin_curriculum_adoption_read_snapshot(context, session)
            .await?;
        let snapshot = self
            .snapshot(&mut transaction, context, session, operation, request)
            .await?;
        let decoded = project(snapshot.preview_facts(operation)?)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(decoded)
    }

    async fn write_materialized<C, T, P, D>(
        &self,
        input: MaterializedWriteContext<C>,
        prepare: P,
        complete: D,
    ) -> Result<T, StoreError>
    where
        C: Clone,
        P: Fn(&C, uuid::Uuid, &OperationFactsV1) -> Result<MaterializationPlanV1, StoreError>
            + Copy,
        D: Fn(&C, &SqlAdoptionResultV1) -> Result<T, StoreError> + Copy,
    {
        let MaterializedWriteContext {
            context,
            session,
            operation,
            command,
            request,
        } = input;
        retry_transaction(|| {
            let command = command.clone();
            let request = request.clone();
            async move {
                let mut transaction = self.begin_tenant_session(context, session).await?;
                let actor = self
                    .materialization_actor(&mut transaction, context, session)
                    .await?;
                let request_sha256 = bridge::request_digest(operation, actor, &request)?;
                let binding = MaterializationBindingV1 {
                    version: bridge::BRIDGE_VERSION,
                    actor,
                    request_sha256,
                };
                let snapshot = self
                    .snapshot_materialized(
                        &mut transaction,
                        context,
                        session,
                        operation,
                        request,
                        &binding,
                    )
                    .await?;
                snapshot.validate_for(operation)?;
                if let SnapshotFactsV1::Replay {
                    actor,
                    request_sha256,
                    result,
                    ..
                } = &snapshot
                {
                    require(
                        *actor == binding.actor && *request_sha256 == binding.request_sha256,
                        "curriculum replay digest disagrees with command",
                    )?;
                    let decoded = complete(&command, result)?;
                    transaction.commit().await.map_err(map_sqlx_error)?;
                    return Ok(decoded);
                }
                let (preparation_id, snapshot_actor, snapshot_digest, facts) =
                    snapshot.preparation(operation)?;
                require(
                    snapshot_actor == binding.actor && snapshot_digest == binding.request_sha256,
                    "curriculum preparation binding disagrees with command",
                )?;
                let plan = prepare(&command, preparation_id, facts)?;
                let prepared = PreparedMaterializationV1 {
                    version: bridge::BRIDGE_VERSION,
                    operation,
                    preparation_id,
                    actor: binding.actor,
                    request_sha256: binding.request_sha256,
                    plan,
                };
                let result: Json<Value> = sqlx::query_scalar(MATERIALIZE_SQL)
                    .bind(context.tenant_id().as_uuid())
                    .bind(session.to_string())
                    .bind(preparation_id)
                    .bind(Json(encode(&prepared)?))
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_curriculum_adoption_error)?;
                let result: SqlAdoptionResultV1 = decode(&bounded_result(result.0)?)?;
                let decoded = complete(&command, &result)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(decoded)
            }
        })
        .await
    }

    /// Repairs only receipt-derived current-import indexes.  Reconciliation is
    /// intentionally outside the receipt-creating command vocabulary, so it
    /// carries the selected immutable repairs rather than a synthetic digest.
    async fn write_reconciliation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReconcileCurriculumAdoptionCommand,
    ) -> Result<CurriculumAdoptionReconciliationResult, StoreError> {
        let request = encode(&command)?;
        retry_transaction(|| {
            let command = command.clone();
            let request = request.clone();
            async move {
                let mut transaction = self.begin_tenant_session(context, session).await?;
                let snapshot = self
                    .snapshot(
                        &mut transaction,
                        context,
                        session,
                        CurriculumAdoptionBridgeOperationV1::Reconcile,
                        request,
                    )
                    .await?;
                let (preparation_id, actor, facts) = snapshot.reconciliation_preparation()?;
                let OperationFactsV1::Reconcile { reconciliation } = facts else {
                    return Err(StoreError::Unavailable(
                        "reconciliation preparation returned the wrong facts".into(),
                    ));
                };
                let repairs = prepare_reconciliation(&command, reconciliation)?;
                let prepared = PreparedReconciliationV1 {
                    version: bridge::BRIDGE_VERSION,
                    operation: CurriculumAdoptionBridgeOperationV1::Reconcile,
                    preparation_id,
                    actor,
                    receipt: command.receipt.clone(),
                    repairs,
                };
                let result: Json<Value> = sqlx::query_scalar(MATERIALIZE_SQL)
                    .bind(context.tenant_id().as_uuid())
                    .bind(session.to_string())
                    .bind(preparation_id)
                    .bind(Json(encode(&prepared)?))
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(map_curriculum_adoption_error)?;
                let result: SqlAdoptionResultV1 = decode(&bounded_result(result.0)?)?;
                let (receipt, repaired_assignments) = reconciliation_result(&result)?;
                require(
                    receipt.idempotency_key == command.receipt.idempotency_key,
                    "reconciliation result receipt disagrees with command",
                )?;
                let projected =
                    project_reconciliation_result(&command, reconciliation, repaired_assignments)?;
                transaction.commit().await.map_err(map_sqlx_error)?;
                Ok(projected)
            }
        })
        .await
    }

    async fn read_snapshot_optional<T, D>(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        operation: CurriculumAdoptionBridgeOperationV1,
        request: Value,
        project: D,
    ) -> Result<Option<T>, StoreError>
    where
        D: Fn(&OperationFactsV1) -> Result<T, StoreError>,
    {
        let mut transaction = self
            .begin_curriculum_adoption_read_snapshot(context, session)
            .await?;
        let snapshot = self
            .snapshot(&mut transaction, context, session, operation, request)
            .await?;
        let decoded = match &snapshot {
            SnapshotFactsV1::Absent { .. } => None,
            _ => Some(project(snapshot.preview_facts(operation)?)?),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(decoded)
    }

    /// Starts one session-bound repeatable-read snapshot that permits the
    /// broker-owned witness locks used by previews and inspection.
    async fn begin_curriculum_adoption_read_snapshot(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, StoreError> {
        // ASVS 2.3.3, 2.3.4, 8.3.1, 15.4.2, 15.4.3: keep authorization,
        // witness locking, fact projection, and commit in one consistent
        // server-owned transaction while binding the presented session.
        let mut transaction = self.begin_tenant_writable_snapshot(context).await?;
        sqlx::query("SELECT set_config('ple.session_hash', $1, true)")
            .bind(session.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }

    async fn snapshot(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        context: TenantContext,
        session: SessionTokenHash,
        operation: CurriculumAdoptionBridgeOperationV1,
        request: Value,
    ) -> Result<SnapshotFactsV1, StoreError> {
        self.snapshot_with_binding(transaction, context, session, operation, request, None)
            .await
    }

    async fn snapshot_materialized(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        context: TenantContext,
        session: SessionTokenHash,
        operation: CurriculumAdoptionBridgeOperationV1,
        request: Value,
        binding: &MaterializationBindingV1,
    ) -> Result<SnapshotFactsV1, StoreError> {
        self.snapshot_with_binding(
            transaction,
            context,
            session,
            operation,
            request,
            Some(binding),
        )
        .await
    }

    async fn snapshot_with_binding(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        context: TenantContext,
        session: SessionTokenHash,
        operation: CurriculumAdoptionBridgeOperationV1,
        request: Value,
        binding: Option<&MaterializationBindingV1>,
    ) -> Result<SnapshotFactsV1, StoreError> {
        let operation_payload = snapshot_operation_payload(operation, request, binding)?;
        let Json(value): Json<Value> = sqlx::query_scalar(SNAPSHOT_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(session.to_string())
            .bind(Json(operation_payload))
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_curriculum_adoption_error)?;
        let snapshot: SnapshotFactsV1 = decode(&bounded_result(value)?)?;
        snapshot.validate_for(operation)?;
        Ok(snapshot)
    }

    async fn materialization_actor(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<question_model::UserId, StoreError> {
        let actor: uuid::Uuid = sqlx::query_scalar(MATERIALIZATION_ACTOR_SQL)
            .bind(context.tenant_id().as_uuid())
            .bind(session.to_string())
            .fetch_one(&mut **transaction)
            .await
            .map_err(map_curriculum_adoption_error)?;
        Ok(question_model::UserId::from_uuid(actor))
    }
}

fn snapshot_operation_payload(
    operation: CurriculumAdoptionBridgeOperationV1,
    request: Value,
    binding: Option<&MaterializationBindingV1>,
) -> Result<Value, StoreError> {
    match binding {
        Some(binding) => encode(&json!({
            "version": bridge::BRIDGE_VERSION,
            "operation": operation,
            "request": request,
            "materializationBinding": binding,
        })),
        None => encode(&json!({
            "version": bridge::BRIDGE_VERSION,
            "operation": operation,
            "request": request,
        })),
    }
}

fn fork_command_wire(command: &ForkAlphaCommand) -> Result<Value, StoreError> {
    encode(
        &json!({ "source": command.source(), "replacements": command.replacements(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn blueprint_command_wire(command: &BlueprintInstantiationCommand) -> Result<Value, StoreError> {
    encode(
        &json!({ "source": command.source(), "course": command.course(), "targetTerm": command.target_term(), "previewWitness": command.preview_witness(), "replacements": command.replacements(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn alpha_command_wire(command: &AlphaInstantiationCommand) -> Result<Value, StoreError> {
    encode(
        &json!({ "source": command.source(), "title": command.title(), "targetTerm": command.target_term(), "replacements": command.replacements(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn rollover_command_wire(command: &CourseRolloverCommand) -> Result<Value, StoreError> {
    encode(
        &json!({ "previewWitness": command.preview_witness(), "title": command.title(), "targetTerm": command.target_term(), "replacements": command.replacements(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn term_shift_command_wire(command: &CourseTermShiftCommand) -> Result<Value, StoreError> {
    encode(
        &json!({ "previewWitness": command.preview_witness(), "targetTerm": command.target_term(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn fast_forward_command_wire(command: &AssignmentFastForwardCommand) -> Result<Value, StoreError> {
    encode(
        &json!({ "course": command.course(), "assignment": command.assignment(), "importRevision": command.import_revision(), "source": command.source(), "previewWitness": command.preview_witness(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn source_derived_command_wire(
    command: &CreateSourceDerivedAssignmentCommand,
) -> Result<Value, StoreError> {
    encode(
        &json!({ "course": command.course(), "source": command.source(), "previewWitness": command.preview_witness(), "replacements": command.replacements(), "idempotencyKey": command.idempotency_key() }),
    )
}

fn encode(value: &impl serde::Serialize) -> Result<Value, StoreError> {
    let value = serde_json::to_value(value)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    bounded_json(
        &value,
        StoreError::InvalidRecord("curriculum adoption payload exceeds its bound".to_string()),
    )?;
    Ok(value)
}

fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, StoreError> {
    serde_json::from_value(value.clone()).map_err(|_| {
        StoreError::Unavailable(
            "curriculum adoption broker returned an invalid closed payload".to_string(),
        )
    })
}

fn bounded_result(value: Value) -> Result<Value, StoreError> {
    bounded_json(
        &value,
        StoreError::Unavailable(
            "curriculum adoption broker returned an oversized payload".to_string(),
        ),
    )?;
    Ok(value)
}

fn bounded_json(value: &Value, error: StoreError) -> Result<(), StoreError> {
    let bytes = serde_json::to_vec(value).map_err(|_| {
        StoreError::Unavailable("curriculum adoption JSON encoding failed".to_string())
    })?;
    (bytes.len() <= MAX_BROKER_JSON_BYTES)
        .then_some(())
        .ok_or(error)
}

fn require(condition: bool, message: &str) -> Result<(), StoreError> {
    condition
        .then_some(())
        .ok_or_else(|| StoreError::Unavailable(message.to_string()))
}

fn map_curriculum_adoption_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(database) = &error {
        match database.code().as_deref() {
            // ASVS 8.2.2, 8.4.1, 16.5.1: conceal absent and cross-tenant
            // locators without weakening direct-Instructor authorization.
            Some("PBN01") => return StoreError::NotFound,
            Some("PBC01") | Some("23505") | Some("55000") => return StoreError::Conflict,
            Some("PBI01") => {
                return StoreError::Unavailable(
                    "curriculum adoption integrity failure".to_string(),
                );
            }
            _ => {}
        }
    }
    map_sqlx_error(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialized_snapshot_binds_the_same_request_without_changing_read_shapes() {
        let request = json!({"idempotencyKey": "binding-contract"});
        let operation = CurriculumAdoptionBridgeOperationV1::ApplyForkAlpha;
        let binding = MaterializationBindingV1 {
            version: bridge::BRIDGE_VERSION,
            actor: question_model::UserId::from_uuid(uuid::Uuid::from_u128(42)),
            request_sha256: [7; 32],
        };

        let materialized = snapshot_operation_payload(operation, request.clone(), Some(&binding))
            .expect("materialized snapshot payload should encode");
        let ordinary = snapshot_operation_payload(operation, request.clone(), None)
            .expect("ordinary snapshot payload should encode");

        assert_eq!(materialized["request"], request);
        assert_eq!(
            materialized["materializationBinding"]["actor"],
            binding.actor.to_string()
        );
        assert_eq!(
            materialized["materializationBinding"]["requestSha256"],
            json!(binding.request_sha256)
        );
        assert_eq!(
            ordinary
                .as_object()
                .expect("ordinary snapshot is an object")
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from([
                "operation".to_string(),
                "request".to_string(),
                "version".to_string(),
            ])
        );
    }
}
