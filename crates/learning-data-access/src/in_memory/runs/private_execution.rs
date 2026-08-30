//! Sealed private-execution preparation for in-memory grading parity.

use async_trait::async_trait;

use super::super::MemorySealedPrivateExecutionStore;
use super::issued_contracts::{
    load_issued_flat_grading, load_issued_qti_grading, load_issued_webwork_grading,
};
use super::submission_preparation;
use crate::{ActorContext, StoreError, StudentWorkRoutingBinding, SubmissionIdempotencyKey};
use question_model::StudentResponse;

#[async_trait]
impl crate::SealedPrivateExecutionStore for MemorySealedPrivateExecutionStore {
    async fn prepare_sealed_private_execution(
        &self,
        context: ActorContext,
        actor: question_model::UserId,
        binding: StudentWorkRoutingBinding,
        intent: crate::AuthorizedSubmissionIntent,
        response: &StudentResponse,
        idempotency_key: &SubmissionIdempotencyKey,
    ) -> Result<crate::SealedPrivateExecutionPreparation, StoreError> {
        let state = self.state.read().map_err(|_| {
            StoreError::Unavailable("sealed private execution state is unavailable".to_string())
        })?;
        match submission_preparation::prepare_question_submission(
            &state,
            context,
            actor,
            binding,
            intent.attempt.id,
            response,
            idempotency_key,
        )? {
            crate::SubmissionPreparation::Replay(record) => {
                Ok(crate::SealedPrivateExecutionPreparation::Replay(record))
            }
            crate::SubmissionPreparation::AcceptedPending(_) => Err(StoreError::Conflict),
            crate::SubmissionPreparation::FirstEffect(prepared_intent) => {
                if *prepared_intent != intent {
                    return Err(StoreError::Unavailable(
                        "sealed preparation disagrees with its authorized intent".to_string(),
                    ));
                }
                let flat_grading = load_issued_flat_grading(&state, &intent.attempt)?;
                let webwork_grading = load_issued_webwork_grading(&state, &intent.attempt)?;
                let issued_qti_grading = load_issued_qti_grading(
                    &state,
                    &intent.attempt,
                    &intent.issued_question_snapshot,
                )?;
                let webwork_replay = state.webwork_grade_replay.get(&intent.attempt.id).cloned();
                crate::validate_issued_flat_grading(
                    intent.issued_question_snapshot.question(),
                    if intent.presentation.is_some() {
                        crate::PresentationCapability::EnvelopeV1
                    } else {
                        crate::PresentationCapability::NotApplicable
                    },
                    if matches!(
                        intent.attempt.issued_capability,
                        question_model::IssuedAttemptCapabilityV1::FlatPresentation
                    ) {
                        crate::FlatGradingCapability::Required
                    } else {
                        crate::FlatGradingCapability::NotApplicable
                    },
                    flat_grading.as_ref(),
                )?;
                crate::validate_issued_webwork_grading(
                    intent.issued_question_snapshot.question(),
                    if matches!(
                        intent.attempt.issued_capability,
                        question_model::IssuedAttemptCapabilityV1::WebworkPresentation
                    ) {
                        crate::WebworkGradingCapability::Required
                    } else {
                        crate::WebworkGradingCapability::NotApplicable
                    },
                    webwork_grading.as_ref(),
                )?;
                crate::validate_issued_qti_grading(
                    intent.issued_question_snapshot.question(),
                    if matches!(
                        intent.attempt.issued_capability,
                        question_model::IssuedAttemptCapabilityV1::QtiPresentation
                    ) {
                        crate::QtiGradingCapability::Required
                    } else {
                        crate::QtiGradingCapability::NotApplicable
                    },
                    issued_qti_grading.as_ref(),
                )?;
                Ok(crate::SealedPrivateExecutionPreparation::Grade(Box::new(
                    crate::PreparedQuestionSubmission {
                        attempt: intent.attempt,
                        issued_question_snapshot: intent.issued_question_snapshot,
                        presentation_binding: intent.presentation_binding,
                        presentation: intent.presentation,
                        grading_envelope: intent.grading_envelope,
                        flat_grading,
                        webwork_grading,
                        issued_qti_grading,
                        webwork_replay,
                    },
                )))
            }
        }
    }
}
