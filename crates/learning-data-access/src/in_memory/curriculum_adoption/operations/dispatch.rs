//! Closed Memory dispatch for BlueprintCourse and CourseInstance adoption.
//!
//! This module owns the transaction envelope. Operation modules receive a
//! server-issued command while this boundary keeps session authorization,
//! canonical intent binding, replay, receipt storage, and rollback together.

use async_trait::async_trait;
use question_model::{
    AdoptBlueprintAssignmentApplyRecord, AdoptBlueprintAssignmentCommand,
    BlueprintAdoptionEligibility, BlueprintCourseCreationWitness, BlueprintReference,
    ControlledUpdateBlueprintAssignmentApplyRecord, ControlledUpdateBlueprintAssignmentCommand,
    ControlledUpdateBlueprintAssignmentReceipt, CourseInstanceApplicationBinding,
    CourseInstanceCreationWitness, CourseInstanceEligibility, CourseInstanceReceiptTarget,
    CreateSelectedBlueprintAssignmentApplyRecord, CreateSelectedBlueprintAssignmentCommand,
    CreateSelectedBlueprintAssignmentReceipt, CurriculumAdoptionApplyIntent,
    CurriculumAdoptionCommandError, CurriculumAdoptionCompleted, CurriculumAdoptionPreview,
    CurriculumAdoptionPreviewRequest, CurriculumAdoptionRequestBinding,
    ForkBlueprintCourseApplyRecord, ForkBlueprintCourseCommand,
    InstantiateBlueprintCourseApplyRecord, InstantiateBlueprintCourseCommand,
    ReconcileCourseInstanceAdoptionApplyRecord, ReconcileCourseInstanceAdoptionCommand,
    ReconcileCourseInstanceAdoptionCompleted, ReconcileCourseInstanceAdoptionIntent,
    RolloverCourseInstanceApplyRecord, RolloverCourseInstanceCommand,
    RolloverCourseInstanceReceipt, ShiftCourseInstanceTermApplyRecord,
    ShiftCourseInstanceTermCommand, ShiftCourseInstanceTermReceipt,
};

use super::super::{
    MemoryCurriculumAdoptionEvidence, MemoryCurriculumAdoptionOutcome,
    MemoryCurriculumAdoptionReceipt, authorized_actor, completed_response,
    course_instance_blueprint_application, course_witness, lookup_replay_or_conflict,
    require_course_instructor, resolve_course, resolve_reconciliation_target,
    store_completed_receipt, validate_destination_pins, validate_receipt_evidence,
};
use super::{AppliedCurriculumAdoption, assignment_update, course_lifecycle, source_adoption};
use crate::curriculum_adoption::{
    CanonicalCurriculumAdoptionIntentV1, CurriculumAdoptionOperation, reconciliation_target_digest,
};
use crate::in_memory::{MemoryStore, State};
use crate::{SessionTokenHash, StoreError, TenantContext};

