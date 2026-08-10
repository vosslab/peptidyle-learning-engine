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

/// The shipped WebWork adapter declares these two capabilities for every PG
/// source. This seed stays within that contract: it is algorithmic and grades
/// only on the server; it does not claim partial credit or hints.
pub(super) fn webwork_capabilities() -> BackendCapabilities {
    BackendCapabilities::from_iter([Capability::AlgorithmicGeneration, Capability::ServerGrading])
}

#[derive(Clone, Copy)]
pub(super) struct SeedIds {
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) assignment_item: AssignmentItemId,
    pub(super) enrollment: EnrollmentId,
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
    pub(super) timing_assignment: AssignmentId,
    pub(super) timing_assignment_item_one: AssignmentItemId,
    pub(super) timing_assignment_item_two: AssignmentItemId,
    pub(super) timing_assignment_item_three: AssignmentItemId,
    pub(super) timing_enrollment: EnrollmentId,
    pub(super) timing_run: RunId,
    pub(super) timing_attempt_one: QuestionAttemptId,
    pub(super) timing_attempt_two: QuestionAttemptId,
    pub(super) timing_attempt_three: QuestionAttemptId,
    pub(super) timing_group: CourseGroupId,
    pub(super) timing_group_exception: AssignmentPolicyExceptionId,
    pub(super) timing_student_exception: AssignmentPolicyExceptionId,
    pub(super) timing_exception_run: RunId,
    pub(super) timing_exception_attempt: QuestionAttemptId,
}

/// A disjoint deterministic ID namespace lets the opt-in WebWork pilot seed
/// coexist with the native replica seed in the same disposable database.
#[derive(Clone, Copy)]
pub(super) struct WebworkPilotSeedIds {
    pub(super) workspace: WorkspaceId,
    pub(super) problem: ProblemId,
    pub(super) version: VersionId,
    pub(super) source_object: ObjectId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) assignment_item: AssignmentItemId,
    pub(super) enrollment: EnrollmentId,
}

impl WebworkPilotSeedIds {
    pub(super) fn for_tenant(tenant: TenantId) -> Self {
        let id = |label| webwork_pilot_uuid(tenant, label);
        Self {
            workspace: WorkspaceId::from_uuid(id("workspace")),
            problem: ProblemId::from_uuid(id("problem")),
            version: VersionId::from_uuid(id("version")),
            source_object: ObjectId::from_uuid(id("source-object")),
            course: CourseId::from_uuid(id("course")),
            assignment: AssignmentId::from_uuid(id("assignment")),
            assignment_item: AssignmentItemId::from_uuid(id("assignment-item")),
            enrollment: EnrollmentId::from_uuid(id("enrollment")),
        }
    }
}

impl SeedIds {
    pub(super) fn for_tenant(tenant: TenantId) -> Self {
        Self {
            workspace: WorkspaceId::from_uuid(derived_uuid(tenant, "workspace")),
            problem: ProblemId::from_uuid(derived_uuid(tenant, "problem")),
            version: VersionId::from_uuid(derived_uuid(tenant, "version")),
            course: CourseId::from_uuid(derived_uuid(tenant, "course")),
            assignment: AssignmentId::from_uuid(derived_uuid(tenant, "assignment")),
            assignment_item: AssignmentItemId::from_uuid(derived_uuid(tenant, "assignment-item")),
            enrollment: EnrollmentId::from_uuid(derived_uuid(tenant, "enrollment")),
            run: RunId::from_uuid(derived_uuid(tenant, "run")),
            attempt: QuestionAttemptId::from_uuid(derived_uuid(tenant, "attempt")),
            concurrent_run: RunId::from_uuid(derived_uuid(tenant, "concurrent-run")),
            concurrent_attempt: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "concurrent-attempt",
            )),
            support_run: RunId::from_uuid(derived_uuid(tenant, "support-run")),
            support_attempt: QuestionAttemptId::from_uuid(derived_uuid(tenant, "support-attempt")),
            support_replacement: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "support-replacement",
            )),
            retirement_run: RunId::from_uuid(derived_uuid(tenant, "retirement-run")),
            retirement_attempt: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "retirement-attempt",
            )),
            post_retirement_run: RunId::from_uuid(derived_uuid(tenant, "post-retirement-run")),
            timing_assignment: AssignmentId::from_uuid(derived_uuid(tenant, "timing-assignment")),
            timing_assignment_item_one: AssignmentItemId::from_uuid(derived_uuid(
                tenant,
                "timing-assignment-item-one",
            )),
            timing_assignment_item_two: AssignmentItemId::from_uuid(derived_uuid(
                tenant,
                "timing-assignment-item-two",
            )),
            timing_assignment_item_three: AssignmentItemId::from_uuid(derived_uuid(
                tenant,
                "timing-assignment-item-three",
            )),
            timing_enrollment: EnrollmentId::from_uuid(derived_uuid(tenant, "timing-enrollment")),
            timing_run: RunId::from_uuid(derived_uuid(tenant, "timing-run")),
            timing_attempt_one: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-attempt-one",
            )),
            timing_attempt_two: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-attempt-two",
            )),
            timing_attempt_three: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-attempt-three",
            )),
            timing_group: CourseGroupId::from_uuid(derived_uuid(tenant, "timing-group")),
            timing_group_exception: AssignmentPolicyExceptionId::from_uuid(derived_uuid(
                tenant,
                "timing-group-exception",
            )),
            timing_student_exception: AssignmentPolicyExceptionId::from_uuid(derived_uuid(
                tenant,
                "timing-student-exception",
            )),
            timing_exception_run: RunId::from_uuid(derived_uuid(tenant, "timing-exception-run")),
            timing_exception_attempt: QuestionAttemptId::from_uuid(derived_uuid(
                tenant,
                "timing-exception-attempt",
            )),
        }
    }
}

/// Stable IDs make the manifest repeatable for an isolated disposable E2E DB.
pub(super) fn derived_uuid(tenant: TenantId, label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-replica-e2e-seed-v1:");
    hasher.update(tenant.as_uuid().as_bytes());
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

pub(super) fn webwork_pilot_uuid(tenant: TenantId, label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"ple-webwork-pilot-e2e-seed-v1:");
    hasher.update(tenant.as_uuid().as_bytes());
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(super) fn native_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
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
            feedback: FeedbackDisclosure::Deferred,
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
            feedback: FeedbackDisclosure::Deferred,
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
