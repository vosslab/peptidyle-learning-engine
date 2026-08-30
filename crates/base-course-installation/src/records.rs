//! Versioned deterministic identities and exact ordinary Base Course recipe.

use std::collections::BTreeMap;

use adapter_native::NativeAdapter;
use question_model::answer::SelectionCardinality;
use question_model::capability::BackendCapabilities;
use question_model::definition::{
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionMetadata,
    QuestionSource,
};
use question_model::envelope::ContentBlock;
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy, RunPolicies,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::{License, Tag};
use question_model::{
    AssignmentDeliveryState, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentScoringMode,
    CourseId, PointValue, ProblemId, ProblemVersionRef, QuestionAttemptId, RunId, VersionId,
    WorkspaceId,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{BaseCourseInstallError, BaseCourseParticipants};

pub(crate) const BASELINE_VERSION: &str = "base-course-v1";
/// Learner-visible title for the installed Biochemistry teaching course.
///
/// `base_course` remains the installer-owned lifecycle boundary; it is not
/// product copy.
pub(crate) const BASE_COURSE_TITLE: &str = "Biochemistry: Protein Structure and Function";
pub(crate) const PRACTICE_COURSE_TITLE: &str = "Genetics Practice Course";
pub(crate) const COURSE_START: &str = "2026-01-01";
pub(crate) const COURSE_END: &str = "2099-12-31";
pub(crate) const ASSIGNMENT_TITLE: &str = "Peptide Bonds: Structure and Resonance";

/// The exact persisted prefix allowed while deterministic publication converges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationState {
    Fresh,
    Course,
    Draft,
    Published,
    Assignment,
}

pub(crate) fn publication_state(
    course_exists: bool,
    draft_exists: bool,
    publication_exists: bool,
    assignment_exists: bool,
) -> Result<PublicationState, BaseCourseInstallError> {
    match (
        course_exists,
        draft_exists,
        publication_exists,
        assignment_exists,
    ) {
        (false, false, false, false) => Ok(PublicationState::Fresh),
        (true, false, false, false) => Ok(PublicationState::Course),
        (true, true, false, false) => Ok(PublicationState::Draft),
        (true, false, true, false) => Ok(PublicationState::Published),
        (true, false, true, true) => Ok(PublicationState::Assignment),
        _ => Err(BaseCourseInstallError::baseline(
            "records are not a resumable deterministic publication prefix; regenerate the disposable database and object storage before retrying",
        )),
    }
}

/// Every deterministic identifier owned by the installed baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BaseCourseIds {
    pub(crate) workspace: WorkspaceId,
    pub(crate) problem: ProblemId,
    pub(crate) version: VersionId,
    pub(crate) base_course: CourseId,
    pub(crate) practice_course: CourseId,
    pub(crate) assignment: AssignmentId,
    pub(crate) assignment_item: AssignmentItemId,
    pub(crate) mary_run: RunId,
    pub(crate) mary_attempt: QuestionAttemptId,
    pub(crate) jack_run: RunId,
    pub(crate) jack_attempt: QuestionAttemptId,
}

impl BaseCourseIds {
    pub(crate) fn for_installation() -> Self {
        let id = deterministic_uuid;
        Self {
            workspace: WorkspaceId::from_uuid(id("workspace")),
            problem: ProblemId::from_uuid(id("problem")),
            version: VersionId::from_uuid(id("version")),
            base_course: CourseId::from_uuid(id("course")),
            practice_course: CourseId::from_uuid(id("practice-course")),
            assignment: AssignmentId::from_uuid(id("assignment")),
            assignment_item: AssignmentItemId::from_uuid(id("assignment-item")),
            mary_run: RunId::from_uuid(id("run")),
            mary_attempt: QuestionAttemptId::from_uuid(id("attempt")),
            jack_run: RunId::from_uuid(id("additional-run")),
            jack_attempt: QuestionAttemptId::from_uuid(id("additional-attempt")),
        }
    }
}

pub(crate) fn deterministic_uuid(label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-installed-base-course-v1:single-installation:");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(crate) fn native_capabilities() -> Result<BackendCapabilities, BaseCourseInstallError> {
    NativeAdapter::new()
        .capabilities(&QuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        })
        .map_err(|source| {
            BaseCourseInstallError::native(
                "resolving capabilities for the Base Course native question",
                source,
            )
        })
}

