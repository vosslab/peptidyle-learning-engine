//! Human-facing typed route references.
//!
//! These values are stable database locators, not authorization capabilities. The server always
//! resolves one inside the authenticated tenant, role, membership, and ownership boundary before
//! loading the internal UUID record.

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Largest route number that remains compact and lossless in every product layer.
pub const MAX_PUBLIC_ROUTE_NUMBER: u32 = i32::MAX as u32;

macro_rules! impl_public_route_number {
    ($name:ident, $prefix:literal, $description:literal) => {
        impl $name {
            /// Builds one typed reference from its positive database identity.
            pub fn new(value: u64) -> Option<Self> {
                u32::try_from(value)
                    .ok()
                    .filter(|value| *value <= MAX_PUBLIC_ROUTE_NUMBER)
                    .and_then(NonZeroU32::new)
                    .map(Self)
            }

            /// Returns the positive database value.
            pub fn value(self) -> u32 {
                self.0.get()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, concat!($prefix, "-{}"), self.value())
            }
        }

        impl std::str::FromStr for $name {
            type Err = &'static str;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let Some(digits) = value.strip_prefix(concat!($prefix, "-")) else {
                    return Err(concat!($description, " must look like ", $prefix, "-123"));
                };
                if digits.is_empty()
                    || digits.len() > 10
                    || !digits.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(concat!($description, " must look like ", $prefix, "-123"));
                }
                digits
                    .parse::<u64>()
                    .ok()
                    .and_then(Self::new)
                    .ok_or(concat!($description, " must be a positive 31-bit value"))
            }
        }
    };
}

/// Public course locator rendered as `C-n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CoursePublicId(NonZeroU32);

/// Public assignment locator rendered as `A-n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssignmentPublicId(NonZeroU32);

/// Public run locator rendered as `R-n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunPublicId(NonZeroU32);

/// Public workspace locator rendered as `W-n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspacePublicId(NonZeroU32);

impl_public_route_number!(CoursePublicId, "C", "course reference");
impl_public_route_number!(AssignmentPublicId, "A", "assignment reference");
impl_public_route_number!(RunPublicId, "R", "run reference");
impl_public_route_number!(WorkspacePublicId, "W", "workspace reference");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_references_are_typed_compact_and_bounded() {
        assert_eq!(
            CoursePublicId::new(123).expect("valid course").to_string(),
            "C-123"
        );
        assert_eq!(
            "A-456"
                .parse::<AssignmentPublicId>()
                .expect("valid assignment")
                .value(),
            456
        );
        assert_eq!(
            "R-789"
                .parse::<RunPublicId>()
                .expect("valid run")
                .to_string(),
            "R-789"
        );
        assert_eq!(
            "W-42"
                .parse::<WorkspacePublicId>()
                .expect("valid workspace")
                .value(),
            42
        );
        assert!(CoursePublicId::new(0).is_none());
        assert!(RunPublicId::new(u64::from(MAX_PUBLIC_ROUTE_NUMBER) + 1).is_none());
        assert!("C-000".parse::<CoursePublicId>().is_err());
        assert!("A-12-extra".parse::<AssignmentPublicId>().is_err());
    }
}
