use async_trait::async_trait;
use learning_data_access::{DraftRecord, TenantContext};
use question_model::{BackendCapabilities, DraftQuestionSource, UserId};

/// Resolves trusted capabilities for the adapter owning one question.
pub trait BackendRegistry: Send + Sync {
    /// Returns the server's capability declaration for this source.
    fn capabilities(
        &self,
        source: &DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError>;
}

/// Failure to resolve a server-owned adapter declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendRegistryError {
    /// The source names no adapter installed in this server.
    Unsupported,
    /// Registry state could not be read.
    Unavailable(String),
}

impl std::fmt::Display for BackendRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => formatter.write_str("question backend is not registered"),
            Self::Unavailable(message) => {
                write!(formatter, "backend registry unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for BackendRegistryError {}

/// Institution-configurable public-catalog review boundary.
#[async_trait]
pub trait PublicReviewGate: Send + Sync {
    /// Returns true only when this exact publication may enter the public catalog.
    async fn allows_publication(
        &self,
        tenant: TenantContext,
        publisher: UserId,
        draft: &DraftRecord,
    ) -> Result<bool, ReviewGateError>;
}

/// Public-review dependency failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewGateError(pub String);

impl std::fmt::Display for ReviewGateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "publication review unavailable: {}", self.0)
    }
}

impl std::error::Error for ReviewGateError {}

/// Default policy for institutions that do not require editorial review.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReviewNotRequired;

#[async_trait]
impl PublicReviewGate for ReviewNotRequired {
    async fn allows_publication(
        &self,
        _tenant: TenantContext,
        _publisher: UserId,
        _draft: &DraftRecord,
    ) -> Result<bool, ReviewGateError> {
        Ok(true)
    }
}
