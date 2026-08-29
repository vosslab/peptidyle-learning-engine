//! Internal entitlement facts and materialization evidence.
//!
//! These contracts are deliberately not serialized.  The browser asks the
//! server to perform an action; it never receives a reusable authority token
//! or a roster/group explanation.

use serde::{Deserialize, Serialize};

use crate::{ActivityTimestamp, CourseGroupId, CourseMembershipId, EnrollmentId, UserId};
/// Closed meaning of a course group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseGroupPurpose {
    Section,
    Lab,
    Cohort,
    Accommodation,
    Work,
}

/// Closed outcome policy when one learner belongs to multiple groups of a purpose.
///
/// This policy informs an instructor about a potentially surprising roster
/// shape. It never changes the validity of a membership write or entitlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MultipleMembershipPolicy {
    /// Preserve the membership and report no informational warning.
    Allow,
    /// Preserve the membership and report an informational warning.
    Warn,
}

impl MultipleMembershipPolicy {
    /// Returns the closed default for a group purpose.
    pub const fn default_for_purpose(purpose: CourseGroupPurpose) -> Self {
        match purpose {
            CourseGroupPurpose::Section => Self::Warn,
            CourseGroupPurpose::Lab
            | CourseGroupPurpose::Cohort
            | CourseGroupPurpose::Accommodation
            | CourseGroupPurpose::Work => Self::Allow,
        }
    }
}

/// One course-owned policy value for one closed group purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseGroupPurposePolicy {
    /// Purpose whose memberships this policy evaluates.
    pub purpose: CourseGroupPurpose,
    /// Informational behavior for multiple memberships of that purpose.
    pub multiple_membership: MultipleMembershipPolicy,
}

impl CourseGroupPurposePolicy {
    /// Constructs the default policy for one purpose.
    pub const fn default_for_purpose(purpose: CourseGroupPurpose) -> Self {
        Self {
            purpose,
            multiple_membership: MultipleMembershipPolicy::default_for_purpose(purpose),
        }
    }
}

/// Deterministic, non-blocking result of evaluating a membership write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MultipleMembershipDisposition {
    /// The valid membership write needs no informational warning.
    Allowed,
    /// The valid membership write is retained and should show a warning.
    AllowedWithWarning,
}

impl MultipleMembershipDisposition {
    /// Whether the valid membership write remains permitted.
    pub const fn permits_write(self) -> bool {
        true
    }
}

/// Total, non-persisted capability mapping for a group purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupPurposeCapabilities {
    pub assignment_audience: bool,
    pub schedule_scope: bool,
    pub accommodation_scope: bool,
    pub student_visible: bool,
}

impl GroupPurposeCapabilities {
    pub const fn for_purpose(purpose: CourseGroupPurpose) -> Self {
        match purpose {
            CourseGroupPurpose::Section | CourseGroupPurpose::Lab | CourseGroupPurpose::Cohort => {
                Self {
                    assignment_audience: true,
                    schedule_scope: true,
                    accommodation_scope: false,
                    student_visible: true,
                }
            }
            CourseGroupPurpose::Accommodation => Self {
                assignment_audience: false,
                schedule_scope: false,
                accommodation_scope: true,
                student_visible: false,
            },
            CourseGroupPurpose::Work => Self {
                assignment_audience: false,
                schedule_scope: false,
                accommodation_scope: false,
                student_visible: false,
            },
        }
    }
}

/// Explicit assignment audience. Group audience is OR over the listed groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentAudience {
    CourseWide,
    AnyOfGroups(NonEmptyAudienceGroups),
}

/// Validated, canonical group set for an `AnyOfGroups` audience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyAudienceGroups(Vec<CourseGroupId>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentAudienceError {
    Empty,
    Duplicate,
}

impl AssignmentAudience {
    pub fn any_of_groups(mut groups: Vec<CourseGroupId>) -> Result<Self, AssignmentAudienceError> {
        if groups.is_empty() {
            return Err(AssignmentAudienceError::Empty);
        }
        groups.sort_unstable();
        if groups.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AssignmentAudienceError::Duplicate);
        }
        Ok(Self::AnyOfGroups(NonEmptyAudienceGroups(groups)))
    }
}

impl NonEmptyAudienceGroups {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = CourseGroupId> + '_ {
        self.0.iter().copied()
    }
}

/// Basis that granted the current assignment audience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationBasis {
    CourseWide,
    GroupAudience {
        group: CourseGroupId,
        purpose: CourseGroupPurpose,
    },
}

/// Event which is allowed to mint an educational receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementPurpose {
    StartRun,
    GradeBearingAction,
    InstructorIssue,
}

/// Closed non-person authority that can materialize a grade-bearing receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationRule {
    ImportedGrade,
    AutomatedGrader,
}

/// Who or what justified the immutable receipt creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationAuthority {
    Actor(UserId),
    Rule(MaterializationRule),
}

/// Version of the pure evaluator used when a receipt was first materialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluatorVersion(pub u16);

impl EvaluatorVersion {
    pub const INITIAL: Self = Self(1);
}

/// Immutable receipt provenance, separate from mutable scoring fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementMaterialization {
    pub enrollment: EnrollmentId,
    pub membership: CourseMembershipId,
    pub user: UserId,
    pub occurred_at: ActivityTimestamp,
    pub purpose: EntitlementPurpose,
    pub authority: MaterializationAuthority,
    pub basis: MaterializationBasis,
    pub evaluator_version: EvaluatorVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationDisposition {
    Created,
    Existing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purpose_capabilities_are_closed_and_total() {
        assert_eq!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Section),
            GroupPurposeCapabilities {
                assignment_audience: true,
                schedule_scope: true,
                accommodation_scope: false,
                student_visible: true,
            }
        );
        assert_eq!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Lab),
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Section)
        );
        assert_eq!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Cohort),
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Section)
        );
        assert_eq!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Accommodation),
            GroupPurposeCapabilities {
                assignment_audience: false,
                schedule_scope: false,
                accommodation_scope: true,
                student_visible: false,
            }
        );
        assert_eq!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Work),
            GroupPurposeCapabilities {
                assignment_audience: false,
                schedule_scope: false,
                accommodation_scope: false,
                student_visible: false,
            }
        );
    }

    #[test]
    fn group_audience_is_nonempty_and_duplicate_free() {
        let group = CourseGroupId::from_uuid(uuid::Uuid::from_u128(1));
        assert_eq!(
            AssignmentAudience::any_of_groups(Vec::new()),
            Err(AssignmentAudienceError::Empty)
        );
        assert_eq!(
            AssignmentAudience::any_of_groups(vec![group, group]),
            Err(AssignmentAudienceError::Duplicate)
        );
    }

    #[test]
    fn purpose_policy_is_a_closed_strict_wire_shape() {
        let policy = CourseGroupPurposePolicy::default_for_purpose(CourseGroupPurpose::Section);
        assert_eq!(
            serde_json::to_string(&policy).expect("policy should serialize"),
            r#"{"purpose":"section","multipleMembership":"warn"}"#
        );
        assert!(
            serde_json::from_str::<CourseGroupPurposePolicy>(
                r#"{"purpose":"section","multipleMembership":"warn","extra":true}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<MultipleMembershipPolicy>(r#""block""#).is_err());
    }
}
