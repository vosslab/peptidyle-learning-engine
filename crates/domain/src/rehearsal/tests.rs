use super::*;
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, CourseLocalDateTime, IanaTimeZone,
    LateSubmissionPolicy, PreviewDeadlineBehaviorField, PreviewGroupFact, PreviewLimitField,
    PreviewPolicySourceLayer, PreviewPriorRunCount, PreviewResolvedPolicy, PreviewSelectedMoment,
    PreviewSubjectKind, PreviewTimeField, ProblemId, ProblemVersionRef, RehearsalAttemptId,
    TenantId, VersionId,
};
use uuid::Uuid;
fn revision(v: u64) -> TeachingOperationRevision {
    TeachingOperationRevision::new(v).unwrap()
}
fn subject() -> PreviewSubject {
    let s = PreviewPolicySourceLayer::Base;
    let p = PreviewResolvedPolicy::new(
        PreviewTimeField {
            value: None,
            source: s,
        },
        PreviewTimeField {
            value: None,
            source: s,
        },
        PreviewTimeField {
            value: None,
            source: s,
        },
        PreviewLimitField {
            value: None,
            source: s,
        },
        PreviewLimitField {
            value: None,
            source: s,
        },
        question_model::PreviewLateSubmissionField {
            value: LateSubmissionPolicy::Accept,
            source: s,
        },
        PreviewDeadlineBehaviorField {
            value: AssignmentDeadlineBehavior::AutoSubmit,
            source: s,
        },
    )
    .unwrap();
    PreviewSubject::new(
        PreviewSubjectKind::Synthetic,
        AssignmentReference::new(1).unwrap(),
        revision(1),
        PreviewSelectedMoment {
            value: CourseLocalDateTime::parse("2026-08-23T09:00:00.000").unwrap(),
            time_zone: IanaTimeZone::parse("America/Chicago").unwrap(),
        },
        vec![PreviewGroupFact::from_purpose(
            question_model::CourseGroupPurpose::Lab,
        )],
        p,
        PreviewPriorRunCount::try_from(0).unwrap(),
    )
    .unwrap()
}
fn context(run: u128) -> RehearsalGenesisContext {
    let s = subject();
    RehearsalGenesisContext {
        rehearsal: RehearsalRunId::from_uuid(Uuid::from_u128(run)),
        tenant: TenantId::from_uuid(Uuid::from_u128(2)),
        course: CourseId::from_uuid(Uuid::from_u128(3)),
        assignment: AssignmentReference::new(1).unwrap(),
        direct_instructor_membership: CourseMembershipId::from_uuid(Uuid::from_u128(4)),
        revision: revision(1),
        subject_fingerprint: fingerprint_resolved_preview_subject(
            AssignmentReference::new(1).unwrap(),
            revision(1),
            &s,
        )
        .unwrap(),
    }
}
fn payload(d: u8) -> RehearsalEvidencePayload {
    RehearsalEvidencePayload::FrozenItem(RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(Uuid::from_u128(5)),
        problem: ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(6)),
            version: VersionId::from_uuid(Uuid::from_u128(7)),
        },
        response_definition: numeric_definition(),
        canonical_content_digest: RehearsalEvidenceDigest::from_bytes([d; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(8),
    })
}

fn numeric_definition() -> question_model::ResponseDefinition {
    question_model::ResponseDefinition::Numeric {
        tolerance: question_model::answer::NumericTolerance::Exact,
        unit: None,
    }
}

