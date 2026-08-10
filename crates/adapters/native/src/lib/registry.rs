use std::collections::BTreeMap;

use domain::generator::GeneratedVariant;
use grading::{GradeOutcome, GradingError, grade};
use question_model::generation::GeneratorReference;
use question_model::{ImplementationVersion, QuestionDefinition, StudentResponse};

use crate::generator::NativeQuestionFamily;
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
        family: &dyn NativeQuestionFamily,
        question: &QuestionDefinition,
        generated: &GeneratedVariant,
    ) -> Result<Option<grading::AnswerKey>, NativeAdapterError> {
        match self {
            Self::V1 => family.derive_answer_key(question, generated),
        }
    }

    pub(super) fn grade(
        self,
        question: &QuestionDefinition,
        response: &StudentResponse,
        answer_key: Option<&grading::AnswerKey>,
    ) -> Result<GradeOutcome, GradingError> {
        match self {
            Self::V1 => grade(question, response, answer_key),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FamilyRegistrationKey {
    pub(super) family: String,
    pub(super) generator: Option<GeneratorReference>,
}

impl FamilyRegistrationKey {
    pub(super) fn from_family(family: &dyn NativeQuestionFamily) -> Self {
        Self {
            family: family.family().to_string(),
            generator: family.generator(),
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
    /// Builds the production registry with reviewed built-in source families.
    pub fn new() -> Self {
        let mut adapter = Self::empty();
        adapter
            .register_family(crate::flat_question::FlatSingleChoiceFamily)
            .expect("the built-in static flat family registration is unique");
        for family in crate::flat_question::FLAT_V2_FAMILIES {
            adapter
                .register_family(family)
                .expect("each built-in version 2 flat family registration is unique");
        }
        adapter
            .register_family(PeptideBondGeometryV1)
            .expect("the built-in family registration is unique");
        adapter
    }

    /// Builds an empty registry for explicit composition and contract tests.
    pub fn empty() -> Self {
        let current_adapter = implementation_version(ADAPTER_ID, ADAPTER_VERSION);
        let current_grading = implementation_version(GRADING_ID, GRADING_VERSION);
        Self {
            families: BTreeMap::new(),
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

    /// Adds one source family without changing adapter dispatch.
    ///
    /// Versions of one family coexist so published content can regenerate
    /// with its pinned generator after a new version is added.
    pub fn register_family<F>(&mut self, family: F) -> Result<(), NativeAdapterError>
    where
        F: NativeQuestionFamily + 'static,
    {
        let key = FamilyRegistrationKey::from_family(&family);
        if self.families.contains_key(&key) {
            return Err(NativeAdapterError::DuplicateFamily(key.family));
        }
        self.families.insert(key, std::sync::Arc::new(family));
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
