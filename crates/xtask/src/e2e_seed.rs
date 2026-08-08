//! Host-only durable seed data for the replica restart E2E gate.
//!
//! This command is deliberately not an API feature. It applies the embedded
//! migrations and uses the production PostgreSQL store contract, then writes
//! only non-secret identifiers to stdout for the browser E2E runner.

use std::collections::BTreeMap;

use adapter_native::NativeAdapter;
use anyhow::{Context, Result, bail};
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
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    RunPolicies, TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::{License, Tag};
use question_model::{
    AssignmentEnrollment, AssignmentId, CourseId, CourseMembership, CourseMembershipRole,
    EnrollmentId, ProblemId, ProblemVersionRef, PublicationScope, StudentId, TenantId, UserId,
    VersionId, WorkspaceId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use store::{
    AssignmentRecord, CatalogStore, CourseRecord, DraftRecord, PublishDraftCommand, Store,
    TenantContext,
};
use uuid::Uuid;

const USAGE: &str = "usage: xtask e2e-seed --database-url <URL> --tenant <UUID> (--instructor <UUID>|--user <UUID>) --student <UUID>";

/// Non-secret identifiers the replica E2E runner needs to start an assignment.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    assignment_id: AssignmentId,
    enrollment_id: EnrollmentId,
    problem_id: ProblemId,
    version_id: VersionId,
}

struct SeedArguments {
    database_url: String,
    tenant: TenantId,
    instructor: UserId,
    student: UserId,
}

/// Dispatches the host-only command without adding an API route or a service.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let arguments = parse_arguments(args)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the e2e seed runtime")?;
    let manifest = runtime.block_on(seed(arguments))?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

fn parse_arguments(args: &[String]) -> Result<SeedArguments> {
    let mut database_url = None;
    let mut tenant = None;
    let mut instructor = None;
    let mut student = None;
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        index += 1;
        let Some(value) = args.get(index) else {
            bail!("{flag} requires a value; {USAGE}");
        };
        index += 1;
        match flag.as_str() {
            "--database-url" if database_url.is_none() => database_url = Some(value.clone()),
            "--tenant" if tenant.is_none() => tenant = Some(parse_tenant(value, "tenant")?),
            "--instructor" | "--user" if instructor.is_none() => {
                instructor = Some(parse_user(value, "instructor")?);
            }
            "--student" if student.is_none() => student = Some(parse_user(value, "student")?),
            _ => bail!("unknown, duplicate, or misplaced argument {flag}; {USAGE}"),
        }
    }
    let arguments = SeedArguments {
        database_url: database_url
            .ok_or_else(|| anyhow::anyhow!("--database-url is required; {USAGE}"))?,
        tenant: tenant.ok_or_else(|| anyhow::anyhow!("--tenant is required; {USAGE}"))?,
        instructor: instructor
            .ok_or_else(|| anyhow::anyhow!("--instructor is required; {USAGE}"))?,
        student: student.ok_or_else(|| anyhow::anyhow!("--student is required; {USAGE}"))?,
    };
    if arguments.instructor == arguments.student {
        bail!("--instructor and --student must identify different users for the E2E course");
    }
    Ok(arguments)
}

fn parse_tenant(value: &str, name: &str) -> Result<TenantId> {
    Ok(TenantId::from_uuid(
        Uuid::parse_str(value).with_context(|| format!("{name} must be a UUID"))?,
    ))
}

fn parse_user(value: &str, name: &str) -> Result<UserId> {
    Ok(UserId::from_uuid(
        Uuid::parse_str(value).with_context(|| format!("{name} must be a UUID"))?,
    ))
}

