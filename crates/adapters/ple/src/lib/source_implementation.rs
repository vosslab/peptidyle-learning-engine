use std::sync::Arc;

use question_model::generation::QuestionGeneratorReference;
use question_model::{
    DraftQuestionBackendLocator, DraftQuestionRevision, QuestionBackendLocator, QuestionRevision,
};

use crate::generator::PleQuestionImplementation;
use crate::{PleQuestionBackend, PleQuestionBackendError};

impl PleQuestionBackend {
    pub(super) fn implementations_for_question(
        &self,
        question: &QuestionRevision,
    ) -> Result<Vec<&dyn PleQuestionImplementation>, PleQuestionBackendError> {
        if !matches!(question.backend_locator, QuestionBackendLocator::Ple) {
            return Err(PleQuestionBackendError::UnsupportedSource);
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
        question: &QuestionRevision,
        generator: Option<&QuestionGeneratorReference>,
    ) -> Result<&dyn PleQuestionImplementation, PleQuestionBackendError> {
        if !matches!(question.backend_locator, QuestionBackendLocator::Ple) {
            return Err(PleQuestionBackendError::UnsupportedSource);
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
        question: &DraftQuestionRevision,
        generator: Option<&QuestionGeneratorReference>,
    ) -> Result<&dyn PleQuestionImplementation, PleQuestionBackendError> {
        if !matches!(question.backend_locator, DraftQuestionBackendLocator::Ple) {
            return Err(PleQuestionBackendError::UnsupportedSource);
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
        generator: Option<&QuestionGeneratorReference>,
    ) -> PleQuestionBackendError {
        PleQuestionBackendError::UnknownQuestionImplementation {
            question_format,
            question_type,
            generator: generator.cloned(),
        }
    }
}
