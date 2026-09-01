use question_model::QuestionRevision;
use question_model::capability::QuestionBackendCapabilities;

use crate::{PleQuestionBackend, PleQuestionBackendError};

impl PleQuestionBackend {
    /// Returns conservative capabilities for a PLE Question contract.
    pub fn capabilities(
        &self,
        question: &QuestionRevision,
    ) -> Result<QuestionBackendCapabilities, PleQuestionBackendError> {
        let implementations = self.implementations_for_question(question)?;
        let mut capabilities = implementations
            .first()
            .expect("a nonempty registry selection has a first implementation")
            .capabilities();
        for implementation in &implementations[1..] {
            capabilities = QuestionBackendCapabilities::from_iter(
                capabilities
                    .declared()
                    .filter(|capability| implementation.capabilities().supports(*capability)),
            );
        }
        Ok(capabilities)
    }
}
