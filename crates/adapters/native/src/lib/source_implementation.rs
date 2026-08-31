use std::sync::Arc;

use question_model::generation::GeneratorReference;
use question_model::{
    DraftQuestionDefinition, DraftQuestionSource, QuestionDefinition, QuestionSource,
};

use crate::generator::NativeQuestionImplementation;
use crate::{NativeAdapter, NativeAdapterError};

impl NativeAdapter {
    pub(super) fn implementations_for_question(
        &self,
        question: &QuestionDefinition,
    ) -> Result<Vec<&dyn NativeQuestionImplementation>, NativeAdapterError> {
        if !matches!(question.source, QuestionSource::Native) {
            return Err(NativeAdapterError::UnsupportedSource);
        }
        let implementations: Vec<_> = self
            .implementations
            .iter()
            .filter(|(key, _)| {
                key.question_format == question.question_format
                    && key.question_type == question.question_type
            })
            .map(|(_, registered)| Arc::as_ref(registered))
            .collect();
        if implementations.is_empty() {
            Err(self.unknown_implementation(question.question_format, question.question_type, None))
        } else {
            Ok(implementations)
        }
    }

    pub(super) fn implementation_for_question(
        &self,
        question: &QuestionDefinition,
        generator: Option<&GeneratorReference>,
    ) -> Result<&dyn NativeQuestionImplementation, NativeAdapterError> {
        if !matches!(question.source, QuestionSource::Native) {
            return Err(NativeAdapterError::UnsupportedSource);
        }
        self.implementations
            .iter()
            .find(|(key, _)| {
                key.question_format == question.question_format
                    && key.question_type == question.question_type
                    && key.generator.as_ref() == generator
            })
            .map(|(_, registered)| Arc::as_ref(registered))
            .ok_or_else(|| {
                self.unknown_implementation(
                    question.question_format,
                    question.question_type,
                    generator,
                )
            })
    }

    pub(super) fn implementation_for_draft(
        &self,
        question: &DraftQuestionDefinition,
        generator: Option<&GeneratorReference>,
    ) -> Result<&dyn NativeQuestionImplementation, NativeAdapterError> {
        if !matches!(question.source, DraftQuestionSource::Native) {
            return Err(NativeAdapterError::UnsupportedSource);
        }
        self.implementations
            .iter()
            .find(|(key, _)| {
                key.question_format == question.question_format
                    && key.question_type == question.question_type
                    && key.generator.as_ref() == generator
            })
            .map(|(_, registered)| Arc::as_ref(registered))
            .ok_or_else(|| {
                self.unknown_implementation(
                    question.question_format,
                    question.question_type,
                    generator,
                )
            })
    }

    fn unknown_implementation(
        &self,
        question_format: question_model::QuestionFormat,
        question_type: question_model::QuestionType,
        generator: Option<&GeneratorReference>,
    ) -> NativeAdapterError {
        NativeAdapterError::UnknownQuestionImplementation {
            question_format,
            question_type,
            generator: generator.cloned(),
        }
    }
}