pub(crate) fn base_course_native_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec!["glycine".to_string(), "alanine".to_string()],
        },
    );
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        },
        prompt: vec![ContentBlock::Text {
            markdown: "In the {{residue}} peptide example, which bond has restricted rotation because resonance gives it partial double-bond character?".to_string(),
        }],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![
                choice("amide", "The carbonyl carbon-to-nitrogen bond"),
                choice("carbonyl", "The carbonyl carbon-to-oxygen bond"),
                choice("alpha-carbon", "The nitrogen-to-alpha-carbon bond"),
            ],
            selection: SelectionCardinality::ExactlyOne,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: Some(1),
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "peptide-bond-choice".to_string(),
                version: "1".to_string(),
            },
            parameters,
        },
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Peptide bond resonance and planarity".to_string(),
            tags: vec![Tag::new("biochemistry"), Tag::new("peptide-bond")],
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

pub(crate) fn base_course(
    id: CourseId,
) -> Result<learning_data_access::CourseRecord, BaseCourseInstallError> {
    course(id, BASE_COURSE_TITLE)
}

pub(crate) fn practice_course(
    id: CourseId,
) -> Result<learning_data_access::CourseRecord, BaseCourseInstallError> {
    course(id, PRACTICE_COURSE_TITLE)
}

/// Builds the exact v1 recipe accepted by the closed installer capability.
///
/// Account attributes remain deployment-owned constants inside the capability;
/// this value binds the five validated identities and both complete course
/// records to the installation generation before any mutation occurs.
pub(crate) fn installation_recipe(
    participants: BaseCourseParticipants,
    ids: BaseCourseIds,
) -> Result<serde_json::Value, BaseCourseInstallError> {
    let base = base_course(ids.base_course)?;
    let practice = practice_course(ids.practice_course)?;
    Ok(serde_json::json!({
        "schemaVersion": 1,
        "participants": {
            "elena": participants.primary_instructor().as_uuid(),
            "mary": participants.mary().as_uuid(),
            "jack": participants.jack().as_uuid(),
            "avery": participants.approval_candidate().as_uuid(),
            "morgan": participants.sysadmin().as_uuid(),
        },
        "courses": {
            "baseCourse": course_recipe(&base, participants.primary_instructor()),
            "geneticsPractice": course_recipe(&practice, participants.sysadmin()),
        },
        "graph": {
            "workspace": ids.workspace.as_uuid(),
            "problem": ids.problem.as_uuid(),
            "version": ids.version.as_uuid(),
            "assignment": ids.assignment.as_uuid(),
            "assignmentItem": ids.assignment_item.as_uuid(),
            "maryRun": ids.mary_run.as_uuid(),
            "maryAttempt": ids.mary_attempt.as_uuid(),
            "jackRun": ids.jack_run.as_uuid(),
            "jackAttempt": ids.jack_attempt.as_uuid(),
        },
    }))
}

fn course_recipe(
    course: &learning_data_access::CourseRecord,
    initial_instructor: question_model::AccountId,
) -> serde_json::Value {
    serde_json::json!({
        "id": course.id.as_uuid(),
        "title": course.title,
        "termStart": COURSE_START,
        "termEnd": COURSE_END,
        "timeZone": "America/Chicago",
        "initialInstructor": initial_instructor.as_uuid(),
    })
}

fn course(
    id: CourseId,
    title: &str,
) -> Result<learning_data_access::CourseRecord, BaseCourseInstallError> {
    let term = question_model::CourseTerm::from_parts(COURSE_START, COURSE_END, "America/Chicago")
        .map_err(|error| {
            BaseCourseInstallError::baseline(format!(
                "the versioned Base Course term is invalid: {error}"
            ))
        })?;
    Ok(learning_data_access::CourseRecord {
        id,
        title: title.to_string(),
        term,
    })
}

