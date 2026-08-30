//! Server-record invariants for CourseInstance adoption commands and receipts.

use super::*;
use crate::{
    ActivityTimestamp, AssignmentReference, AssignmentRevision, BlueprintAssignmentId,
    BlueprintReference, BlueprintRevision, CourseReference, CourseScheduleRevision, UserId,
};
use uuid::Uuid;

fn source() -> ObservedBlueprintSource {
    ObservedBlueprintSource {
        reference: BlueprintReference::new(7).expect("BlueprintCourse reference"),
        revision: BlueprintRevision::new(2).expect("BlueprintCourse revision"),
    }
}

fn blueprint_application() -> CourseInstanceBlueprintApplication {
    CourseInstanceBlueprintApplication { source: source() }
}

fn application_binding(destination: CourseInstanceWitness) -> CourseInstanceApplicationBinding {
    CourseInstanceApplicationBinding::new(destination, blueprint_application())
}

fn request_binding(
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
) -> CurriculumAdoptionRequestBinding {
    CurriculumAdoptionRequestBinding::new(authorized_actor, request_digest, idempotency_key)
}

fn term() -> crate::CourseTerm {
    crate::CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago")
        .expect("course term")
}

fn witness() -> CourseInstanceWitness {
    CourseInstanceWitness::new(
        CourseReference::new(3).expect("course"),
        CourseScheduleRevision::new(1).expect("schedule revision"),
        vec![],
    )
    .expect("bounded witness")
}

fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(1))
}

fn assignment() -> ObservedCourseInstanceAssignment {
    ObservedCourseInstanceAssignment {
        assignment: AssignmentReference::new(8).expect("assignment"),
        revision: AssignmentRevision::new(2).expect("assignment revision"),
    }
}

fn witnessed_assignment() -> CourseInstanceWitness {
    CourseInstanceWitness::new(
        CourseReference::new(3).expect("course"),
        CourseScheduleRevision::new(1).expect("schedule revision"),
        vec![assignment()],
    )
    .expect("bounded witness")
}

fn selected_copy_outcome() -> CourseInstanceWitness {
    CourseInstanceWitness::new(
        CourseReference::new(3).expect("course"),
        CourseScheduleRevision::new(2).expect("post-copy schedule revision"),
        vec![assignment()],
    )
    .expect("selected-copy outcome")
}

fn assignment_source() -> AssignmentDefinitionSourceView {
    AssignmentDefinitionSourceView::new(
        source(),
        BlueprintAssignmentId::from_uuid(Uuid::from_u128(9)),
    )
}

fn newer_assignment_source() -> AssignmentDefinitionSourceView {
    AssignmentDefinitionSourceView::new(
        ObservedBlueprintSource {
            reference: source().reference,
            revision: BlueprintRevision::new(3).expect("newer revision"),
        },
        BlueprintAssignmentId::from_uuid(Uuid::from_u128(9)),
    )
}

fn schedule() -> crate::ResolvedRelativeAssignmentSchedule {
    serde_json::from_value(serde_json::json!({
        "time_zone": "America/Chicago",
        "available_at": null,
        "due_at": null,
        "closes_at": null
    }))
    .expect("resolved schedule")
}

fn applied(
    source: AssignmentDefinitionSourceView,
    observed: ObservedCourseInstanceAssignment,
    revision: CurriculumImportRevision,
) -> AppliedAssignmentImportEvidence {
    AppliedAssignmentImportEvidence::new(
        source,
        CurriculumPinReplacements::default(),
        crate::curriculum_adoption::CurriculumSemanticDigest::test_value([11; 32]),
        observed,
        revision,
    )
}

#[test]
fn rollover_browser_request_is_closed() {
    let mut wire = serde_json::to_value(RolloverCourseInstancePreviewRequest {
        source_course: CourseReference::new(3).expect("course"),
        target_term: term(),
    })
    .expect("wire");
    wire["authority"] = serde_json::json!(true);
    assert!(serde_json::from_value::<RolloverCourseInstancePreviewRequest>(wire).is_err());
}

#[test]
fn initial_blueprint_application_is_closed_snake_case_and_answer_free() {
    let mut wire = serde_json::to_value(blueprint_application()).expect("wire");
    assert!(wire.get("source").is_some());
    assert!(!wire.to_string().contains("answer"));
    wire["authority"] = serde_json::json!(true);
    assert!(serde_json::from_value::<CourseInstanceBlueprintApplication>(wire).is_err());
}

