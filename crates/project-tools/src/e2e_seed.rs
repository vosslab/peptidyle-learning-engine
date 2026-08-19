//! Host-only durable seed data for the replica restart E2E gate.
//!
//! This command is deliberately not an API feature. It applies the embedded
//! migrations and uses the production PostgreSQL store contract, then writes
//! only non-secret identifiers to stdout for the browser E2E runner.

use std::collections::BTreeMap;

use adapter_native::NativeAdapter;
use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::{
    AssignmentRecord, AssignmentScoringCommitOutcome, AssignmentScoringWorkerCommand,
    AssignmentScoringWorkerStore, AssignmentUpdate, AttemptSupportActionId, AuthoritativeTimeStore,
    CatalogSourceStore, CatalogStore, ClearAttemptCommand, CourseRecord, CourseRosterStore,
    DeleteAndRegradeAssignmentItemCommand, DraftRecord, FlatGradingCapability,
    FlatQuestionGradingPayload, ForceSubmitAttemptCommand, IssueQuestionAttemptCommand,
    IssuedFlatGradingContract, JobClaimFilter, JobLeaseDuration, JobPayload, JobStore, PageRequest,
    PageSize, PresentationCapability, PublishDraftCommand, Store, StoreError,
    SubmissionIdempotencyKey, SubmitQuestionAttemptCommand, TenantContext, UpsertCourseMember,
};
use objects::{ObjectCategory, ObjectKey, ObjectStore, PutObject};
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
    AssignmentDeliveryState, AssignmentEnrollment, AssignmentId, AssignmentItem, AssignmentItemId,
    AssignmentScoringMode, AttemptProvenance, AttemptResult, AttemptStatus, CatalogLifecycle,
    CourseId, EnrollmentId, FeedbackContent, ImplementationVersion, ObjectId, PointValue,
    PresentationBindingV1, ProblemId, ProblemVersionRef, PublicationScope, QuestionAttemptId,
    QuestionId, RunId, TenantId, UserId, VersionId, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[path = "e2e_seed/native.rs"]
mod native;
use native::*;

#[path = "e2e_seed/webwork.rs"]
mod webwork;
use webwork::*;

#[path = "e2e_seed/chapter_one_identity.rs"]
mod chapter_one_identity;
use chapter_one_identity::*;

#[path = "e2e_seed/chapter_one_manifest.rs"]
mod chapter_one_manifest;
use chapter_one_manifest::*;

#[path = "e2e_seed/chapter_one.rs"]
mod chapter_one;
use chapter_one::*;

#[path = "e2e_seed/chapter_one_statistics.rs"]
mod chapter_one_statistics;
use chapter_one_statistics::*;

#[path = "e2e_seed/scoring.rs"]
mod scoring;
use scoring::*;

#[path = "e2e_seed/records.rs"]
mod records;
use records::*;

const USAGE: &str = "usage: cargo tools e2e-seed [--database-url <URL>] --tenant <UUID> (--instructor <UUID>|--user <UUID>) --student <UUID> --apply-migrations [--exercise-scoring] [(--webwork-pilot|--chapter-one-pilot) --s3-endpoint <URL> --s3-region <REGION> --private-content-bucket <BUCKET> [--chapter-one-existing-manifest <PATH>]] (database URL also reads PLE_MIGRATION_DATABASE_URL)";
const WEBWORK_PILOT_SOURCE_PATH: &str = "content/pilot/webwork/which_hydrophobic-simple.pgml";
const WEBWORK_PILOT_SOURCE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../content/pilot/webwork/which_hydrophobic-simple.pgml"
));
const WEBWORK_PILOT_SOURCE_SHA256: &str =
    "2a662d3af1385dc180c529509106208424c978ba3890c411ae451b1be0369b2b";
const WEBWORK_PILOT_SOURCE_PROVENANCE: &str = "Copied byte-for-byte from OTHER_REPOS/biology-problems-website/site_docs/biochemistry/topic01/downloads/which_hydrophobic-simple.pgml; source header declares CC BY 4.0 and notes that source code portions are LGPLv3.";
const WEBWORK_PILOT_CONVERGENCE_ATTEMPTS: u8 = 3;

