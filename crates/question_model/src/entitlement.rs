//! Internal entitlement facts and materialization evidence.
//!
//! These contracts are deliberately not serialized.  The browser asks the
//! server to perform an action; it never receives a reusable authority token
//! or a roster/group explanation.

use crate::{ActivityTimestamp, CourseGroupId, CourseMembershipId, EnrollmentId, UserId};
/// Closed meaning of a course group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CourseGroupPurpose {
    Section,
    Lab,
    Cohort,
    Accommodation,
    Work,
}

/// Total, non-persisted capability mapping for a group purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupPurposeCapabilities {
    pub assignment_audience: bool,
    pub schedule_scope: bool,
    pub accommodation_scope: bool,
    pub learner_visible: bool,
}

impl GroupPurposeCapabilities {
    pub const fn for_purpose(purpose: CourseGroupPurpose) -> Self {
        match purpose {
            CourseGroupPurpose::Section | CourseGroupPurpose::Lab | CourseGroupPurpose::Cohort => {
                Self {
                    assignment_audience: true,
                    schedule_scope: true,
                    accommodation_scope: false,
                    learner_visible: true,
                }
            }
            CourseGroupPurpose::Accommodation => Self {
                assignment_audience: false,
                schedule_scope: false,
                accommodation_scope: true,
                learner_visible: false,
            },
            CourseGroupPurpose::Work => Self {
                assignment_audience: false,
                schedule_scope: false,
                accommodation_scope: false,
                learner_visible: false,
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
        assert!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Section).assignment_audience
        );
        assert!(
            GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Accommodation)
                .accommodation_scope
        );
        assert!(
            !GroupPurposeCapabilities::for_purpose(CourseGroupPurpose::Work).assignment_audience
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
}