#[test]
fn rollover_exclusion_policy_has_no_browser_selected_records() {
    let manifest = RolloverCourseInstanceManifest::new(
        RolloverReusableStateManifest::new(source(), vec![], vec![]).expect("bounded manifest"),
    );
    let wire = serde_json::to_value(manifest).expect("wire");
    assert_eq!(
        wire["exclusion_policy"],
        "exclude_all_student_and_delivery_records"
    );
    assert!(wire.get("excluded").is_none());
}

#[test]
fn term_shift_receipt_uses_the_server_record_operation_kind() {
    let record = ShiftCourseInstanceTermApplyRecord::new(
        application_binding(witness()),
        term(),
        vec![],
        request_binding(
            actor(),
            [4; 32],
            CurriculumAdoptionIdempotencyKey::parse("receipt-kind").expect("key"),
        ),
        CourseInstanceEligibility::Eligible,
    )
    .expect("server record");
    let receipt = ShiftCourseInstanceTermReceipt::from_server_record(
        record,
        CourseInstanceWitness::new(
            CourseReference::new(3).expect("course"),
            CourseScheduleRevision::new(2).expect("post-shift schedule revision"),
            vec![],
        )
        .expect("post-shift witness"),
        ActivityTimestamp::from_unix_millis(1),
    )
    .expect("receipt");
    assert_eq!(
        receipt.binding().operation(),
        CourseInstanceOperationKind::ShiftTerm
    );
}

#[test]
fn rollover_command_consumes_the_server_reservation_before_mutation() {
    let source_witness = witness();
    let creation = CourseInstanceCreationWitness::for_rollover(
        source_witness.clone(),
        term(),
        actor(),
        [6; 32],
        CurriculumAdoptionIdempotencyKey::parse("rollover-record").expect("key"),
        CourseReference::new(4).expect("reserved course"),
    );
    let manifest = RolloverCourseInstanceManifest::new(
        RolloverReusableStateManifest::new(source(), vec![], vec![]).expect("bounded manifest"),
    );
    let command = RolloverCourseInstanceCommand::from_server_record(
        RolloverCourseInstanceApplyRecord::new(
            source_witness.clone(),
            blueprint_application(),
            term(),
            manifest,
            creation.clone(),
            CourseInstanceEligibility::Eligible,
        )
        .expect("server record"),
    );
    assert_eq!(command.source_course_instance(), &source_witness);
    assert_eq!(command.blueprint_application(), blueprint_application());
    assert_eq!(command.creation(), &creation);
}

#[test]
fn server_records_ignore_mutated_term_shift_controlled_update_and_selected_copy_previews() {
    let key = CurriculumAdoptionIdempotencyKey::parse("record-authority").expect("key");
    let original_destination = witnessed_assignment();
    let original_term = term();
    let original_schedule = schedule();
    let source = assignment_source();

    let mut shift_preview = ShiftCourseInstanceTermPreview {
        witness: original_destination.clone(),
        target_term: original_term.clone(),
        schedules: BoundedResolvedScheduleSet::new(vec![original_schedule.clone()])
            .expect("bounded schedules"),
        eligibility: CourseInstanceEligibility::Eligible,
    };
    let shift_record = ShiftCourseInstanceTermApplyRecord::new(
        application_binding(shift_preview.witness.clone()),
        shift_preview.target_term.clone(),
        shift_preview.schedules.as_slice().to_vec(),
        request_binding(actor(), [1; 32], key.clone()),
        shift_preview.eligibility.clone(),
    )
    .expect("server record");
    shift_preview.witness = witness();
    shift_preview.schedules = BoundedResolvedScheduleSet::new(vec![]).expect("bounded schedules");
    let shift = ShiftCourseInstanceTermCommand::from_server_record(shift_record);
    assert_eq!(shift.destination(), &original_destination);
    assert_eq!(shift.target_term(), &original_term);
    assert_eq!(shift.schedules(), std::slice::from_ref(&original_schedule));

    let import = CourseInstanceImportWitness {
        source,
        destination: assignment(),
        import_revision: CurriculumImportRevision::new(3).expect("import revision"),
    };
    let mut update_preview = ControlledUpdateBlueprintAssignmentPreview {
        import: import.clone(),
        witness: original_destination.clone(),
        eligibility: CourseInstanceEligibility::Eligible,
    };
    let update_record = ControlledUpdateBlueprintAssignmentApplyRecord::new(
        newer_assignment_source(),
        update_preview.import.clone(),
        application_binding(update_preview.witness.clone()),
        request_binding(actor(), [2; 32], key.clone()),
        update_preview.eligibility.clone(),
    )
    .expect("server record");
    update_preview.witness = witness();
    update_preview.import.import_revision = CurriculumImportRevision::new(4).expect("revision");
    let update = ControlledUpdateBlueprintAssignmentCommand::from_server_record(update_record);
    assert_eq!(update.destination(), &original_destination);
    assert_eq!(update.import(), &import);

    let mut selected_preview = CreateSelectedBlueprintAssignmentPreview {
        source,
        witness: original_destination.clone(),
        schedule: original_schedule.clone(),
        eligibility: CourseInstanceEligibility::Eligible,
    };
    let selected_record = CreateSelectedBlueprintAssignmentApplyRecord::new(
        selected_preview.source,
        application_binding(selected_preview.witness.clone()),
        selected_preview.schedule.clone(),
        CurriculumPinReplacements::default(),
        request_binding(actor(), [3; 32], key),
        selected_preview.eligibility.clone(),
    )
    .expect("server record");
    selected_preview.witness = witness();
    selected_preview.schedule = schedule();
    let selected = CreateSelectedBlueprintAssignmentCommand::from_server_record(selected_record);
    assert_eq!(selected.destination(), &original_destination);
    assert_eq!(selected.schedule(), &original_schedule);
}

