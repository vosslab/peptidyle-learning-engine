//! Typed generation for tracked fixture evidence.
//!
//! The committed JSON and asset files are intentional review evidence. The
//! TypeScript module is a fully derivative projection written under ignored
//! root `generated/` for mock handlers to import.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use adapter_native::peptide_bond_geometry::{GENERATOR_ID, GENERATOR_VERSION};
use adapter_native::{AssetObjectBinding, NativeAdapter};
use anyhow::{Context, Result, bail};
use grading::GradeOutcome;
use question_model::answer::SelectionCardinality;
use question_model::assignment_activity_rules::{
    AssignmentActivityRules, AttemptPolicy, CompletionRequirement, ContinuedPractice, GradePolicy,
    TimingPolicy, VariationPolicy,
};
use question_model::definition::{
    DraftQuestionDefinition, DraftQuestionSource, GradingDefinition, QuestionDefinition,
    QuestionFormat, QuestionMetadata, QuestionSource,
};
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::{
    GeneratorReference, ParameterSpec, RandomizationDefinition, Seed,
};
use question_model::identity::{AssetId, ObjectId, WorkspaceId};
use question_model::response::{
    ChoiceId, ChoiceOption, QuestionResponseFormat, QuestionType, StudentResponse,
};
use question_model::taxonomy::{License, Tag, TaxonomyTerm};
use question_model::{
    ActivityTimestamp, AssignmentAttempt, AssignmentAttemptId, AssignmentDeliveryState,
    AssignmentEntryId, AssignmentEntrySummary, AssignmentId,
    FixedQuestionAssignmentEntrySummary, AssignmentProgressRecord,
    AssignmentScoringMode, AssignmentSummary, CatalogQuestionSummary,
    CourseId, CourseMembershipRole, CourseSummary, GradebookSummaryRow,
    IssuedQuestion, IssuedQuestionId, PointValue, QuestionAttempt, QuestionAttemptId,
    QuestionAttemptTiming, QuestionBackend, QuestionId, QuestionVersionNumber,
    QuestionVersionAvailability, QuestionVersionReference, StudentRecordId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const PEPTIDE_BOND_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="480" height="180" viewBox="0 0 480 180" role="img" aria-labelledby="title description">
  <title id="title">Peptide bond structure</title>
  <desc id="description">A carbonyl carbon double bonded to oxygen and single bonded to nitrogen.</desc>
  <rect width="480" height="180" fill="#ffffff"/>
  <g stroke="#172033" stroke-width="5" stroke-linecap="round">
    <line x1="90" y1="105" x2="190" y2="105"/>
    <line x1="190" y1="98" x2="245" y2="43"/>
    <line x1="199" y1="107" x2="254" y2="52"/>
    <line x1="205" y1="105" x2="315" y2="105"/>
    <line x1="335" y1="105" x2="420" y2="105"/>
  </g>
  <g fill="#172033" font-family="sans-serif" font-size="30" text-anchor="middle">
    <text x="65" y="115">R</text>
    <text x="195" y="120">C</text>
    <text x="270" y="48">O</text>
    <text x="330" y="115">N</text>
    <text x="440" y="115">R</text>
  </g>
</svg>
"##;

const PEPTIDE_PLANE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="480" height="180" viewBox="0 0 480 180" role="img" aria-labelledby="title description">
  <title id="title">Peptide plane</title>
  <desc id="description">Six peptide-group atoms lie in one shaded plane.</desc>
  <rect width="480" height="180" fill="#ffffff"/>
  <polygon points="70,145 135,35 405,35 340,145" fill="#dce9f7" stroke="#356a9a" stroke-width="3"/>
  <g fill="#172033" font-family="sans-serif" font-size="25" text-anchor="middle">
    <text x="115" y="125">Ca</text>
    <text x="195" y="88">C</text>
    <text x="245" y="58">O</text>
    <text x="300" y="105">N</text>
    <text x="360" y="72">H</text>
    <text x="355" y="130">Ca</text>
  </g>
</svg>
"##;

/// Whether tracked fixture evidence is compared or deliberately refreshed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Refuse any tracked-byte difference.
    Check,
    /// Write the current typed fixture bytes.
    Write,
}

