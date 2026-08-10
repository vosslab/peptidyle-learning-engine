use question_model::QuestionSource;
use question_model::capability::BackendCapabilities;

use crate::{NativeAdapter, NativeAdapterError};

impl NativeAdapter {
    /// Returns conservative catalog capabilities for a native source family.
    pub fn capabilities(
        &self,
        source: &QuestionSource,
    ) -> Result<BackendCapabilities, NativeAdapterError> {
        let families = self.families_for_source(source)?;
        let mut capabilities = families
            .first()
            .expect("a nonempty registry selection has a first family")
            .capabilities();
        for family in &families[1..] {
            capabilities = BackendCapabilities::from_iter(
                capabilities
                    .declared()
                    .filter(|capability| family.capabilities().supports(*capability)),
            );
        }
        Ok(capabilities)
    }
}
