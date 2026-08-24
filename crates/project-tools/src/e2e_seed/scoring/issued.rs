//! Shared immutable issue evidence for the native scoring seed.

use super::*;

pub(super) struct NativeIssueRequest {
    pub(super) ids: SeedIds,
    pub(super) actor: UserId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) run: RunId,
    pub(super) seed: u64,
    pub(super) presentation_binding: PresentationBindingV1,
    pub(super) presentation: learning_data_access::ReceiptPresentationSnapshot,
    pub(super) grading_envelope: QuestionEnvelope,
    pub(super) parameter_hash: String,
    pub(super) provenance: AttemptProvenance,
}

pub(super) fn native_issue_command(
    request: NativeIssueRequest,
) -> Result<IssueQuestionAttemptCommand> {
    let NativeIssueRequest {
        ids,
        actor,
        attempt,
        run,
        seed,
        presentation_binding,
        presentation,
        grading_envelope,
        parameter_hash,
        provenance,
    } = request;
    let question = question_model::QuestionDefinition::from_draft(
        replica_native_draft(ids.workspace),
        ids.problem,
        ids.version,
        QuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        },
    );
    let snapshot = learning_data_access::IssuedQuestionSnapshotV1::new(
        question,
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )?;
    snapshot.validate_for_attempt(ids.problem, ids.version)?;
    snapshot.validate_for_issuance_context(
        learning_data_access::FlatGradingCapability::NotApplicable,
        learning_data_access::WebworkGradingCapability::NotApplicable,
        Some(&presentation),
    )?;
    Ok(IssueQuestionAttemptCommand {
        actor,
        binding: learning_data_access::LearnerWorkRoutingBinding::new(ids.course, ids.assignment),
        attempt,
        run,
        assignment_position: 0,
        problem: ids.problem,
        question_version: ids.version,
        issued_question_snapshot: snapshot,
        seed,
        presentation_capability: PresentationCapability::EnvelopeV1,
        presentation: Some(presentation_binding),
        presentation_snapshot: Some(presentation),
        grading_envelope: Some(grading_envelope),
        flat_grading: None,
        flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
        webwork_grading: None,
        webwork_grading_capability: learning_data_access::WebworkGradingCapability::NotApplicable,
        webwork_replay: None,
        parameter_hash,
        provenance,
        prefetched: None,
        predecessor_submission: None,
    })
}
