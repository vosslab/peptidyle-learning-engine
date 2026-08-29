//! Deterministic authored inputs and publication records for adoption behavior tests.

use question_model::answer::NumericTolerance;
use question_model::generation::RandomizationDefinition;
use question_model::run_policy::{AttemptPolicy, TimingPolicy};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentInstructions, AssignmentScoringMode,
    CompletionRequirement, ContinuedPractice, CurriculumAdoptionIdempotencyKey, GradePolicy,
    LateSubmissionPolicy, PointValue, RelativeAssignmentSchedule, ReusableAssignmentDefaults,
    ReusableAssignmentDefinitionInput, ReusableAssignmentEntryInput, ReusableFixedQuestionInput,
    RunPolicies, StudentDisclosurePolicy, UserId, VariationPolicy,
};
use question_model::{
    BackendCapabilities, Capability, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ProblemId, PublicationScope, QuestionDefinition, QuestionMetadata,
    ResponseDefinition, VersionId, WorkspaceId,
};
use uuid::Uuid;

pub(super) fn definition(
    question_id: question_model::QuestionId,
) -> ReusableAssignmentDefinitionInput {
    ReusableAssignmentDefinitionInput {
        title: "Protein structure practice".into(),
        instructions: AssignmentInstructions::try_new("Explain each choice.".into())
            .expect("instructions"),
        entries: vec![ReusableAssignmentEntryInput::Fixed(
            ReusableFixedQuestionInput {
                question_id,
                points_possible: PointValue::from_whole(3),
                scoring_mode: AssignmentScoringMode::Normal,
            },
        )],
        defaults: ReusableAssignmentDefaults {
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
            student_disclosure: StudentDisclosurePolicy::default(),
        },
        schedule: RelativeAssignmentSchedule::default(),
    }
}

pub(super) fn key(value: &str) -> CurriculumAdoptionIdempotencyKey {
    CurriculumAdoptionIdempotencyKey::parse(value).expect("idempotency key")
}

pub(super) fn published_record(number: u128) -> crate::PublishedProblemRecord {
    let problem = ProblemId::from_uuid(Uuid::from_u128(number));
    let version = VersionId::from_uuid(Uuid::from_u128(20_000 + number));
    let question = QuestionDefinition::from_draft(
        DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(30_000 + number)),
            source: DraftQuestionSource::Native {
                family: "curriculum_adoption_test".into(),
            },
            prompt: Vec::new(),
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: format!("Curriculum adoption item {number}"),
                tags: vec![Tag::new("biochemistry")],
                taxonomy: vec![TaxonomyTerm {
                    scheme: "discipline".into(),
                    code: "biochemistry".into(),
                    label: "Biochemistry".into(),
                }],
                license: License::CcBy,
                language: "en".into(),
            },
        },
        problem,
        version,
        question_model::QuestionSource::Native {
            family: "curriculum_adoption_test".into(),
        },
    );
    let mut value = u32::try_from(number).expect("fixture Question ID fits 30 bits");
    let mut bytes = [b'0'; 6];
    for output in bytes.iter_mut().rev() {
        *output = question_model::QUESTION_ID_ALPHABET[(value & 0x1f) as usize];
        value >>= 5;
    }
    crate::PublishedProblemRecord {
        problem,
        question_id: crate::QuestionIdCodec::from_server_secret([0x42; 32])
            .issue_for_identifier(std::str::from_utf8(&bytes).expect("alphabet is ASCII"))
            .expect("fixture Question ID issues"),
        version,
        question,
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        scope: PublicationScope::Public,
        lifecycle: question_model::CatalogLifecycle::Published,
        author_ids: vec![UserId::from_uuid(Uuid::from_u128(40_000))],
        byline: question_model::PublicByline::new(vec![
            question_model::PublicAuthorName::new("Curriculum test author".into())
                .expect("valid test byline"),
        ])
        .expect("valid test byline"),
        derived_from: None,
        published_at: ActivityTimestamp::from_unix_millis(0),
    }
}
