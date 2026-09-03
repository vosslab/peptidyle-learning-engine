//! Private Authentication Email value objects.
//!
//! AuthenticationEmail is a validated, redacted private
//! normalized-and-delivery credential value. Account Product Role owns its
//! lifecycle and mutability: a Student Authentication Email is immutable, and
//! a future verified Instructor Authentication Email replacement owns
//! Instructor changes. It is deliberately separate from account identity,
//! session, and course-record authorization.

/// Maximum accepted Authentication Email length after trimming whitespace.
pub const MAX_AUTHENTICATION_EMAIL_BYTES: usize = 320;

/// Validated email with a stable lookup form and a delivery spelling.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthenticationEmail {
    normalized: String,
    delivery: String,
    domain: EmailDomain,
}

impl AuthenticationEmail {
    /// Parses the conservative mailbox form accepted by PLE authentication.
    ///
    /// The local part is ASCII and lowercased for lookup. The domain is strict
    /// IDNA ASCII. Quoted local parts and SMTP comments are intentionally not
    /// accepted at the HTTP boundary.
    pub fn parse(value: &str) -> Result<Self, AuthenticationEmailError> {
        let delivery = value.trim();
        if delivery.is_empty() || delivery.len() > MAX_AUTHENTICATION_EMAIL_BYTES {
            return Err(AuthenticationEmailError::InvalidEmail);
        }
        let (local, domain) = delivery
            .rsplit_once('@')
            .ok_or(AuthenticationEmailError::InvalidEmail)?;
        if local.is_empty()
            || local.len() > 64
            || local.starts_with('.')
            || local.ends_with('.')
            || local.contains("..")
            || local.contains('@')
            || !local.bytes().all(valid_local_part_byte)
        {
            return Err(AuthenticationEmailError::InvalidEmail);
        }
        let domain =
            EmailDomain::parse(domain).map_err(|_| AuthenticationEmailError::InvalidEmail)?;
        let normalized = format!("{}@{}", local.to_ascii_lowercase(), domain.as_str());
        Ok(Self {
            normalized,
            delivery: delivery.to_string(),
            domain,
        })
    }

    /// Canonical private lookup form.
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    /// Original spelling retained solely for email delivery.
    pub fn delivery(&self) -> &str {
        &self.delivery
    }

    /// Exact normalized domain for course-email policy evaluation.
    pub fn domain(&self) -> &EmailDomain {
        &self.domain
    }
}

impl std::fmt::Debug for AuthenticationEmail {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthenticationEmail([redacted])")
    }
}

/// Complete normalized domain; it is never a suffix-matching policy value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailDomain(String);

impl EmailDomain {
    /// Applies strict IDNA conversion and DNS-label validation.
    pub fn parse(value: &str) -> Result<Self, AuthenticationEmailError> {
        let value = value.trim().trim_end_matches('.');
        if value.is_empty() || value.len() > 253 || value.contains('@') {
            return Err(AuthenticationEmailError::InvalidDomain);
        }
        let ascii = idna::domain_to_ascii_strict(value)
            .map_err(|_| AuthenticationEmailError::InvalidDomain)?
            .to_ascii_lowercase();
        if ascii.is_empty()
            || ascii.len() > 253
            || !ascii.contains('.')
            || ascii.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        {
            return Err(AuthenticationEmailError::InvalidDomain);
        }
        Ok(Self(ascii))
    }

    /// Canonical full-domain comparison value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Email input rejected before credential state is created or queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationEmailError {
    InvalidEmail,
    InvalidDomain,
}

impl std::fmt::Display for AuthenticationEmailError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("authentication email is invalid")
    }
}

impl std::error::Error for AuthenticationEmailError {}

fn valid_local_part_byte(value: u8) -> bool {
    value.is_ascii_alphanumeric()
        || matches!(
            value,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalizes_lookup_but_preserves_delivery_spelling() {
        let email =
            AuthenticationEmail::parse("Student@B\u{fc}cher.example").expect("valid IDNA email");
        assert_eq!(email.normalized(), "student@xn--bcher-kva.example");
        assert_eq!(email.delivery(), "Student@B\u{fc}cher.example");
        assert_eq!(format!("{email:?}"), "AuthenticationEmail([redacted])");
    }

    #[test]
    fn malformed_or_ambiguous_mailboxes_are_rejected() {
        for value in [
            "",
            "student",
            "@example.edu",
            ".student@example.edu",
            "a..b@example.edu",
        ] {
            assert!(AuthenticationEmail::parse(value).is_err(), "{value:?}");
        }
    }
}