fn frozen_attempt(attempt: RehearsalAttemptId) -> RehearsalFrozenItemEvidence {
    RehearsalFrozenItemEvidence {
        attempt,
        problem: ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(6)),
            version: VersionId::from_uuid(Uuid::from_u128(7)),
        },
        response_definition: numeric_definition(),
        canonical_content_digest: RehearsalEvidenceDigest::from_bytes([1; 32]),
        frozen_at: ActivityTimestamp::from_unix_millis(8),
    }
}
fn entry(c: RehearsalGenesisContext, p: RehearsalEvidencePayload) -> RehearsalEvidenceChainEntry {
    let g = evidence_genesis_digest(c);
    RehearsalEvidenceChainEntry {
        record: RehearsalEvidenceRecord {
            sequence: 1,
            kind: p.kind(),
            previous_digest: Some(g),
            digest: evidence_entry_digest(
                1,
                p.kind(),
                g,
                private_payload_digest(&p),
                ActivityTimestamp::from_unix_millis(9),
            ),
            recorded_at: ActivityTimestamp::from_unix_millis(9),
        },
        payload: p,
    }
}
#[test]
fn subject_fingerprint_requires_route_binding() {
    let s = subject();
    assert_eq!(
        fingerprint_resolved_preview_subject(AssignmentReference::new(2).unwrap(), revision(1), &s),
        Err(RehearsalIntegrityError::SubjectBindingMismatch)
    );
}

#[test]
fn subject_fingerprint_has_a_stable_v1_vector() {
    let s = subject();
    assert_eq!(
        fingerprint_resolved_preview_subject(AssignmentReference::new(1).unwrap(), revision(1), &s)
            .unwrap()
            .to_hex(),
        "db28ec494b4cd571b121d8c1d7171def02f35972d80859c6b8de2a0f9043805a"
    );
}

#[test]
fn source_context_removal_has_its_own_terminal_lifecycle_mapping() {
    assert_eq!(
        apply_terminal_transition(
            RehearsalLifecycle::Active,
            RehearsalTerminalTransition::DiscardSourceContextRemoved,
        ),
        Ok(RehearsalLifecycle::DiscardedSourceContextRemoved)
    );
    assert_eq!(
        apply_terminal_transition(
            RehearsalLifecycle::DiscardedSourceContextRemoved,
            RehearsalTerminalTransition::DiscardSourceContextRemoved,
        ),
        Err(RehearsalIntegrityError::TerminalLifecycle)
    );
}
#[test]
fn rehearsal_evidence_encoders_have_stable_vectors() {
    let context = context(1);
    let frozen = payload(1);
    let accepted = submission_payload(
        RehearsalAttemptId::from_uuid(Uuid::from_u128(20)),
        question_model::StudentResponse::Numeric { value: 1.0 },
        graded_submission_result(),
        ActivityTimestamp::from_unix_millis(21),
    );
    let genesis = evidence_genesis_digest(context);
    assert_eq!(
        genesis.to_hex(),
        "2064c76a4c10a9121546ac9eb30b73a410b4e8de031c5a30bb7ff5d3f1144dd4"
    );
    assert_eq!(
        private_payload_digest(&frozen).to_hex(),
        "53bf313f68ad333cddfa56ccc7ae068a1c505748bd6bcaf342a81bd4a0873ee8"
    );
    assert_eq!(
        private_payload_digest(&accepted).to_hex(),
        "c455357b240bf62e60c6dbfd5b81185d779a678749a2f97e6892738517edc8db"
    );
    assert_eq!(
        evidence_entry_digest(
            1,
            RehearsalEvidenceKind::FrozenItem,
            genesis,
            private_payload_digest(&frozen),
            ActivityTimestamp::from_unix_millis(9),
        )
        .to_hex(),
        "f5ab605a846affdc3560385dae2c2db8448a30dc9152a3e912da17f10a207ba2"
    );
}
#[test]
fn genesis_prevents_cross_run_and_owner_replay() {
    let baseline = context(1);
    let digest = evidence_genesis_digest(baseline);
    let mut mutations = Vec::new();
    let mut changed = baseline;
    changed.rehearsal = RehearsalRunId::from_uuid(Uuid::from_u128(10));
    mutations.push(changed);
    let mut changed = baseline;
    changed.tenant = TenantId::from_uuid(Uuid::from_u128(11));
    mutations.push(changed);
    let mut changed = baseline;
    changed.course = CourseId::from_uuid(Uuid::from_u128(12));
    mutations.push(changed);
    let mut changed = baseline;
    changed.assignment = AssignmentReference::new(2).unwrap();
    mutations.push(changed);
    let mut changed = baseline;
    changed.direct_instructor_membership = CourseMembershipId::from_uuid(Uuid::from_u128(99));
    mutations.push(changed);
    let mut changed = baseline;
    changed.revision = revision(2);
    mutations.push(changed);
    let mut changed = baseline;
    changed.subject_fingerprint = RehearsalSubjectFingerprint([13; 32]);
    mutations.push(changed);
    for changed in mutations {
        assert_ne!(digest, evidence_genesis_digest(changed));
    }
}
#[test]
fn payload_mutation_and_chain_damage_fail_closed() {
    let c = context(1);
    let valid = entry(c, payload(1));
    let head = RehearsalEvidenceHead::from_persisted(valid.record.digest, 1);
    assert!(verify_evidence_chain(c, head, std::slice::from_ref(&valid)).is_ok());
    let swapped = RehearsalEvidenceChainEntry {
        record: valid.record.clone(),
        payload: payload(2),
    };
    assert_eq!(
        verify_evidence_chain(c, head, &[swapped]),
        Err(RehearsalIntegrityError::DigestMismatch)
    );
    let mut gap = valid;
    gap.record.previous_digest = None;
    assert_eq!(
        verify_evidence_chain(c, head, &[gap]),
        Err(RehearsalIntegrityError::PreviousDigestMismatch)
    );
    let mut altered_timestamp = entry(c, payload(1));
    altered_timestamp.record.recorded_at = ActivityTimestamp::from_unix_millis(10);
    assert_eq!(
        verify_evidence_chain(c, head, &[altered_timestamp]),
        Err(RehearsalIntegrityError::DigestMismatch)
    );
    let mut altered_sequence = entry(c, payload(1));
    altered_sequence.record.sequence = 2;
    assert_eq!(
        verify_evidence_chain(c, head, &[altered_sequence]),
        Err(RehearsalIntegrityError::SequenceGap)
    );
    let mut altered_kind = entry(c, payload(1));
    altered_kind.record.kind = RehearsalEvidenceKind::AcceptedSubmission;
    assert_eq!(
        verify_evidence_chain(c, head, &[altered_kind]),
        Err(RehearsalIntegrityError::InvalidEvidenceKind)
    );
    let mut altered_digest = entry(c, payload(1));
    altered_digest.record.digest = RehearsalEvidenceDigest::from_bytes([99; 32]);
    assert_eq!(
        verify_evidence_chain(c, head, &[altered_digest]),
        Err(RehearsalIntegrityError::DigestMismatch)
    );
}