#[async_trait]
impl crate::CurriculumAdoptionStore for MemoryStore {
    async fn preflight_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
    ) -> Result<(), StoreError> {
        let state = self.read_state()?;
        authorized_actor(&state, context, session).map(|_| ())
    }

    async fn preview_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CurriculumAdoptionPreviewRequest,
    ) -> Result<CurriculumAdoptionPreview, StoreError> {
        match request {
            CurriculumAdoptionPreviewRequest::ForkBlueprintCourse { request } => {
                source_adoption::preview_fork_blueprint_course(self, context, session, request)
                    .await
                    .map(|preview| CurriculumAdoptionPreview::ForkBlueprintCourse { preview })
            }
            CurriculumAdoptionPreviewRequest::AdoptBlueprintAssignment { request } => {
                source_adoption::preview_adopt_blueprint_assignment(self, context, session, request)
                    .await
                    .map(|preview| CurriculumAdoptionPreview::AdoptBlueprintAssignment { preview })
            }
            CurriculumAdoptionPreviewRequest::InstantiateBlueprintCourse { request } => {
                source_adoption::preview_instantiate_blueprint_course(
                    self, context, session, request,
                )
                .await
                .map(|preview| CurriculumAdoptionPreview::InstantiateBlueprintCourse { preview })
            }
            CurriculumAdoptionPreviewRequest::RolloverCourseInstance { request } => {
                course_lifecycle::preview_rollover_course_instance(self, context, session, request)
                    .await
                    .map(|preview| CurriculumAdoptionPreview::RolloverCourseInstance { preview })
            }
            CurriculumAdoptionPreviewRequest::ShiftCourseInstanceTerm { request } => {
                course_lifecycle::preview_shift_course_instance_term(
                    self, context, session, request,
                )
                .await
                .map(|preview| CurriculumAdoptionPreview::ShiftCourseInstanceTerm { preview })
            }
            CurriculumAdoptionPreviewRequest::ControlledUpdateBlueprintAssignment { request } => {
                assignment_update::preview_controlled_update_blueprint_assignment(
                    self, context, session, request,
                )
                .await
                .map(|preview| {
                    CurriculumAdoptionPreview::ControlledUpdateBlueprintAssignment { preview }
                })
            }
            CurriculumAdoptionPreviewRequest::CreateSelectedBlueprintAssignment { request } => {
                assignment_update::preview_create_selected_blueprint_assignment(
                    self, context, session, request,
                )
                .await
                .map(|preview| {
                    CurriculumAdoptionPreview::CreateSelectedBlueprintAssignment { preview }
                })
            }
        }
    }

    async fn apply_curriculum_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        intent: CurriculumAdoptionApplyIntent,
    ) -> Result<CurriculumAdoptionCompleted, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let operation = operation_for_request(&intent.request);
        let canonical =
            CanonicalCurriculumAdoptionIntentV1::new(operation, actor, &intent.request)?;
        let digest = canonical.request_digest();
        if let Some(outcome) = lookup_replay_or_conflict(
            &state,
            tenant,
            actor,
            &intent.idempotency_key,
            operation,
            digest,
        )? {
            return completed_response(&outcome, true);
        }
        let snapshot = state.clone();
        let result = apply_locked(self, &mut state, context, actor, intent, digest);
        restore_on_error(&mut state, snapshot, result)
    }

    async fn inspect_course_instance_blueprint_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: question_model::CourseReference,
    ) -> Result<Option<question_model::CourseInstanceBlueprintInspectionView>, StoreError> {
        assignment_update::inspect_course_instance_blueprint_adoption(
            self, context, session, course,
        )
        .await
    }

    async fn reconcile_course_instance_adoption(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        intent: ReconcileCourseInstanceAdoptionIntent,
    ) -> Result<ReconcileCourseInstanceAdoptionCompleted, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let target = resolve_reconciliation_target(&state, tenant, &intent.target)?;
        let digest = reconciliation_target_digest(actor, &target);
        if let Some(outcome) = lookup_replay_or_conflict(
            &state,
            tenant,
            actor,
            &intent.idempotency_key,
            CurriculumAdoptionOperation::ReconcileCourseInstanceAdoption,
            digest,
        )? {
            return outcome
                .reconciliation_completed(question_model::CurriculumReplayStatus::Replayed)
                .ok_or(StoreError::Conflict);
        }
        let snapshot = state.clone();
        let result = (|| {
            let course = resolve_course(&state, tenant, target.destination().course)?;
            require_course_instructor(&state, tenant, course, actor)?;
            let application = course_instance_blueprint_application(&state, tenant, course)?;
            let record = ReconcileCourseInstanceAdoptionApplyRecord::new(
                target.clone(),
                application,
                actor,
                *digest.as_bytes(),
                intent.idempotency_key.clone(),
                CourseInstanceEligibility::Eligible,
            )
            .map_err(|_| StoreError::Conflict)?;
            let command =
                ReconcileCourseInstanceAdoptionCommand::from_server_record(record.clone());
            let completed = assignment_update::reconcile_course_instance_adoption_locked(
                &mut state,
                tenant,
                actor,
                command.receipt(),
            )?;
            let target = CourseInstanceReceiptTarget::Reconcile(
                question_model::ReconcileCourseInstanceAdoptionReceipt::from_server_record(
                    record,
                    state.authoritative_time,
                )
                .map_err(|_| StoreError::Conflict)?,
            );
            let receipt = MemoryCurriculumAdoptionReceipt {
                operation: CurriculumAdoptionOperation::ReconcileCourseInstanceAdoption,
                actor,
                idempotency_key: intent.idempotency_key,
                request_digest: digest,
                occurred_at: state.authoritative_time,
                outcome: MemoryCurriculumAdoptionOutcome::ReconcileCourseInstanceAdoption {
                    course: completed.course,
                },
                evidence: MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(Box::new(target)),
            };
            store_completed_receipt(&mut state, tenant, receipt)?;
            Ok(completed)
        })();
        restore_on_error(&mut state, snapshot, result)
    }
}