#[test]
fn receipts_retain_the_actor_from_the_consumed_server_record() {
    let destination = witnessed_assignment();
    let source_location = newer_assignment_source();
    let import = CourseInstanceImportWitness {
        source: assignment_source(),
        destination: assignment(),
        import_revision: CurriculumImportRevision::new(3).expect("import revision"),
    };
    let record_actor = actor();
    let receipt = ControlledUpdateBlueprintAssignmentReceipt::from_server_record(
        ControlledUpdateBlueprintAssignmentApplyRecord::new(
            source_location,
            import.clone(),
            application_binding(destination.clone()),
            request_binding(
                record_actor,
                [9; 32],
                CurriculumAdoptionIdempotencyKey::parse("receipt-binding").expect("key"),
            ),
            CourseInstanceEligibility::Eligible,
        )
        .expect("server record"),
        destination.clone(),
        applied(
            source_location,
            assignment(),
            CurriculumImportRevision::new(4).expect("next import revision"),
        ),
        ControlledUpdateEffect::SourceRevisionOnly,
        ActivityTimestamp::from_unix_millis(1),
    )
    .expect("receipt");
    assert_eq!(receipt.binding().authorized_actor(), record_actor);
    assert_eq!(receipt.binding().destination(), &destination);
    assert_eq!(
        receipt.binding().blueprint_application(),
        blueprint_application()
    );
    assert_eq!(receipt.applied().source(), source_location);
    assert_eq!(receipt.consumed_import(), &import);
}

#[test]
fn assignment_receipt_binds_the_consumed_precondition_to_its_exact_outcome() {
    let precondition = witnessed_assignment();
    let outcome = CourseInstanceWitness::new(
        precondition.course,
        CourseScheduleRevision::new(2).expect("post-update schedule revision"),
        vec![ObservedCourseInstanceAssignment {
            assignment: assignment().assignment,
            revision: AssignmentRevision::new(3).expect("post-update assignment revision"),
        }],
    )
    .expect("outcome witness");
    let source_location = newer_assignment_source();
    let import = CourseInstanceImportWitness {
        source: assignment_source(),
        destination: assignment(),
        import_revision: CurriculumImportRevision::new(3).expect("import revision"),
    };
    let receipt = ControlledUpdateBlueprintAssignmentReceipt::from_server_record(
        ControlledUpdateBlueprintAssignmentApplyRecord::new(
            source_location,
            import,
            application_binding(precondition.clone()),
            request_binding(
                actor(),
                [12; 32],
                CurriculumAdoptionIdempotencyKey::parse("receipt-outcome").expect("key"),
            ),
            CourseInstanceEligibility::Eligible,
        )
        .expect("record"),
        outcome.clone(),
        applied(
            source_location,
            outcome.assignments()[0],
            CurriculumImportRevision::new(4).expect("next import revision"),
        ),
        ControlledUpdateEffect::MeaningChanged,
        ActivityTimestamp::from_unix_millis(1),
    )
    .expect("receipt");
    assert_eq!(receipt.binding().precondition(), &precondition);
    assert_eq!(receipt.binding().outcome(), &outcome);
    assert_ne!(
        receipt.binding().precondition(),
        receipt.binding().outcome()
    );
    let target = CourseInstanceReceiptTarget::ControlledUpdate(receipt)
        .assignment_import_target()
        .expect("assignment receipt locator");
    assert_eq!(target.assignment(), assignment().assignment);
    assert_eq!(target.import_revision().value(), 4);
}

