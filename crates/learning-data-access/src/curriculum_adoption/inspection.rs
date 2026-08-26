//! Canonical evidence validation and answer-free import inspection projection.

use question_model::curriculum_adoption::{
    CurriculumSemanticComparison, CurriculumSemanticPayload,
};
use question_model::{
    AssignmentReference, CourseScheduleWitness, CourseTerm, CurriculumAdoptionTitle,
    CurriculumAssignmentImportSourceView, CurriculumCourseImportOriginView,
    CurriculumCourseImportView, CurriculumImportRevision, CurriculumImportView,
};

use super::semantic_snapshot::{SemanticEvidenceMismatch, SemanticPlannerError};

/// Persisted canonical facts observed by an adapter alongside normalized meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObservedSemanticEnvelope<'a> {
    pub(crate) canonical_version: u8,
    pub(crate) canonical_bytes: &'a [u8],
    pub(crate) digest: [u8; 32],
}

/// Validates every persisted canonical fact against qmodel-owned normalized meaning.
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

/// Adapter-resolved facts for one destination assignment's current teaching projection.
pub(crate) struct CurrentTeachingImportInput<'a> {
    pub(crate) assignment: AssignmentReference,
    pub(crate) source: CurriculumAssignmentImportSourceView,
    pub(crate) revision: CurriculumImportRevision,
    pub(crate) baseline: &'a CurriculumSemanticPayload,
    pub(crate) baseline_evidence: ObservedSemanticEnvelope<'a>,
    pub(crate) current: &'a CurriculumSemanticPayload,
}

/// Validates immutable evidence and compares current reusable meaning without exposing answers.
pub(crate) fn project_current_teaching_import(
    input: CurrentTeachingImportInput<'_>,
) -> Result<CurriculumImportView, SemanticPlannerError> {
    let CurriculumSemanticPayload::Assignment(current_assignment) = input.current else {
        return Err(SemanticPlannerError::InvalidInspection(
            "assignment inspection requires assignment semantic payloads".into(),
        ));
    };
    if !matches!(input.baseline, CurriculumSemanticPayload::Assignment(_)) {
        return Err(SemanticPlannerError::InvalidInspection(
            "assignment inspection requires assignment semantic payloads".into(),
        ));
    }
    validate_semantic_evidence(input.baseline, input.baseline_evidence)?;
    Ok(CurriculumImportView {
        assignment: input.assignment,
        title: CurriculumAdoptionTitle::parse(current_assignment.title()).map_err(|_| {
            SemanticPlannerError::InvalidInspection(
                "current assignment title violates the shared curriculum title contract".into(),
            )
        })?,
        source: input.source,
        revision: input.revision,
        reusable_meaning_matches_baseline: matches!(
            input.baseline.compare(input.current),
            CurriculumSemanticComparison::Equivalent { .. }
        ),
    })
}

/// Adapter-resolved course facts and deterministic assignment projections.
pub(crate) struct CurriculumImportInspectionInput {
    pub(crate) witness: CourseScheduleWitness,
    pub(crate) origin: CurriculumCourseImportOriginView,
    pub(crate) term: CourseTerm,
    pub(crate) assignments: Vec<CurriculumImportView>,
}

