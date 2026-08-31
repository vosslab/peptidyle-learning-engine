use std::collections::BTreeMap;

use domain::generator::QuestionVariationParameters;
use grading::{GradingError, QuestionGradingOutcome, grade};
use question_model::generation::QuestionGeneratorReference;
use question_model::{
    QuestionBackendVersion, QuestionFormat, QuestionGraderVersion, QuestionType, QuestionVersion,
    StudentResponse,
};

use crate::generator::NativeQuestionImplementation;
use crate::peptide_bond_geometry::PeptideBondGeometryV1;
use crate::{
    ADAPTER_ID, ADAPTER_VERSION, GRADING_ID, GRADING_VERSION, NativeAdapter, NativeAdapterError,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum NativeExecution {
    V1,
}

impl NativeExecution {
    pub(super) fn derive_answer_key(
        self,
        implementation: &dyn NativeQuestionImplementation,
        question: &QuestionVersion,
        generated: &QuestionVariationParameters,
    ) -> Result<Option<grading::AnswerKey>, NativeAdapterError> {
        match self {
            Self::V1 => implementation.derive_answer_key(question, generated),
        }
    }

    pub(super) fn grade(
        self,
        question: &QuestionVersion,
        response: &StudentResponse,
        answer_key: Option<&grading::AnswerKey>,
    ) -> Result<QuestionGradingOutcome, GradingError> {
        match self {
            Self::V1 => grade(question, response, answer_key),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Exact source contract claimed by one installed Native Question Implementation.
///
/// The Question Source owns generator identity and version. An implementation release
/// remains reproduction evidence and is intentionally not a dispatch dimension.
pub(super) struct NativeQuestionImplementationKey {
    pub(super) question_format: QuestionFormat,
    pub(super) question_type: QuestionType,
    pub(super) generator: Option<QuestionGeneratorReference>,
}

impl NativeQuestionImplementationKey {
    pub(super) fn from_implementation(implementation: &dyn NativeQuestionImplementation) -> Self {
        Self {
            question_format: implementation.question_format(),
            question_type: implementation.question_type(),
            generator: implementation.generator(),
        }
    }
}

impl NativeAdapter {
    /// Builds the production registry with reviewed built-in implementations.
    pub fn new() -> Self {
        let mut adapter = Self::empty();
        for implementation in crate::flat_question::FLAT_V2_IMPLEMENTATIONS {
            adapter
                .register_implementation(implementation)
                .expect("each built-in version 2 flat implementation registration is unique");
        }
        adapter
            .register_implementation(PeptideBondGeometryV1)
            .expect("the built-in implementation registration is unique");
        adapter
    }

    /// Builds an empty registry for explicit composition and contract tests.
    pub fn empty() -> Self {
        let current_backend = backend_version(ADAPTER_ID, ADAPTER_VERSION);
        let current_grader = grader_version(GRADING_ID, GRADING_VERSION);
        Self {
            implementations: BTreeMap::new(),
            backend_versions: BTreeMap::from([(
                (
                    current_backend.name.clone(),
                    current_backend.version.clone(),
                ),
                NativeExecution::V1,
            )]),
            grader_versions: BTreeMap::from([(
                (current_grader.name.clone(), current_grader.version.clone()),
                NativeExecution::V1,
            )]),
            current_backend,
            current_grader,
        }
    }

    /// Selects installed Question Backend and Question Grader Versions for newly issued attempts.
    ///
    /// Future execution versions must first be added to the exact registry;
    /// unknown persisted versions are refused.
    pub fn select_current_versions(
        &mut self,
        backend: QuestionBackendVersion,
        grader: QuestionGraderVersion,
    ) -> Result<(), NativeAdapterError> {
        self.backend_execution_for(&backend)?;
        self.grader_execution_for(&grader)?;
        self.current_backend = backend;
        self.current_grader = grader;
        Ok(())
    }

    /// Adds one Question Implementation without changing adapter dispatch.
    ///
    /// Versions of one implementation coexist so published content can
    /// regenerate with its pinned generator after a new release is added.
    pub fn register_implementation<F>(
        &mut self,
        implementation: F,
    ) -> Result<(), NativeAdapterError>
    where
        F: NativeQuestionImplementation + 'static,
    {
        let key = NativeQuestionImplementationKey::from_implementation(&implementation);
        if self.implementations.contains_key(&key) {
            return Err(NativeAdapterError::DuplicateQuestionImplementation {
                question_format: key.question_format,
                question_type: key.question_type,
                generator: key.generator.clone(),
            });
        }
        self.implementations
            .insert(key, std::sync::Arc::new(implementation));
        Ok(())
    }

    pub(super) fn backend_execution_for(
        &self,
        version: &QuestionBackendVersion,
    ) -> Result<&NativeExecution, NativeAdapterError> {
        self.backend_versions
            .get(&(version.name.clone(), version.version.clone()))
            .ok_or_else(|| NativeAdapterError::UnknownQuestionBackendVersion {
                version: version.clone(),
            })
    }

    pub(super) fn grader_execution_for(
        &self,
        version: &QuestionGraderVersion,
    ) -> Result<&NativeExecution, NativeAdapterError> {
        self.grader_versions
            .get(&(version.name.clone(), version.version.clone()))
            .ok_or_else(|| NativeAdapterError::UnknownQuestionGraderVersion {
                version: version.clone(),
            })
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn backend_version(name: &str, version: &str) -> QuestionBackendVersion {
    QuestionBackendVersion {
        name: name.to_string(),
        version: version.to_string(),
    }
}

pub(super) fn grader_version(name: &str, version: &str) -> QuestionGraderVersion {
    QuestionGraderVersion {
        name: name.to_string(),
        version: version.to_string(),
    }
}
