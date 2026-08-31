use std::collections::BTreeMap;

use domain::generator::GeneratedVariant;
use grading::{GradingError, QuestionGradingOutcome, grade};
use question_model::generation::GeneratorReference;
use question_model::{
    ImplementationVersion, QuestionDefinition, QuestionFormat, QuestionType, StudentResponse,
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
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
    ) -> Result<Option<grading::AnswerKey>, NativeAdapterError> {
        match self {
            Self::V1 => implementation.derive_answer_key(question, generated),
        }
    }

    pub(super) fn grade(
        self,
        question: &QuestionDefinition,
        response: &StudentResponse,
        answer_key: Option<&grading::AnswerKey>,
    ) -> Result<QuestionGradingOutcome, GradingError> {
        match self {
            Self::V1 => grade(question, response, answer_key),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NativeQuestionImplementationRegistrationKey {
    pub(super) question_format: QuestionFormat,
    pub(super) question_type: QuestionType,
    pub(super) generator: Option<GeneratorReference>,
    pub(super) implementation_id: String,
    pub(super) implementation_version: String,
}

impl NativeQuestionImplementationRegistrationKey {
    pub(super) fn from_implementation(implementation: &dyn NativeQuestionImplementation) -> Self {
        let release = implementation.implementation_release();
        Self {
            question_format: implementation.question_format(),
            question_type: implementation.question_type(),
            generator: implementation.generator(),
            implementation_id: release.id,
            implementation_version: release.version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ImplementationRegistrationKey {
    id: String,
    version: String,
}

impl From<&ImplementationVersion> for ImplementationRegistrationKey {
    fn from(value: &ImplementationVersion) -> Self {
        Self {
            id: value.id.clone(),
            version: value.version.clone(),
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
        let current_adapter = implementation_version(ADAPTER_ID, ADAPTER_VERSION);
        let current_grading = implementation_version(GRADING_ID, GRADING_VERSION);
        Self {
            implementations: BTreeMap::new(),
            adapter_implementations: BTreeMap::from([(
                ImplementationRegistrationKey::from(&current_adapter),
                NativeExecution::V1,
            )]),
            grading_implementations: BTreeMap::from([(
                ImplementationRegistrationKey::from(&current_grading),
                NativeExecution::V1,
            )]),
            current_adapter,
            current_grading,
        }
    }

    /// Selects installed execution versions for newly issued attempts.
    ///
    /// Future execution versions must first be added to the exact registry;
    /// unknown persisted versions are refused.
    pub fn select_current_implementations(
        &mut self,
        adapter: ImplementationVersion,
        grading: ImplementationVersion,
    ) -> Result<(), NativeAdapterError> {
        self.execution_for(&self.adapter_implementations, &adapter, "adapter")?;
        self.execution_for(&self.grading_implementations, &grading, "grading")?;
        self.current_adapter = adapter;
        self.current_grading = grading;
        Ok(())
    }

    /// Adds one Question Implementation without changing adapter dispatch.
    ///
    /// Releases of one implementation coexist so published content can
    /// regenerate with its pinned generator after a new release is added.
    pub fn register_implementation<F>(
        &mut self,
        implementation: F,
    ) -> Result<(), NativeAdapterError>
    where
        F: NativeQuestionImplementation + 'static,
    {
        let key = NativeQuestionImplementationRegistrationKey::from_implementation(&implementation);
        if self.implementations.contains_key(&key) {
            return Err(NativeAdapterError::DuplicateQuestionImplementation {
                question_format: key.question_format,
                question_type: key.question_type,
                generator: key.generator.clone(),
                implementation: ImplementationVersion {
                    id: key.implementation_id.clone(),
                    version: key.implementation_version.clone(),
                },
            });
        }
        self.implementations
            .insert(key, std::sync::Arc::new(implementation));
        Ok(())
    }

    pub(super) fn execution_for<'a>(
        &self,
        implementations: &'a BTreeMap<ImplementationRegistrationKey, NativeExecution>,
        version: &ImplementationVersion,
        field: &'static str,
    ) -> Result<&'a NativeExecution, NativeAdapterError> {
        implementations
            .get(&ImplementationRegistrationKey::from(version))
            .ok_or(NativeAdapterError::UnknownImplementation {
                field,
                version: version.clone(),
            })
    }
}

impl Default for NativeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn implementation_version(id: &str, version: &str) -> ImplementationVersion {
    ImplementationVersion {
        id: id.to_string(),
        version: version.to_string(),
    }
}