pub(crate) fn assignment(
    ids: BaseCourseIds,
    reference: ProblemVersionRef,
) -> Result<learning_data_access::AssignmentRecord, BaseCourseInstallError> {
    let instructions = question_model::AssignmentInstructions::try_new(
        "Work through the peptide-bond geometry evidence before submitting.".to_string(),
    )
    .map_err(|error| {
        BaseCourseInstallError::baseline(format!(
            "the versioned Base Course instructions are invalid: {error}"
        ))
    })?;
    Ok(learning_data_access::AssignmentRecord {
        id: ids.assignment,
        course_id: ids.base_course,
        title: ASSIGNMENT_TITLE.to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions,
        audience: question_model::AssignmentAudience::CourseWide,
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        items: vec![AssignmentItem {
            id: ids.assignment_item,
            reference,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
    })
}

fn choice(id: &str, text: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: text.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use question_model::AccountId;

    use super::*;

    fn participants() -> BaseCourseParticipants {
        BaseCourseParticipants::try_new(
            AccountId::from_uuid(Uuid::from_u128(10)),
            AccountId::from_uuid(Uuid::from_u128(11)),
            AccountId::from_uuid(Uuid::from_u128(12)),
            AccountId::from_uuid(Uuid::from_u128(13)),
            AccountId::from_uuid(Uuid::from_u128(14)),
        )
        .unwrap()
    }

    #[test]
    fn deterministic_ids_repeat_and_remain_disjoint() {
        let first = BaseCourseIds::for_installation();
        let second = BaseCourseIds::for_installation();

        assert_eq!(first, second);
        assert_ne!(first.base_course.as_uuid(), first.practice_course.as_uuid());
        assert_ne!(first.base_course.as_uuid(), first.assignment.as_uuid());
        assert_ne!(first.mary_run, first.jack_run);
        assert_ne!(first.mary_attempt, first.jack_attempt);
    }

    #[test]
    fn recipe_has_the_exact_course_assignment_and_question() {
        let participants = participants();
        let ids = BaseCourseIds::for_installation();
        let reference = ProblemVersionRef {
            problem: ids.problem,
            version: ids.version,
        };
        let base_course = base_course(ids.base_course).unwrap();
        let practice_course = practice_course(ids.practice_course).unwrap();
        let assignment = assignment(ids, reference).unwrap();
        let draft = base_course_native_draft(ids.workspace);

        assert_eq!(base_course.title, BASE_COURSE_TITLE);
        assert_eq!(practice_course.title, PRACTICE_COURSE_TITLE);
        assert_eq!(assignment.title, ASSIGNMENT_TITLE);
        assert_eq!(assignment.items[0].reference, reference);
        assert_eq!(draft.metadata.title, "Peptide bond resonance and planarity");
        assert_eq!(
            draft
                .metadata
                .tags
                .iter()
                .map(Tag::as_str)
                .collect::<Vec<_>>(),
            ["biochemistry", "peptide-bond"]
        );
    }

    #[test]
    fn installation_recipe_is_the_exact_closed_v1_contract() {
        let participants = participants();
        let ids = BaseCourseIds::for_installation();
        let recipe = installation_recipe(participants, ids).unwrap();

        assert_eq!(
            recipe,
            serde_json::json!({
                "schemaVersion": 1,
                "participants": {
                    "elena": Uuid::from_u128(10),
                    "mary": Uuid::from_u128(11),
                    "jack": Uuid::from_u128(12),
                    "avery": Uuid::from_u128(13),
                    "morgan": Uuid::from_u128(14),
                },
                "courses": {
                    "baseCourse": {
                        "id": ids.base_course.as_uuid(),
                        "title": BASE_COURSE_TITLE,
                        "termStart": COURSE_START,
                        "termEnd": COURSE_END,
                        "timeZone": "America/Chicago",
                        "initialInstructor": Uuid::from_u128(10),
                    },
                    "geneticsPractice": {
                        "id": ids.practice_course.as_uuid(),
                        "title": PRACTICE_COURSE_TITLE,
                        "termStart": COURSE_START,
                        "termEnd": COURSE_END,
                        "timeZone": "America/Chicago",
                        "initialInstructor": Uuid::from_u128(14),
                    },
                },
                "graph": {
                    "workspace": ids.workspace.as_uuid(),
                    "problem": ids.problem.as_uuid(),
                    "version": ids.version.as_uuid(),
                    "assignment": ids.assignment.as_uuid(),
                    "assignmentItem": ids.assignment_item.as_uuid(),
                    "maryRun": ids.mary_run.as_uuid(),
                    "maryAttempt": ids.mary_attempt.as_uuid(),
                    "jackRun": ids.jack_run.as_uuid(),
                    "jackAttempt": ids.jack_attempt.as_uuid(),
                },
            })
        );
    }

    #[test]
    fn only_exact_publication_prefixes_can_resume() {
        for (markers, expected) in [
            ([false, false, false, false], PublicationState::Fresh),
            ([true, false, false, false], PublicationState::Course),
            ([true, true, false, false], PublicationState::Draft),
            ([true, false, true, false], PublicationState::Published),
            ([true, false, true, true], PublicationState::Assignment),
        ] {
            assert_eq!(
                publication_state(markers[0], markers[1], markers[2], markers[3]).unwrap(),
                expected
            );
        }
        assert!(publication_state(false, true, false, false).is_err());
        assert!(publication_state(true, true, true, false).is_err());
        assert!(publication_state(false, false, false, true).is_err());
    }

    #[test]
    fn installed_namespace_stays_separate_from_acceptance_namespace() {
        let installed = deterministic_uuid("course");
        let mut hasher = Sha256::new();
        hasher.update(b"ple-replica-e2e-seed-v1:");
        hasher.update(b"course");
        let digest = hasher.finalize();
        assert_ne!(installed.as_bytes(), &digest[..16]);
    }
}
