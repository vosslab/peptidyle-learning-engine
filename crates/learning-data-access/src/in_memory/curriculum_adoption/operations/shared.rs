use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticPayload,
};
use question_model::{
    AlphaInstantiationCompleted, AssignmentDefinitionSourceView, AssignmentFastForwardCompleted,
    AssignmentId, BlueprintInstantiationCompleted, CourseRolloverCompleted,
    CourseTermShiftCompleted, CurriculumAdoptionIdempotencyKey, CurriculumAdoptionReceiptBinding,
    CurriculumImportRevision, CurriculumReplayStatus, ForkAlphaCompleted,
    SourceDerivedAssignmentCompleted, UserId,
};

use super::dispatch::{
    MemoryCurriculumAdoptionOutcome, State, StoreError, StoredAssignmentAdoptionEvidence,
    StoredAssignmentImport, StoredAssignmentImportProvenance, StoredAssignmentImportSource,
    StoredCurriculumBaseline,
};
use question_model::TenantId;

pub(crate) fn store_import(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
    semantic: CurriculumSemanticAssignment,
    source: AssignmentDefinitionSourceView,
    actor: UserId,
    receipt: &CurriculumAdoptionIdempotencyKey,
) {
    let digest = CurriculumSemanticPayload::assignment(semantic.clone()).digest();
    let baseline = StoredCurriculumBaseline {
        payload: semantic,
        digest,
        revision: CurriculumImportRevision::new(1).expect("initial import revision"),
    };
    let provenance = StoredAssignmentImportProvenance {
        source: StoredAssignmentImportSource::Reusable(source),
        actor,
        occurred_at: state.authoritative_time,
        receipt: receipt.clone(),
    };
    state.curriculum_adoption.import_records.insert(
        (tenant, assignment),
        StoredAssignmentImport {
            baseline: baseline.clone(),
            provenance: provenance.clone(),
        },
    );
    state.curriculum_adoption.assignment_evidence.insert(
        (tenant, receipt.clone(), assignment),
        StoredAssignmentAdoptionEvidence {
            baseline,
            provenance,
        },
    );
}

pub(crate) fn store_rollover_import(
    state: &mut State,
    tenant: TenantId,
    assignment: AssignmentId,
    semantic: CurriculumSemanticAssignment,
    source: StoredAssignmentImportSource,
    actor: UserId,
    receipt: &CurriculumAdoptionIdempotencyKey,
) {
    let digest = CurriculumSemanticPayload::assignment(semantic.clone()).digest();
    let baseline = StoredCurriculumBaseline {
        payload: semantic,
        digest,
        revision: CurriculumImportRevision::new(1).expect("initial import revision"),
    };
    let provenance = StoredAssignmentImportProvenance {
        source,
        actor,
        occurred_at: state.authoritative_time,
        receipt: receipt.clone(),
    };
    state.curriculum_adoption.import_records.insert(
        (tenant, assignment),
        StoredAssignmentImport {
            baseline: baseline.clone(),
            provenance: provenance.clone(),
        },
    );
    state.curriculum_adoption.assignment_evidence.insert(
        (tenant, receipt.clone(), assignment),
        StoredAssignmentAdoptionEvidence {
            baseline,
            provenance,
        },
    );
}

pub(crate) fn next_import_revision(
    revision: CurriculumImportRevision,
) -> Result<CurriculumImportRevision, StoreError> {
    revision
        .value()
        .checked_add(1)
        .and_then(CurriculumImportRevision::new)
        .ok_or_else(|| StoreError::Unavailable("curriculum import revision exhausted".into()))
}

fn receipt(key: &CurriculumAdoptionIdempotencyKey) -> CurriculumAdoptionReceiptBinding {
    CurriculumAdoptionReceiptBinding {
        idempotency_key: key.clone(),
    }
}

pub(crate) fn fork_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<ForkAlphaCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::ForkAlpha { source, alpha } = outcome else {
        return Err(StoreError::Conflict);
    };
    Ok(ForkAlphaCompleted {
        source,
        alpha,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

pub(crate) fn blueprint_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<BlueprintInstantiationCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::InstantiateBlueprint { course, assignment } = outcome
    else {
        return Err(StoreError::Conflict);
    };
    Ok(BlueprintInstantiationCompleted {
        course,
        assignment,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

pub(crate) fn alpha_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<AlphaInstantiationCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::InstantiateAlpha { source, course } = outcome else {
        return Err(StoreError::Conflict);
    };
    Ok(AlphaInstantiationCompleted {
        source,
        course,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

pub(crate) fn rollover_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<CourseRolloverCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::RolloverCourse {
        source_course,
        course,
    } = outcome
    else {
        return Err(StoreError::Conflict);
    };
    Ok(CourseRolloverCompleted {
        source_course,
        course,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

pub(crate) fn term_shift_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<CourseTermShiftCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::ShiftCourseTerm { course, term } = outcome else {
        return Err(StoreError::Conflict);
    };
    Ok(CourseTermShiftCompleted {
        course,
        term,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

pub(crate) fn fast_forward_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<AssignmentFastForwardCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::FastForwardAssignment {
        course,
        assignment,
        import_revision,
    } = outcome
    else {
        return Err(StoreError::Conflict);
    };
    Ok(AssignmentFastForwardCompleted {
        course,
        assignment,
        import_revision,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

pub(crate) fn source_derived_completed(
    outcome: MemoryCurriculumAdoptionOutcome,
    key: &CurriculumAdoptionIdempotencyKey,
    replayed: bool,
) -> Result<SourceDerivedAssignmentCompleted, StoreError> {
    let MemoryCurriculumAdoptionOutcome::CreateSourceDerivedAssignment { course, assignment } =
        outcome
    else {
        return Err(StoreError::Conflict);
    };
    Ok(SourceDerivedAssignmentCompleted {
        course,
        assignment,
        replay: replay_status(replayed),
        receipt: receipt(key),
    })
}

fn replay_status(replayed: bool) -> CurriculumReplayStatus {
    if replayed {
        CurriculumReplayStatus::Replayed
    } else {
        CurriculumReplayStatus::Applied
    }
}
