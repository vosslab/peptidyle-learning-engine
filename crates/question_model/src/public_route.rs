//! Human-facing typed public IDs used in routes.
//!
//! These strings identify records; they are never authority.  The server resolves one inside the
//! authenticated course membership boundary before using its internal identity.

use serde::{Deserialize, Serialize};

use crate::question_library::{QUESTION_ID_ALPHABET, public_id_checksum_character};
use crate::{AssessmentAttemptId, StudentRecordId, WorkspaceId};

/// Prefixes reserved by the public-ID grammar. Compact hyphenated locators are
/// not public IDs; the hyphen is specific to Question IDs.
pub const RESERVED_PUBLIC_ID_PREFIXES: &[&str] = &["G", "U", "QC", "QS", "BP", "CI", "A"];

/// Every public ID includes seven server-random Crockford characters and one
/// public SHA-256 checksum character.
pub const PUBLIC_ID_RANDOM_LENGTH: usize = 7;

macro_rules! impl_public_id {
    ($name:ident, $wire_prefix:literal, $checksum_prefix:literal, $description:literal) => {
        impl $name {
            /// Validates the exact canonical public ID returned by a data boundary.
            pub fn new(value: impl AsRef<str>) -> Result<Self, &'static str> {
                value.as_ref().parse()
            }

            /// Mints a public ID from seven server-random Crockford characters.
            ///
            /// ASVS V2.1.1 and V2.2.1: callers must use a cryptographically
            /// secure source for the supplied random characters.
            pub fn from_random_identity(identity: impl AsRef<str>) -> Result<Self, &'static str> {
                let identity = identity.as_ref();
                if identity.len() != PUBLIC_ID_RANDOM_LENGTH
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

            /// Builds a canonical ID from a dense serial. Production minting
            /// uses cryptographically random Crockford characters.
            pub fn from_debug_serial(serial: u128) -> Self {
                Self::from_random_identity(crockford_serial(serial))
                    .expect("debug serial encodes to Crockford")
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
                    || suffix.len() != PUBLIC_ID_RANDOM_LENGTH + 1
                    || !suffix
                        .bytes()
                        .all(|character| QUESTION_ID_ALPHABET.contains(&character))
                {
                    return Err(concat!(
                        $description,
                        " must use its exact canonical syntax"
                    ));
                }
                let checksum_input =
                    format!("{}{}", $checksum_prefix, &suffix[..PUBLIC_ID_RANDOM_LENGTH]);
                if suffix.as_bytes()[PUBLIC_ID_RANDOM_LENGTH]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInstanceId(String);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssessmentId(String);
/// An authorized Account ID for an existing platform account. It carries neither email nor authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AccountId(String);
/// An authorized Blueprint Course ID for one reusable Blueprint Course.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintCourseId(String);

impl_public_id!(CourseInstanceId, "CI", "CI", "Course Instance ID");
impl_public_id!(AssessmentId, "A", "A", "Assessment ID");
impl_public_id!(AccountId, "U", "U", "Account ID");
impl_public_id!(BlueprintCourseId, "BP", "BP", "Blueprint Course ID");

fn crockford_serial(mut serial: u128) -> String {
    let mut chars = [b'0'; 7];
    for index in (0..7).rev() {
        chars[index] = QUESTION_ID_ALPHABET[(serial % 32) as usize];
        serial /= 32;
    }
    String::from_utf8(chars.to_vec()).expect("Crockford alphabet is ASCII")
}

/// One authorized navigation target. IDs remain transport details after Store authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NavigationResolution {
    Course {
        course_instance_id: CourseInstanceId,
    },
    Assessment {
        course_instance_id: CourseInstanceId,
        assessment_id: AssessmentId,
    },
    AssessmentAttempt {
        course_instance_id: CourseInstanceId,
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
    use crate::PublishedQuestionId;

    #[test]
    fn public_ids_are_exact_checksum_validated_values() {
        type PublicIdParser = fn(&str) -> bool;

        fn question(value: &str) -> bool {
            value.parse::<PublishedQuestionId>().is_ok()
        }
        fn blueprint(value: &str) -> bool {
            value.parse::<BlueprintCourseId>().is_ok()
        }
        fn course(value: &str) -> bool {
            value.parse::<CourseInstanceId>().is_ok()
        }
        fn assessment(value: &str) -> bool {
            value.parse::<AssessmentId>().is_ok()
        }
        fn account(value: &str) -> bool {
            value.parse::<AccountId>().is_ok()
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
            PublishedQuestionId::from_random_identifier("ABCDEFG")
                .expect("Question random identity")
                .to_string(),
            "ABCD-XEFG"
        );
        assert_eq!(
            BlueprintCourseId::from_random_identity("ABCDEFG")
                .expect("Blueprint random identity")
                .to_string(),
            "BPABCDEFGJ"
        );
        assert_eq!(
            CourseInstanceId::from_random_identity("ABCDEFG")
                .expect("Course Instance random identity")
                .to_string(),
            "CIABCDEFGS"
        );
        let course_instance_id: CourseInstanceId =
            "CIABCDEFGS".parse().expect("Course Instance ID");
        assert_eq!(course_instance_id.as_string(), course_instance_id.as_str());
        assert_eq!(
            AssessmentId::from_random_identity("ABCDEFG")
                .expect("Assessment random identity")
                .to_string(),
            "AABCDEFG8"
        );
        assert_eq!(
            AccountId::from_random_identity("ABCDEFG")
                .expect("Account random identity")
                .to_string(),
            "UABCDEFGM"
        );
        assert!(!RESERVED_PUBLIC_ID_PREFIXES.contains(&"AC"));
    }
}