#[test]
fn server_records_reject_detached_import_or_receipt_evidence() {
    let destination = witness();
    let source_location = newer_assignment_source();
    let import = CourseInstanceImportWitness {
        source: assignment_source(),
        destination: assignment(),
        import_revision: CurriculumImportRevision::new(3).expect("import revision"),
    };
    let key = CurriculumAdoptionIdempotencyKey::parse("server-record-check").expect("key");
    assert!(source_location.is_strictly_newer_revision_of(import.source));
    assert!(!import.source.is_strictly_newer_revision_of(import.source));
    assert!(
        !AssignmentDefinitionSourceView::new(
            ObservedBlueprintSource {
                reference: source().reference,
                revision: BlueprintRevision::new(3).expect("newer revision"),
            },
            BlueprintAssignmentId::from_uuid(Uuid::from_u128(10)),
        )
        .is_strictly_newer_revision_of(import.source)
    );
    assert_eq!(
        ControlledUpdateBlueprintAssignmentApplyRecord::new(
            AssignmentDefinitionSourceView::new(
                ObservedBlueprintSource {
                    reference: source().reference,
                    revision: BlueprintRevision::new(3).expect("newer revision"),
                },
                BlueprintAssignmentId::from_uuid(Uuid::from_u128(10)),
            ),
            import,
            application_binding(destination.clone()),
            request_binding(actor(), [1; 32], key.clone()),
            CourseInstanceEligibility::Eligible,
        ),
        Err(CourseInstanceCommandError::ControlledUpdateLineageMismatch)
    );

    let receipt = CreateSelectedBlueprintAssignmentReceipt::from_server_record(
        CreateSelectedBlueprintAssignmentApplyRecord::new(
            source_location,
            application_binding(destination),
            schedule(),
            CurriculumPinReplacements::default(),
            request_binding(actor(), [2; 32], key.clone()),
            CourseInstanceEligibility::Eligible,
        )
        .expect("server record"),
        selected_copy_outcome(),
        applied(
            source_location,
            assignment(),
            CurriculumImportRevision::new(1).expect("revision"),
        ),
        ActivityTimestamp::from_unix_millis(1),
    )
    .expect("receipt");
    assert!(
        ReconcileCourseInstanceAdoptionApplyRecord::new(
            CourseInstanceReceiptTarget::SelectedCopy(receipt),
            blueprint_application(),
            actor(),
            [3; 32],
            CurriculumAdoptionIdempotencyKey::parse("fresh-reconcile-receipt").expect("fresh key"),
            CourseInstanceEligibility::Eligible,
        )
        .is_ok()
    );
}

#[test]
fn direct_course_instance_and_rollover_construction_refuse_overflow() {
    let assignments = vec![assignment(); crate::MAX_ASSIGNMENT_ORDERED_ENTRIES + 1];
    assert_eq!(
        CourseInstanceWitness::new(
            CourseReference::new(3).expect("course"),
            CourseScheduleRevision::new(1).expect("schedule revision"),
            assignments,
        ),
        Err(CourseInstanceWitnessError)
    );

    let source_location = assignment_source();
    let sources = vec![source_location; crate::MAX_ASSIGNMENT_ORDERED_ENTRIES + 1];
    assert_eq!(
        RolloverReusableStateManifest::new(source(), sources, vec![]),
        Err(RolloverReusableStateManifestError)
    );

    let schedules = vec![schedule(); crate::MAX_ASSIGNMENT_ORDERED_ENTRIES + 1];
    assert_eq!(
        RolloverReusableStateManifest::new(source(), vec![], schedules),
        Err(RolloverReusableStateManifestError)
    );

    let mut witness_wire = serde_json::to_value(witness()).expect("witness wire");
    witness_wire["assignments"] = serde_json::Value::Array(
        std::iter::repeat_with(|| serde_json::to_value(assignment()).expect("assignment wire"))
            .take(crate::MAX_ASSIGNMENT_ORDERED_ENTRIES + 1)
            .collect(),
    );
    assert!(serde_json::from_value::<CourseInstanceWitness>(witness_wire).is_err());

    let mut manifest_wire = serde_json::to_value(
        RolloverReusableStateManifest::new(source(), vec![], vec![]).expect("manifest"),
    )
    .expect("manifest wire");
    manifest_wire["schedules"] = serde_json::Value::Array(
        std::iter::repeat_with(|| serde_json::to_value(schedule()).expect("schedule wire"))
            .take(crate::MAX_ASSIGNMENT_ORDERED_ENTRIES + 1)
            .collect(),
    );
    assert!(serde_json::from_value::<RolloverReusableStateManifest>(manifest_wire).is_err());
}

