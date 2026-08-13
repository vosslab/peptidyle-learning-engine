use question_model::answer::NumericTolerance;
use question_model::envelope::{ContentBlock, QuestionEnvelope};
use question_model::{PresentationBindingV1, QuestionAttempt, ResponseDefinition, VersionId};

struct ReceiptNonce([u8; 16]);

impl question_model::presentation::NonceSourceV1 for ReceiptNonce {
    fn next_nonce(
        &mut self,
    ) -> Result<[u8; 16], question_model::presentation::PresentationBuildError> {
        Ok(self.0)
    }
}

pub(crate) fn receipt_presentation(
    version: VersionId,
    seed: u64,
    marker: u8,
) -> (
    PresentationBindingV1,
    learning_data_access::ReceiptPresentationSnapshot,
) {
    let mut nonce = ReceiptNonce([marker; 16]);
    let presentation = question_model::presentation::build_presentation_v1_with_nonce_source(
        &QuestionEnvelope {
            version,
            seed: question_model::generation::Seed::new(seed),
            title: "Molar mass".to_string(),
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molar mass?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
        },
        &[],
        &mut nonce,
    )
    .expect("receipt fixture presentation");
    (
        PresentationBindingV1::new(
            presentation.envelope.presentation_nonce,
            presentation.digest,
        ),
        learning_data_access::ReceiptPresentationSnapshot {
            envelope: presentation.envelope,
            asset_bindings: presentation.asset_bindings,
        },
    )
}

pub(crate) fn grading_envelope(version: VersionId, seed: u64) -> QuestionEnvelope {
    QuestionEnvelope {
        version,
        seed: question_model::generation::Seed::new(seed),
        title: "Molar mass".to_string(),
        prompt: vec![ContentBlock::Text {
            markdown: "What is the molar mass?".to_string(),
        }],
        response: ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Relative { fraction: 0.01 },
            unit: Some("g/mol".to_string()),
        },
    }
}

pub(crate) fn receipt_next_attempt(
    attempt: &QuestionAttempt,
) -> learning_data_access::ReceiptNextAttempt {
    learning_data_access::ReceiptNextAttempt {
        id: attempt.id,
        run: attempt.run,
        question_version: attempt.question_version,
        seed: attempt.seed,
        deadline: attempt.timer.deadline,
        assignment_position: attempt.assignment_position,
        rendered_question_sha256: attempt.provenance.rendered_question_sha256.clone(),
    }
}
