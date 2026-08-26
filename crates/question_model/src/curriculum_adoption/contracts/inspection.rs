//! Closed, answer-free import provenance projections for Instructor inspection.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    AssignmentDefinitionSourceView, CourseScheduleWitness, CurriculumAdoptionTitle,
    CurriculumImportRevision, ObservedAlphaSource, ObservedAssignmentRevision,
    bounded::deserialize_course_imports,
};
use crate::{AssignmentReference, CourseTerm, MAX_ASSIGNMENT_ORDERED_ENTRIES};

/// Exact source provenance for one imported teaching assignment.
///
/// The tagged union prevents a reusable-definition import from carrying rollover-only evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CurriculumAssignmentImportSourceView {
    /// An exact revision-bound reusable Blueprint or Alpha definition.
    Reusable {
        /// The exact source definition and its observed reusable revision.
        definition: AssignmentDefinitionSourceView,
    },
    /// One exact source-course assignment observed through its course schedule witness.
    Rollover {
        /// The coupled source-course schedule and exact source-assignment witness.
        source: RolloverAssignmentSourceView,
    },
}

/// An exact source assignment observed during a course rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RolloverAssignmentSourceView {
    assignment: ObservedAssignmentRevision,
}

impl RolloverAssignmentSourceView {
    /// Builds an assignment-local view after checking the supplied rollover witness.
    ///
    /// The course view owns that witness on the wire and rechecks this relation when it
    /// assembles the complete inspection projection.
    pub fn new(
        source_schedule: &CourseScheduleWitness,
        assignment: ObservedAssignmentRevision,
    ) -> Result<Self, RolloverAssignmentSourceViewError> {
        if !source_schedule.contains_assignment(assignment) {
            return Err(RolloverAssignmentSourceViewError);
        }
        Ok(Self { assignment })
    }

    /// Returns the exact source assignment and definition revision.
    pub const fn assignment(&self) -> ObservedAssignmentRevision {
        self.assignment
    }
}

/// Rollover provenance named an assignment outside its source-course schedule witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolloverAssignmentSourceViewError;

impl std::fmt::Display for RolloverAssignmentSourceViewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("rollover assignment source is absent from its schedule witness")
    }
}

impl std::error::Error for RolloverAssignmentSourceViewError {}

/// Closed provenance for the inspected teaching course itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CurriculumCourseImportOriginView {
    /// An ordinary course that contains assignment-level reusable imports.
    Ordinary,
    /// A course instantiated from one public Alpha revision.
    Alpha {
        /// The exact public Alpha source and revision that established the course.
        source: ObservedAlphaSource,
    },
    /// A course created by rollover from one observed source-course schedule witness.
    Rollover {
        /// The exact source-course schedule witness observed at rollover.
        source: RolloverCourseImportOriginView,
    },
}

/// Safe source-course provenance for a rollover-created destination course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RolloverCourseImportOriginView {
    /// The exact source course, schedule revision, and assignment revisions observed at rollover.
    pub source_schedule: CourseScheduleWitness,
}

/// One answer-free durable import binding for one destination assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumImportView {
    /// Destination assignment that owns this import.
    pub assignment: AssignmentReference,
    /// Current human-facing destination assignment title.
    pub title: CurriculumAdoptionTitle,
    /// Closed exact source evidence appropriate to this import kind.
    pub source: CurriculumAssignmentImportSourceView,
    /// Revision advanced whenever the import baseline/envelope changes.
    pub revision: CurriculumImportRevision,
    /// Whether current destination reusable meaning still equals its immutable baseline.
    pub reusable_meaning_matches_baseline: bool,
}

/// Answer-free durable course-import inspection view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "CurriculumCourseImportViewParts")]
pub struct CurriculumCourseImportView {
    /// Complete current destination course and assignment-revision concurrency evidence.
    pub witness: CourseScheduleWitness,
    /// Closed provenance for the course creation operation, if any.
    pub origin: CurriculumCourseImportOriginView,
    /// Current destination term and authoritative IANA zone.
    pub term: CourseTerm,
    /// Imported subset in deterministic teaching-assignment order.
    pub assignments: Vec<CurriculumImportView>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CurriculumCourseImportViewParts {
    witness: CourseScheduleWitness,
    origin: CurriculumCourseImportOriginView,
    term: CourseTerm,
    #[serde(deserialize_with = "deserialize_course_imports")]
    assignments: Vec<CurriculumImportView>,
}

impl TryFrom<CurriculumCourseImportViewParts> for CurriculumCourseImportView {
    type Error = CurriculumCourseImportViewError;

