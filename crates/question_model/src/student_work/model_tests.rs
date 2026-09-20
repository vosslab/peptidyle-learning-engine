use super::*;

fn attempt_evidence() -> AssessmentAttemptEvidence {
    AssessmentAttemptEvidence {
        title: crate::AssessmentTitle::try_new("Assessment".to_string()).expect("valid title"),
        instructions: crate::AssessmentInstructions::default(),
        base_policy: crate::BaseAssessmentPolicy::default(),
        activity_rules: crate::AssessmentActivityRules::default(),
        student_feedback_release_rule: crate::StudentFeedbackReleaseRule::default(),
        effective_policy_sources: AssessmentAttemptPolicySources::default(),
    }
}

fn reproduction_details() -> QuestionAttemptReproductionDetails {
    QuestionAttemptReproductionDetails {
        backend: QuestionBackendVersion {
            name: "ple".to_string(),
            version: "test".to_string(),
        },
        renderer_version: None,
        source_object_id: None,
        source_object_checksum: None,
        asset_objects: vec![],
        grader: QuestionGraderVersion {
            name: "ple".to_string(),
            version: "test".to_string(),
        },
        rendered_question_sha256: "0".repeat(64),
    }
}

#[test]
fn assessment_attempt_retains_interpretation_evidence() {
    let accommodation = AccommodationId::from_uuid(Uuid::from_u128(4));
    let mut evidence = attempt_evidence();
    evidence.effective_policy_sources.schedule =
        AssessmentAttemptPolicySource::Accommodation { accommodation };
    evidence.activity_rules.question_variation_rule =
        crate::AssessmentQuestionVariationRule::ReuseVariation;
    let mut attempt = AssessmentAttempt {
        id: AssessmentAttemptId::from_uuid(Uuid::from_u128(1)),
        student_record: StudentRecordId::from_uuid(Uuid::from_u128(2)),
        assessment_id: AssessmentId::from_debug_serial(3),
        evidence,
        attempt_number: 1,
        started_at: Timestamp::from_unix_millis(1_000),
        submitted_at: None,
        score: None,
    };

    assert_eq!(attempt.student_record.as_uuid(), Uuid::from_u128(2));
    assert_eq!(
        attempt.assessment_id.as_str(),
        AssessmentId::from_debug_serial(3).as_str()
    );
    assert_eq!(
        attempt.completion(),
        AssessmentAttemptCompletion::InProgress
    );
    attempt.score = Some(1.0);
    assert_eq!(
        attempt.completion(),
        AssessmentAttemptCompletion::InProgress
    );
    attempt.submitted_at = Some(Timestamp::from_unix_millis(2_000));
    attempt.score = None;
    assert_eq!(attempt.completion(), AssessmentAttemptCompletion::Completed);
    let attempt_wire = serde_json::to_value(&attempt).expect("Attempt serializes");
    assert!(attempt_wire.get("submittedAt").is_some());
    assert!(attempt_wire.get("completedAt").is_none());
    assert_eq!(attempt.evidence.title.as_str(), "Assessment");
    assert_eq!(
        attempt.question_variation_rule(),
        crate::AssessmentQuestionVariationRule::ReuseVariation
    );
    assert_eq!(
        attempt.evidence.effective_policy_sources.schedule,
        AssessmentAttemptPolicySource::Accommodation { accommodation }
    );
    assert_eq!(
        serde_json::to_value(attempt.evidence.effective_policy_sources)
            .expect("qualified evidence serializes")["schedule"],
        serde_json::json!({
            "kind": "accommodation",
            "accommodation": accommodation.to_string(),
        })
    );
}

