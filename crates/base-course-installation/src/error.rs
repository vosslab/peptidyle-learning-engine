//! Concrete failures returned by Base Course installation.

use adapter_native::NativeAdapterError;
use learning_data_access::StoreError;
use question_model::presentation::PresentationBuildError;

/// A request, deterministic recipe, or persistence failure during installation.
#[derive(Debug)]
pub enum BaseCourseInstallError {
    /// Participant identities or another typed request input are invalid.
    Request(String),
    /// The generation-bound storage receipt is malformed or does not match.
    Receipt(String),
    /// Retained records differ from the versioned deterministic recipe.
    BaselineMismatch(String),
    /// An LDA-owned Store or lifecycle transition failed.
    Persistence {
        operation: &'static str,
        source: StoreError,
    },
    /// The production native adapter could not issue, reproduce, or grade the recipe.
    NativeAdapter {
        operation: &'static str,
        source: NativeAdapterError,
    },
    /// A browser-safe presentation could not be built or reproduced.
    Presentation {
        operation: &'static str,
        source: PresentationBuildError,
    },
    /// Canonical receipt or output JSON serialization failed.
    Serialization {
        operation: &'static str,
        source: serde_json::Error,
    },
    /// Installation failed and the locked PostgreSQL session also failed to close safely.
    LockCleanup {
        install: Box<BaseCourseInstallError>,
        cleanup: StoreError,
    },
}

impl BaseCourseInstallError {
    pub(crate) fn request(message: impl Into<String>) -> Self {
        Self::Request(message.into())
    }

    pub(crate) fn receipt(message: impl Into<String>) -> Self {
        Self::Receipt(message.into())
    }

    pub(crate) fn baseline(message: impl Into<String>) -> Self {
        Self::BaselineMismatch(message.into())
    }

    pub(crate) fn persistence(operation: &'static str, source: StoreError) -> Self {
        Self::Persistence { operation, source }
    }

    pub(crate) fn native(operation: &'static str, source: NativeAdapterError) -> Self {
        Self::NativeAdapter { operation, source }
    }

    pub(crate) fn presentation(operation: &'static str, source: PresentationBuildError) -> Self {
        Self::Presentation { operation, source }
    }

    pub(crate) fn serialization(operation: &'static str, source: serde_json::Error) -> Self {
        Self::Serialization { operation, source }
    }
}

impl std::fmt::Display for BaseCourseInstallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(message) => write!(formatter, "invalid Base Course request: {message}"),
            Self::Receipt(message) => write!(formatter, "invalid Base Course receipt: {message}"),
            Self::BaselineMismatch(message) => {
                write!(
                    formatter,
                    "Base Course baseline cannot safely converge: {message}"
                )
            }
            Self::Persistence { operation, source } => {
                write!(formatter, "{operation}: {source}")
            }
            Self::NativeAdapter { operation, source } => {
                write!(formatter, "{operation}: {source}")
            }
            Self::Presentation { operation, source } => {
                write!(formatter, "{operation}: {source}")
            }
            Self::Serialization { operation, source } => {
                write!(formatter, "{operation}: {source}")
            }
            Self::LockCleanup { install, cleanup } => write!(
                formatter,
                "{install}; closing the failed Base Course installation lock also failed: {cleanup}"
            ),
        }
    }
}

impl std::error::Error for BaseCourseInstallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Persistence { source, .. } => Some(source),
            Self::NativeAdapter { source, .. } => Some(source),
            Self::Presentation { source, .. } => Some(source),
            Self::Serialization { source, .. } => Some(source),
            Self::LockCleanup { install, .. } => Some(install.as_ref()),
            Self::Request(_) | Self::Receipt(_) | Self::BaselineMismatch(_) => None,
        }
    }
}