/// Summary printed by the project-tools command.
pub struct Report {
    /// Human-readable action for the terminal summary.
    pub action: &'static str,
    /// Number of tracked fixture files checked or written.
    pub tracked_files: usize,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureAsset {
    id: AssetId,
    object: ObjectId,
    filename: String,
    media_type: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCorpus {
    fixture_schema_version: u32,
    model_schema_version: u32,
    catalog_question: CatalogQuestionSummary,
    published_problem: QuestionDefinition,
    draft: DraftQuestionDefinition,
    assets: Vec<FixtureAsset>,
    course: CourseSummary,
    assignment: AssignmentSummary,
    student_record: StudentRecordId,
    runs: Vec<AssignmentAttempt>,
    issued_questions: Vec<IssuedQuestion>,
    attempts: Vec<QuestionAttempt>,
    summary: AssignmentProgressRecord,
    gradebook: Vec<GradebookSummaryRow>,
}

struct TrackedArtifact {
    relative_path: PathBuf,
    bytes: Vec<u8>,
}

/// Checks or deliberately refreshes the tracked fixture evidence.
pub fn run(fixture_dir: &Path, mode: Mode) -> Result<Report> {
    let corpus = build_corpus()?;
    let artifacts = tracked_artifacts(&corpus)?;

    for artifact in &artifacts {
        let path = fixture_dir.join(&artifact.relative_path);
        match mode {
            Mode::Check => check_artifact(&path, &artifact.bytes)?,
            Mode::Write => write_artifact(&path, &artifact.bytes)?,
        }
    }

    let action = match mode {
        Mode::Check => "verified",
        Mode::Write => "wrote",
    };
    Ok(Report {
        action,
        tracked_files: artifacts.len(),
    })
}

fn build_corpus() -> Result<FixtureCorpus> {
    let workspace = workspace_id("0198e000-0000-7000-8000-000000000002");
    let question_id =
        QuestionId::from_canonical_parts("7K3M9Q", 'P').expect("fixture Question ID is canonical");
    let version_number = QuestionVersionNumber::new(1).expect("fixture version is positive");
    let assignment_id = assignment_id("0198e000-0000-7000-8000-000000000006");
    let student_record = student_record_id("0198e000-0000-7000-8000-000000000007");
    let asset_specs = [
        (
            asset_id("0198e000-0000-7000-8000-000000000010"),
            object_id("0198e000-0000-7000-8000-000000000011"),
            "peptide_bond.svg",
            PEPTIDE_BOND_SVG,
        ),
        (
            asset_id("0198e000-0000-7000-8000-000000000012"),
            object_id("0198e000-0000-7000-8000-000000000013"),
            "peptide_plane.svg",
            PEPTIDE_PLANE_SVG,
        ),
    ];
    let assets: Vec<FixtureAsset> = asset_specs
        .iter()
        .map(|(id, object, filename, body)| FixtureAsset {
            id: *id,
            object: *object,
            filename: (*filename).to_string(),
            media_type: "image/svg+xml".to_string(),
            sha256: sha256(body.as_bytes()),
        })
        .collect();

    let published_problem = QuestionDefinition::from_draft(
        draft_question(workspace, &assets),
        question_id.clone(),
        version_number,
        QuestionSource::Native,
    );
    let adapter = NativeAdapter::new();
    let catalog_question = CatalogQuestionSummary {
        question_id: "7K3-M9QP"
            .parse()
            .expect("fixture Question ID is canonical"),
        backend: QuestionBackend::Native,
        question_type: QuestionType::MultipleChoice,
        capabilities: adapter.capabilities(&published_problem)?,
        metadata: published_problem.metadata.clone(),
        byline: question_model::PublicByline::new(vec![question_model::PublicAuthorName::new(
            "Fixture Instructor".to_string(),
        )?])?,
        availability: QuestionVersionAvailability::Available,
        published_at: timestamp(1_786_000_000_000),
    };
    let mut draft = draft_question(workspace, &assets);
    draft.metadata.title = "Draft: peptide resonance wording revision".to_string();

    let course_id = course_id("0198e000-0000-7000-8000-000000000014");
    let run_ids = [
        run_id("0198e000-0000-7000-8000-000000000020"),
        run_id("0198e000-0000-7000-8000-000000000021"),
        run_id("0198e000-0000-7000-8000-000000000022"),
        run_id("0198e000-0000-7000-8000-000000000023"),
    ];
    let policies = AssignmentActivityRules {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    };
    let completion_times = [
        Some(1_786_000_001_300),
        Some(1_786_000_002_300),
        Some(1_786_000_003_300),
        None,
    ];
    let scores = [Some(0.0), Some(1.0), Some(0.0), None];
    let runs = run_ids
        .iter()
        .enumerate()
        .map(|(index, id)| AssignmentAttempt {
            id: *id,
            reference: question_model::AssignmentAttemptReference::new(
                u64::try_from(index + 1).expect("four fixture runs fit u64"),
            )
            .expect("four fixture public run IDs are valid"),
            student_record,
            assignment: assignment_id,
            attempt_number: u32::try_from(index + 1).expect("four fixture attempts fit u32"),
            started_at: timestamp(
                1_786_000_001_000 + i64::try_from(index).expect("index fits") * 1_000,
            ),
            completed_at: completion_times[index].map(timestamp),
            score: scores[index],
            variation: VariationPolicy::NewSeeds,
        })
        .collect();

    let assignment_entry = assignment_entry_id("0198e000-0000-7000-8000-000000000017");
    let issued_questions = run_ids
        .iter()
        .enumerate()
        .map(|(index, assignment_attempt)| IssuedQuestion {
            id: issued_question_id(match index {
                0 => "0198e000-0000-7000-8000-000000000040",
                1 => "0198e000-0000-7000-8000-000000000041",
                2 => "0198e000-0000-7000-8000-000000000042",
                _ => "0198e000-0000-7000-8000-000000000043",
            }),
            assignment_attempt: *assignment_attempt,
            assignment_entry,
            definition_entry_index: 0,
            issued_position: 0,
            reference: QuestionVersionReference {
                question_id: question_id.clone(),
                version_number,
            },
            statistics_eligible: true,
            question_pool_entry: None,
            selection_seed: None,
        })
        .collect::<Vec<_>>();

    let attempts = issued_questions
        .iter()
        .enumerate()
        .map(|(index, issued_question)| {
            let seed = 1_001 + u64::try_from(index).expect("fixture index fits u64");
            question_attempt(
                &adapter,
                index,
                issued_question.id,
                &published_problem,
                seed,
                &assets,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let summary = AssignmentProgressRecord {
        student_record,
        assignment: assignment_id,
        current_score: Some(1.0),
        best_score: Some(1.0),
        latest_score: Some(0.0),
        completed_assignment_attempt_count: 3,
        total_question_attempts: 4,
        last_activity_at: Some(timestamp(1_786_000_004_100)),
    };

    Ok(FixtureCorpus {
        fixture_schema_version: 4,
        model_schema_version: 1,
        catalog_question: catalog_question.clone(),
        published_problem,
        draft,
        assets,
        course: CourseSummary {
            id: course_id,
            reference: question_model::CourseInstanceReference::new(1).expect("valid Course Instance reference"),
            title: "BIOC 301: Biochemistry".to_string(),
            term: question_model::CourseTerm::from_parts(
                "2026-08-24",
                "2026-12-18",
                "America/Chicago",
            )
            .expect("explicit fixture course term"),
            role: CourseMembershipRole::Student,
        },
        assignment: AssignmentSummary {
            id: assignment_id,
            reference: question_model::AssignmentReference::new(1)
                .expect("valid assignment reference"),
            course_id,
            title: "Peptide bond mastery".to_string(),
            disclosure_policy: question_model::StudentDisclosurePolicy::default(),
            entries: vec![AssignmentEntrySummary::FixedQuestion(
                FixedQuestionAssignmentEntrySummary {
                    id: assignment_entry_id("0198e000-0000-7000-8000-000000000017"),
                    question_id: catalog_question.question_id.clone(),
                    title: catalog_question.metadata.title.clone(),
                    backend: catalog_question.backend,
                    capabilities: catalog_question.capabilities.clone(),
                    points_possible: PointValue::from_whole(1),
                    delivery_state: AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                },
            )],
            policies,
        },
        student_record,
        runs,
        issued_questions,
        attempts,
        gradebook: vec![GradebookSummaryRow {
            course_id,
            student_record_id: student_record,
            student_name: "Jordan Student".to_string(),
            assignment_id,
            assignment_title: "Peptide bond mastery".to_string(),
            summary: summary.clone(),
            scoring_status: question_model::ScoringStatus::Current,
        }],
        summary,
    })
}

fn draft_question(workspace: WorkspaceId, assets: &[FixtureAsset]) -> DraftQuestionDefinition {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "residue".to_string(),
        ParameterSpec::Choice {
            options: vec![
                "glycine".to_string(),
                "alanine".to_string(),
                "proline".to_string(),
            ],
        },
    );

    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Native,
        question_format: QuestionFormat::NativeAlgorithmic,
        prompt: vec![
            ContentBlock::Text {
                markdown: "In the {{residue}} peptide example, which bond has restricted rotation because resonance gives it partial double-bond character?".to_string(),
            },
            image_block(&assets[0], "Structural formula highlighting the carbonyl carbon-to-nitrogen bond."),
            image_block(&assets[1], "The six atoms of a peptide group shown in one plane."),
        ],
        response: QuestionResponseFormat::MultipleChoice {
            choices: vec![
                choice("amide", "The carbonyl carbon-to-nitrogen bond"),
                choice("carbonyl", "The carbonyl carbon-to-oxygen bond"),
                choice("alpha-carbon", "The nitrogen-to-alpha-carbon bond"),
            ],
            selection: SelectionCardinality::ExactlyOne,
        },
        question_type: QuestionType::MultipleChoice,
        attempt_policy: AttemptPolicy {
            max_attempts: None,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: GENERATOR_ID.to_string(),
                version: GENERATOR_VERSION.to_string(),
            },
            parameters,
        },
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Peptide bond resonance and planarity".to_string(),
            tags: vec![Tag::new("biochemistry"), Tag::new("protein-structure")],
            taxonomy: vec![TaxonomyTerm {
                scheme: "Peptidyle".to_string(),
                code: "BIOCHEM.PEPTIDE_BOND".to_string(),
                label: "Peptide bond structure".to_string(),
            }],
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

fn image_block(asset: &FixtureAsset, description: &str) -> ContentBlock {
    ContentBlock::Image {
        asset: AssetRef {
            asset: asset.id,
            checksum: asset.sha256.clone(),
        },
        description: description.to_string(),
    }
}

fn asset_bindings(assets: &[FixtureAsset]) -> Vec<AssetObjectBinding> {
    assets
        .iter()
        .map(|asset| AssetObjectBinding {
            asset: asset.id,
            object: asset.object,
        })
        .collect()
}

fn choice(id: &str, markdown: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: markdown.to_string(),
        }],
    }
}

#[allow(clippy::too_many_arguments)]
fn question_attempt(
    adapter: &NativeAdapter,
    index: usize,
    issued_question: IssuedQuestionId,
    question: &QuestionDefinition,
    seed: u64,
    assets: &[FixtureAsset],
) -> Result<QuestionAttempt> {
    let completed = index < 3;
    let issued_at = 1_786_000_001_100 + i64::try_from(index).expect("index fits") * 1_000;
    let selected = if index == 1 { "amide" } else { "carbonyl" };

    let asset_bindings = asset_bindings(assets);
    let issued = adapter.issue(question, Seed::new(seed), &asset_bindings)?;
    let response = completed.then(|| StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new(selected)],
    });
    let result = match response.as_ref() {
        Some(response) => match adapter.grade(
            question,
            Seed::new(seed),
            &issued.parameter_hash,
            &issued.provenance,
            &asset_bindings,
            response,
        )? {
            GradeOutcome::Graded(result) => Some(result),
            GradeOutcome::Ungraded => bail!("fixture native question must be graded"),
        },
        None => None,
    };

    Ok(QuestionAttempt {
        id: question_attempt_id(match index {
            0 => "0198e000-0000-7000-8000-000000000030",
            1 => "0198e000-0000-7000-8000-000000000031",
            2 => "0198e000-0000-7000-8000-000000000032",
            _ => "0198e000-0000-7000-8000-000000000033",
        }),
        issued_question,
        seed,
        parameter_hash: issued.parameter_hash,
        response,
        status: if completed {
            question_model::AttemptStatus::Submitted
        } else {
            question_model::AttemptStatus::InProgress
        },
        result,
        timing: QuestionAttemptTiming {
            issued_at: timestamp(issued_at),
            deadline: None,
            submitted_at: completed.then(|| timestamp(issued_at + 100)),
        },
        provenance: issued.provenance,
        issued_capability: question_model::IssuedAttemptCapabilityV1::FlatPresentation,
    })
}

