//! Revision-bound source and destination witnesses for curriculum adoption.

use serde::{Deserialize, Serialize};

use super::super::CourseScheduleRevision;
use super::bounded::deserialize_assignment_witnesses;
use crate::{
    AlphaCourseReference, AlphaCourseRevision, AssignmentReference, AssignmentRevision,
    BlueprintReference, BlueprintRevision, CourseReference, MAX_ASSIGNMENT_ORDERED_ENTRIES,
};

/// A revision-bound Blueprint source observed through the authorized read plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedBlueprintSource {
    /// Route locator resolved under the current authorized owner boundary.
    pub reference: BlueprintReference,
    /// Complete source revision selected for preview or write.
    pub revision: BlueprintRevision,
}

/// A revision-bound public Alpha source observed through the authorized read plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedAlphaSource {
    /// Public route locator resolved under approved-Instructor authority.
    pub reference: AlphaCourseReference,
    /// Complete Alpha tree revision selected for preview or write.
    pub revision: AlphaCourseRevision,
}

/// An ordinary assignment revision observed in a course schedule preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedAssignmentRevision {
    /// Assignment route locator resolved inside the previewed course.
    pub assignment: AssignmentReference,
    /// Exact assignment-definition revision observed by the preview.
    pub revision: AssignmentRevision,
}

/// All revision evidence a whole-course schedule preview binds to apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "CourseScheduleWitnessParts")]
pub struct CourseScheduleWitness {
    /// Course route locator resolved under direct Instructor authority.
    pub course: CourseReference,
    /// Schedule revision advanced by every course-term or base-schedule writer.
    pub schedule_revision: CourseScheduleRevision,
    assignment_revisions: Vec<ObservedAssignmentRevision>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CourseScheduleWitnessParts {
    course: CourseReference,
    schedule_revision: CourseScheduleRevision,
    #[serde(deserialize_with = "deserialize_assignment_witnesses")]
    assignment_revisions: Vec<ObservedAssignmentRevision>,
}

impl TryFrom<CourseScheduleWitnessParts> for CourseScheduleWitness {
    type Error = CourseScheduleWitnessError;

    fn try_from(value: CourseScheduleWitnessParts) -> Result<Self, Self::Error> {
        Self::new(
            value.course,
            value.schedule_revision,
            value.assignment_revisions,
        )
    }
}

impl CourseScheduleWitness {
    /// Builds a bounded deterministic witness, rejecting duplicate assignment bindings.
    pub fn new(
        course: CourseReference,
        schedule_revision: CourseScheduleRevision,
        mut assignment_revisions: Vec<ObservedAssignmentRevision>,
    ) -> Result<Self, CourseScheduleWitnessError> {
        if assignment_revisions.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(CourseScheduleWitnessError::TooManyAssignments);
        }
        assignment_revisions.sort_unstable();
        if assignment_revisions
            .windows(2)
            .any(|pair| pair[0].assignment == pair[1].assignment)
        {
            return Err(CourseScheduleWitnessError::DuplicateAssignment);
        }
        Ok(Self {
            course,
            schedule_revision,
            assignment_revisions,
        })
    }

    /// Returns assignment revision bindings in deterministic route-reference order.
    pub fn assignment_revisions(&self) -> &[ObservedAssignmentRevision] {
        &self.assignment_revisions
    }

    /// Returns whether this exact course witness binds the assignment and revision.
    pub fn contains_assignment(&self, assignment: ObservedAssignmentRevision) -> bool {
        self.assignment_revisions.binary_search(&assignment).is_ok()
    }
}

/// A course-schedule preview repeated one assignment witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseScheduleWitnessError {
    /// The witness exceeded the shared bound for one course operation.
    TooManyAssignments,
    /// One assignment had more than one revision in the same preview witness.
    DuplicateAssignment,
}

impl std::fmt::Display for CourseScheduleWitnessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyAssignments => {
                formatter.write_str("course schedule witness has too many assignments")
            }
            Self::DuplicateAssignment => {
                formatter.write_str("course schedule witness repeats an assignment")
            }
        }
    }
}

impl std::error::Error for CourseScheduleWitnessError {}

/// Browser-safe source descriptor with public locators and observed revisions only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CurriculumSourceView {
    /// A private owner-scoped Blueprint selected under the current session.
    Blueprint(ObservedBlueprintSource),
    /// A public Alpha selected under approved-Instructor authority.
    Alpha(ObservedAlphaSource),
}
