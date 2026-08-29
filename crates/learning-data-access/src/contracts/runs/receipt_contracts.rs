//! Focused completed-submission receipt contracts.

use super::*;

#[derive(Clone, PartialEq)]
pub struct CompletedSubmissionReceipt {
    pub attempt: QuestionAttempt,
    pub feedback: AttemptFeedbackRecord,
    pub run: AssignmentRun,
    pub summary: StudentAssignmentSummary,
    pub presentation: Option<ReceiptPresentationSnapshot>,
}

impl CompletedSubmissionReceipt {
    pub fn into_submission_record(self, disclosure: StudentDisclosureInput) -> SubmissionRecord {
        SubmissionRecord {
            attempt: self.attempt,
            run: self.run,
            summary: self.summary,
            feedback: self.feedback,
            presentation: self.presentation,
            disclosure,
        }
    }
}

impl std::fmt::Debug for CompletedSubmissionReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompletedSubmissionReceipt")
            .field("attempt", &"[ANSWER-FREE RECEIPT]")
            .field("run", &"[SERVER-ONLY RECEIPT]")
            .field("summary", &"[SERVER-ONLY RECEIPT]")
            .field("feedback", &"[SERVER-ONLY]")
            .field(
                "presentation",
                &self.presentation.as_ref().map(|_| "[ANSWER-FREE]"),
            )
            .finish()
    }
}
