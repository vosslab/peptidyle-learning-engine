use super::*;

#[test]
fn fast_forward_recoveries_are_structured_and_answer_free() {
    let question = "7K3-M9QX".parse::<QuestionId>().expect("question ID");
    let recovery = UnavailablePinRecoveryAction::SelectReplacementQuestion {
        position: CurriculumPinPosition::new(Some(1), 2, 3, Some(4)).expect("position"),
        candidates: ReplacementQuestionChoices::new(vec![question]).expect("candidate"),
    };
    let wire = serde_json::to_value(&recovery).expect("recovery serializes");
    assert_eq!(wire["kind"], "selectReplacementQuestion");
    assert_eq!(wire["candidates"][0], "7K3-M9QX");
    for absent in ["version", "pin", "uuid", "authority", "tenant"] {
        assert!(!wire.to_string().contains(absent));
    }
    assert!(serde_json::from_value::<UnavailablePinRecoveryAction>(wire.clone()).is_ok());
    assert!(
        serde_json::from_value::<AssignmentFastForwardDecision>(serde_json::json!({
            "kind": "issuedWork",
            "recovery": wire,
        }))
        .is_err()
    );

    let preserve = PreservedAssignmentRecoveryAction::CreateSourceDerivedAssignment;
    let preserve_wire = serde_json::to_value(&preserve).expect("preservation action serializes");
    assert_eq!(preserve_wire["kind"], "createSourceDerivedAssignment");
    assert!(serde_json::from_value::<UnavailablePinRecoveryAction>(preserve_wire.clone()).is_err());
    assert!(
        serde_json::from_value::<PreservedAssignmentRecoveryAction>(preserve_wire.clone()).is_ok()
    );
    assert!(
        serde_json::from_value::<AssignmentFastForwardDecision>(serde_json::json!({
            "kind": "unavailablePin",
            "recovery": preserve_wire,
        }))
        .is_err()
    );
    assert!(ReplacementQuestionChoices::new(vec![]).is_err());
    let oversized_choices = serde_json::json!(vec![
        "7K3-M9QX";
        MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP
            + 1
    ]);
    let error = serde_json::from_value::<ReplacementQuestionChoices>(oversized_choices)
        .expect_err("oversized replacement choices must fail during sequence decoding");
    assert!(error.to_string().contains("at most"));
    assert!(CurriculumPinPosition::new(None, 0, 0, Some(u16::MAX)).is_err());

    let replacement = CurriculumPinReplacement {
        position: CurriculumPinPosition::new(None, 0, 0, None).expect("fixed-item position"),
        question: "7K3-M9QX".parse().expect("public question ID"),
    };
    assert!(
        CurriculumPinReplacements::new(vec![replacement.clone(), replacement.clone()]).is_err()
    );
    let later_replacement = CurriculumPinReplacement {
        position: CurriculumPinPosition::new(None, 0, 1, None).expect("later fixed-item position"),
        question: "7K3-M9QX".parse().expect("public question ID"),
    };
    let replacements = CurriculumPinReplacements::new(vec![later_replacement, replacement])
        .expect("unique replacements");
    assert_eq!(replacements.as_slice()[0].position.entry_index(), 0);
    assert_eq!(replacements.as_slice()[1].position.entry_index(), 1);
    let replacements_wire =
        serde_json::to_value(replacements).expect("replacement selections serialize");
    assert!(serde_json::from_value::<CurriculumPinReplacements>(replacements_wire).is_ok());
    let oversized_replacements = serde_json::json!(vec![
        serde_json::json!({
            "position": {
                "moduleIndex": null,
                "assignmentIndex": 0,
                "entryIndex": 0,
                "candidateIndex": null,
            },
            "question": "7K3-M9QX",
        });
        crate::MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES
            + 1
    ]);
    let error = serde_json::from_value::<CurriculumPinReplacements>(oversized_replacements)
        .expect_err("oversized pin replacements must fail during sequence decoding");
    assert!(error.to_string().contains("at most"));

    let correction = CurriculumScheduleCorrection::from(
        AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
            AssignmentTeachingSettingsField::DueAt,
        ),
    );
    assert_eq!(
        correction.correction.reason,
        AssignmentTeachingSettingsFailureReason::NonexistentLocalTime
    );
}
