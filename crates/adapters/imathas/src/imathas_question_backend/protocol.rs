//! Signed launch-wire encoding for the fixed iMathAS Question Backend protocol.

use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};

use crate::ImathasAdapterError;

pub(super) fn signed_launch_jwt(
    secret: &[u8],
    item_reference: &str,
    imathas_seed: u16,
    expiry_millis: i64,
    challenge: &str,
    binding: &str,
) -> Result<String, ImathasAdapterError> {
    let exp = expiry_millis
        .checked_add(999)
        .and_then(|value| value.checked_div(1_000))
        .ok_or(ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)?;
    let payload = serde_json::json!({
        "id": item_reference,
        "seed": imathas_seed,
        "exp": exp,
        "ple_launch_challenge": challenge,
        "ple_binding": binding,
    });
    let payload = serde_json::to_vec(&payload)
        .map_err(|_| ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)?;
    let base64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = base64.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = base64.encode(payload);
    let signed = format!("{header}.{payload}");
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret)
        .map_err(|_| ImathasAdapterError::InvalidImathasQuestionBackendSessionAuthentication)?;
    mac.update(signed.as_bytes());
    Ok(format!(
        "{signed}.{}",
        base64.encode(mac.finalize().into_bytes())
    ))
}
