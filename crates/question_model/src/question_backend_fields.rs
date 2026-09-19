//! Exact Question Backend bindings and their closed record-field matrix.

use serde::{Deserialize, Serialize};

/// Maximum bytes in an opaque iMathAS deployment, item, or profile identifier.
///
/// These identifiers are configuration and source-location keys, not URLs,
/// credentials, or arbitrary path fragments.
pub const MAX_IMATHAS_IDENTIFIER_BYTES: usize = 128;

/// Why an iMathAS deployment, item, or profile identifier is not safe to
/// retain in a Question Backend binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImathasQuestionBackendBindingError {
    InvalidDeploymentId,
    InvalidItemId,
    InvalidProfile,
}

impl std::fmt::Display for ImathasQuestionBackendBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeploymentId => formatter.write_str("iMathAS deployment ID is invalid"),
            Self::InvalidItemId => formatter.write_str("iMathAS item ID is invalid"),
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
pub struct ImathasDeploymentId(String);

impl ImathasDeploymentId {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) {
            return Err(ImathasQuestionBackendBindingError::InvalidDeploymentId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasDeploymentId {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasDeploymentId> for String {
    fn from(value: ImathasDeploymentId) -> Self {
        value.0
    }
}

/// iMathAS-backend-local item selector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImathasItemId(String);

impl ImathasItemId {
    pub fn new(value: impl Into<String>) -> Result<Self, ImathasQuestionBackendBindingError> {
        let value = value.into();
        if !has_imathas_identifier_grammar(&value) || value.contains("..") {
            return Err(ImathasQuestionBackendBindingError::InvalidItemId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImathasItemId {
    type Error = ImathasQuestionBackendBindingError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImathasItemId> for String {
    fn from(value: ImathasItemId) -> Self {
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
    deployment_id: ImathasDeploymentId,
    item_id: ImathasItemId,
    profile: ImathasProfile,
}

impl ImathasQuestionBackendBinding {
    pub fn new(
        deployment_id: ImathasDeploymentId,
        item_id: ImathasItemId,
        profile: ImathasProfile,
    ) -> Self {
        Self {
            deployment_id,
            item_id,
            profile,
        }
    }

    pub fn deployment_id(&self) -> &ImathasDeploymentId {
        &self.deployment_id
    }

    pub fn item_id(&self) -> &ImathasItemId {
        &self.item_id
    }

    pub fn profile(&self) -> &ImathasProfile {
        &self.profile
    }
}

/// iMathAS location permitted before source snapshot preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImathasQuestionBackendBinding {
    deployment_id: ImathasDeploymentId,
    item_id: ImathasItemId,
}

impl DraftImathasQuestionBackendBinding {
    pub fn new(deployment_id: ImathasDeploymentId, item_id: ImathasItemId) -> Self {
        Self {
            deployment_id,
            item_id,
        }
    }

    pub fn deployment_id(&self) -> &ImathasDeploymentId {
        &self.deployment_id
    }

    pub fn item_id(&self) -> &ImathasItemId {
        &self.item_id
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imathas_item_id_refuses_path_traversal_segments() {
        assert_eq!(
            ImathasItemId::new("item..17"),
            Err(ImathasQuestionBackendBindingError::InvalidItemId)
        );
    }
}
