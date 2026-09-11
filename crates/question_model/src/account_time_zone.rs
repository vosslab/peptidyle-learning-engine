//! Exact IANA time-zone preference owned by one Account.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// One exact case-sensitive IANA name used for an Account preference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AccountTimeZone(String);

impl AccountTimeZone {
    /// Validates exact case-sensitive membership in the embedded IANA database.
    ///
    /// ASVS 2.2.1--2.2.2: this positive validation is repeated by the trusted
    /// PostgreSQL boundary before the preference is persisted.
    pub fn parse(value: &str) -> Result<Self, AccountTimeZoneError> {
        let parsed = value
            .parse::<chrono_tz::Tz>()
            .map_err(|_| AccountTimeZoneError)?;
        if parsed.name() != value {
            return Err(AccountTimeZoneError);
        }
        Ok(Self(value.to_string()))
    }

    /// Returns the exact stable IANA spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AccountTimeZone {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AccountTimeZone {
    type Err = AccountTimeZoneError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for AccountTimeZone {
    type Error = AccountTimeZoneError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<AccountTimeZone> for String {
    fn from(value: AccountTimeZone) -> Self {
        value.0
    }
}

/// An input is not an exact known IANA time-zone name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountTimeZoneError;

impl Display for AccountTimeZoneError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("account time zone must be an exact known IANA name")
    }
}

impl Error for AccountTimeZoneError {}

#[cfg(test)]
mod tests {
    use super::AccountTimeZone;

    #[test]
    fn accepts_exact_iana_zone_names() {
        for zone in ["America/Chicago", "US/Central"] {
            assert_eq!(AccountTimeZone::parse(zone).unwrap().as_str(), zone);
        }
    }

    #[test]
    fn rejects_noncanonical_or_malformed_zone_names() {
        for zone in [
            "america/chicago",
            " America/Chicago",
            "UTC+01:00",
            "not/a-zone",
        ] {
            assert!(AccountTimeZone::parse(zone).is_err(), "{zone}");
        }
    }
}
