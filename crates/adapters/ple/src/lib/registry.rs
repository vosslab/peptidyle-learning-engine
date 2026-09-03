use std::collections::{BTreeMap, BTreeSet};

use question_model::{QuestionBackendVersion, QuestionFormat, QuestionGraderVersion, QuestionType};

use crate::generator::PleQuestionImplementation;
use crate::{
    ADAPTER_ID, ADAPTER_VERSION, GRADING_ID, GRADING_VERSION, PleQuestionBackend,
    PleQuestionBackendError,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PleQuestionImplementationKey {
    pub(super) question_format: QuestionFormat,
    pub(super) question_type: QuestionType,
}

impl PleQuestionImplementationKey {
    pub(super) fn from_implementation(implementation: &dyn PleQuestionImplementation) -> Self {
        Self {
            question_format: implementation.question_format(),
            question_type: implementation.question_type(),
        }
    }
}

impl PleQuestionBackend {
    pub fn new() -> Self {
        let mut adapter = Self::empty();
        for implementation in crate::question_json::PLE_QUESTION_JSON_IMPLEMENTATIONS {
            adapter.register_implementation(implementation).expect(
                "each built-in schema-version-2 PLE Question JSON implementation is unique",
            );
        }
        adapter
    }

    pub fn empty() -> Self {
        let current_backend = backend_version(ADAPTER_ID, ADAPTER_VERSION);
        let current_grader = grader_version(GRADING_ID, GRADING_VERSION);
        Self {
            implementations: BTreeMap::new(),
            backend_versions: BTreeSet::from([(
                current_backend.name.clone(),
                current_backend.version.clone(),
            )]),
            grader_versions: BTreeSet::from([(
                current_grader.name.clone(),
                current_grader.version.clone(),
            )]),
            current_backend,
            current_grader,
        }
    }

    pub fn select_current_versions(
        &mut self,
        backend: QuestionBackendVersion,
        grader: QuestionGraderVersion,
    ) -> Result<(), PleQuestionBackendError> {
        self.require_backend_version(&backend)?;
        self.require_grader_version(&grader)?;
        self.current_backend = backend;
        self.current_grader = grader;
        Ok(())
    }

    pub fn register_implementation<F>(
        &mut self,
        implementation: F,
    ) -> Result<(), PleQuestionBackendError>
    where
        F: PleQuestionImplementation + 'static,
    {
        let key = PleQuestionImplementationKey::from_implementation(&implementation);
        if self.implementations.contains_key(&key) {
            return Err(PleQuestionBackendError::DuplicateQuestionImplementation {
                question_format: key.question_format,
                question_type: key.question_type,
            });
        }
        self.implementations
            .insert(key, std::sync::Arc::new(implementation));
        Ok(())
    }

    pub(super) fn require_backend_version(
        &self,
        version: &QuestionBackendVersion,
    ) -> Result<(), PleQuestionBackendError> {
        self.backend_versions
            .contains(&(version.name.clone(), version.version.clone()))
            .then_some(())
            .ok_or_else(|| PleQuestionBackendError::UnknownQuestionBackendVersion {
                version: version.clone(),
            })
    }

    pub(super) fn require_grader_version(
        &self,
        version: &QuestionGraderVersion,
    ) -> Result<(), PleQuestionBackendError> {
        self.grader_versions
            .contains(&(version.name.clone(), version.version.clone()))
            .then_some(())
            .ok_or_else(|| PleQuestionBackendError::UnknownQuestionGraderVersion {
                version: version.clone(),
            })
    }
}

impl Default for PleQuestionBackend {
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