/// Assembles the bounded public course inspection under qmodel provenance checks.
pub(crate) fn project_curriculum_import_inspection(
    input: CurriculumImportInspectionInput,
) -> Result<CurriculumCourseImportView, SemanticPlannerError> {
    CurriculumCourseImportView::new(input.witness, input.origin, input.term, input.assignments)
        .map_err(|error| SemanticPlannerError::InvalidInspection(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::{
        CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry,
    };
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
        BlueprintReference, BlueprintRevision, CompletionRequirement, ContinuedPractice,
        CourseReference, CourseScheduleRevision, CourseScheduleWitness, GradePolicy,
        LateSubmissionPolicy, LearnerDisclosurePolicy, ObservedAssignmentRevision,
        ObservedBlueprintSource, PointValue, ProblemId, ProblemVersionRef,
        RelativeAssignmentSchedule, ReusableAssignmentDefaults, RunPolicies, VariationPolicy,
        VersionId,
    };

    fn payload(title: &str) -> CurriculumSemanticPayload {
        CurriculumSemanticPayload::assignment(
            CurriculumSemanticAssignment::new(
                title.into(),
                AssignmentInstructions::default(),
                vec![CurriculumSemanticAssignmentEntry::Fixed {
                    reference: ProblemVersionRef {
                        problem: ProblemId::from_uuid(uuid::Uuid::from_u128(1)),
                        version: VersionId::from_uuid(uuid::Uuid::from_u128(2)),
                    },
                    points_possible: PointValue::from_whole(1),
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                ReusableAssignmentDefaults {
                    time_limit_seconds: None,
                    attempt_limit: None,
                    late_submission: LateSubmissionPolicy::Accept,
                    deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
                    run_policies: RunPolicies {
                        completion: CompletionRequirement::AnswerAll,
                        grade: GradePolicy::Highest,
                        continued_practice: ContinuedPractice::Unlimited,
                        variation: VariationPolicy::NewSeeds,
                    },
                    learner_disclosure: LearnerDisclosurePolicy::default(),
                },
                RelativeAssignmentSchedule::default(),
            )
            .expect("semantic assignment"),
        )
    }

    fn source() -> CurriculumAssignmentImportSourceView {
        CurriculumAssignmentImportSourceView::Reusable {
            definition: question_model::AssignmentDefinitionSourceView::Blueprint(
                ObservedBlueprintSource {
                    reference: BlueprintReference::new(1).expect("blueprint"),
                    revision: BlueprintRevision::new(2).expect("revision"),
                },
            ),
        }
    }

    #[test]
    fn canonical_evidence_rejects_each_tampered_envelope_field() {
        let payload = payload("Baseline");
        let envelope = payload.canonical_envelope();
        let valid = ObservedSemanticEnvelope {
            canonical_version: envelope.version(),
            canonical_bytes: envelope.canonical_bytes(),
            digest: envelope.digest().as_bytes(),
        };
        assert_eq!(validate_semantic_evidence(&payload, valid), Ok(()));
        let mut tampered_digest = envelope.digest().as_bytes();
        tampered_digest[0] ^= 1;
        let mismatches = [
            (
                ObservedSemanticEnvelope {
                    canonical_version: envelope.version().wrapping_add(1),
                    ..valid
                },
                SemanticEvidenceMismatch::CanonicalVersion,
            ),
            (
                ObservedSemanticEnvelope {
                    canonical_bytes: b"not canonical",
                    ..valid
                },
                SemanticEvidenceMismatch::CanonicalBytes,
            ),
            (
                ObservedSemanticEnvelope {
                    digest: tampered_digest,
                    ..valid
                },
                SemanticEvidenceMismatch::Digest,
            ),
        ];
        for (observed, mismatch) in mismatches {
            assert_eq!(
                validate_semantic_evidence(&payload, observed),
                Err(SemanticPlannerError::InvalidEvidence(mismatch))
            );
        }
    }

    #[test]
    fn current_teaching_projection_reports_equivalent_and_changed_meaning() {
        let baseline = payload("Baseline");
        let changed = payload("Changed");
        let envelope = baseline.canonical_envelope();
        let evidence = ObservedSemanticEnvelope {
            canonical_version: envelope.version(),
            canonical_bytes: envelope.canonical_bytes(),
            digest: envelope.digest().as_bytes(),
        };
        let project = |current| {
            project_current_teaching_import(CurrentTeachingImportInput {
                assignment: AssignmentReference::new(3).expect("assignment"),
                source: source(),
                revision: "4".parse().expect("import revision"),
                baseline: &baseline,
                baseline_evidence: evidence,
                current,
            })
            .expect("inspection projection")
        };
        assert!(project(&baseline).reusable_meaning_matches_baseline);
        assert!(!project(&changed).reusable_meaning_matches_baseline);
    }

    #[test]
    fn course_projection_preserves_qmodel_bounds_and_order() {
        let baseline = payload("Baseline");
        let envelope = baseline.canonical_envelope();
        let assignment = project_current_teaching_import(CurrentTeachingImportInput {
            assignment: AssignmentReference::new(3).expect("assignment"),
            source: source(),
            revision: "4".parse().expect("import revision"),
            baseline: &baseline,
            baseline_evidence: ObservedSemanticEnvelope {
                canonical_version: envelope.version(),
                canonical_bytes: envelope.canonical_bytes(),
                digest: envelope.digest().as_bytes(),
            },
            current: &baseline,
        })
        .expect("assignment projection");
        let view = project_curriculum_import_inspection(CurriculumImportInspectionInput {
            witness: CourseScheduleWitness::new(
                CourseReference::new(5).expect("course"),
                CourseScheduleRevision::new(6).expect("schedule revision"),
                vec![ObservedAssignmentRevision {
                    assignment: assignment.assignment,
                    revision: "7".parse().expect("assignment revision"),
                }],
            )
            .expect("course witness"),
            origin: CurriculumCourseImportOriginView::Ordinary,
            term: CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
                .expect("term"),
            assignments: vec![assignment.clone()],
        })
        .expect("course projection");
        assert_eq!(view.assignments(), [assignment]);
    }
}