async fn seed(arguments: SeedArguments) -> Result<Manifest> {
    let pool = store::postgres::lazy_pool(&arguments.database_url)
        .context("invalid --database-url for e2e seed")?;
    store::postgres::apply_migrations(&pool)
        .await
        .context("applying embedded migrations for e2e seed")?;
    let store = store::postgres::PostgresStore::new(pool);
    let context = TenantContext::from_authenticated_session(arguments.tenant);
    let ids = SeedIds::for_tenant(arguments.tenant);
    let draft = DraftRecord {
        tenant: arguments.tenant,
        question: native_draft(ids.workspace),
        revises: None,
        derived_from: None,
    };
    let capabilities = native_capabilities()?;
    let violations = domain::policy::validate_draft_for_publication(&draft.question, &capabilities);
    if !violations.is_empty() {
        bail!("native E2E seed draft failed publication capability admission: {violations:?}");
    }

    let saved_draft = store
        .upsert_draft(context, arguments.instructor, None, draft.clone())
        .await
        .context("writing deterministic native E2E draft")?;
    store
        .publish_draft(
            context,
            arguments.instructor,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved_draft.revision,
                publication: ProblemVersionRef {
                    problem: ids.problem,
                    version: ids.version,
                },
                published_source: QuestionSource::Native {
                    family: "peptide_bond_geometry".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                publisher: arguments.instructor,
                scope: PublicationScope::Institution,
                capabilities,
            },
        )
        .await
        .context("publishing deterministic native E2E question")?;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: ids.course,
                tenant: arguments.tenant,
                title: "PLE replica E2E course".to_string(),
                members: vec![
                    CourseMembership {
                        user: arguments.instructor,
                        role: CourseMembershipRole::Instructor,
                    },
                    CourseMembership {
                        user: arguments.student,
                        role: CourseMembershipRole::Student,
                    },
                ],
            },
        )
        .await
        .context("creating E2E course")?;
    store
        .create_assignment(
            context,
            AssignmentRecord {
                id: ids.assignment,
                tenant: arguments.tenant,
                course_id: ids.course,
                title: "PLE replica E2E assignment".to_string(),
                problems: vec![ProblemVersionRef {
                    problem: ids.problem,
                    version: ids.version,
                }],
                policies: RunPolicies {
                    completion: CompletionRequirement::AnswerAll,
                    grade: GradePolicy::Highest,
                    continued_practice: ContinuedPractice::Unlimited,
                    variation: VariationPolicy::NewSeeds,
                },
            },
        )
        .await
        .context("creating E2E assignment")?;
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: ids.enrollment,
                tenant: arguments.tenant,
                assignment: ids.assignment,
                user: arguments.student,
                student: StudentId::from_uuid(arguments.student.as_uuid()),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .context("creating E2E enrollment")?;

    Ok(Manifest {
        assignment_id: ids.assignment,
        enrollment_id: ids.enrollment,
        problem_id: ids.problem,
        version_id: ids.version,
    })
}

/// Reads the production native registry instead of maintaining a second
/// capability declaration in the E2E bootstrap.
fn native_capabilities() -> Result<BackendCapabilities> {
    NativeAdapter::new()
        .capabilities(&QuestionSource::Native {
            family: "peptide_bond_geometry".to_string(),
        })
        .context("resolving capabilities for the native E2E question family")
}

struct SeedIds {
    workspace: WorkspaceId,
    problem: ProblemId,
    version: VersionId,
    course: CourseId,
    assignment: AssignmentId,
    enrollment: EnrollmentId,
}

impl SeedIds {
    fn for_tenant(tenant: TenantId) -> Self {
        Self {
            workspace: WorkspaceId::from_uuid(derived_uuid(tenant, "workspace")),
            problem: ProblemId::from_uuid(derived_uuid(tenant, "problem")),
            version: VersionId::from_uuid(derived_uuid(tenant, "version")),
            course: CourseId::from_uuid(derived_uuid(tenant, "course")),
            assignment: AssignmentId::from_uuid(derived_uuid(tenant, "assignment")),
            enrollment: EnrollmentId::from_uuid(derived_uuid(tenant, "enrollment")),
        }
    }
}

/// Stable IDs make the manifest repeatable for an isolated disposable E2E DB.
fn derived_uuid(tenant: TenantId, label: &str) -> Uuid {
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

fn native_draft(workspace: WorkspaceId) -> DraftQuestionDefinition {
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
    use super::*;

    #[test]
    fn help_has_no_secret_or_browser_seed_endpoint() {
        assert!(USAGE.contains("e2e-seed"));
        assert!(!USAGE.contains("token"));
        assert!(!USAGE.contains("answer"));
    }

    #[test]
    fn parsing_requires_distinct_course_members() {
        let id = "00000000-0000-0000-0000-000000000001".to_string();
        let result = parse_arguments(&[
            "--database-url".to_string(),
            "postgres://example".to_string(),
            "--tenant".to_string(),
            id.clone(),
            "--instructor".to_string(),
            id.clone(),
            "--student".to_string(),
            id,
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn ids_are_stable_and_separated_by_purpose() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(9));
        let first = SeedIds::for_tenant(tenant);
        let second = SeedIds::for_tenant(tenant);
        assert_eq!(first.assignment, second.assignment);
        assert_ne!(first.assignment.as_uuid(), first.enrollment.as_uuid());
        assert_ne!(first.problem.as_uuid(), first.version.as_uuid());
    }

    #[test]
    fn native_seed_matches_catalog_publication_capability_admission() {
        let draft = native_draft(WorkspaceId::from_uuid(Uuid::from_u128(12)));
        let violations = domain::policy::validate_draft_for_publication(
            &draft,
            &native_capabilities().expect("registered native family has capabilities"),
        );

        assert!(
            violations.is_empty(),
            "the host seed must pass the same capability check as catalog publication: {violations:?}"
        );
    }
}