#[test]
fn question_pool_selection_retains_exact_entries_and_issued_question_link() {
    let selection_id = QuestionPoolSelectionId::from_uuid(Uuid::from_u128(10));
    let question_pool_id: crate::QuestionId = "7654-Z321".parse().expect("valid Pool ID");
    let question_pool_edit_number =
        crate::QuestionPoolEditNumber::new(1).expect("positive Pool Edit Number");
    let question_revision_tuple = QuestionRevisionTuple {
        question_id: "1234-H567".parse().expect("valid Question ID"),
        revision_number: crate::QuestionRevisionNumber::new(1).expect("positive version"),
    };
    let selection = QuestionPoolSelection {
        id: selection_id,
        assessment_attempt_id: AssessmentAttemptId::from_uuid(Uuid::from_u128(12)),
        question_pool_assessment_entry: AssessmentEntryId::from_uuid(Uuid::from_u128(13)),
        question_pool_id: question_pool_id.clone(),
        question_pool_edit_number,
        created_at: Timestamp::from_unix_millis(1_000),
        selected_items: vec![QuestionPoolSelectedItem {
            question_pool_id: question_pool_id.clone(),
            question_pool_edit_number,
            member_position: 0,
            question_revision_tuple: question_revision_tuple.clone(),
        }],
    };
    let issued_question = IssuedQuestion {
        id: IssuedQuestionId::from_uuid(Uuid::from_u128(14)),
        assessment_attempt_id: selection.assessment_attempt_id,
        assessment_entry_id: selection.question_pool_assessment_entry,
        assessment_content_entry_index: 0,
        issued_position: 0,
        question_revision_tuple,
        source_selection: QuestionSourceSelection::Static,
        reproduction_details: reproduction_details(),
        point_value: crate::AssessmentPointValue::from_whole(1),
        scoring_rule: crate::AssessmentEntryScoringRule::Normal,
        question_statistics_eligibility: true,
        question_pool_selection: Some(selection_id),
        question_pool_id: Some(question_pool_id.clone()),
        question_pool_edit_number: Some(question_pool_edit_number),
        question_pool_member_position: Some(0),
    };

    assert_eq!(selection.selected_items.len(), 1);
    assert_eq!(issued_question.question_pool_selection, Some(selection.id));
    assert_eq!(issued_question.question_pool_id, Some(question_pool_id));
    assert_eq!(
        issued_question.question_pool_edit_number,
        Some(question_pool_edit_number)
    );
}

#[test]
fn student_assessment_progress_separates_activity_from_disclosed_grade() {
    assert_eq!(
        StudentAssessmentGrade::no_activity(crate::AssessmentScoringState::Current),
        StudentAssessmentGrade {
            score_state: AssessmentGradeScoreState::NoActivity,
            assessment_scoring_state: crate::AssessmentScoringState::Current,
            current_score: None,
            best_score: None,
            latest_score: None,
            class_statistics: None,
        }
    );
    let mut progress = AssessmentProgressRecord::empty(
        StudentRecordId::from_uuid(Uuid::from_u128(2)),
        AssessmentId::from_debug_serial(3),
    );
    let mut grade = AssessmentGrade::empty(progress.student_record, progress.assessment_id.clone());
    assert_eq!(
        StudentAssessmentGrade::from_assessment_grade(
            &grade,
            &progress,
            true,
            crate::AssessmentScoringState::Current
        )
        .score_state,
        AssessmentGradeScoreState::NoActivity
    );

    progress.total_question_attempts = 1;
    grade.current_score = Some(0.5);
    grade.best_score = Some(0.5);
    grade.latest_score = Some(0.5);
    let withheld = StudentAssessmentGrade::from_assessment_grade(
        &grade,
        &progress,
        false,
        crate::AssessmentScoringState::Current,
    );
    assert_eq!(withheld.score_state, AssessmentGradeScoreState::Withheld);
    assert_eq!(
        (
            withheld.current_score,
            withheld.best_score,
            withheld.latest_score
        ),
        (None, None, None)
    );

    let available = StudentAssessmentGrade::from_assessment_grade(
        &grade,
        &progress,
        true,
        crate::AssessmentScoringState::Current,
    );
    assert_eq!(available.score_state, AssessmentGradeScoreState::Available);
    assert_eq!(available.current_score, Some(0.5));
    assert!(available.class_statistics.is_none());
}

#[test]
fn student_assessment_grade_hides_scores_while_scoring_is_not_current() {
    let mut progress = AssessmentProgressRecord::empty(
        StudentRecordId::from_uuid(Uuid::from_u128(2)),
        AssessmentId::from_debug_serial(3),
    );
    progress.total_question_attempts = 1;
    let mut grade = AssessmentGrade::empty(progress.student_record, progress.assessment_id.clone());
    grade.current_score = Some(0.5);
    for assessment_scoring_state in [
        crate::AssessmentScoringState::Recalculating,
        crate::AssessmentScoringState::Failed,
    ] {
        let student_grade = StudentAssessmentGrade::from_assessment_grade(
            &grade,
            &progress,
            true,
            assessment_scoring_state,
        );
        assert_eq!(
            student_grade.score_state,
            AssessmentGradeScoreState::Available
        );
        assert_eq!(
            student_grade.assessment_scoring_state,
            assessment_scoring_state
        );
        assert_eq!(student_grade.current_score, None);
    }
}