#[test]
fn evidence_head_binds_empty_and_nonempty_chain_terminals() {
    let c = context(1);
    let genesis = evidence_genesis_head(c);
    assert!(verify_evidence_chain(c, genesis, &[]).is_ok());
    assert_eq!(
        verify_evidence_chain(
            c,
            RehearsalEvidenceHead::from_persisted(RehearsalEvidenceDigest::from_bytes([8; 32]), 0),
            &[],
        ),
        Err(RehearsalIntegrityError::HeadMismatch)
    );
    let valid = entry(c, payload(1));
    let head = RehearsalEvidenceHead::from_persisted(valid.record.digest, 1);
    assert!(verify_evidence_chain(c, head, std::slice::from_ref(&valid)).is_ok());
    assert_eq!(
        verify_evidence_chain(
            c,
            RehearsalEvidenceHead::from_persisted(RehearsalEvidenceDigest::from_bytes([9; 32]), 1),
            std::slice::from_ref(&valid),
        ),
        Err(RehearsalIntegrityError::HeadMismatch)
    );
    assert_eq!(
        verify_evidence_chain(
            c,
            RehearsalEvidenceHead::from_persisted(valid.record.digest, 2),
            std::slice::from_ref(&valid),
        ),
        Err(RehearsalIntegrityError::HeadMismatch)
    );
}
#[test]
fn frozen_payload_digest_covers_each_persisted_field() {
    let baseline = payload(1);
    let digest = private_payload_digest(&baseline);
    let RehearsalEvidencePayload::FrozenItem(value) = baseline else {
        unreachable!()
    };
    let mutations = [
        RehearsalFrozenItemEvidence {
            attempt: RehearsalAttemptId::from_uuid(Uuid::from_u128(50)),
            ..value.clone()
        },
        RehearsalFrozenItemEvidence {
            problem: ProblemVersionRef {
                problem: ProblemId::from_uuid(Uuid::from_u128(60)),
                ..value.problem
            },
            ..value.clone()
        },
        RehearsalFrozenItemEvidence {
            problem: ProblemVersionRef {
                version: VersionId::from_uuid(Uuid::from_u128(70)),
                ..value.problem
            },
            ..value.clone()
        },
        RehearsalFrozenItemEvidence {
            response_definition: question_model::ResponseDefinition::ShortText {
                match_mode: question_model::answer::TextMatchMode::Exact,
                max_length: 1,
            },
            ..value.clone()
        },
        RehearsalFrozenItemEvidence {
            canonical_content_digest: RehearsalEvidenceDigest::from_bytes([2; 32]),
            ..value.clone()
        },
        RehearsalFrozenItemEvidence {
            frozen_at: ActivityTimestamp::from_unix_millis(80),
            ..value
        },
    ];
    for mutation in mutations {
        assert_ne!(
            digest,
            private_payload_digest(&RehearsalEvidencePayload::FrozenItem(mutation))
        );
    }
}

