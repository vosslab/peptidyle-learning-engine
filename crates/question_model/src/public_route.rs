//! Human-facing typed route references.
//!
//! These strings locate records; they are never authority.  The server resolves one inside the
//! authenticated course membership boundary before using its internal identity.

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use crate::question_library::{QUESTION_ID_ALPHABET, public_id_checksum_character};
use crate::{AssessmentAttemptId, AssessmentId, CourseId, StudentRecordId, WorkspaceId};

/// Largest route number that remains compact and lossless in every product layer.
pub const MAX_PUBLIC_ROUTE_NUMBER: u32 = i32::MAX as u32;

/// Prefixes reserved by the route grammar.
pub const RESERVED_REFERENCE_PREFIXES: &[&str] =
    &["R", "W", "G", "U", "M", "I", "QC", "QS", "BP", "CI", "A"];

/// Every public ID includes seven server-random Crockford characters and one
/// public SHA-256 checksum character.
pub const PUBLIC_REFERENCE_RANDOM_LENGTH: usize = 7;

macro_rules! impl_public_reference {
    ($name:ident, $wire_prefix:literal, $checksum_prefix:literal, $description:literal) => {
        impl $name {
            /// Validates the exact canonical reference returned by a data boundary.
            pub fn new(value: impl AsRef<str>) -> Result<Self, &'static str> {
                value.as_ref().parse()
            }

            /// Mints a reference from seven server-random Crockford characters.
            ///
            /// ASVS V2.1.1 and V2.2.1: callers must use a cryptographically
            /// secure source for the supplied random characters.
            pub fn from_random_identity(identity: impl AsRef<str>) -> Result<Self, &'static str> {
                let identity = identity.as_ref();
                if identity.len() != PUBLIC_REFERENCE_RANDOM_LENGTH
                    || !identity
                        .bytes()
                        .all(|character| QUESTION_ID_ALPHABET.contains(&character))
                {
                    return Err(concat!(
                        $description,
                        " random characters must be exact uppercase Crockford Base32"
                    ));
                }
                let checksum_input = format!("{}{}", $checksum_prefix, identity);
                let checksum = public_id_checksum_character(checksum_input.as_bytes());
                Ok(Self(format!("{}{}{}", $wire_prefix, identity, checksum)))
            }

            /// The exact canonical public value for every storage and transport boundary.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// An owned copy of the exact canonical public value.
            pub fn as_string(&self) -> String {
                self.0.clone()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = &'static str;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                // ASVS V2.2.1 and V2.2.2: this untrusted boundary preserves
                // only one exact canonical spelling and never translates it.
                let Some(suffix) = value.strip_prefix($wire_prefix) else {
                    return Err(concat!(
                        $description,
                        " must use its exact canonical syntax"
                    ));
                };
                if !value.is_ascii()
                    || suffix.len() != PUBLIC_REFERENCE_RANDOM_LENGTH + 1
                    || !suffix
                        .bytes()
                        .all(|character| QUESTION_ID_ALPHABET.contains(&character))
                {
                    return Err(concat!(
                        $description,
                        " must use its exact canonical syntax"
                    ));
                }
                let checksum_input = format!(
                    "{}{}",
                    $checksum_prefix,
                    &suffix[..PUBLIC_REFERENCE_RANDOM_LENGTH]
                );
                if suffix.as_bytes()[PUBLIC_REFERENCE_RANDOM_LENGTH]
                    != public_id_checksum_character(checksum_input.as_bytes()) as u8
                {
                    return Err(concat!(
                        $description,
                        " checksum does not match its canonical characters"
                    ));
                }
                Ok(Self(value.to_owned()))
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

macro_rules! impl_numeric_reference {
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
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, concat!($prefix, "-{}"), self.number())
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInstanceReference(String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentReference(String);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentAttemptReference(NonZeroU32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AuthoringWorkspaceReference(NonZeroU32);
/// An authorized Account Reference for an existing platform account. It carries neither email nor authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AccountReference(String);
/// An authorized Course Membership Reference for one course-membership episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseMembershipReference(NonZeroU32);
/// An authorized Course Invitation Reference for one target-bound Course Invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInvitationReference(NonZeroU32);
/// An authorized Blueprint Course Reference for one reusable Blueprint Course.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintCourseReference(String);

impl_public_reference!(
    CourseInstanceReference,
    "CI",
    "CI",
    "Course Instance reference"
);
impl_public_reference!(AssessmentReference, "A", "A", "Assessment reference");
impl_public_reference!(AccountReference, "U", "U", "Account reference");
impl_public_reference!(
    BlueprintCourseReference,
    "BP",
    "BP",
    "Blueprint Course reference"
);
impl_numeric_reference!(
    AssessmentAttemptReference,
    "R",
    "Assessment Attempt reference"
);
impl_numeric_reference!(
    AuthoringWorkspaceReference,
    "W",
    "Authoring Workspace reference"
);
impl_numeric_reference!(
    CourseMembershipReference,
    "M",
    "course-membership reference"
);
impl_numeric_reference!(
    CourseInvitationReference,
    "I",
    "Course Invitation reference"
);

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
    use crate::QuestionId;

    #[test]
    fn public_ids_are_exact_checksum_validated_values() {
        type PublicIdParser = fn(&str) -> bool;

        fn question(value: &str) -> bool {
            value.parse::<QuestionId>().is_ok()
        }
        fn blueprint(value: &str) -> bool {
            value.parse::<BlueprintCourseReference>().is_ok()
        }
        fn course(value: &str) -> bool {
            value.parse::<CourseInstanceReference>().is_ok()
        }
        fn assessment(value: &str) -> bool {
            value.parse::<AssessmentReference>().is_ok()
        }
        fn account(value: &str) -> bool {
            value.parse::<AccountReference>().is_ok()
        }

        let cases: [(&str, PublicIdParser); 5] = [
            ("ABCD-XEFG", question),
            ("BPABCDEFGJ", blueprint),
            ("CIABCDEFGS", course),
            ("AABCDEFG8", assessment),
            ("UABCDEFGM", account),
        ];
        for (canonical, parses) in cases {
            assert!(parses(canonical), "{canonical}");
            assert!(!parses(&format!("{}0", &canonical[..canonical.len() - 1])));
            assert!(!parses(&canonical.to_ascii_lowercase()));
            assert!(!parses(&format!(" {canonical}")));
            assert!(!parses(&format!("{canonical} ")));
        }

        assert_eq!(
            QuestionId::from_random_identifier("ABCDEFG")
                .expect("Question random identity")
                .to_string(),
            "ABCD-XEFG"
        );
        assert_eq!(
            BlueprintCourseReference::from_random_identity("ABCDEFG")
                .expect("Blueprint random identity")
                .to_string(),
            "BPABCDEFGJ"
        );
        assert_eq!(
            CourseInstanceReference::from_random_identity("ABCDEFG")
                .expect("Course random identity")
                .to_string(),
            "CIABCDEFGS"
        );
        let course: CourseInstanceReference = "CIABCDEFGS".parse().expect("Course reference");
        assert_eq!(course.as_string(), course.as_str());
        assert_eq!(
            AssessmentReference::from_random_identity("ABCDEFG")
                .expect("Assessment random identity")
                .to_string(),
            "AABCDEFG8"
        );
        assert_eq!(
            AccountReference::from_random_identity("ABCDEFG")
                .expect("Account random identity")
                .to_string(),
            "UABCDEFGM"
        );
        assert!(!RESERVED_REFERENCE_PREFIXES.contains(&"AC"));
    }
}