    fn try_from(value: CurriculumCourseImportViewParts) -> Result<Self, Self::Error> {
        Self::new(value.witness, value.origin, value.term, value.assignments)
    }
}

impl CurriculumCourseImportView {
    /// Builds a bounded inspection view whose imported subset belongs to the current witness.
    pub fn new(
        witness: CourseScheduleWitness,
        origin: CurriculumCourseImportOriginView,
        term: CourseTerm,
        assignments: Vec<CurriculumImportView>,
    ) -> Result<Self, CurriculumCourseImportViewError> {
        // Store returns no inspection projection until it has at least one import to show.
        if assignments.is_empty() {
            return Err(CurriculumCourseImportViewError::EmptyAssignments);
        }
        if assignments.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(CurriculumCourseImportViewError::TooManyAssignments);
        }
        let mut references = BTreeSet::new();
        if assignments
            .iter()
            .any(|import| !references.insert(import.assignment))
        {
            return Err(CurriculumCourseImportViewError::DuplicateAssignment);
        }
        let witnessed_assignments = witness
            .assignment_revisions()
            .iter()
            .map(|assignment| assignment.assignment)
            .collect::<BTreeSet<_>>();
        if assignments
            .iter()
            .any(|import| !witnessed_assignments.contains(&import.assignment))
        {
            return Err(CurriculumCourseImportViewError::ImportAbsentFromWitness);
        }
        Self::validate_rollover_sources(&origin, &assignments)?;
        Ok(Self {
            witness,
            origin,
            term,
            assignments,
        })
    }

    /// Returns the bounded imports in deterministic teaching-assignment order.
    pub fn assignments(&self) -> &[CurriculumImportView] {
        &self.assignments
    }

    fn validate_rollover_sources(
        origin: &CurriculumCourseImportOriginView,
        assignments: &[CurriculumImportView],
    ) -> Result<(), CurriculumCourseImportViewError> {
        let rollover_witness = match origin {
            CurriculumCourseImportOriginView::Rollover { source } => Some(&source.source_schedule),
            CurriculumCourseImportOriginView::Ordinary
            | CurriculumCourseImportOriginView::Alpha { .. } => None,
        };
        for import in assignments {
            let CurriculumAssignmentImportSourceView::Rollover { source } = &import.source else {
                continue;
            };
            let Some(witness) = rollover_witness else {
                return Err(CurriculumCourseImportViewError::RolloverSourceWithoutCourseOrigin);
            };
            if !witness.contains_assignment(source.assignment()) {
                return Err(CurriculumCourseImportViewError::RolloverSourceAbsentFromOrigin);
            }
        }
        Ok(())
    }
}

/// A course import inspection view was empty, exceeded its bound, or had inconsistent provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurriculumCourseImportViewError {
    /// No inspected assignment exists for this course.
    EmptyAssignments,
    /// More than one course can hold under the shared assignment bound.
    TooManyAssignments,
    /// The same destination assignment appeared more than once.
    DuplicateAssignment,
    /// An imported assignment is absent from the complete current teaching witness.
    ImportAbsentFromWitness,
    /// A rollover assignment appeared in a course without rollover provenance.
    RolloverSourceWithoutCourseOrigin,
    /// A rollover assignment was not observed in the course-origin witness.
    RolloverSourceAbsentFromOrigin,
}

impl std::fmt::Display for CurriculumCourseImportViewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAssignments => formatter.write_str("course import view has no assignments"),
            Self::TooManyAssignments => {
                formatter.write_str("course import view has too many assignments")
            }
            Self::DuplicateAssignment => {
                formatter.write_str("course import view repeats an assignment")
            }
            Self::ImportAbsentFromWitness => {
                formatter.write_str("course import is absent from the current teaching witness")
            }
            Self::RolloverSourceWithoutCourseOrigin => {
                formatter.write_str("rollover assignment source requires a rollover course origin")
            }
            Self::RolloverSourceAbsentFromOrigin => formatter
                .write_str("rollover assignment source is absent from the course-origin witness"),
        }
    }
}

impl std::error::Error for CurriculumCourseImportViewError {}