fn graded_submission_result() -> RehearsalPrivateGradingResult {
    RehearsalPrivateGradingResult::Graded {
        result: question_model::AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 2.0,
        },
        feedback: DisclosedFeedback {
            correctness: Some(true),
            points_earned: Some(1.0),
            points_possible: Some(2.0),
            hint: Some(vec![ContentBlock::Text {
                markdown: "hint".into(),
            }]),
            correct_response: Some(vec![ContentBlock::Text {
                markdown: "correct".into(),
            }]),
            rationale: Some(vec![ContentBlock::Text {
                markdown: "rationale".into(),
            }]),
        },
        backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
            "native-1".into(),
        )
        .unwrap(),
    }
}

fn submission_payload(
    attempt: RehearsalAttemptId,
    response: question_model::StudentResponse,
    result: RehearsalPrivateGradingResult,
    accepted_at: ActivityTimestamp,
) -> RehearsalEvidencePayload {
    let frozen = frozen_attempt(attempt);
    let request =
        RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(&frozen, attempt, response)
            .unwrap();
    let root = claim_root_for_test(request.clone());
    RehearsalEvidencePayload::AcceptedSubmission(
        RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
            &root,
            request,
            &frozen,
            result,
            accepted_at,
        )
        .unwrap(),
    )
}

fn validated_request(
    attempt: RehearsalAttemptId,
    response: question_model::StudentResponse,
) -> RehearsalValidatedSubmissionRequest {
    RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        &frozen_attempt(attempt),
        attempt,
        response,
    )
    .unwrap()
}

fn claim_root_for_test(request: RehearsalValidatedSubmissionRequest) -> RehearsalClaimRoot {
    let context = context(700);
    let frozen = frozen_attempt(request.attempt());
    let fingerprint = rehearsal_submission_request_fingerprint(context, &frozen, &request).unwrap();
    RehearsalClaimRoot::verify_persisted(
        context,
        &frozen,
        RehearsalPersistedClaimRoot::from_persisted(
            context.rehearsal,
            question_model::RehearsalSubmissionClaimId::from_uuid(Uuid::from_u128(701)),
            fingerprint,
            request,
        ),
    )
    .unwrap()
}

#[test]
fn submission_request_fingerprint_has_a_fixed_v1_vector() {
    let context = context(1);
    let frozen = frozen_attempt(RehearsalAttemptId::from_uuid(Uuid::from_u128(20)));
    let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        &frozen,
        frozen.attempt,
        question_model::StudentResponse::Numeric { value: 1.0 },
    )
    .unwrap();
    assert_eq!(
        rehearsal_submission_request_fingerprint(context, &frozen, &request)
            .unwrap()
            .to_hex(),
        "95d86b99275021b6db9c8de3ec585e4c2d46714b03a91a5bccf91dddd8c06944"
    );
}

