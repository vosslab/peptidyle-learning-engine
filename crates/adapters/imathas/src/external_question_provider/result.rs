//! Server-to-server signed-result retrieval and verification.

use super::{
    ContractedLaunchSession, ContractedScoredEmbedConfig, ImathasAdapterError,
    ResultTransportRequest, ScoredEmbedFailure, ScoredEmbedTransport, map_transport,
};
use crate::{ProviderFailure, VerifiedProviderGrade};
use question_model::Timestamp;

/// Fetches only the broker-held provider result and gives it directly to the
/// sealed verifier. Browser messages never enter this path.
pub(super) async fn retrieve_and_verify<T: ScoredEmbedTransport>(
    transport: &T,
    config: &ContractedScoredEmbedConfig,
    session: &mut ContractedLaunchSession,
    now: Timestamp,
) -> Result<VerifiedProviderGrade, ImathasAdapterError> {
    session
        .ledger
        .ensure_eligible_at(now)
        .map_err(ScoredEmbedFailure::into_adapter_error)?;
    let bytes = transport
        .fetch_signed_grade_get(ResultTransportRequest {
            handle: &session.handle,
            launch_session_authentication: session.ledger.launch_session_authentication(),
            provider_key: config.profile.provider_key(),
        })
        .await
        .map_err(map_transport)?;
    if bytes.is_empty() || bytes.len() > config.max_result_bytes {
        return Err(ImathasAdapterError::Provider(
            ProviderFailure::InvalidResponse,
        ));
    }
    let token = std::str::from_utf8(&bytes)
        .map_err(|_| ImathasAdapterError::Provider(ProviderFailure::InvalidResponse))?;
    config
        .result_verifier
        .verify_result(&mut session.ledger, token, now)
        .map_err(ScoredEmbedFailure::into_adapter_error)
}
