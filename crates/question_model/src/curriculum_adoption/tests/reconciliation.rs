use super::*;

fn receipt() -> CurriculumAdoptionReceiptBinding {
    CurriculumAdoptionReceiptBinding {
        idempotency_key: CurriculumAdoptionIdempotencyKey::parse("repair-2026-08-25")
            .expect("bounded opaque receipt key"),
    }
}

#[test]
fn term_shift_preview_keeps_the_apply_witness_only_in_its_eligible_outcome() {
    let course = CourseReference::new(7).expect("course reference");
    let witness = CourseScheduleWitness::new(
        course,
        CourseScheduleRevision::new(3).expect("schedule revision"),
        Vec::new(),
    )
    .expect("schedule witness");
    let term =
        CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago").expect("target term");
    let eligible = CourseTermShiftPreviewOutcome::Eligible {
        preview: CourseTermShiftPreviewView {
            witness: witness.clone(),
            target_term: term,
            assignments: Vec::new(),
            corrections: Vec::new(),
        },
    };
    let command = CourseTermShiftCommand::from_preview(
        &eligible,
        CurriculumAdoptionIdempotencyKey::parse("term-shift-eligible").expect("key"),
    )
    .expect("eligible preview creates command");
    assert_eq!(command.preview_witness(), &witness);

    let ineligible = CourseTermShiftPreviewOutcome::Ineligible {
        course,
        reason: CourseTermShiftIneligibility::IssuedWork,
        recovery: CourseTermShiftRecoveryAction::RolloverCourse,
    };
    assert_eq!(
        CourseTermShiftCommand::from_preview(
            &ineligible,
            CurriculumAdoptionIdempotencyKey::parse("term-shift-issued").expect("key"),
        ),
        Err(CurriculumAdoptionCommandError::TermShiftNotEligible)
    );

    let wire = serde_json::to_value(&ineligible).expect("ineligible outcome serializes");
    assert_eq!(wire["kind"], "ineligible");
    assert_eq!(wire["reason"], "issuedWork");
    assert_eq!(wire["recovery"], "rolloverCourse");
    assert!(serde_json::from_value::<CourseTermShiftPreviewOutcome>(wire.clone()).is_ok());
    let mut unauthorized = wire;
    unauthorized["tenant"] = serde_json::json!("browser-supplied");
    assert!(serde_json::from_value::<CourseTermShiftPreviewOutcome>(unauthorized).is_err());
}

#[test]
fn reconciliation_contract_is_closed_and_reports_only_current_import_repairs() {
    let command = ReconcileCurriculumAdoptionCommand { receipt: receipt() };
    let wire = serde_json::to_value(&command).expect("reconciliation command serializes");
    assert_eq!(wire["receipt"]["idempotencyKey"], "repair-2026-08-25");
    assert!(serde_json::from_value::<ReconcileCurriculumAdoptionCommand>(wire.clone()).is_ok());
    let mut authority = wire;
    authority["actor"] = serde_json::json!("browser-supplied");
    assert!(serde_json::from_value::<ReconcileCurriculumAdoptionCommand>(authority).is_err());

    let assignment = AssignmentReference::new(9).expect("assignment reference");
    let repaired = CurriculumAdoptionRepairedProjections::new(vec![
        CurriculumAdoptionRepairedProjection::AssignmentImportCurrent { assignment },
    ])
    .expect("one current import projection");
    let result = CurriculumAdoptionReconciliationResult::Repaired {
        receipt: receipt(),
        projections: repaired,
    };
    let result_wire = serde_json::to_value(&result).expect("reconciliation result serializes");
    assert_eq!(result_wire["kind"], "repaired");
    assert_eq!(
        result_wire["projections"][0]["kind"],
        "assignmentImportCurrent"
    );
    assert!(
        serde_json::from_value::<CurriculumAdoptionReconciliationResult>(result_wire.clone())
            .is_ok()
    );
    let mut leaking_result = result_wire;
    leaking_result["grade"] = serde_json::json!("not a B2 repair field");
    assert!(
        serde_json::from_value::<CurriculumAdoptionReconciliationResult>(leaking_result).is_err()
    );
    assert!(CurriculumAdoptionRepairedProjections::new(Vec::new()).is_err());
    assert!(
        CurriculumAdoptionRepairedProjections::new(vec![
            CurriculumAdoptionRepairedProjection::AssignmentImportCurrent { assignment },
            CurriculumAdoptionRepairedProjection::AssignmentImportCurrent { assignment },
        ])
        .is_err()
    );
    assert!(
        serde_json::from_value::<CurriculumAdoptionRepairedProjections>(serde_json::json!([]))
            .is_err()
    );
    assert!(
        serde_json::from_value::<CurriculumAdoptionRepairedProjections>(serde_json::json!([
            { "kind": "assignmentImportCurrent", "assignment": "A-9" },
            { "kind": "assignmentImportCurrent", "assignment": "A-9" },
        ]))
        .is_err()
    );
}