fn question_id_store(
    pool: learning_data_access::postgres::Pool,
) -> Result<learning_data_access::postgres::PostgresStore> {
    let encoded = match std::env::var("PLE_QUESTION_ID_SECRET_FILE") {
        Ok(path) => std::fs::read_to_string(path)
            .context("reading PLE_QUESTION_ID_SECRET_FILE for E2E publication")?,
        Err(std::env::VarError::NotPresent) => std::env::var("PLE_QUESTION_ID_SECRET")
            .context("PLE_QUESTION_ID_SECRET_FILE or PLE_QUESTION_ID_SECRET is required")?,
        Err(error) => return Err(error).context("PLE_QUESTION_ID_SECRET_FILE is not Unicode"),
    };
    let encoded = encoded.trim_end_matches(['\r', '\n']);
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Question ID secret must be canonical base64url")?;
    if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(&decoded) != encoded {
        bail!("Question ID secret must be canonical 32-byte base64url");
    }
    Ok(
        learning_data_access::postgres::PostgresStore::with_question_id_secret(
            pool,
            decoded
                .try_into()
                .expect("checked Question ID secret length"),
        ),
    )
}

/// Private host-only identifiers the E2E runner needs to start an assignment.
///
/// This serializer is consumed only by the trusted host runner. Browser routes
/// receive the assigned Question ID and never this opaque replay evidence.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    assignment_id: AssignmentId,
    enrollment_id: EnrollmentId,
    question_id: QuestionId,
    problem_id: ProblemId,
    version_id: VersionId,
}

#[derive(Debug)]
struct SeedArguments {
    database_url: String,
    tenant: TenantId,
    instructor: UserId,
    student: UserId,
    apply_migrations: bool,
    exercise_scoring: bool,
    webwork_pilot: Option<WebworkPilotStorage>,
    chapter_one_pilot: Option<WebworkPilotStorage>,
    chapter_one_existing_manifest: Option<String>,
}

/// Non-secret host-only storage parameters for reviewed WeBWorK sources.
///
/// Both the legacy walkthrough seed and the Chapter 1 corpus use this shape.
/// Credentials remain in `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` so
/// they never appear in command output or process arguments.
#[derive(Debug, Clone)]
struct WebworkPilotStorage {
    endpoint_url: String,
    region: String,
    private_content_bucket: String,
}