#[test]
fn every_activity_identifier_stays_distinct_but_round_trips() {
    let raw = Uuid::from_u128(7);
    let assessment_attempt = AssessmentAttemptId::from_uuid(raw);
    let attempt = QuestionAttemptId::from_uuid(raw);

    assert_eq!(
        (assessment_attempt.as_uuid(), attempt.as_uuid()),
        (raw, raw)
    );
}

#[test]
fn question_attempt_state_uses_the_closed_operational_wire_vocabulary() {
    assert_eq!(
        serde_json::to_value(QuestionAttemptState::Open).expect("open state serializes"),
        serde_json::json!("open")
    );
    assert_eq!(
        serde_json::to_value(QuestionAttemptState::ResponseFinalized)
            .expect("accepted-submission state serializes"),
        serde_json::json!("response_finalized")
    );
    assert_eq!(
        serde_json::to_value(QuestionAttemptState::ClosedAtDeadline)
            .expect("deadline-closed state serializes"),
        serde_json::json!("closed_at_deadline")
    );
}

#[test]
fn reproduction_details_serialize_role_specific_versions() {
    let record = QuestionAttemptReproductionDetails {
        backend: QuestionBackendVersion {
            name: "ple-question-backend".to_string(),
            version: "1".to_string(),
        },
        renderer_version: None,
        source_object_id: Some(ObjectId::from_uuid(Uuid::from_u128(7))),
        source_object_checksum: Some(
            SourceObjectChecksum::parse("a".repeat(64)).expect("canonical checksum"),
        ),
        asset_objects: Vec::new(),
        grader: QuestionGraderVersion {
            name: "generic-grader".to_string(),
            version: "1".to_string(),
        },
        rendered_question_sha256: "a".repeat(64),
    };

    let wire = serde_json::to_value(record).expect("reproduction details serialize");
    assert!(wire.get("backend").is_some());
    assert!(wire.get("grader").is_some());
    assert_eq!(
        wire["sourceObjectId"],
        serde_json::json!("00000000-0000-0000-0000-000000000007")
    );
    assert_eq!(
        wire["sourceObjectChecksum"],
        serde_json::json!("a".repeat(64))
    );
    assert!(wire.get("adapter").is_none());
    assert!(wire.get("grading").is_none());
}

#[test]
fn question_attempt_browser_wire_omits_reproduction_details() {
    let attempt = QuestionAttempt {
        id: QuestionAttemptId::from_uuid(Uuid::from_u128(1)),
        issued_question: IssuedQuestionId::from_uuid(Uuid::from_u128(2)),
        reproduction: QuestionReproduction::Static,
        finalized_response: None,
        state: QuestionAttemptState::Open,
        timing: QuestionAttemptTiming {
            issued_at: Timestamp::from_unix_millis(4),
            deadline: None,
            finalized_at: None,
        },
        reproduction_details: QuestionAttemptReproductionDetails {
            backend: QuestionBackendVersion {
                name: "ple-question-backend".to_string(),
                version: "1".to_string(),
            },
            renderer_version: None,
            source_object_id: None,
            source_object_checksum: None,
            asset_objects: Vec::new(),
            grader: QuestionGraderVersion {
                name: "generic-grader".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: "b".repeat(64),
        },
        issued_capability: IssuedAttemptCapability::NotApplicable,
    };

    let view = StudentQuestionAttemptView::from(&attempt);
    let wire = serde_json::to_value(view).expect("Student Question Attempt View serializes");
    assert!(wire.get("parameterHash").is_none());
    assert!(wire.get("questionSeed").is_none());
    assert!(wire.get("reproduction").is_none());
    assert!(wire.get("reproductionDetails").is_none());
    assert_eq!(
        wire.get("id"),
        Some(&serde_json::json!(attempt.id.to_string()))
    );
    assert_eq!(
        wire.get("issuedQuestion"),
        Some(&serde_json::json!(attempt.issued_question.to_string()))
    );
    assert!(wire.get("issuedCapability").is_some());
    assert_eq!(
        wire.get("finalizedResponse"),
        Some(&serde_json::Value::Null)
    );
    assert!(wire.get("submission").is_none());
}

#[test]
fn saved_assessment_attempt_navigation_state_serializes() {
    assert_eq!(
        serde_json::to_value(StudentAssessmentAttemptResponseState::Saved)
            .expect("saved response state serializes"),
        serde_json::json!("saved")
    );
}
