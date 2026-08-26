use async_trait::async_trait;
use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticCourse, CurriculumSemanticPayload,
};
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationCompleted, AlphaInstantiationPreviewRequest,
    AlphaInstantiationPreviewView, AssignmentFastForwardCommand, AssignmentFastForwardCompleted,
    AssignmentFastForwardPreviewRequest, AssignmentFastForwardPreviewView,
    BlueprintInstantiationCommand, BlueprintInstantiationCompleted,
    BlueprintInstantiationPreviewRequest, BlueprintInstantiationPreviewView, CourseRolloverCommand,
    CourseRolloverCompleted, CourseRolloverPreviewRequest, CourseRolloverPreviewView,
    CourseTermShiftCommand, CourseTermShiftCompleted, CourseTermShiftPreviewRequest,
    CourseTermShiftPreviewView, CreateSourceDerivedAssignmentCommand, CurriculumAdoptionTitle,
    CurriculumCourseImportView, CurriculumScheduleCorrection, CurriculumSourceView,
    ForkAlphaCommand, ForkAlphaCompleted, ForkAlphaPreviewRequest, ForkAlphaPreviewView,
    PreparedCurriculumAssignmentView, PreparedCurriculumCourseView,
    SourceDerivedAssignmentCompleted, SourceDerivedAssignmentPreviewRequest,
    SourceDerivedAssignmentPreviewView,
};

use super::*;

