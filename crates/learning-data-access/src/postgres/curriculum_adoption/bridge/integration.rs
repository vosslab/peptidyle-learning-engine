//! Typed reconstruction of public B2 completion records.
//!
//! SQL returns only relational references and the receipt replay bit.  The
//! adapter calls these functions after it has verified the locked command and
//! request digest, so this module makes the final operation/result binding
//! explicit before an answer-free DTO leaves the persistence boundary.

use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationCompleted, AssignmentFastForwardCommand,
    AssignmentFastForwardCompleted, BlueprintInstantiationCommand, BlueprintInstantiationCompleted,
    CourseRolloverCommand, CourseRolloverCompleted, CourseTermShiftCommand,
    CourseTermShiftCompleted, CreateSourceDerivedAssignmentCommand,
    CurriculumAdoptionReceiptBinding, CurriculumReplayStatus, ForkAlphaCommand, ForkAlphaCompleted,
    SourceDerivedAssignmentCompleted,
};

use crate::StoreError;

use super::{SqlAdoptionResultV1, SqlReceiptResultV1};

fn receipt(receipt: &SqlReceiptResultV1) -> CurriculumAdoptionReceiptBinding {
    CurriculumAdoptionReceiptBinding {
        idempotency_key: receipt.idempotency_key.clone(),
    }
}

fn replay(receipt: &SqlReceiptResultV1) -> CurriculumReplayStatus {
    if receipt.replayed {
        CurriculumReplayStatus::Replayed
    } else {
        CurriculumReplayStatus::Applied
    }
}

fn require_key(
    actual: &SqlReceiptResultV1,
    expected: &question_model::CurriculumAdoptionIdempotencyKey,
) -> Result<(), StoreError> {
    (actual.idempotency_key == *expected)
        .then_some(())
        .ok_or_else(|| {
            StoreError::Unavailable("curriculum receipt key disagrees with command".into())
        })
}

pub(in crate::postgres::curriculum_adoption) fn complete_fork(
    command: &ForkAlphaCommand,
    result: &SqlAdoptionResultV1,
) -> Result<ForkAlphaCompleted, StoreError> {
    let SqlAdoptionResultV1::ForkAlpha {
        receipt: row,
        source,
        alpha,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (*source == command.source())
        .then_some(ForkAlphaCompleted {
            source: *source,
            alpha: *alpha,
            replay: replay(row),
            receipt: receipt(row),
        })
        .ok_or_else(|| StoreError::Unavailable("fork result source disagrees with command".into()))
}

pub(in crate::postgres::curriculum_adoption) fn complete_blueprint(
    command: &BlueprintInstantiationCommand,
    result: &SqlAdoptionResultV1,
) -> Result<BlueprintInstantiationCompleted, StoreError> {
    let SqlAdoptionResultV1::BlueprintInstantiation {
        receipt: row,
        course,
        assignment,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (*course == command.course())
        .then_some(BlueprintInstantiationCompleted {
            course: *course,
            assignment: *assignment,
            replay: replay(row),
            receipt: receipt(row),
        })
        .ok_or_else(|| {
            StoreError::Unavailable("Blueprint result course disagrees with command".into())
        })
}

pub(in crate::postgres::curriculum_adoption) fn complete_alpha(
    command: &AlphaInstantiationCommand,
    result: &SqlAdoptionResultV1,
) -> Result<AlphaInstantiationCompleted, StoreError> {
    let SqlAdoptionResultV1::AlphaInstantiation {
        receipt: row,
        source,
        course,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (*source == command.source())
        .then_some(AlphaInstantiationCompleted {
            source: *source,
            course: *course,
            replay: replay(row),
            receipt: receipt(row),
        })
        .ok_or_else(|| StoreError::Unavailable("Alpha result source disagrees with command".into()))
}

pub(in crate::postgres::curriculum_adoption) fn complete_rollover(
    command: &CourseRolloverCommand,
    result: &SqlAdoptionResultV1,
) -> Result<CourseRolloverCompleted, StoreError> {
    let SqlAdoptionResultV1::CourseRollover {
        receipt: row,
        source_course,
        course,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (source_course == &command.preview_witness().course)
        .then_some(CourseRolloverCompleted {
            source_course: *source_course,
            course: *course,
            replay: replay(row),
            receipt: receipt(row),
        })
        .ok_or_else(|| {
            StoreError::Unavailable("rollover result source disagrees with command".into())
        })
}

pub(in crate::postgres::curriculum_adoption) fn complete_term_shift(
    command: &CourseTermShiftCommand,
    result: &SqlAdoptionResultV1,
) -> Result<CourseTermShiftCompleted, StoreError> {
    let SqlAdoptionResultV1::CourseTermShift {
        receipt: row,
        course,
        term,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (course == &command.preview_witness().course && term == command.target_term())
        .then_some(CourseTermShiftCompleted {
            course: *course,
            term: term.clone(),
            replay: replay(row),
            receipt: receipt(row),
        })
        .ok_or_else(|| StoreError::Unavailable("term-shift result disagrees with command".into()))
}

pub(in crate::postgres::curriculum_adoption) fn complete_fast_forward(
    command: &AssignmentFastForwardCommand,
    result: &SqlAdoptionResultV1,
) -> Result<AssignmentFastForwardCompleted, StoreError> {
    let SqlAdoptionResultV1::AssignmentFastForward {
        receipt: row,
        course,
        assignment,
        import_revision,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (*course == command.course()
        && assignment == &command.assignment().assignment
        && import_revision.value() > command.import_revision().value())
    .then_some(AssignmentFastForwardCompleted {
        course: *course,
        assignment: *assignment,
        import_revision: *import_revision,
        replay: replay(row),
        receipt: receipt(row),
    })
    .ok_or_else(|| StoreError::Unavailable("fast-forward result disagrees with command".into()))
}

pub(in crate::postgres::curriculum_adoption) fn complete_source_derived(
    command: &CreateSourceDerivedAssignmentCommand,
    result: &SqlAdoptionResultV1,
) -> Result<SourceDerivedAssignmentCompleted, StoreError> {
    let SqlAdoptionResultV1::SourceDerivedAssignment {
        receipt: row,
        course,
        assignment,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    require_key(row, command.idempotency_key())?;
    (*course == command.course())
        .then_some(SourceDerivedAssignmentCompleted {
            course: *course,
            assignment: *assignment,
            replay: replay(row),
            receipt: receipt(row),
        })
        .ok_or_else(|| {
            StoreError::Unavailable("source-derived result course disagrees with command".into())
        })
}

pub(in crate::postgres::curriculum_adoption) fn reconciliation_result(
    result: &SqlAdoptionResultV1,
) -> Result<(&SqlReceiptResultV1, &[question_model::AssignmentReference]), StoreError> {
    let SqlAdoptionResultV1::Reconcile {
        receipt,
        repaired_assignments,
    } = result
    else {
        return Err(StoreError::Unavailable(
            "curriculum materializer returned the wrong result".into(),
        ));
    };
    Ok((receipt, repaired_assignments))
}