#[test]
fn sealed_submission_request_refuses_same_attempt_frozen_record_substitution() {
    let frozen_a = frozen_attempt(RehearsalAttemptId::from_uuid(Uuid::from_u128(20)));
    let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        &frozen_a,
        frozen_a.attempt,
        question_model::StudentResponse::Numeric { value: 1.0 },
    )
    .unwrap();
    let frozen_b = RehearsalFrozenItemEvidence {
        problem: ProblemVersionRef {
            version: VersionId::from_uuid(Uuid::from_u128(99)),
            ..frozen_a.problem
        },
        ..frozen_a.clone()
    };
    let problem_b = RehearsalFrozenItemEvidence {
        problem: ProblemVersionRef {
            problem: ProblemId::from_uuid(Uuid::from_u128(98)),
            ..frozen_a.problem
        },
        ..frozen_a.clone()
    };
    for mutated in [
        frozen_b,
        problem_b,
        RehearsalFrozenItemEvidence {
            canonical_content_digest: RehearsalEvidenceDigest::from_bytes([7; 32]),
            ..frozen_a.clone()
        },
        RehearsalFrozenItemEvidence {
            response_definition: question_model::ResponseDefinition::Numeric {
                tolerance: question_model::answer::NumericTolerance::Absolute { epsilon: 0.1 },
                unit: None,
            },
            ..frozen_a.clone()
        },
    ] {
        assert_eq!(
            request.validate_frozen_attempt(&mutated),
            Err(RehearsalEvidenceValidationError::ResponseDefinitionMismatch)
        );
        assert_eq!(
            rehearsal_submission_request_fingerprint(context(1), &mutated, &request),
            Err(RehearsalEvidenceValidationError::ResponseDefinitionMismatch)
        );
        assert!(matches!(
            RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
                &claim_root_for_test(request.clone()),
                request.clone(),
                &mutated,
                graded_submission_result(),
                ActivityTimestamp::from_unix_millis(30),
            ),
            Err(RehearsalEvidenceValidationError::ResponseDefinitionMismatch)
        ));
    }
    assert!(
        RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
            &claim_root_for_test(request.clone()),
            request,
            &frozen_a,
            graded_submission_result(),
            ActivityTimestamp::from_unix_millis(30),
        )
        .is_ok()
    );
}

