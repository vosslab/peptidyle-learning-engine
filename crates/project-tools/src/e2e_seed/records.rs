//! Host-only E2E seed records capability.

use super::*;

/// Reads the production native registry instead of maintaining a second
/// capability declaration in the E2E bootstrap.
pub(super) fn native_capabilities() -> Result<BackendCapabilities> {
    NativeAdapter::new()
        .capabilities(&QuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        })
        .context("resolving capabilities for the native E2E question family")
}

/// Reads the production WeBWorK registry instead of maintaining a second
/// capability declaration in the E2E bootstrap.
pub(super) fn webwork_capabilities() -> BackendCapabilities {
    adapter_webwork::webwork_source_capabilities(&webwork_pilot_published_source())
        .expect("tracked pilot source uses the WeBWorK backend")
}

/// The deterministic course and assignment records guard host-only replay.
/// Creating the course before publication turns every interrupted prefix into
/// an explicit reset-or-repair state rather than a second fresh publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SeedReplayState {
    Fresh,
    Replay,
}

pub(super) fn seed_replay_state(
    course_marker_exists: bool,
    assignment_marker_exists: bool,
    seed_name: &str,
) -> Result<SeedReplayState> {
    match (course_marker_exists, assignment_marker_exists) {
        (false, false) => Ok(SeedReplayState::Fresh),
        (true, false) => bail!(
            "{seed_name} has an incomplete deterministic marker state; reset the disposable database before seeding"
        ),
        (true, true) => Ok(SeedReplayState::Replay),
        (false, true) => bail!(
            "{seed_name} has an invalid deterministic marker state; the assignment exists without its course"
        ),
    }
}

#[derive(Clone, Copy)]
pub(super) struct SeedIds {
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) assignment_item: AssignmentItemId,
    pub(super) run: RunId,
    pub(super) attempt: QuestionAttemptId,
    pub(super) concurrent_run: RunId,
    pub(super) concurrent_attempt: QuestionAttemptId,
    pub(super) support_run: RunId,
    pub(super) support_attempt: QuestionAttemptId,
    pub(super) support_replacement: QuestionAttemptId,
    pub(super) retirement_run: RunId,
    pub(super) retirement_attempt: QuestionAttemptId,
    pub(super) post_retirement_run: RunId,
}

/// The opt-in WebWork seed keeps its disposable course scaffold stable while
/// each published question receives fresh opaque identities.
#[derive(Clone, Copy)]
pub(super) struct WebworkPilotSeedIds {
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) source_object: ObjectId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) assignment_item: AssignmentItemId,
}

/// Immutable catalog-only identities for the frozen WebWork baseline.
///
/// This record intentionally excludes every course, assignment, roster, and
/// learner identity. The host seed owns immutable provider material; visible
/// PLE workflows own teaching and learner state.
#[derive(Clone, Copy)]
pub(super) struct WebworkCatalogBaselineIds {
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) source_object: ObjectId,
}

impl WebworkCatalogBaselineIds {
    pub(super) fn for_installation() -> Self {
        let id = webwork_catalog_baseline_uuid;
        Self {
            workspace: WorkspaceId::from_uuid(id("workspace")),
            problem: ProblemId::from_uuid(id("problem")),
            version: VersionId::from_uuid(id("version")),
            source_object: ObjectId::from_uuid(id("source-object")),
        }
    }
}

impl WebworkPilotSeedIds {
    pub(super) fn fresh_for_installation() -> Self {
        let id = webwork_pilot_scaffold_uuid;
        Self {
            workspace: WorkspaceId::generate(),
            problem: ProblemId::generate(),
            version: VersionId::generate(),
            source_object: ObjectId::generate(),
            course: CourseId::from_uuid(id("course")),
            assignment: AssignmentId::from_uuid(id("assignment")),
            assignment_item: AssignmentItemId::from_uuid(id("assignment-item")),
        }
    }

    pub(super) fn from_published(
        record: &learning_data_access::PublishedProblemRecord,
        source_object: ObjectId,
    ) -> Self {
        let id = webwork_pilot_scaffold_uuid;
        Self {
            workspace: record.question.workspace,
            problem: record.problem,
            version: record.version,
            source_object,
            course: CourseId::from_uuid(id("course")),
            assignment: AssignmentId::from_uuid(id("assignment")),
            assignment_item: AssignmentItemId::from_uuid(id("assignment-item")),
        }
    }
}

