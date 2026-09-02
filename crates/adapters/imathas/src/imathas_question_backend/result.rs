//! Server-to-server signed-result retrieval and verification.

use super::{
    ImathasAdapterError, ImathasLaunchState, ImathasQuestionBackendConfig,
    ImathasQuestionBackendTransport, ImathasRemoteGradingFailure, ResultTransportRequest,
    map_transport,
};
use crate::{ImathasQuestionBackendFailure, VerifiedImathasQuestionBackendResult};
use question_model::Timestamp;

/// Fetches only the server-held iMathAS result and gives it directly to the
/// sealed verifier. Browser messages never enter this path.
pub(super) async fn retrieve_and_verify<T: ImathasQuestionBackendTransport>(
    transport: &T,
    config: &ImathasQuestionBackendConfig,
    validation: &learning_data_access::ImathasQuestionBackendSessionValidation,
    imathas_launch_state: &ImathasLaunchState,
    now: Timestamp,
) -> Result<VerifiedImathasQuestionBackendResult, ImathasAdapterError> {
    super::validate_loaded_imathas_launch_state(config, validation, now)?;
    let bytes = transport
        .fetch_signed_grade_get(ResultTransportRequest {
            handle: imathas_launch_state.handle(),
            launch_session_authentication: validation.authentication.as_str(),
            deployment_reference: config.profile.deployment_reference(),
        })
        .await
        .map_err(map_transport)?;
    if bytes.len() > config.max_result_bytes {
        return Err(ImathasAdapterError::QuestionBackend(
            ImathasQuestionBackendFailure::InvalidResponse,
        ));
    }
    // ASVS 2.2.1: LDA bounds untrusted iMathAS bytes before this adapter
    // performs UTF-8 or JWT parsing. The verifier receives those exact bytes,
    // so its receipt cannot describe a different representation.
    let token =
        learning_data_access::ImathasQuestionBackendResultToken::from_server_adapter_bytes(bytes)
            .map_err(|_| {
            ImathasAdapterError::QuestionBackend(ImathasQuestionBackendFailure::InvalidResponse)
        })?;
    config
        .result_verifier
        .verify_result(validation, &token, now)
        .map_err(ImathasRemoteGradingFailure::into_adapter_error)
}