#[test]
fn submission_request_fingerprint_covers_every_input_and_excludes_acceptance_event_fields() {
    let base_context = context(1);
    let base_frozen = frozen_attempt(RehearsalAttemptId::from_uuid(Uuid::from_u128(20)));
    let base_request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        &base_frozen,
        base_frozen.attempt,
        question_model::StudentResponse::Numeric { value: 1.0 },
    )
    .unwrap();
    let baseline =
        rehearsal_submission_request_fingerprint(base_context, &base_frozen, &base_request)
            .unwrap();

    let mut contexts = Vec::new();
    let mut changed = base_context;
    changed.rehearsal = RehearsalRunId::from_uuid(Uuid::from_u128(101));
    contexts.push(changed);
    let mut changed = base_context;
    changed.tenant = TenantId::from_uuid(Uuid::from_u128(102));
    contexts.push(changed);
    let mut changed = base_context;
    changed.course = CourseId::from_uuid(Uuid::from_u128(103));
    contexts.push(changed);
    let mut changed = base_context;
    changed.assignment = AssignmentReference::new(2).unwrap();
    contexts.push(changed);
    let mut changed = base_context;
    changed.direct_instructor_membership = CourseMembershipId::from_uuid(Uuid::from_u128(104));
    contexts.push(changed);
    let mut changed = base_context;
    changed.revision = revision(2);
    contexts.push(changed);
    let mut changed = base_context;
    changed.subject_fingerprint = RehearsalSubjectFingerprint([105; 32]);
    contexts.push(changed);
    for changed in contexts {
        assert_ne!(
            baseline,
            rehearsal_submission_request_fingerprint(changed, &base_frozen, &base_request).unwrap()
        );
    }

    let frozen_mutations = [
        RehearsalFrozenItemEvidence {
            attempt: RehearsalAttemptId::from_uuid(Uuid::from_u128(21)),
            ..base_frozen.clone()
        },
        RehearsalFrozenItemEvidence {
            problem: ProblemVersionRef {
                problem: ProblemId::from_uuid(Uuid::from_u128(22)),
                ..base_frozen.problem
            },
            ..base_frozen.clone()
        },
        RehearsalFrozenItemEvidence {
            problem: ProblemVersionRef {
                version: VersionId::from_uuid(Uuid::from_u128(23)),
                ..base_frozen.problem
            },
            ..base_frozen.clone()
        },
        RehearsalFrozenItemEvidence {
            canonical_content_digest: RehearsalEvidenceDigest::from_bytes([24; 32]),
            ..base_frozen.clone()
        },
        RehearsalFrozenItemEvidence {
            response_definition: question_model::ResponseDefinition::Numeric {
                tolerance: question_model::answer::NumericTolerance::Absolute { epsilon: 0.1 },
                unit: None,
            },
            ..base_frozen.clone()
        },
    ];
    for frozen in frozen_mutations {
        let request = RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
            &frozen,
            frozen.attempt,
            question_model::StudentResponse::Numeric { value: 1.0 },
        )
        .unwrap();
        assert_ne!(
            baseline,
            rehearsal_submission_request_fingerprint(base_context, &frozen, &request).unwrap()
        );
    }

    let changed_response = validated_request(
        base_frozen.attempt,
        question_model::StudentResponse::Numeric { value: 2.0 },
    );
    assert_ne!(
        baseline,
        rehearsal_submission_request_fingerprint(base_context, &base_frozen, &changed_response)
            .unwrap()
    );

    let accepted_one = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        &claim_root_for_test(base_request.clone()),
        base_request.clone(),
        &base_frozen,
        graded_submission_result(),
        ActivityTimestamp::from_unix_millis(30),
    )
    .unwrap();
    let accepted_two = RehearsalValidatedSubmissionEvidence::try_complete_with_frozen_attempt(
        &claim_root_for_test(base_request.clone()),
        base_request.clone(),
        &base_frozen,
        graded_submission_result(),
        ActivityTimestamp::from_unix_millis(31),
    )
    .unwrap();
    assert_ne!(
        private_payload_digest(&RehearsalEvidencePayload::AcceptedSubmission(accepted_one)),
        private_payload_digest(&RehearsalEvidencePayload::AcceptedSubmission(accepted_two))
    );
    assert_eq!(
        baseline,
        rehearsal_submission_request_fingerprint(base_context, &base_frozen, &base_request)
            .unwrap()
    );
}

