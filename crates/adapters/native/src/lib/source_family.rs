use std::sync::Arc;

use question_model::generation::GeneratorReference;
use question_model::{DraftQuestionSource, QuestionSource};

use crate::generator::NativeQuestionFamily;
use crate::registry::FamilyRegistrationKey;
use crate::{NativeAdapter, NativeAdapterError};

impl NativeAdapter {
    pub(super) fn families_for_source(
        &self,
        source: &QuestionSource,
    ) -> Result<Vec<&dyn NativeQuestionFamily>, NativeAdapterError> {
        let QuestionSource::Native { family } = source else {
            return Err(NativeAdapterError::UnsupportedSource);
        };
        let families: Vec<_> = self
            .families
            .iter()
            .filter(|(key, _)| key.family == *family)
            .map(|(_, registered)| Arc::as_ref(registered))
            .collect();
        if families.is_empty() {
            Err(NativeAdapterError::UnknownFamily(family.clone()))
        } else {
            Ok(families)
        }
    }

    pub(super) fn family_for_generated_source(
        &self,
        source: &QuestionSource,
        generator: Option<&GeneratorReference>,
    ) -> Result<&dyn NativeQuestionFamily, NativeAdapterError> {
        let QuestionSource::Native { family } = source else {
            return Err(NativeAdapterError::UnsupportedSource);
        };
        let key = FamilyRegistrationKey {
            family: family.clone(),
            generator: generator.cloned(),
        };
        self.families.get(&key).map(Arc::as_ref).ok_or_else(|| {
            NativeAdapterError::UnknownGenerator {
                family: family.clone(),
                generator: generator.cloned(),
            }
        })
    }

    pub(super) fn family_for_draft_source(
        &self,
        source: &DraftQuestionSource,
        generator: Option<&GeneratorReference>,
    ) -> Result<&dyn NativeQuestionFamily, NativeAdapterError> {
        let DraftQuestionSource::Native { family } = source else {
            return Err(NativeAdapterError::UnsupportedSource);
        };
        let key = FamilyRegistrationKey {
            family: family.clone(),
            generator: generator.cloned(),
        };
        self.families.get(&key).map(Arc::as_ref).ok_or_else(|| {
            NativeAdapterError::UnknownGenerator {
                family: family.clone(),
                generator: generator.cloned(),
            }
        })
    }
}
