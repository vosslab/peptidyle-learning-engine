//! Exact Question Backend bindings and their closed record-field matrix.

use serde::{Deserialize, Serialize};

use crate::QuestionBackend;

/// Maximum bytes in an opaque iMathAS deployment, item, or profile identifier.
///
/// These identifiers are configuration and source-location keys, not URLs,
/// credentials, or arbitrary path fragments.
pub const MAX_IMATHAS_IDENTIFIER_BYTES: usize = 128;

/// Why an iMathAS deployment, item, or profile identifier is not safe to
/// retain in a Question Backend binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImathasQuestionBackendBindingError {
    InvalidDeploymentReference,
    InvalidItemReference,
    InvalidProfile,
}

impl std::fmt::Display for ImathasQuestionBackendBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeploymentReference => {
                formatter.write_str("iMathAS deployment reference is invalid")
            }
            Self::InvalidItemReference => formatter.write_str("iMathAS item reference is invalid"),
            Self::InvalidProfile => formatter.write_str("iMathAS profile is invalid"),
        }
    }
}

impl std::error::Error for ImathasQuestionBackendBindingError {}

fn has_imathas_identifier_grammar(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IMATHAS_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Opaque configured iMathAS deployment selector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasDeploymentReference(String);

impl ImathasDeploymentReference {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) {
            return Err(ImathasQuestionBackendBindingError::InvalidDeploymentReference);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasDeploymentReference {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasDeploymentReference> for String {
    fn from(value: ImathasDeploymentReference) -> Self {
        value.0
    }
}

/// iMathAS-backend-local item selector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasItemReference(String);

impl ImathasItemReference {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) || value.contains("..") {
            return Err(ImathasQuestionBackendBindingError::InvalidItemReference);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasItemReference {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasItemReference> for String {
    fn from(value: ImathasItemReference) -> Self {
        value.0
    }
}

/// Pinned iMathAS profile selected at publication.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasProfile(String);

impl ImathasProfile {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) {
            return Err(ImathasQuestionBackendBindingError::InvalidProfile);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasProfile {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasProfile> for String {
    fn from(value: ImathasProfile) -> Self {
        value.0
    }
}

/// Immutable iMathAS backend location and profile pinned by a Question Revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImathasQuestionBackendBinding {
    deployment_reference: ImathasDeploymentReference,
    item_reference: ImathasItemReference,
    profile: ImathasProfile,
}

impl ImathasQuestionBackendBinding {
    pub fn new(
        deployment_reference: ImathasDeploymentReference,
        item_reference: ImathasItemReference,
        profile: ImathasProfile,
    ) -> Self {
        Self {
            deployment_reference,
            item_reference,
            profile,
        }
    }

    pub fn deployment_reference(&self) -> &ImathasDeploymentReference {
        &self.deployment_reference
    }

    pub fn item_reference(&self) -> &ImathasItemReference {
        &self.item_reference
    }

    pub fn profile(&self) -> &ImathasProfile {
        &self.profile
    }
}

/// iMathAS location permitted before source snapshot preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImathasQuestionBackendBinding {
    deployment_reference: ImathasDeploymentReference,
    item_reference: ImathasItemReference,
}

impl DraftImathasQuestionBackendBinding {
    pub fn new(
        deployment_reference: ImathasDeploymentReference,
        item_reference: ImathasItemReference,
    ) -> Self {
        Self {
            deployment_reference,
            item_reference,
        }
    }

    pub fn deployment_reference(&self) -> &ImathasDeploymentReference {
        &self.deployment_reference
    }

    pub fn item_reference(&self) -> &ImathasItemReference {
        &self.item_reference
    }
}

/// Why direct Question Backend fields do not describe one permitted backend record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionBackendFieldsError {
    MissingRequiredField,
    UnexpectedField,
}

impl std::fmt::Display for QuestionBackendFieldsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredField => formatter.write_str("Question Backend field is required"),
            Self::UnexpectedField => {
                formatter.write_str("Question Backend record carries an inapplicable field")
            }
        }
    }
}

impl std::error::Error for QuestionBackendFieldsError {}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuestionBackendFieldPresence {
    pub(crate) webwork_pg_path: bool,
    pub(crate) qti_package_item_identifier: bool,
    pub(crate) workspace_import_id: bool,
    pub(crate) imathas_question_backend_binding: bool,
    pub(crate) draft_imathas_question_backend_binding: bool,
}

/// Validates the closed Question Backend field matrix shared by editable and
/// published Question records.
pub(crate) fn validate_question_backend_field_matrix(
    question_backend: QuestionBackend,
    fields: QuestionBackendFieldPresence,
    is_draft: bool,
) -> Result<(), QuestionBackendFieldsError> {
    let QuestionBackendFieldPresence {
        webwork_pg_path,
        qti_package_item_identifier,
        workspace_import_id,
        imathas_question_backend_binding,
        draft_imathas_question_backend_binding,
    } = fields;

    let no_location_fields = !webwork_pg_path
        && !qti_package_item_identifier
        && !workspace_import_id
        && !imathas_question_backend_binding
        && !draft_imathas_question_backend_binding;
    match question_backend {
        QuestionBackend::Ple if no_location_fields => Ok(()),
        QuestionBackend::Ple => Err(QuestionBackendFieldsError::UnexpectedField),
        QuestionBackend::Webwork
            if webwork_pg_path
                && !qti_package_item_identifier
                && !workspace_import_id
                && !imathas_question_backend_binding
                && !draft_imathas_question_backend_binding =>
        {
            Ok(())
        }
        QuestionBackend::Webwork if !webwork_pg_path => {
            Err(QuestionBackendFieldsError::MissingRequiredField)
        }
        QuestionBackend::Webwork => Err(QuestionBackendFieldsError::UnexpectedField),
        QuestionBackend::Qti
            if qti_package_item_identifier
                && (is_draft == workspace_import_id)
                && !webwork_pg_path
                && !imathas_question_backend_binding
                && !draft_imathas_question_backend_binding =>
        {
            Ok(())
        }
        QuestionBackend::Qti
            if !qti_package_item_identifier || (is_draft && !workspace_import_id) =>
        {
            Err(QuestionBackendFieldsError::MissingRequiredField)
        }
        QuestionBackend::Qti => Err(QuestionBackendFieldsError::UnexpectedField),
        QuestionBackend::Imathas
            if !webwork_pg_path
                && !qti_package_item_identifier
                && !workspace_import_id
                && ((is_draft
                    && draft_imathas_question_backend_binding
                    && !imathas_question_backend_binding)
                    || (!is_draft
                        && imathas_question_backend_binding
                        && !draft_imathas_question_backend_binding)) =>
        {
            Ok(())
        }
        QuestionBackend::Imathas
            if (is_draft && !draft_imathas_question_backend_binding)
                || (!is_draft && !imathas_question_backend_binding) =>
        {
            Err(QuestionBackendFieldsError::MissingRequiredField)
        }
        QuestionBackend::Imathas => Err(QuestionBackendFieldsError::UnexpectedField),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imathas_item_reference_refuses_path_traversal_segments() {
        assert_eq!(
            ImathasItemReference::new("item..17"),
            Err(ImathasQuestionBackendBindingError::InvalidItemReference)
        );
    }

    #[test]
    fn matrix_requires_qti_workspace_import_for_drafts_only() {
        let fields = QuestionBackendFieldPresence {
            webwork_pg_path: false,
            qti_package_item_identifier: true,
            workspace_import_id: false,
            imathas_question_backend_binding: false,
            draft_imathas_question_backend_binding: false,
        };
        assert_eq!(
            validate_question_backend_field_matrix(QuestionBackend::Qti, fields, true),
            Err(QuestionBackendFieldsError::MissingRequiredField)
        );
    }
}