#[test]
fn accepted_submission_digest_covers_response_grading_receipt_and_timestamp() {
    let attempt = RehearsalAttemptId::from_uuid(Uuid::from_u128(20));
    let response = question_model::StudentResponse::Numeric { value: 1.0 };
    let result = graded_submission_result();
    let accepted_at = ActivityTimestamp::from_unix_millis(21);
    let baseline = submission_payload(attempt, response.clone(), result.clone(), accepted_at);
    let digest = private_payload_digest(&baseline);
    let mut mutations = vec![
        submission_payload(
            RehearsalAttemptId::from_uuid(Uuid::from_u128(22)),
            response.clone(),
            result.clone(),
            accepted_at,
        ),
        submission_payload(
            attempt,
            question_model::StudentResponse::Numeric { value: 2.0 },
            result.clone(),
            accepted_at,
        ),
        submission_payload(
            attempt,
            response.clone(),
            result.clone(),
            ActivityTimestamp::from_unix_millis(23),
        ),
    ];
    let RehearsalPrivateGradingResult::Graded {
        result: baseline_result,
        feedback: baseline_feedback,
        backend_receipt_reference: baseline_receipt,
    } = result;
    let mut changed_result = baseline_result;
    changed_result.correct = false;
    mutations.push(submission_payload(
        attempt,
        response.clone(),
        RehearsalPrivateGradingResult::Graded {
            result: changed_result,
            feedback: baseline_feedback.clone(),
            backend_receipt_reference: baseline_receipt.clone(),
        },
        accepted_at,
    ));
    for changed_value in [0.5, 3.0] {
        let mut changed_result = baseline_result;
        changed_result.points_earned = changed_value;
        mutations.push(submission_payload(
            attempt,
            response.clone(),
            RehearsalPrivateGradingResult::Graded {
                result: changed_result,
                feedback: baseline_feedback.clone(),
                backend_receipt_reference: baseline_receipt.clone(),
            },
            accepted_at,
        ));
    }
    let mut changed_result = baseline_result;
    changed_result.points_possible = 3.0;
    mutations.push(submission_payload(
        attempt,
        response.clone(),
        RehearsalPrivateGradingResult::Graded {
            result: changed_result,
            feedback: baseline_feedback.clone(),
            backend_receipt_reference: baseline_receipt.clone(),
        },
        accepted_at,
    ));
    let mut feedback_mutations = Vec::new();
    let mut changed = baseline_feedback.clone();
    changed.correctness = Some(false);
    feedback_mutations.push(changed);
    let mut changed = baseline_feedback.clone();
    changed.points_earned = Some(0.5);
    feedback_mutations.push(changed);
    let mut changed = baseline_feedback.clone();
    changed.points_possible = Some(3.0);
    feedback_mutations.push(changed);
    let mut changed = baseline_feedback.clone();
    changed.hint = Some(vec![ContentBlock::Text {
        markdown: "changed".into(),
    }]);
    feedback_mutations.push(changed);
    let mut changed = baseline_feedback.clone();
    changed.correct_response = Some(vec![ContentBlock::Text {
        markdown: "changed".into(),
    }]);
    feedback_mutations.push(changed);
    let mut changed = baseline_feedback.clone();
    changed.rationale = Some(vec![ContentBlock::Text {
        markdown: "changed".into(),
    }]);
    feedback_mutations.push(changed);
    for feedback in feedback_mutations {
        mutations.push(submission_payload(
            attempt,
            response.clone(),
            RehearsalPrivateGradingResult::Graded {
                result: baseline_result,
                feedback,
                backend_receipt_reference: baseline_receipt.clone(),
            },
            accepted_at,
        ));
    }
    mutations.push(submission_payload(
        attempt,
        response,
        RehearsalPrivateGradingResult::Graded {
            result: baseline_result,
            feedback: baseline_feedback,
            backend_receipt_reference: question_model::RehearsalBackendReceiptReference::new(
                "native-2".into(),
            )
            .unwrap(),
        },
        accepted_at,
    ));
    for mutation in mutations {
        assert_ne!(digest, private_payload_digest(&mutation));
    }
}

#[test]
fn submission_is_bound_to_the_store_retrieved_frozen_attempt_and_schema() {
    let actual = RehearsalAttemptId::from_uuid(Uuid::from_u128(30));
    let unrelated = RehearsalAttemptId::from_uuid(Uuid::from_u128(31));
    let frozen = RehearsalFrozenItemEvidence {
        response_definition: question_model::ResponseDefinition::ShortText {
            match_mode: question_model::answer::TextMatchMode::Exact,
            max_length: 1,
        },
        ..frozen_attempt(actual)
    };
    let permissive = question_model::ResponseDefinition::ShortText {
        match_mode: question_model::answer::TextMatchMode::Exact,
        max_length: 100,
    };
    let response = question_model::StudentResponse::ShortText {
        text: "valid-only-there".into(),
    };
    assert!(validate_response_format(&permissive, &response).is_valid());
    assert!(matches!(
        RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
            &frozen,
            actual,
            response.clone(),
        ),
        Err(RehearsalEvidenceValidationError::InvalidResponseShape)
    ));
    assert!(matches!(
        RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(&frozen, unrelated, response,),
        Err(RehearsalEvidenceValidationError::ResponseDefinitionMismatch)
    ));
}
