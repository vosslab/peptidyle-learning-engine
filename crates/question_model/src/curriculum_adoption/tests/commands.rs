use super::*;

fn pin_position() -> CurriculumPinPosition {
    CurriculumPinPosition::new(None, 0, 1, Some(2)).expect("source pin position")
}

fn pin_recovery() -> UnavailablePinRecoveryAction {
    UnavailablePinRecoveryAction::SelectReplacementQuestion {
        position: pin_position(),
        candidates: ReplacementQuestionChoices::new(vec![
            "7K3-M9QX".parse().expect("public question ID"),
        ])
        .expect("replacement choice"),
    }
}

fn schedule_correction() -> CurriculumScheduleCorrection {
    AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
        AssignmentTeachingSettingsField::DueAt,
    )
    .into()
}

fn alpha_assignment_source(source: ObservedAlphaSource) -> AssignmentDefinitionSourceView {
    AssignmentDefinitionSourceView::Alpha(
        ObservedAlphaAssignmentSource::new(source, 3, 5).expect("bounded Alpha assignment source"),
    )
}

#[test]
fn every_write_command_is_derived_from_its_exact_preview() {
    let term =
        CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("target term");
    let title = CurriculumAdoptionTitle::parse("Fall Biochemistry").expect("course title");
    let assignment = PreparedCurriculumAssignmentView {
        title: CurriculumAdoptionTitle::parse("Protein Structure").expect("assignment title"),
        schedule: RelativeAssignmentSchedule::default()
            .resolve_for_target_term(&term)
            .expect("default schedule"),
    };
    let course = PreparedCurriculumCourseView {
        title: title.clone(),
        assignments: vec![assignment.clone()],
    };
    let source = ObservedAlphaSource {
        reference: AlphaCourseReference::new(4).expect("Alpha reference"),
        revision: "2".parse().expect("Alpha revision"),
    };
    let replacements = CurriculumPinReplacements::new(vec![CurriculumPinReplacement {
        position: pin_position(),
        question: "7K3-M9QX".parse().expect("public question ID"),
    }])
    .expect("replacement selection");
    let fork_preview = ForkAlphaPreviewView {
        source,
        resulting_alpha_title: title.clone(),
        replacements: replacements.clone(),
        pin_correction: None,
    };
    let fork_key = CurriculumAdoptionIdempotencyKey::parse("fork-alpha").expect("key");
    let fork_command = ForkAlphaCommand::from_preview(&fork_preview, fork_key.clone())
        .expect("corrected fork preview creates apply command");
    assert_eq!(fork_command.source(), source);
    assert_eq!(fork_command.replacements(), &replacements);
    assert_eq!(fork_command.idempotency_key(), &fork_key);
    let mut fork_with_pin = fork_preview.clone();
    fork_with_pin.pin_correction = Some(pin_recovery());
    assert_eq!(
        ForkAlphaCommand::from_preview(&fork_with_pin, fork_key.clone()),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );

    let alpha_preview = AlphaInstantiationPreviewView {
        source,
        target_term: term.clone(),
        course: course.clone(),
        replacements: replacements.clone(),
        corrections: Vec::new(),
        pin_correction: None,
    };
    let alpha_key = CurriculumAdoptionIdempotencyKey::parse("alpha-create").expect("key");
    let alpha_command = AlphaInstantiationCommand::from_preview(&alpha_preview, alpha_key.clone())
        .expect("corrected Alpha preview creates apply command");
    assert_eq!(alpha_command.source(), source);
    assert_eq!(alpha_command.title(), &title);
    assert_eq!(alpha_command.target_term(), &term);
    assert_eq!(alpha_command.replacements(), &replacements);
    assert_eq!(alpha_command.idempotency_key(), &alpha_key);
    let mut alpha_with_correction = alpha_preview.clone();
    alpha_with_correction
        .corrections
        .push(schedule_correction());
    assert_eq!(
        AlphaInstantiationCommand::from_preview(&alpha_with_correction, alpha_key.clone()),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );
    let mut alpha_with_pin = alpha_preview.clone();
    alpha_with_pin.pin_correction = Some(pin_recovery());
    assert_eq!(
        AlphaInstantiationCommand::from_preview(&alpha_with_pin, alpha_key.clone()),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );

    let destination_course = CourseReference::new(7).expect("course reference");
    let observed_assignment = ObservedAssignmentRevision {
        assignment: AssignmentReference::new(9).expect("assignment reference"),
        revision: "5".parse().expect("assignment revision"),
    };
    let witness = CourseScheduleWitness::new(
        destination_course,
        CourseScheduleRevision::new(3).expect("schedule revision"),
        vec![observed_assignment],
    )
    .expect("source course witness");

    let blueprint_source = ObservedBlueprintSource {
        reference: BlueprintReference::new(6).expect("Blueprint reference"),
        revision: "4".parse().expect("Blueprint revision"),
    };
    let blueprint_preview = BlueprintInstantiationPreviewView {
        source: blueprint_source,
        course: destination_course,
        target_term: term.clone(),
        witness: witness.clone(),
        assignment: assignment.clone(),
        replacements: replacements.clone(),
        corrections: Vec::new(),
        pin_correction: None,
    };
    let blueprint_key =
        CurriculumAdoptionIdempotencyKey::parse("instantiate-blueprint").expect("key");
    let blueprint_command =
        BlueprintInstantiationCommand::from_preview(&blueprint_preview, blueprint_key.clone())
            .expect("corrected Blueprint preview creates apply command");
    assert_eq!(blueprint_command.source(), blueprint_source);
    assert_eq!(blueprint_command.course(), destination_course);
    assert_eq!(blueprint_command.target_term(), &term);
    assert_eq!(blueprint_command.preview_witness(), &witness);
    assert_eq!(blueprint_command.replacements(), &replacements);
    assert_eq!(blueprint_command.idempotency_key(), &blueprint_key);
    let mut blueprint_with_correction = blueprint_preview.clone();
    blueprint_with_correction
        .corrections
        .push(schedule_correction());
    assert_eq!(
        BlueprintInstantiationCommand::from_preview(
            &blueprint_with_correction,
            blueprint_key.clone()
        ),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );
    let mut blueprint_with_pin = blueprint_preview.clone();
    blueprint_with_pin.pin_correction = Some(pin_recovery());
    assert_eq!(
        BlueprintInstantiationCommand::from_preview(&blueprint_with_pin, blueprint_key.clone()),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );

    let rollover_preview = CourseRolloverPreviewView {
        witness: witness.clone(),
        target_term: term.clone(),
        course: PreparedCurriculumCourseView {
            title: title.clone(),
            assignments: vec![assignment.clone()],
        },
        replacements: replacements.clone(),
        corrections: Vec::new(),
        pin_correction: None,
    };
    let rollover_key = CurriculumAdoptionIdempotencyKey::parse("rollover").expect("key");
    let rollover_command =
        CourseRolloverCommand::from_preview(&rollover_preview, rollover_key.clone())
            .expect("corrected rollover preview creates apply command");
    assert_eq!(rollover_command.preview_witness(), &witness);
    assert_eq!(rollover_command.title(), &title);
    assert_eq!(rollover_command.target_term(), &term);
    assert_eq!(rollover_command.replacements(), &replacements);
    assert_eq!(rollover_command.idempotency_key(), &rollover_key);
    let mut rollover_with_correction = rollover_preview.clone();
    rollover_with_correction
        .corrections
        .push(schedule_correction());
    assert_eq!(
        CourseRolloverCommand::from_preview(&rollover_with_correction, rollover_key.clone()),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );
    let mut rollover_with_pin = rollover_preview.clone();
    rollover_with_pin.pin_correction = Some(pin_recovery());
    assert_eq!(
        CourseRolloverCommand::from_preview(&rollover_with_pin, rollover_key.clone()),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );

    let term_shift_preview = CourseTermShiftPreviewView {
        witness: witness.clone(),
        target_term: term.clone(),
        assignments: Vec::new(),
        corrections: Vec::new(),
    };
    let term_shift_key = CurriculumAdoptionIdempotencyKey::parse("shift-term").expect("key");
    let term_shift_command = CourseTermShiftCommand::from_preview(
        &CourseTermShiftPreviewOutcome::Eligible {
            preview: term_shift_preview.clone(),
        },
        term_shift_key.clone(),
    )
    .expect("corrected term-shift preview creates apply command");
    assert_eq!(term_shift_command.preview_witness(), &witness);
    assert_eq!(term_shift_command.target_term(), &term);
    assert_eq!(term_shift_command.idempotency_key(), &term_shift_key);
    let mut term_shift_with_correction = term_shift_preview.clone();
    term_shift_with_correction
        .corrections
        .push(schedule_correction());
    assert_eq!(
        CourseTermShiftCommand::from_preview(
            &CourseTermShiftPreviewOutcome::Eligible {
                preview: term_shift_with_correction,
            },
            term_shift_key.clone(),
        ),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );

    let fast_forward_preview = AssignmentFastForwardPreviewView {
        course: destination_course,
        assignment: observed_assignment,
        import_revision: CurriculumImportRevision::new(8).expect("import revision"),
        source: alpha_assignment_source(source),
        witness: witness.clone(),
        decision: AssignmentFastForwardDecision::Eligible,
    };
    let fast_forward_key = CurriculumAdoptionIdempotencyKey::parse("fast-forward").expect("key");
    let fast_forward_command =
        AssignmentFastForwardCommand::from_preview(&fast_forward_preview, fast_forward_key.clone())
            .expect("eligible preview creates apply command");
    assert_eq!(fast_forward_command.course(), destination_course);
    assert_eq!(fast_forward_command.assignment(), observed_assignment);
    assert_eq!(
        fast_forward_command.import_revision(),
        fast_forward_preview.import_revision
    );
    assert_eq!(fast_forward_command.source(), fast_forward_preview.source);
    let AssignmentDefinitionSourceView::Alpha(observed_alpha_assignment) =
        fast_forward_command.source()
    else {
        panic!("fast-forward source remains an exact Alpha assignment");
    };
    assert_eq!(observed_alpha_assignment.module_index(), 3);
    assert_eq!(observed_alpha_assignment.assignment_index(), 5);
    assert_eq!(fast_forward_command.preview_witness(), &witness);
    assert_eq!(fast_forward_command.idempotency_key(), &fast_forward_key);

    for decision in [
        AssignmentFastForwardDecision::Divergent {
            recovery: PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment,
        },
        AssignmentFastForwardDecision::UnavailablePin {
            recovery: pin_recovery(),
        },
        AssignmentFastForwardDecision::SourceRevisionDrift {
            source: AssignmentDefinitionSourceView::Blueprint(blueprint_source),
        },
        AssignmentFastForwardDecision::IssuedWork {
            recovery: PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment,
        },
    ] {
        let mut recovery_preview = fast_forward_preview.clone();
        recovery_preview.decision = decision;
        assert_eq!(
            AssignmentFastForwardCommand::from_preview(&recovery_preview, fast_forward_key.clone(),),
            Err(CurriculumAdoptionCommandError::FastForwardNotEligible)
        );
    }

    let source_derived_preview = SourceDerivedAssignmentPreviewView {
        course: destination_course,
        source: alpha_assignment_source(source),
        witness: witness.clone(),
        assignment,
        replacements: replacements.clone(),
        corrections: Vec::new(),
        pin_correction: None,
    };
    let source_derived_key =
        CurriculumAdoptionIdempotencyKey::parse("source-derived").expect("key");
    let source_derived_command = CreateSourceDerivedAssignmentCommand::from_preview(
        &source_derived_preview,
        source_derived_key.clone(),
    )
    .expect("corrected source-derived preview creates apply command");
    assert_eq!(source_derived_command.course(), destination_course);
    assert_eq!(
        source_derived_command.source(),
        source_derived_preview.source
    );
    assert_eq!(source_derived_command.preview_witness(), &witness);
    assert_eq!(source_derived_command.replacements(), &replacements);
    assert_eq!(
        source_derived_command.idempotency_key(),
        &source_derived_key
    );
    let mut source_derived_with_correction = source_derived_preview.clone();
    source_derived_with_correction
        .corrections
        .push(schedule_correction());
    assert_eq!(
        CreateSourceDerivedAssignmentCommand::from_preview(
            &source_derived_with_correction,
            source_derived_key.clone()
        ),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );
    let mut source_derived_with_pin = source_derived_preview.clone();
    source_derived_with_pin.pin_correction = Some(pin_recovery());
    assert_eq!(
        CreateSourceDerivedAssignmentCommand::from_preview(
            &source_derived_with_pin,
            source_derived_key
        ),
        Err(CurriculumAdoptionCommandError::CorrectionsRequired)
    );
}