impl SeedIds {
    pub(super) fn fresh_for_installation() -> Self {
        let id = derived_uuid;
        Self {
            workspace: WorkspaceId::generate(),
            problem: ProblemId::generate(),
            version: VersionId::generate(),
            course: CourseId::from_uuid(id("course")),
            assignment: AssignmentId::from_uuid(id("assignment")),
            assignment_item: AssignmentItemId::from_uuid(id("assignment-item")),
            run: RunId::from_uuid(id("run")),
            attempt: QuestionAttemptId::from_uuid(id("attempt")),
            concurrent_run: RunId::from_uuid(id("concurrent-run")),
            concurrent_attempt: QuestionAttemptId::from_uuid(id("concurrent-attempt")),
            support_run: RunId::from_uuid(id("support-run")),
            support_attempt: QuestionAttemptId::from_uuid(id("support-attempt")),
            support_replacement: QuestionAttemptId::from_uuid(id("support-replacement")),
            retirement_run: RunId::from_uuid(id("retirement-run")),
            retirement_attempt: QuestionAttemptId::from_uuid(id("retirement-attempt")),
            post_retirement_run: RunId::from_uuid(id("post-retirement-run")),
        }
    }

    pub(super) fn from_published(record: &learning_data_access::PublishedProblemRecord) -> Self {
        let mut ids = Self::fresh_for_installation();
        ids.workspace = record.question.workspace;
        ids.problem = record.problem;
        ids.version = record.version;
        ids
    }
}

/// Stable IDs make the manifest repeatable for an isolated disposable E2E DB.
pub(super) fn derived_uuid(label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-single-installation-replica-e2e-seed-v1:");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // Mark as RFC 4122 variant / deterministic version 5-shaped UUID without
    // claiming a UUIDv7 was minted by a browser-facing boundary.
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

/// Keeps the native and WebWork disposable scaffolds disjoint without
/// assigning any publication identity from an installation scope.
fn webwork_pilot_scaffold_uuid(label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-single-installation-webwork-pilot-e2e-scaffold-v1:");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

/// Derives the immutable provider-only identity from one reviewed baseline
/// label. No browser input participates in this identity.
fn webwork_catalog_baseline_uuid(label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-single-installation-webwork-catalog-baseline-v1:");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(super) fn replica_native_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
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
            tags: vec![Tag::new("replica-e2e")],
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    }
}

pub(super) fn webwork_pilot_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
    DraftQuestionDefinition {
        workspace,
        source: DraftQuestionSource::Webwork {
            pg_path: WEBWORK_PILOT_SOURCE_PATH.to_string(),
        },
        // The renderer replaces this neutral catalog placeholder with the
        // immutable PGML's prompt and radio choices before learner delivery.
        // No answer value is stored here.
        prompt: vec![ContentBlock::Text {
            markdown: "This question is rendered by the private WeBWorK service.".to_string(),
        }],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![choice("renderer-owned", "Rendered by WeBWorK")],
            selection: SelectionCardinality::ExactlyOne,
        },
        attempt_policy: AttemptPolicy {
            max_attempts: Some(1),
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Seeded {
            generator: GeneratorReference {
                id: "webwork-problem-seed".to_string(),
                version: "1".to_string(),
            },
            parameters: BTreeMap::new(),
        },
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Biochemistry: Identify hydrophobic compounds from formulas".to_string(),
            tags: vec![Tag::new("webwork-pilot"), Tag::new("hydrophobicity")],
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en".to_string(),
        },
    }
}

pub(super) fn webwork_pilot_published_source() -> QuestionSource {
    QuestionSource::Webwork {
        pg_path: WEBWORK_PILOT_SOURCE_PATH.to_string(),
    }
}

pub(super) fn webwork_pilot_source_key(
    reference: ProblemVersionRef,
    object: ObjectId,
) -> ObjectKey {
    ObjectKey::ProblemSource {
        problem: reference.problem,
        version: reference.version,
        object,
    }
}

pub(super) fn choice(id: &str, text: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: text.to_string(),
        }],
    }
}