fn tracked_artifacts(corpus: &FixtureCorpus) -> Result<Vec<TrackedArtifact>> {
    let mut corpus_bytes =
        serde_json::to_vec_pretty(corpus).context("serializing fixture corpus")?;
    corpus_bytes.push(b'\n');
    Ok(vec![
        TrackedArtifact {
            relative_path: PathBuf::from("corpus.json"),
            bytes: corpus_bytes,
        },
        TrackedArtifact {
            relative_path: PathBuf::from("assets/peptide_bond.svg"),
            bytes: PEPTIDE_BOND_SVG.as_bytes().to_vec(),
        },
        TrackedArtifact {
            relative_path: PathBuf::from("assets/peptide_plane.svg"),
            bytes: PEPTIDE_PLANE_SVG.as_bytes().to_vec(),
        },
    ])
}

fn check_artifact(path: &Path, expected: &[u8]) -> Result<()> {
    let actual = fs::read(path).with_context(|| {
        format!(
            "reading {}; refresh with `cargo tools fixtures --write`",
            path.display()
        )
    })?;
    if actual != expected {
        bail!(
            "fixture drift in {}; refresh deliberately with `cargo tools fixtures --write`",
            path.display()
        );
    }
    Ok(())
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn parsed_uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("fixture UUID literals are valid")
}

