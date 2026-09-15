//! The native PLE branch of the Student-authorized delivery shell.
//!
//! Assessment authorization, immutable Question ID and revision pins,
//! attempt persistence, lifecycle, and public outcome projection remain in
//! the parent delivery route and data-access procedure. This focused branch
//! consumes the common backend interface solely to select PLE's native
//! renderer. It neither accepts an external backend here nor interprets an
//! external control, response, state, or runtime payload.

use adapter_ple::PleQuestionBackend;
use question_model::QuestionBackend;
use question_model::question_library::{
    QuestionBackendInteractionPolicy, QuestionBackendInterface,
};

use super::StartError;

/// Returns PLE's native Question JSON backend only for the common interface's
/// PLE-owned branch.
///
/// ASVS 2.2.1/2.2.2 and 2.3.1: the caller obtains the interface only after
/// Student authorization and immutable source selection. Rejecting every
/// backend-owned adapter here prevents an accidental future fallback that
/// would make PLE parse or own an external adapter's interaction lifecycle.
pub(super) fn native_ple_backend(
    interface: QuestionBackendInterface,
) -> Result<PleQuestionBackend, StartError> {
    match (interface.backend, interface.backend.interaction_policy()) {
        (QuestionBackend::Ple, QuestionBackendInteractionPolicy::PleOwned) => {
            Ok(PleQuestionBackend::new())
        }
        _ => Err(StartError::Invalid),
    }
}
