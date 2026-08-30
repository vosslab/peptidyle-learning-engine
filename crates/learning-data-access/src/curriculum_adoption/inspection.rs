//! Canonical evidence validation and answer-free CourseInstance inspection.

use question_model::{
    BlueprintAssignmentProvenance, CourseInstanceBlueprintApplication,
    CourseInstanceBlueprintInspectionView, CourseInstanceWitness,
};

use super::semantic_snapshot::SemanticPlannerError;

#[cfg(feature = "postgres")]
use super::semantic_snapshot::SemanticEvidenceMismatch;
#[cfg(feature = "postgres")]
use question_model::curriculum_adoption::CurriculumSemanticPayload;

/// Persisted canonical facts observed by an adapter alongside normalized meaning.
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObservedSemanticEnvelope<'a> {
    pub(crate) canonical_version: u8,
    pub(crate) canonical_bytes: &'a [u8],
    pub(crate) digest: [u8; 32],
}

/// Validates every persisted canonical fact against qmodel-owned normalized meaning.
#[cfg(feature = "postgres")]
pub(crate) fn validate_semantic_evidence(
    payload: &CurriculumSemanticPayload,
    observed: ObservedSemanticEnvelope<'_>,
) -> Result<(), SemanticPlannerError> {
    let expected = payload.canonical_envelope();
    if observed.canonical_version != expected.version() {
        return Err(SemanticPlannerError::InvalidEvidence(
            SemanticEvidenceMismatch::CanonicalVersion,
        ));
    }
    if observed.canonical_bytes != expected.canonical_bytes() {
        return Err(SemanticPlannerError::InvalidEvidence(
            SemanticEvidenceMismatch::CanonicalBytes,
        ));
    }
    if observed.digest != expected.digest().as_bytes() {
        return Err(SemanticPlannerError::InvalidEvidence(
            SemanticEvidenceMismatch::Digest,
        ));
    }
    Ok(())
}

/// Adapter-resolved, answer-free evidence for the current CourseInstance inspection.
pub(crate) struct CourseInstanceInspectionInput {
    pub(crate) initial_blueprint_application: CourseInstanceBlueprintApplication,
    pub(crate) witness: CourseInstanceWitness,
    pub(crate) assignments: Vec<BlueprintAssignmentProvenance>,
}

/// Assembles the bounded current inspection projection without reviving import DTOs.
pub(crate) fn project_course_instance_blueprint_inspection(
    input: CourseInstanceInspectionInput,
) -> Result<CourseInstanceBlueprintInspectionView, SemanticPlannerError> {
    Ok(CourseInstanceBlueprintInspectionView {
        initial_blueprint_application: input.initial_blueprint_application,
        witness: input.witness,
        assignments: input.assignments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        AssignmentDefinitionSourceView, AssignmentReference, AssignmentRevision,
        BlueprintAssignmentId, BlueprintReference, BlueprintRevision, CourseReference,
        CourseScheduleRevision, CurriculumImportRevision, ObservedBlueprintSource,
        ObservedCourseInstanceAssignment,
    };
    use uuid::Uuid;

    fn source() -> ObservedBlueprintSource {
        ObservedBlueprintSource {
            reference: BlueprintReference::new(7).expect("blueprint reference"),
            revision: BlueprintRevision::new(2).expect("blueprint revision"),
        }
    }

    #[test]
    fn inspection_keeps_initial_application_separate_from_assignment_provenance() {
        let source = source();
        let other = ObservedBlueprintSource {
            revision: BlueprintRevision::new(3).expect("other revision"),
            ..source
        };
        let input = CourseInstanceInspectionInput {
            initial_blueprint_application: CourseInstanceBlueprintApplication { source },
            witness: CourseInstanceWitness::new(
                CourseReference::new(8).expect("course"),
                CourseScheduleRevision::new(1).expect("schedule revision"),
                vec![ObservedCourseInstanceAssignment {
                    assignment: AssignmentReference::new(9).expect("assignment"),
                    revision: AssignmentRevision::new(1).expect("assignment revision"),
                }],
            )
            .expect("bounded witness"),
            assignments: vec![BlueprintAssignmentProvenance {
                source: AssignmentDefinitionSourceView::new(
                    other,
                    BlueprintAssignmentId::from_uuid(Uuid::from_u128(10)),
                ),
                import_revision: "1".parse::<CurriculumImportRevision>().expect("import"),
            }],
        };

        let view = project_course_instance_blueprint_inspection(input).expect("inspection");
        assert_eq!(view.initial_blueprint_application.source, source);
        assert_eq!(view.assignments[0].source.source(), other);
        assert_eq!(
            view.assignments[0].source.assignment_id(),
            BlueprintAssignmentId::from_uuid(Uuid::from_u128(10))
        );
    }
}
