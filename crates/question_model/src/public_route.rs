//! Human-facing typed route references.
//!
//! These strings locate records; they are never authority.  The server resolves one inside the
//! authenticated course membership boundary before using its internal identity.

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use crate::{AssessmentAttemptId, AssessmentId, CourseId, StudentRecordId, WorkspaceId};

/// Largest route number that remains compact and lossless in every product layer.
pub const MAX_PUBLIC_ROUTE_NUMBER: u32 = i32::MAX as u32;

/// Prefixes reserved by the route grammar.
pub const RESERVED_REFERENCE_PREFIXES: &[&str] = &[
    "R", "W", "D", "G", "U", "M", "I", "QC", "QS", "BP", "CI", "A",
];

/// The alphabet used for short human reference identities.  It deliberately
/// excludes the visually ambiguous Crockford letters I, L, O, and U.
const CROCKFORD_BASE32: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

macro_rules! impl_human_reference {
    ($name:ident, $prefix:literal, $description:literal) => {
        impl $name {
            /// Builds a validated opaque reference returned by the data boundary.
            pub fn new(value: impl AsRef<str>) -> Result<Self, &'static str> {
                value.as_ref().parse()
            }

            /// The exact opaque string to bind at the public data boundary.
            pub fn as_string(&self) -> String {
                self.to_string()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let bytes = self.0.to_be_bytes();
                let first = bytes
                    .iter()
                    .position(|byte| *byte != 0)
                    .unwrap_or(bytes.len());
                let value = std::str::from_utf8(&bytes[first..]).map_err(|_| std::fmt::Error)?;
                f.write_str(value)
            }
        }

        impl std::str::FromStr for $name {
            type Err = &'static str;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                const PREFIX: &str = $prefix;
                let Some(suffix) = value.strip_prefix(PREFIX) else {
                    return Err(concat!($description, " has an invalid prefix"));
                };
                if suffix.len() != 6
                    || !suffix
                        .bytes()
                        .all(|byte| CROCKFORD_BASE32.as_bytes().contains(&byte))
                {
                    return Err(concat!(
                        $description,
                        " must use six Crockford Base32 characters"
                    ));
                }
                let packed = value
                    .bytes()
                    .fold(0u64, |packed, byte| (packed << 8) | u64::from(byte));
                Ok(Self(packed))
            }
        }

        impl TryFrom<String> for $name {
            type Error = &'static str;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }
    };
}

macro_rules! impl_reference {
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

            /// Returns the positive database scalar, for persistence only.
            pub fn number(self) -> u32 {
                self.0.get()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!($prefix, "-{}"), self.number())
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
                    || digits.starts_with('0')
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

        impl TryFrom<String> for $name {
            type Error = &'static str;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInstanceReference(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentReference(u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentAttemptReference(NonZeroU32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AuthoringWorkspaceReference(NonZeroU32);
/// An authorized Draft Question Reference for one private Draft Question lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DraftQuestionReference(NonZeroU32);
/// An authorized Account Reference for an existing platform account. It carries neither email nor authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AccountReference(u64);
/// An authorized Course Membership Reference for one course-membership episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseMembershipReference(NonZeroU32);
/// An authorized Course Invitation Reference for one target-bound Course Invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInvitationReference(NonZeroU32);
/// An authorized Blueprint Course Reference for one reusable Blueprint Course.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintCourseReference(u64);

impl_human_reference!(CourseInstanceReference, "CI", "Course Instance reference");
impl_human_reference!(AssessmentReference, "A", "Assessment reference");
impl_reference!(
    AssessmentAttemptReference,
    "R",
    "Assessment Attempt reference"
);
impl_reference!(
    AuthoringWorkspaceReference,
    "W",
    "Authoring Workspace reference"
);
impl_reference!(DraftQuestionReference, "D", "Draft Question reference");
impl_human_reference!(AccountReference, "U", "Account reference");
impl_reference!(
    CourseMembershipReference,
    "M",
    "course-membership reference"
);
impl_reference!(
    CourseInvitationReference,
    "I",
    "Course Invitation reference"
);
impl_human_reference!(BlueprintCourseReference, "BP", "Blueprint Course reference");

/// One authorized navigation target. IDs remain transport details after Store authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NavigationResolution {
    Course {
        course_id: CourseId,
    },
    Assessment {
        course_id: CourseId,
        assessment_id: AssessmentId,
    },
    AssessmentAttempt {
        course_id: CourseId,
        assessment_id: AssessmentId,
        student_record_id: StudentRecordId,
        assessment_attempt_id: AssessmentAttemptId,
    },
    Workspace {
        workspace_id: WorkspaceId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn references_use_exact_full_string_wire_values() {
        macro_rules! assert_human_reference_wire {
            ($reference:ty, $valid:literal, $wrong_prefix:literal) => {{
                let reference: $reference = $valid.parse().expect("valid opaque reference");
                assert_eq!(reference.to_string(), $valid);
                assert_eq!(
                    serde_json::to_value(reference.clone()).expect("serializes"),
                    $valid
                );
                assert_eq!(
                    serde_json::from_value::<$reference>(serde_json::json!($valid))
                        .expect("parses"),
                    reference
                );
                for invalid in [
                    $wrong_prefix,
                    concat!($valid, "0"),
                    "CI00000I",
                    "CI00000O",
                    "CI00000U",
                ] {
                    assert!(invalid.parse::<$reference>().is_err(), "{invalid}");
                }
            }};
        }
        macro_rules! assert_reference_wire {
            ($reference:ty, $valid:literal, $wrong_prefix:literal, $zero:literal, $leading_zero:literal, $overflow:literal) => {{
                let reference: $reference = $valid.parse().expect("valid reference");
                assert_eq!(reference.to_string(), $valid);
                assert_eq!(serde_json::to_value(reference).expect("serializes"), $valid);
                assert_eq!(
                    serde_json::from_value::<$reference>(serde_json::json!($valid))
                        .expect("parses"),
                    reference
                );
                for invalid in [
                    $wrong_prefix,
                    concat!(stringify!($valid), "0"),
                    $zero,
                    $leading_zero,
                    $overflow,
                ] {
                    assert!(invalid.parse::<$reference>().is_err(), "{invalid}");
                }
            }};
        }
        assert_human_reference_wire!(CourseInstanceReference, "CI7K3M2Q", "C7K3M2Q");
        assert_human_reference_wire!(AssessmentReference, "A7K3M2Q", "CI7K3M2Q");
        assert_reference_wire!(
            AssessmentAttemptReference,
            "R-125",
            "C-125",
            "R-0",
            "R-01",
            "R-2147483648"
        );
        assert_reference_wire!(
            AuthoringWorkspaceReference,
            "W-126",
            "C-126",
            "W-0",
            "W-01",
            "W-2147483648"
        );
        assert_human_reference_wire!(AccountReference, "U7K3M2Q", "A7K3M2Q");
        assert_reference_wire!(
            CourseMembershipReference,
            "M-129",
            "C-129",
            "M-0",
            "M-01",
            "M-2147483648"
        );
        assert_reference_wire!(
            CourseInvitationReference,
            "I-130",
            "C-130",
            "CI-0",
            "CI-01",
            "CI-2147483648"
        );
        assert_human_reference_wire!(BlueprintCourseReference, "BP7K3M2Q", "A7K3M2Q");
        assert!(!RESERVED_REFERENCE_PREFIXES.contains(&"AC"));
    }
}