#[test]
fn course_instance_witness_refuses_duplicate_assignment_references() {
    let reference = AssignmentReference::new(8).expect("assignment");
    assert!(
        CourseInstanceWitness::new(
            CourseReference::new(3).expect("course"),
            CourseScheduleRevision::new(1).expect("schedule revision"),
            vec![
                ObservedCourseInstanceAssignment {
                    assignment: reference,
                    revision: AssignmentRevision::new(1).expect("first revision"),
                },
                ObservedCourseInstanceAssignment {
                    assignment: reference,
                    revision: AssignmentRevision::new(2).expect("second revision"),
                },
            ],
        )
        .is_err()
    );
}

#[test]
fn course_instance_completion_dtos_are_closed_snake_case_and_answer_free() {
    let course = CourseReference::new(3).expect("course");
    let assignment = AssignmentReference::new(8).expect("assignment");
    macro_rules! assert_closed_completion {
        ($value:expr, $type:ty) => {{
            let completion = serde_json::to_value($value).expect("completion serializes");
            let object = completion.as_object().expect("completion object");
            assert!(object.contains_key("course"));
            assert!(object.contains_key("replay"));
            assert!(
                !object
                    .keys()
                    .any(|key| key.contains("answer") || key.contains("receipt"))
            );
            assert!(serde_json::from_value::<$type>(completion.clone()).is_ok());
            let mut unknown = completion;
            unknown["authority"] = serde_json::json!(true);
            assert!(serde_json::from_value::<$type>(unknown).is_err());
        }};
    }

    assert_closed_completion!(
        RolloverCourseInstanceCompleted {
            course,
            replay: CurriculumReplayStatus::Applied,
        },
        RolloverCourseInstanceCompleted
    );
    assert_closed_completion!(
        ShiftCourseInstanceTermCompleted {
            course,
            replay: CurriculumReplayStatus::Replayed,
        },
        ShiftCourseInstanceTermCompleted
    );
    assert_closed_completion!(
        ControlledUpdateBlueprintAssignmentCompleted {
            course,
            assignment,
            replay: CurriculumReplayStatus::Applied,
        },
        ControlledUpdateBlueprintAssignmentCompleted
    );
    assert_closed_completion!(
        CreateSelectedBlueprintAssignmentCompleted {
            course,
            assignment,
            replay: CurriculumReplayStatus::Replayed,
        },
        CreateSelectedBlueprintAssignmentCompleted
    );
    assert_closed_completion!(
        ReconcileCourseInstanceAdoptionCompleted {
            course,
            replay: CurriculumReplayStatus::Applied,
        },
        ReconcileCourseInstanceAdoptionCompleted
    );
}

#[test]
fn reconciliation_projection_is_receipt_targeted_and_server_only() {
    let destination = witness();
    let key = CurriculumAdoptionIdempotencyKey::parse("reconcile-target").expect("key");
    let selected = CreateSelectedBlueprintAssignmentReceipt::from_server_record(
        CreateSelectedBlueprintAssignmentApplyRecord::new(
            assignment_source(),
            application_binding(destination),
            schedule(),
            CurriculumPinReplacements::default(),
            request_binding(actor(), [7; 32], key.clone()),
            CourseInstanceEligibility::Eligible,
        )
        .expect("server record"),
        selected_copy_outcome(),
        applied(
            assignment_source(),
            assignment(),
            CurriculumImportRevision::new(1).expect("revision"),
        ),
        ActivityTimestamp::from_unix_millis(1),
    )
    .expect("receipt");
    let target = CourseInstanceReceiptTarget::SelectedCopy(selected);
    let reconcile_actor = UserId::from_uuid(Uuid::from_u128(12));
    let record = ReconcileCourseInstanceAdoptionApplyRecord::new(
        target,
        blueprint_application(),
        reconcile_actor,
        [7; 32],
        key,
        CourseInstanceEligibility::Eligible,
    )
    .expect("receipt-targeted record");
    let receipt = ReconcileCourseInstanceAdoptionReceipt::from_server_record(
        record,
        ActivityTimestamp::from_unix_millis(2),
    )
    .expect("receipt");
    let projection = ReconcileCourseInstanceAdoptionPreview::new(
        CourseInstanceReceiptTarget::Reconcile(receipt),
        CourseInstanceEligibility::Eligible,
    );
    assert_eq!(projection.receipt().authorized_actor(), reconcile_actor);
    assert_eq!(
        projection.receipt().operation(),
        CourseInstanceOperationKind::Reconcile
    );
}