fn apply_locked(
    store: &MemoryStore,
    state: &mut State,
    context: TenantContext,
    actor: question_model::UserId,
    intent: CurriculumAdoptionApplyIntent,
    digest: crate::curriculum_adoption::CurriculumAdoptionRequestDigest,
) -> Result<CurriculumAdoptionCompleted, StoreError> {
    let tenant = context.tenant_id();
    let key = intent.idempotency_key;
    let operation = operation_for_request(&intent.request);
    let applied = match intent.request {
        CurriculumAdoptionPreviewRequest::ForkBlueprintCourse { request } => {
            validate_blueprint_source(
                state,
                store,
                tenant,
                actor,
                request.source,
                &request.replacements,
            )?;
            let reserved =
                BlueprintReference::new(u64::from(state.next_blueprint_course_reference) + 1)
                    .ok_or(StoreError::Conflict)?;
            let creation = BlueprintCourseCreationWitness::new(
                request.source,
                actor,
                *digest.as_bytes(),
                key.clone(),
                reserved,
            );
            let record = ForkBlueprintCourseApplyRecord::new(
                request.source,
                request.replacements,
                creation,
                BlueprintAdoptionEligibility::Eligible,
            )
            .map_err(command_error)?;
            let command = ForkBlueprintCourseCommand::from_server_record(record);
            source_adoption::apply_fork_blueprint_course_locked(
                state, store, context, actor, &command,
            )?
        }
        CurriculumAdoptionPreviewRequest::AdoptBlueprintAssignment { request } => {
            let course = resolve_course(state, tenant, request.course)?;
            require_course_instructor(state, tenant, course, actor)?;
            let destination = course_witness(state, tenant, course)?;
            let application = course_instance_blueprint_application(state, tenant, course)?;
            validate_assignment_source(
                state,
                store,
                tenant,
                actor,
                request.source,
                &request.replacements,
                course,
            )?;
            let record = AdoptBlueprintAssignmentApplyRecord::new(
                request.source,
                CourseInstanceApplicationBinding::new(destination, application),
                request.replacements,
                CurriculumAdoptionRequestBinding::new(actor, *digest.as_bytes(), key.clone()),
                BlueprintAdoptionEligibility::Eligible,
            )
            .map_err(command_error)?;
            let command = AdoptBlueprintAssignmentCommand::from_server_record(record);
            source_adoption::apply_adopt_blueprint_assignment_locked(
                state, store, context, actor, &command,
            )?
        }
        CurriculumAdoptionPreviewRequest::InstantiateBlueprintCourse { request } => {
            validate_blueprint_source(
                state,
                store,
                tenant,
                actor,
                request.source,
                &request.replacements,
            )?;
            let snapshot = super::super::source_snapshot_with_replacements(
                state,
                store,
                tenant,
                actor,
                request.source,
                &request.replacements,
            )?;
            let question_model::curriculum_adoption::CurriculumSemanticPayload::Course(course) =
                snapshot.payload
            else {
                return Err(StoreError::Conflict);
            };
            let (_, corrections) =
                crate::curriculum_adoption::preview_course(&course, &request.target_term)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            if !corrections.is_empty() {
                return Err(StoreError::Conflict);
            }
            let creation = CourseInstanceCreationWitness::for_blueprint(
                request.source,
                request.target_term.clone(),
                actor,
                *digest.as_bytes(),
                key.clone(),
                next_course_reference(state)?,
            );
            let record = InstantiateBlueprintCourseApplyRecord::new(
                request.source,
                request.target_term,
                request.replacements,
                creation,
                BlueprintAdoptionEligibility::Eligible,
            )
            .map_err(command_error)?;
            let command = InstantiateBlueprintCourseCommand::from_server_record(record);
            source_adoption::apply_instantiate_blueprint_course_locked(
                state, store, context, actor, &command,
            )?
        }
        CurriculumAdoptionPreviewRequest::RolloverCourseInstance { request } => {
            let source_course = resolve_course(state, tenant, request.source_course)?;
            require_course_instructor(state, tenant, source_course, actor)?;
            let witness = course_witness(state, tenant, source_course)?;
            let application = course_instance_blueprint_application(state, tenant, source_course)?;
            let input =
                super::super::rollover_input(state, tenant, source_course, &request.target_term)?;
            let manifest = course_lifecycle::manifest(&input)?;
            let creation = CourseInstanceCreationWitness::for_rollover(
                witness.clone(),
                request.target_term.clone(),
                actor,
                *digest.as_bytes(),
                key.clone(),
                next_course_reference(state)?,
            );
            let record = RolloverCourseInstanceApplyRecord::new(
                witness,
                application,
                request.target_term,
                manifest,
                creation,
                CourseInstanceEligibility::Eligible,
            )
            .map_err(|_| StoreError::Conflict)?;
            let command = RolloverCourseInstanceCommand::from_server_record(record.clone());
            let result = course_lifecycle::apply_rollover_course_instance_locked(
                state, context, actor, &command,
            )?;
            let target = CourseInstanceReceiptTarget::Rollover(Box::new(
                RolloverCourseInstanceReceipt::from_server_record(
                    record,
                    result.outcome,
                    state.authoritative_time,
                )
                .map_err(|_| StoreError::Conflict)?,
            ));
            AppliedCurriculumAdoption {
                outcome: MemoryCurriculumAdoptionOutcome::RolloverCourseInstance {
                    course: result.course,
                },
                evidence: MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(Box::new(target)),
            }
        }
        CurriculumAdoptionPreviewRequest::ShiftCourseInstanceTerm { request } => {
            let course = resolve_course(state, tenant, request.course)?;
            require_course_instructor(state, tenant, course, actor)?;
            let destination = course_witness(state, tenant, course)?;
            let application = course_instance_blueprint_application(state, tenant, course)?;
            if super::super::course_has_any_run(state, tenant, course) {
                return Err(StoreError::Conflict);
            }
            let schedules = course_lifecycle::shift_schedules(
                state,
                tenant,
                &destination,
                &request.target_term,
            )?;
            let record = ShiftCourseInstanceTermApplyRecord::new(
                CourseInstanceApplicationBinding::new(destination, application),
                request.target_term,
                schedules.as_slice().to_vec(),
                CurriculumAdoptionRequestBinding::new(actor, *digest.as_bytes(), key.clone()),
                CourseInstanceEligibility::Eligible,
            )
            .map_err(|_| StoreError::Conflict)?;
            let command = ShiftCourseInstanceTermCommand::from_server_record(record.clone());
            let result = course_lifecycle::apply_shift_course_instance_term_locked(
                state, context, actor, &command,
            )?;
            let target = CourseInstanceReceiptTarget::ShiftTerm(
                ShiftCourseInstanceTermReceipt::from_server_record(
                    record,
                    result.outcome,
                    state.authoritative_time,
                )
                .map_err(|_| StoreError::Conflict)?,
            );
            AppliedCurriculumAdoption {
                outcome: MemoryCurriculumAdoptionOutcome::ShiftCourseInstanceTerm {
                    course: result.course,
                },
                evidence: MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(Box::new(target)),
            }
        }
        CurriculumAdoptionPreviewRequest::ControlledUpdateBlueprintAssignment { request } => {
            let course = resolve_course(state, tenant, request.course)?;
            require_course_instructor(state, tenant, course, actor)?;
            let destination = course_witness(state, tenant, course)?;
            let application = course_instance_blueprint_application(state, tenant, course)?;
            let import = assignment_update::current_import_witness(
                state,
                tenant,
                course,
                request.assignment,
            )?;
            let eligibility = assignment_update::controlled_update_eligibility(
                state,
                tenant,
                actor,
                &destination,
                &import,
                request.source,
            )?;
            let record = ControlledUpdateBlueprintAssignmentApplyRecord::new(
                request.source,
                import,
                CourseInstanceApplicationBinding::new(destination, application),
                CurriculumAdoptionRequestBinding::new(actor, *digest.as_bytes(), key.clone()),
                eligibility,
            )
            .map_err(|_| StoreError::Conflict)?;
            let command =
                ControlledUpdateBlueprintAssignmentCommand::from_server_record(record.clone());
            let result = assignment_update::apply_controlled_update_blueprint_assignment_locked(
                state, context, actor, &command,
            )?;
            let target = CourseInstanceReceiptTarget::ControlledUpdate(
                ControlledUpdateBlueprintAssignmentReceipt::from_server_record(
                    record,
                    result.outcome,
                    result.applied,
                    result.effect,
                    state.authoritative_time,
                )
                .map_err(|_| StoreError::Conflict)?,
            );
            AppliedCurriculumAdoption {
                outcome: MemoryCurriculumAdoptionOutcome::ControlledUpdateBlueprintAssignment {
                    course: result.course,
                    assignment: result.assignment,
                },
                evidence: MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(Box::new(target)),
            }
        }
        CurriculumAdoptionPreviewRequest::CreateSelectedBlueprintAssignment { request } => {
            let course = resolve_course(state, tenant, request.course)?;
            require_course_instructor(state, tenant, course, actor)?;
            let destination = course_witness(state, tenant, course)?;
            let application = course_instance_blueprint_application(state, tenant, course)?;
            validate_assignment_source(
                state,
                store,
                tenant,
                actor,
                request.source,
                &request.replacements,
                course,
            )?;
            let source = super::super::assignment_source_snapshot_with_replacements(
                state,
                store,
                tenant,
                actor,
                request.source,
                &request.replacements,
            )?;
            let question_model::curriculum_adoption::CurriculumSemanticPayload::Assignment(
                assignment,
            ) = source.payload
            else {
                return Err(StoreError::Conflict);
            };
            let term = &state
                .courses
                .get(&(tenant, course))
                .ok_or(StoreError::NotFound)?
                .term;
            let (schedule, corrections) =
                crate::curriculum_adoption::preview_assignment(&assignment, term)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            if !corrections.is_empty() {
                return Err(StoreError::Conflict);
            }
            let record = CreateSelectedBlueprintAssignmentApplyRecord::new(
                request.source,
                CourseInstanceApplicationBinding::new(destination, application),
                schedule,
                request.replacements,
                CurriculumAdoptionRequestBinding::new(actor, *digest.as_bytes(), key.clone()),
                CourseInstanceEligibility::Eligible,
            )
            .map_err(|_| StoreError::Conflict)?;
            let command =
                CreateSelectedBlueprintAssignmentCommand::from_server_record(record.clone());
            let result = assignment_update::apply_create_selected_blueprint_assignment_locked(
                state, store, context, actor, &command,
            )?;
            let target = CourseInstanceReceiptTarget::SelectedCopy(
                CreateSelectedBlueprintAssignmentReceipt::from_server_record(
                    record,
                    result.outcome,
                    result.applied,
                    state.authoritative_time,
                )
                .map_err(|_| StoreError::Conflict)?,
            );
            AppliedCurriculumAdoption {
                outcome: MemoryCurriculumAdoptionOutcome::CreateSelectedBlueprintAssignment {
                    course: result.course,
                    assignment: result.assignment,
                },
                evidence: MemoryCurriculumAdoptionEvidence::CourseInstanceReceipt(Box::new(target)),
            }
        }
    };
    let receipt = MemoryCurriculumAdoptionReceipt {
        operation,
        actor,
        idempotency_key: key,
        request_digest: digest,
        occurred_at: state.authoritative_time,
        outcome: applied.outcome.clone(),
        evidence: applied.evidence,
    };
    validate_receipt_evidence(state, tenant, &receipt)?;
    store_completed_receipt(state, tenant, receipt)?;
    completed_response(&applied.outcome, false)
}