fn timestamp(value: i64) -> ActivityTimestamp {
    ActivityTimestamp::from_unix_millis(value)
}

macro_rules! id_constructor {
    ($function:ident, $type:ty) => {
        fn $function(value: &str) -> $type {
            <$type>::from_uuid(parsed_uuid(value))
        }
    };
}

id_constructor!(asset_id, AssetId);
id_constructor!(assignment_id, AssignmentId);
id_constructor!(assignment_entry_id, AssignmentEntryId);
id_constructor!(course_id, CourseId);
id_constructor!(issued_question_id, IssuedQuestionId);
id_constructor!(object_id, ObjectId);
id_constructor!(question_attempt_id, QuestionAttemptId);
id_constructor!(run_id, AssignmentAttemptId);
id_constructor!(student_record_id, StudentRecordId);
id_constructor!(workspace_id, WorkspaceId);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn corpus_meets_the_wp_c7_learning_history_contract() {
        let corpus =
            build_corpus().expect("fixture corpus should build through the native adapter");
        let completed = corpus
            .runs
            .iter()
            .filter(|run| run.completed_at.is_some())
            .count();
        let seeds: BTreeSet<u64> = corpus.attempts.iter().map(|attempt| attempt.seed).collect();

        assert_eq!(corpus.assets.len(), 2);
        assert_eq!(corpus.assignment.entries.len(), 1);
        assert_eq!(completed, 3);
        assert_eq!(corpus.runs.len() - completed, 1);
        assert_eq!(corpus.issued_questions.len(), corpus.attempts.len());
        assert!(corpus.attempts.iter().all(|attempt| {
            corpus
                .issued_questions
                .iter()
                .any(|issued_question| issued_question.id == attempt.issued_question)
        }));
        assert_eq!(seeds.len(), corpus.attempts.len());
        assert!(corpus.attempts.iter().all(|attempt| {
            attempt.provenance.generator.is_some()
                && attempt.provenance.source_artifact.is_none()
                && attempt.provenance.asset_objects.len() == 2
                && attempt.parameter_hash.len() == 64
                && attempt.provenance.rendered_question_sha256.len() == 64
        }));
    }

    #[test]
    fn committed_corpus_reproduces_and_grades_through_the_native_adapter() {
        let corpus: FixtureCorpus = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/published_problem/corpus.json"
        ))
        .expect("committed fixture corpus should deserialize");
        let adapter = NativeAdapter::new();
        let asset_bindings = asset_bindings(&corpus.assets);

        assert!(matches!(
            &corpus.published_problem.source,
            QuestionSource::Native { .. }
        ));
        for attempt in &corpus.attempts {
            assert!(attempt.provenance.source_artifact.is_none());
            let envelope = adapter
                .reproduce(
                    &corpus.published_problem,
                    Seed::new(attempt.seed),
                    &attempt.parameter_hash,
                    &attempt.provenance,
                    &asset_bindings,
                )
                .expect("committed attempt should reproduce without an answer key");
            assert_eq!(
                envelope.question_version,
                QuestionVersionReference {
                    question_id: corpus.published_problem.question_id.clone(),
                    version_number: corpus.published_problem.version_number,
                }
            );

            match (&attempt.response, &attempt.result) {
                (Some(response), Some(recorded_result)) => {
                    let outcome = adapter
                        .grade(
                            &corpus.published_problem,
                            Seed::new(attempt.seed),
                            &attempt.parameter_hash,
                            &attempt.provenance,
                            &asset_bindings,
                            response,
                        )
                        .expect("committed response should grade through the native adapter");
                    assert_eq!(outcome, GradeOutcome::Graded(*recorded_result));
                }
                (None, None) => {}
                _ => panic!("fixture attempt must carry both response and result or neither"),
            }
        }
    }
}
