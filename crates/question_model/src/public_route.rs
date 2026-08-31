//! Human-facing typed route references.
//!
//! These strings locate records; they are never authority.  The server resolves one inside the
//! authenticated course membership boundary before using its internal identity.

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use crate::{AssignmentAttemptId, AssignmentId, CourseId, StudentRecordId, WorkspaceId};

/// Largest route number that remains compact and lossless in every product layer.
pub const MAX_PUBLIC_ROUTE_NUMBER: u32 = i32::MAX as u32;

/// Prefixes reserved by the route grammar.
pub const RESERVED_REFERENCE_PREFIXES: &[&str] = &[
    "C", "A", "R", "W", "G", "U", "M", "CI", "QC", "QS", "BP", "GO",
];

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
pub struct CourseInstanceReference(NonZeroU32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssignmentReference(NonZeroU32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssignmentAttemptReference(NonZeroU32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AuthoringWorkspaceReference(NonZeroU32);
/// An authorized locator for an existing platform account. It carries neither email nor authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AccountReference(NonZeroU32);
/// An authorized locator for one course-membership episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseMembershipReference(NonZeroU32);
/// An authorized locator for one target-bound Course Invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseInvitationReference(NonZeroU32);
/// An authorized locator for one private Question Folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct QuestionFolderReference(NonZeroU32);
/// An authorized locator for one personal saved Question Search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SavedQuestionSearchReference(NonZeroU32);
/// An authorized locator for one reusable Blueprint Course.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlueprintCourseReference(NonZeroU32);
/// An authorized locator for one Instructor-facing automated-grading recovery thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct InstructorGradingOperationReference(NonZeroU32);

impl_reference!(CourseInstanceReference, "C", "Course Instance reference");
impl_reference!(AssignmentReference, "A", "assignment reference");
impl_reference!(
    AssignmentAttemptReference,
    "R",
    "Assignment Attempt reference"
);
impl_reference!(AuthoringWorkspaceReference, "W", "Authoring Workspace reference");
impl_reference!(AccountReference, "U", "account reference");
impl_reference!(
    CourseMembershipReference,
    "M",
    "course-membership reference"
);
impl_reference!(
    CourseInvitationReference,
    "CI",
    "Course Invitation reference"
);
impl_reference!(
    QuestionFolderReference,
    "QC",
    "question-folder reference"
);
impl_reference!(
    SavedQuestionSearchReference,
    "QS",
    "saved-question-search reference"
);
impl_reference!(BlueprintCourseReference, "BP", "Blueprint Course reference");
impl_reference!(
    InstructorGradingOperationReference,
    "GO",
    "grading-operation reference"
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
    Assignment {
        course_id: CourseId,
        assignment_id: AssignmentId,
    },
    AssignmentAttempt {
        course_id: CourseId,
        assignment_id: AssignmentId,
        student_record_id: StudentRecordId,
        assignment_attempt_id: AssignmentAttemptId,
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
        assert_reference_wire!(
            CourseInstanceReference,
            "C-123",
            "A-123",
            "C-0",
            "C-01",
            "C-2147483648"
        );
        assert_reference_wire!(
            AssignmentReference,
            "A-124",
            "C-124",
            "A-0",
            "A-01",
            "A-2147483648"
        );
        assert_reference_wire!(
            AssignmentAttemptReference,
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
        assert_reference_wire!(
            AccountReference,
            "U-128",
            "C-128",
            "U-0",
            "U-01",
            "U-2147483648"
        );
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
            "CI-130",
            "C-130",
            "CI-0",
            "CI-01",
            "CI-2147483648"
        );
        assert_reference_wire!(
            QuestionFolderReference,
            "QC-131",
            "QS-131",
            "QC-0",
            "QC-01",
            "QC-2147483648"
        );
        assert_reference_wire!(
            SavedQuestionSearchReference,
            "QS-132",
            "QC-132",
            "QS-0",
            "QS-01",
            "QS-2147483648"
        );
        assert_reference_wire!(
            BlueprintCourseReference,
            "BP-133",
            "GO-133",
            "BP-0",
            "BP-01",
            "BP-2147483648"
        );
        assert_reference_wire!(
            InstructorGradingOperationReference,
            "GO-135",
            "A-135",
            "GO-0",
            "GO-01",
            "GO-2147483648"
        );
        assert!(!RESERVED_REFERENCE_PREFIXES.contains(&"AC"));
        assert!(RESERVED_REFERENCE_PREFIXES.contains(&"GO"));
    }
}
