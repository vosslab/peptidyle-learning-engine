//! Opaque session-token and browser-cookie mechanics for the auth boundary.
//!
//! Tokens are generated once, stored only as hashes, and deliberately keep
//! their credential bytes out of debug output.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cookie::Cookie;
use learning_data_access::SessionTokenHash;

use super::{SESSION_COOKIE_NAME, SESSION_TOKEN_BYTES, SessionConfig};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SessionToken(pub(super) [u8; SESSION_TOKEN_BYTES]);

impl SessionToken {
    pub(super) fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; SESSION_TOKEN_BYTES];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    fn decode(value: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
        let bytes: [u8; SESSION_TOKEN_BYTES] = bytes.try_into().ok()?;
        Some(Self(bytes))
    }

    fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0)
    }

    pub(super) fn hash(&self) -> SessionTokenHash {
        SessionTokenHash::compute(&self.0)
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionToken([redacted])")
    }
}

pub(super) fn session_cookie(token: &SessionToken, config: SessionConfig) -> Cookie<'static> {
    Cookie::build((
        wire_cookie_name(SESSION_COOKIE_NAME, config),
        token.encode(),
    ))
    .path("/")
    .http_only(true)
    .secure(config.secure())
    .same_site(config.same_site())
    .build()
}

pub(super) fn wire_cookie_name(name: &'static str, _config: SessionConfig) -> &'static str {
    match name {
        SESSION_COOKIE_NAME => "__Host-ple_session",
        _ => name,
    }
}

pub(super) fn presented_token(cookie_header: Option<&str>) -> Option<SessionToken> {
    let mut tokens = Cookie::split_parse(cookie_header?)
        .filter_map(Result::ok)
        .filter_map(|cookie| {
            (cookie.name() == SESSION_COOKIE_NAME || cookie.name() == "__Host-ple_session")
                .then(|| SessionToken::decode(cookie.value()))
                .flatten()
        });
    let token = tokens.next()?;
    if tokens.next().is_some() {
        return None;
    }
    Some(token)
}