mod course;
mod shared;
mod updates;
use course::*;
use shared::*;
use updates::*;

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

    async fn preview_fork_alpha(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: ForkAlphaPreviewRequest,
    ) -> Result<ForkAlphaPreviewView, StoreError> {
        let state = self.read_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let snapshot = source_snapshot_with_replacements(
            &state,
            self,
            context.tenant_id(),
            actor,
            CurriculumSourceView::Alpha(request.source),
            &request.replacements,
        )?;
        let CurriculumSemanticPayload::Course(course) = &snapshot.payload else {
            return Err(StoreError::InvalidRecord(
                "Alpha source is not course-sized".into(),
            ));
        };
        Ok(ForkAlphaPreviewView {
            source: request.source,
            resulting_alpha_title: title(course.title())?,
            replacements: request.replacements,
            pin_correction: pin_correction(&state, context.tenant_id(), &snapshot.payload)?,
        })
    }

    async fn apply_fork_alpha(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ForkAlphaCommand,
    ) -> Result<ForkAlphaCompleted, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let digest = request_digest(
            "fork-alpha",
            actor,
            (command.source(), command.replacements()),
        )?;
        if let Some(outcome) = matching_receipt(
            &state,
            tenant,
            command.idempotency_key(),
            CurriculumAdoptionOperation::ForkAlpha,
            actor,
            digest,
        )? {
            return fork_completed(outcome, command.idempotency_key(), true);
        }
        let snapshot = state.clone();
        let result = (|| {
            let source = source_snapshot_with_replacements(
                &state,
                self,
                tenant,
                actor,
                CurriculumSourceView::Alpha(command.source()),
                command.replacements(),
            )?;
            validate_destination_pins(&state, tenant, &source.payload)
                .map_err(|_| StoreError::Conflict)?;
            let CurriculumSemanticPayload::Course(course) = source.payload else {
                return Err(StoreError::InvalidRecord(
                    "Alpha source is not course-sized".into(),
                ));
            };
            let alpha = super::super::reusable_curriculum::create_alpha_from_semantic_locked(
                &mut state, actor, &course,
            )?;
            let occurred_at = state.authoritative_time;
            let semantic_digest = CurriculumSemanticPayload::course(course.clone()).digest();
            state.curriculum_alpha_fork_lineage.insert(
                alpha,
                StoredAlphaForkLineage {
                    payload: course,
                    digest: semantic_digest,
                    source: command.source(),
                    actor,
                    occurred_at,
                    receipt: command.idempotency_key().clone(),
                },
            );
            let outcome = MemoryCurriculumAdoptionOutcome::ForkAlpha {
                source: command.source(),
                alpha,
            };
            store_receipt(
                &mut state,
                tenant,
                command.idempotency_key().clone(),
                CurriculumAdoptionOperation::ForkAlpha,
                actor,
                digest,
                outcome.clone(),
            );
            fork_completed(outcome, command.idempotency_key(), false)
        })();
        rollback(&mut state, snapshot, result)
    }

    async fn preview_blueprint_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: BlueprintInstantiationPreviewRequest,
    ) -> Result<BlueprintInstantiationPreviewView, StoreError> {
        let tenant = context.tenant_id();
        let state = self.read_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let course = resolve_course(&state, tenant, request.course)?;
        require_course_instructor(&state, tenant, course, actor)?;
        if state.courses.get(&(tenant, course)).map(|row| &row.term) != Some(&request.target_term) {
            return Err(StoreError::Conflict);
        }
        let source = source_snapshot_with_replacements(
            &state,
            self,
            tenant,
            actor,
            CurriculumSourceView::Blueprint(request.source),
            &request.replacements,
        )?;
        let assignment = only_assignment(&source.payload)?;
        let (assignment, corrections) = preview_assignment(assignment, &request.target_term)?;
        Ok(BlueprintInstantiationPreviewView {
            source: request.source,
            course: request.course,
            target_term: request.target_term,
            witness: course_witness(&state, tenant, course)?,
            assignment,
            corrections,
            replacements: request.replacements,
            pin_correction: pin_correction(&state, tenant, &source.payload)?,
        })
    }

    async fn apply_blueprint_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: BlueprintInstantiationCommand,
    ) -> Result<BlueprintInstantiationCompleted, StoreError> {
        let tenant = context.tenant_id();
        let mut state = self.write_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let digest = request_digest(
            "instantiate-blueprint",
            actor,
            (
                command.source(),
                command.course(),
                command.target_term(),
                command.preview_witness(),
                command.replacements(),
            ),
        )?;
        if let Some(outcome) = matching_receipt(
            &state,
            tenant,
            command.idempotency_key(),
            CurriculumAdoptionOperation::InstantiateBlueprint,
            actor,
            digest,
        )? {
            return blueprint_completed(outcome, command.idempotency_key(), true);
        }
        let before = state.clone();
        let result = (|| {
            let course = require_exact_witness(&state, tenant, command.preview_witness())?;
            require_course_instructor(&state, tenant, course, actor)?;
            if state.courses.get(&(tenant, course)).map(|row| &row.term)
                != Some(command.target_term())
            {
                return Err(StoreError::Conflict);
            }
            let source = source_snapshot_with_replacements(
                &state,
                self,
                tenant,
                actor,
                CurriculumSourceView::Blueprint(command.source()),
                command.replacements(),
            )?;
            validate_destination_pins(&state, tenant, &source.payload)
                .map_err(|_| StoreError::Conflict)?;
            let semantic = only_assignment(&source.payload)?.clone();
            semantic
                .schedule()
                .resolve_for_target_term(command.target_term())
                .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
            let (assignment, reference) = destination::materialize_semantic_assignment(
                &mut state, context, course, &semantic,
            )?;
            store_import(
                &mut state,
                tenant,
                assignment,
                semantic,
                AssignmentDefinitionSourceView::Blueprint(command.source()),
                actor,
                command.idempotency_key(),
            );
            let outcome = MemoryCurriculumAdoptionOutcome::InstantiateBlueprint {
                course: command.course(),
                assignment: reference,
            };
            store_receipt(
                &mut state,
                tenant,
                command.idempotency_key().clone(),
                CurriculumAdoptionOperation::InstantiateBlueprint,
                actor,
                digest,
                outcome.clone(),
            );
            blueprint_completed(outcome, command.idempotency_key(), false)
        })();
        rollback(&mut state, before, result)
    }

    async fn preview_alpha_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: AlphaInstantiationPreviewRequest,
    ) -> Result<AlphaInstantiationPreviewView, StoreError> {
        let state = self.read_state()?;
        let actor = authorized_actor(&state, context, session)?;
        let source = source_snapshot_with_replacements(
            &state,
            self,
            context.tenant_id(),
            actor,
            CurriculumSourceView::Alpha(request.source),
            &request.replacements,
        )?;
        let CurriculumSemanticPayload::Course(course) = &source.payload else {
            return Err(StoreError::InvalidRecord(
                "Alpha source is not course-sized".into(),
            ));
        };
        let (course, corrections) = preview_course(&request.title, course, &request.target_term)?;
        Ok(AlphaInstantiationPreviewView {
            source: request.source,
            target_term: request.target_term,
            course,
            corrections,
            replacements: request.replacements,
            pin_correction: pin_correction(&state, context.tenant_id(), &source.payload)?,
        })
    }

    async fn apply_alpha_instantiation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: AlphaInstantiationCommand,
    ) -> Result<AlphaInstantiationCompleted, StoreError> {
        apply_new_alpha_course(self, context, session, command).await
    }

    async fn preview_course_rollover(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CourseRolloverPreviewRequest,
    ) -> Result<CourseRolloverPreviewView, StoreError> {
        preview_rollover(self, context, session, request).await
    }

    async fn apply_course_rollover(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CourseRolloverCommand,
    ) -> Result<CourseRolloverCompleted, StoreError> {
        apply_rollover(self, context, session, command).await
    }

    async fn preview_course_term_shift(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: CourseTermShiftPreviewRequest,
    ) -> Result<CourseTermShiftPreviewView, StoreError> {
        preview_term_shift(self, context, session, request).await
    }

    async fn apply_course_term_shift(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CourseTermShiftCommand,
    ) -> Result<CourseTermShiftCompleted, StoreError> {
        apply_term_shift(self, context, session, command).await
    }

    async fn preview_assignment_fast_forward(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: AssignmentFastForwardPreviewRequest,
    ) -> Result<AssignmentFastForwardPreviewView, StoreError> {
        preview_fast_forward(self, context, session, request).await
    }

    async fn apply_assignment_fast_forward(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: AssignmentFastForwardCommand,
    ) -> Result<AssignmentFastForwardCompleted, StoreError> {
        apply_fast_forward(self, context, session, command).await
    }

    async fn preview_source_derived_assignment(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        request: SourceDerivedAssignmentPreviewRequest,
    ) -> Result<SourceDerivedAssignmentPreviewView, StoreError> {
        preview_source_derived(self, context, session, request).await
    }

    async fn create_source_derived_assignment(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateSourceDerivedAssignmentCommand,
    ) -> Result<SourceDerivedAssignmentCompleted, StoreError> {
        apply_source_derived(self, context, session, command).await
    }

    async fn inspect_curriculum_imports(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseReference,
    ) -> Result<Option<CurriculumCourseImportView>, StoreError> {
        inspect_imports(self, context, session, course).await
    }
}

