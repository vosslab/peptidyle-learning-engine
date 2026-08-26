//! Topology-only rollover input shared by persistence adapters.

use question_model::curriculum_adoption::{
    CurriculumSemanticAssignment, CurriculumSemanticCourse, CurriculumSemanticModule,
    CurriculumSemanticPayload,
};

use super::SemanticPlannerError;

/// Ordered course meaning without destination or source storage identities.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RolloverInput {
    course: CurriculumSemanticCourse,
}

impl RolloverInput {
    /// Validates an ID-free module tree through qmodel's semantic constructors.
    pub(crate) fn new(
        title: String,
        modules: Vec<(String, Vec<CurriculumSemanticAssignment>)>,
    ) -> Result<Self, SemanticPlannerError> {
        let modules = modules
            .into_iter()
            .map(|(label, assignments)| {
                CurriculumSemanticModule::new(label, assignments)
                    .map_err(|error| SemanticPlannerError::InvalidMeaning(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let course = CurriculumSemanticCourse::new(title, modules)
            .map_err(|error| SemanticPlannerError::InvalidMeaning(error.to_string()))?;
        Ok(Self { course })
    }

    /// Returns the exact validated course-sized reusable meaning.
    pub(crate) fn payload(&self) -> CurriculumSemanticPayload {
        CurriculumSemanticPayload::course(self.course.clone())
    }

    /// Returns assignments in module and authored assignment order.
    pub(crate) fn assignments(&self) -> impl Iterator<Item = &CurriculumSemanticAssignment> {
        self.course
            .modules()
            .iter()
            .flat_map(|module| module.assignments())
    }

    /// Replaces meaning only when the validated course topology is identical.
    pub(crate) fn with_replaced_payload(
        self,
        payload: CurriculumSemanticPayload,
    ) -> Result<Self, SemanticPlannerError> {
        let CurriculumSemanticPayload::Course(replacement) = payload else {
            return Err(SemanticPlannerError::RolloverTopology(
                "replacement is not course-sized".into(),
            ));
        };
        if replacement.modules().len() != self.course.modules().len() {
            return Err(SemanticPlannerError::RolloverTopology(
                "replacement changed the module count".into(),
            ));
        }
        for (source, replacement) in self.course.modules().iter().zip(replacement.modules()) {
            if source.label() != replacement.label() {
                return Err(SemanticPlannerError::RolloverTopology(format!(
                    "replacement changed module label {:?}",
                    source.label()
                )));
            }
            if source.assignments().len() != replacement.assignments().len() {
                return Err(SemanticPlannerError::RolloverTopology(format!(
                    "replacement changed assignment count in module {:?}",
                    source.label()
                )));
            }
        }
        Ok(Self {
            course: replacement,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::CurriculumSemanticAssignmentEntry;
    use question_model::{
        AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
        CompletionRequirement, ContinuedPractice, GradePolicy, LateSubmissionPolicy,
        LearnerDisclosurePolicy, PointValue, ProblemId, ProblemVersionRef,
        RelativeAssignmentSchedule, ReusableAssignmentDefaults, RunPolicies, VariationPolicy,
        VersionId,
    };

    fn reference(value: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(value)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(value + 1)),
        }
    }

    fn assignment(reference: ProblemVersionRef) -> CurriculumSemanticAssignment {
        CurriculumSemanticAssignment::new(
            "Quiz".into(),
            AssignmentInstructions::default(),
            vec![CurriculumSemanticAssignmentEntry::Fixed {
                reference,
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            defaults(),
            RelativeAssignmentSchedule::default(),
        )
        .expect("semantic assignment")
    }

    fn defaults() -> ReusableAssignmentDefaults {
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
        }
    }

    #[test]
    fn replacement_preserves_module_topology_while_updating_meaning() {
        let source = RolloverInput::new(
            "Source".into(),
            vec![("Module A".into(), vec![assignment(reference(10))])],
        )
        .expect("rollover source");
        let replacement = RolloverInput::new(
            "Source".into(),
            vec![("Module A".into(), vec![assignment(reference(20))])],
        )
        .expect("replacement")
        .payload();
        let replaced = source
            .with_replaced_payload(replacement)
            .expect("same topology replaces");
        assert!(matches!(
            replaced.assignments().next().expect("assignment").entries(),
            [CurriculumSemanticAssignmentEntry::Fixed { reference, .. }]
                if *reference == self::reference(20)
        ));
    }

    #[test]
    fn replacement_refuses_changed_topology() {
        let source = RolloverInput::new(
            "Source".into(),
            vec![("Module A".into(), vec![assignment(reference(10))])],
        )
        .expect("rollover source");
        let changed = RolloverInput::new(
            "Source".into(),
            vec![("Module B".into(), vec![assignment(reference(10))])],
        )
        .expect("changed topology")
        .payload();
        assert!(source.with_replaced_payload(changed).is_err());
    }
}
