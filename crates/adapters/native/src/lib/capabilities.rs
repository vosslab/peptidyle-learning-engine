use question_model::QuestionVersion;
use question_model::capability::QuestionBackendCapabilities;

use crate::{NativeAdapter, NativeAdapterError};

impl NativeAdapter {
    /// Returns conservative capabilities for a native Question contract.
    pub fn capabilities(
        &self,
        question: &QuestionVersion,
    ) -> Result<QuestionBackendCapabilities, NativeAdapterError> {
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