fn validate_blueprint_source(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    actor: question_model::UserId,
    source: question_model::ObservedBlueprintSource,
    replacements: &question_model::CurriculumPinReplacements,
) -> Result<(), StoreError> {
    let snapshot = super::super::source_snapshot_with_replacements(
        state,
        store,
        tenant,
        actor,
        source,
        replacements,
    )?;
    validate_destination_pins(state, tenant, &snapshot.payload).map_err(|_| StoreError::Conflict)
}

fn validate_assignment_source(
    state: &State,
    store: &MemoryStore,
    tenant: question_model::TenantId,
    actor: question_model::UserId,
    source: question_model::AssignmentDefinitionSourceView,
    replacements: &question_model::CurriculumPinReplacements,
    course: question_model::CourseId,
) -> Result<(), StoreError> {
    let snapshot = super::super::assignment_source_snapshot_with_replacements(
        state,
        store,
        tenant,
        actor,
        source,
        replacements,
    )?;
    validate_destination_pins(state, tenant, &snapshot.payload)
        .map_err(|_| StoreError::Conflict)?;
    let question_model::curriculum_adoption::CurriculumSemanticPayload::Assignment(assignment) =
        snapshot.payload
    else {
        return Err(StoreError::Conflict);
    };
    let term = &state
        .courses
        .get(&(tenant, course))
        .ok_or(StoreError::NotFound)?
        .term;
    let (_, corrections) = crate::curriculum_adoption::preview_assignment(&assignment, term)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    corrections
        .is_empty()
        .then_some(())
        .ok_or(StoreError::Conflict)
}