/// Dispatches the host-only command without adding an API route or a service.
pub fn run(args: &[String]) -> Result<()> {
    if matches!(args, [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let environment_database_url = std::env::var("PLE_MIGRATION_DATABASE_URL").ok();
    let arguments = parse_arguments_with_database_url(args, environment_database_url)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the e2e seed runtime")?;
    let manifest = runtime.block_on(seed(arguments))?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

#[cfg(test)]
fn parse_arguments(args: &[String]) -> Result<SeedArguments> {
    parse_arguments_with_database_url(args, None)
}

fn parse_arguments_with_database_url(
    args: &[String],
    environment_database_url: Option<String>,
) -> Result<SeedArguments> {
    let mut database_url = None;
    let mut tenant = None;
    let mut instructor = None;
    let mut student = None;
    let mut apply_migrations = false;
    let mut exercise_scoring = false;
    let mut webwork_pilot = false;
    let mut chapter_one_pilot = false;
    let mut s3_endpoint = None;
    let mut s3_region = None;
    let mut private_content_bucket = None;
    let mut chapter_one_existing_manifest = None;
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        index += 1;
        if flag == "--exercise-scoring" && !exercise_scoring {
            exercise_scoring = true;
            continue;
        }
        if flag == "--apply-migrations" && !apply_migrations {
            apply_migrations = true;
            continue;
        }
        if flag == "--webwork-pilot" && !webwork_pilot {
            webwork_pilot = true;
            continue;
        }
        if flag == "--chapter-one-pilot" && !chapter_one_pilot {
            chapter_one_pilot = true;
            continue;
        }
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
            "--s3-endpoint" if s3_endpoint.is_none() => s3_endpoint = Some(value.clone()),
            "--s3-region" if s3_region.is_none() => s3_region = Some(value.clone()),
            "--private-content-bucket" if private_content_bucket.is_none() => {
                private_content_bucket = Some(value.clone())
            }
            "--chapter-one-existing-manifest" if chapter_one_existing_manifest.is_none() => {
                chapter_one_existing_manifest = Some(value.clone())
            }
            _ => bail!("unknown, duplicate, or misplaced argument {flag}; {USAGE}"),
        }
    }
    if webwork_pilot && chapter_one_pilot {
        bail!("--webwork-pilot and --chapter-one-pilot are mutually exclusive; {USAGE}");
    }
    if chapter_one_existing_manifest.is_some() && !chapter_one_pilot {
        bail!("--chapter-one-existing-manifest requires --chapter-one-pilot; {USAGE}");
    }
    let storage = match (
        webwork_pilot || chapter_one_pilot,
        s3_endpoint,
        s3_region,
        private_content_bucket,
    ) {
        (false, None, None, None) => None,
        (false, _, _, _) => {
            bail!(
                "--s3-endpoint, --s3-region, and --private-content-bucket require a pilot flag; {USAGE}"
            )
        }
        (true, Some(endpoint_url), Some(region), Some(private_content_bucket)) => {
            Some(WebworkPilotStorage {
                endpoint_url: validate_s3_endpoint(&endpoint_url)?,
                region,
                private_content_bucket,
            })
        }
        (true, _, _, _) => bail!(
            "the selected pilot requires --s3-endpoint, --s3-region, and --private-content-bucket; {USAGE}"
        ),
    };
    let webwork_pilot = webwork_pilot.then(|| storage.clone().expect("pilot storage is complete"));
    let chapter_one_pilot = chapter_one_pilot.then(|| storage.expect("pilot storage is complete"));
    let arguments = SeedArguments {
        database_url: database_url
            .or(environment_database_url)
            .ok_or_else(|| anyhow::anyhow!("--database-url is required; {USAGE}"))?,
        tenant: tenant.ok_or_else(|| anyhow::anyhow!("--tenant is required; {USAGE}"))?,
        instructor: instructor
            .ok_or_else(|| anyhow::anyhow!("--instructor is required; {USAGE}"))?,
        student: student.ok_or_else(|| anyhow::anyhow!("--student is required; {USAGE}"))?,
        apply_migrations,
        exercise_scoring,
        webwork_pilot,
        chapter_one_pilot,
        chapter_one_existing_manifest,
    };
    if arguments.instructor == arguments.student {
        bail!("--instructor and --student must identify different users for the E2E course");
    }
    if !arguments.apply_migrations {
        bail!("--apply-migrations is required because e2e-seed changes database schema; {USAGE}");
    }
    Ok(arguments)
}

fn validate_s3_endpoint(value: &str) -> Result<String> {
    let endpoint =
        url::Url::parse(value).context("--s3-endpoint must be an absolute HTTP(S) URL")?;
    if !matches!(endpoint.scheme(), "http" | "https") || endpoint.host_str().is_none() {
        bail!("--s3-endpoint must be an absolute HTTP(S) URL");
    }
    if !endpoint.username().is_empty() || endpoint.password().is_some() {
        bail!("--s3-endpoint must not include credentials");
    }
    if endpoint.query().is_some() || endpoint.fragment().is_some() {
        bail!("--s3-endpoint must not include a query or fragment");
    }
    Ok(endpoint.into())
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

async fn seed(arguments: SeedArguments) -> Result<serde_json::Value> {
    if arguments.chapter_one_pilot.is_some() {
        return serde_json::to_value(seed_chapter_one_pilot(&arguments).await?)
            .context("encoding Chapter 1 pilot manifest");
    }
    if arguments.webwork_pilot.is_some() {
        return serde_json::to_value(seed_webwork_pilot(&arguments).await?)
            .context("encoding WebWork pilot manifest");
    }
    serde_json::to_value(seed_native(arguments).await?).context("encoding native seed manifest")
}

#[cfg(test)]
#[path = "e2e_seed/tests.rs"]
mod tests;