fn title(value: &str) -> Result<CurriculumAdoptionTitle, StoreError> {
    CurriculumAdoptionTitle::parse(value)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))
}

fn only_assignment(
    payload: &CurriculumSemanticPayload,
) -> Result<&CurriculumSemanticAssignment, StoreError> {
    match payload {
        CurriculumSemanticPayload::Assignment(assignment) => Ok(assignment),
        CurriculumSemanticPayload::Course(_) => Err(StoreError::InvalidRecord(
            "operation requires an assignment-sized source".into(),
        )),
    }
}

fn preview_assignment(
    semantic: &CurriculumSemanticAssignment,
    term: &question_model::CourseTerm,
) -> Result<
    (
        PreparedCurriculumAssignmentView,
        Vec<CurriculumScheduleCorrection>,
    ),
    StoreError,
> {
    match semantic.schedule().resolve_for_target_term(term) {
        Ok(schedule) => Ok((
            PreparedCurriculumAssignmentView {
                title: title(semantic.title())?,
                schedule,
            },
            Vec::new(),
        )),
        Err(error) => Ok((
            PreparedCurriculumAssignmentView {
                title: title(semantic.title())?,
                schedule: question_model::RelativeAssignmentSchedule::default()
                    .resolve_for_target_term(term)
                    .expect("empty schedule resolves for every valid term"),
            },
            vec![error.into()],
        )),
    }
}

fn preview_course(
    title_value: &CurriculumAdoptionTitle,
    semantic: &CurriculumSemanticCourse,
    term: &question_model::CourseTerm,
) -> Result<
    (
        PreparedCurriculumCourseView,
        Vec<CurriculumScheduleCorrection>,
    ),
    StoreError,
> {
    let mut assignments = Vec::new();
    let mut corrections = Vec::new();
    for assignment in semantic
        .modules()
        .iter()
        .flat_map(|module| module.assignments())
    {
        let (view, mut assignment_corrections) = preview_assignment(assignment, term)?;
        assignments.push(view);
        corrections.append(&mut assignment_corrections);
    }
    Ok((
        PreparedCurriculumCourseView {
            title: title_value.clone(),
            assignments,
        },
        corrections,
    ))
}

fn rollback<T>(
    state: &mut State,
    before: State,
    result: Result<T, StoreError>,
) -> Result<T, StoreError> {
    if result.is_err() {
        *state = before;
    }
    result
}