fn operation_for_request(
    request: &CurriculumAdoptionPreviewRequest,
) -> CurriculumAdoptionOperation {
    match request {
        CurriculumAdoptionPreviewRequest::ForkBlueprintCourse { .. } => {
            CurriculumAdoptionOperation::ForkBlueprintCourse
        }
        CurriculumAdoptionPreviewRequest::AdoptBlueprintAssignment { .. } => {
            CurriculumAdoptionOperation::AdoptBlueprintAssignment
        }
        CurriculumAdoptionPreviewRequest::InstantiateBlueprintCourse { .. } => {
            CurriculumAdoptionOperation::InstantiateBlueprintCourse
        }
        CurriculumAdoptionPreviewRequest::RolloverCourseInstance { .. } => {
            CurriculumAdoptionOperation::RolloverCourseInstance
        }
        CurriculumAdoptionPreviewRequest::ShiftCourseInstanceTerm { .. } => {
            CurriculumAdoptionOperation::ShiftCourseInstanceTerm
        }
        CurriculumAdoptionPreviewRequest::ControlledUpdateBlueprintAssignment { .. } => {
            CurriculumAdoptionOperation::ControlledUpdateBlueprintAssignment
        }
        CurriculumAdoptionPreviewRequest::CreateSelectedBlueprintAssignment { .. } => {
            CurriculumAdoptionOperation::CreateSelectedBlueprintAssignment
        }
    }
}

fn next_course_reference(state: &State) -> Result<question_model::CourseReference, StoreError> {
    question_model::CourseReference::new(u64::from(state.next_course_reference) + 1)
        .ok_or(StoreError::Conflict)
}

fn command_error(_: CurriculumAdoptionCommandError) -> StoreError {
    StoreError::Conflict
}

fn restore_on_error<T>(
    state: &mut State,
    snapshot: State,
    result: Result<T, StoreError>,
) -> Result<T, StoreError> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            *state = snapshot;
            Err(error)
        }
    }
}
