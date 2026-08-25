//! Private, closed v1 wire codec for the assignment-definition broker.
//!
//! This is intentionally not a browser DTO.  The server has already resolved
//! question references and authenticated the actor before this module encodes
//! the bounded command for the `SECURITY DEFINER` capability.

use super::*;
use crate::ReplaceUnissuedAssignmentDefinitionOutcome;
use serde::{Deserialize, Serialize};

const MAX_PAYLOAD_BYTES: usize = 512 * 1024;
const RECALCULATION_MAX_ATTEMPTS: i32 = 10;

/// Exact, broker-locked source state for one assignment creation transaction.
///
/// The witness deliberately projects no membership or learner data.  Its
/// bindings are checked against the server-owned command before the returned
/// course term can influence policy validation (ASVS 2.2.2, 2.2.3, 8.2.2).
pub(super) struct AssignmentCreationWitness {
    course_term: question_model::CourseTerm,
}

struct AssignmentCreationWitnessFields {
    tenant_id: Uuid,
    actor_id: Uuid,
    course_id: Uuid,
    assignment_id: Uuid,
    term_start_date: String,
    term_end_date: String,
    time_zone: String,
}

impl AssignmentCreationWitness {
    fn decode(
        row: &PgRow,
        context: TenantContext,
        actor: UserId,
        assignment: &AssignmentRecord,
    ) -> Result<Self, StoreError> {
        Self::decode_fields(
            context,
            actor,
            assignment,
            AssignmentCreationWitnessFields {
                tenant_id: row.try_get("tenant_id").map_err(map_sqlx_error)?,
                actor_id: row.try_get("actor_id").map_err(map_sqlx_error)?,
                course_id: row.try_get("course_id").map_err(map_sqlx_error)?,
                assignment_id: row.try_get("assignment_id").map_err(map_sqlx_error)?,
                term_start_date: row.try_get("term_start_date").map_err(map_sqlx_error)?,
                term_end_date: row.try_get("term_end_date").map_err(map_sqlx_error)?,
                time_zone: row.try_get("time_zone").map_err(map_sqlx_error)?,
            },
        )
    }

    fn decode_fields(
        context: TenantContext,
        actor: UserId,
        assignment: &AssignmentRecord,
        fields: AssignmentCreationWitnessFields,
    ) -> Result<Self, StoreError> {
        if fields.tenant_id != context.tenant_id().as_uuid()
            || fields.actor_id != actor.as_uuid()
            || fields.course_id != assignment.course_id.as_uuid()
            || fields.assignment_id != assignment.id.as_uuid()
        {
            return Err(StoreError::Unavailable(
                "assignment creation preparation returned changed bindings".to_string(),
            ));
        }
        let course_term = question_model::CourseTerm::from_parts(
            &fields.term_start_date,
            &fields.term_end_date,
            &fields.time_zone,
        )
        .map_err(|error| {
            StoreError::Unavailable(format!(
                "assignment creation preparation returned an invalid course term: {error}"
            ))
        })?;
        Ok(Self { course_term })
    }

    pub(super) const fn course_term(&self) -> &question_model::CourseTerm {
        &self.course_term
    }
}

pub(super) async fn prepare_creation(
    tx: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    assignment: &AssignmentRecord,
) -> Result<AssignmentCreationWitness, StoreError> {
    // ASVS 1.2.4: all command bindings are SQL parameters.  ASVS 2.3.1,
    // 2.3.3, and 2.3.4: the broker acquires the canonical locks inside the
    // same transaction that validates references, policy, and final creation.
    let row = sqlx::query(
        "SELECT tenant_id, actor_id, course_id, assignment_id, \
                term_start_date::text AS term_start_date, \
                term_end_date::text AS term_end_date, time_zone \
           FROM ple_prepare_assignment_creation_v1($1,$2,$3,$4)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(actor.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    AssignmentCreationWitness::decode(&row, context, actor, assignment)
}

pub(super) async fn create(
    tx: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    assignment: &AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
) -> Result<StoredAssignment, StoreError> {
    let payload = encode(assignment, base_policy)?;
    let row = sqlx::query(
        "SELECT assignment_id, revision, scoring_generation, scoring_status \
         FROM ple_create_assignment_definition_v1($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(actor.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(payload)
    .bind(Option::<Uuid>::None)
    .bind(Option::<i32>::None)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if AssignmentId::from_uuid(row.try_get("assignment_id").map_err(map_sqlx_error)?)
        != assignment.id
    {
        return Err(StoreError::Unavailable(
            "assignment capability returned an unexpected identity".to_string(),
        ));
    }
    reload_and_compare(tx, assignment, base_policy, &row).await
}

pub(super) async fn replace(
    tx: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    previous: &AssignmentRecord,
    assignment: &AssignmentRecord,
    expected_revision: AssignmentRevision,
) -> Result<StoredAssignment, StoreError> {
    let base_policy =
        super::course_policy::load_base_policy(tx, assignment.tenant, assignment.id).await?;
    let payload = encode(assignment, base_policy)?;
    let has_scores: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM attempt_score_current WHERE tenant_id=$1 AND assignment_id=$2)",
    )
    .bind(assignment.tenant.as_uuid())
    .bind(assignment.id.as_uuid())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let job = (assignment_scoring_changed(previous, assignment) && has_scores)
        .then(JobId::generate)
        .transpose()?;
    let row = sqlx::query(
        "SELECT revision, scoring_generation, scoring_status \
         FROM ple_replace_assignment_definition_v1($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(actor.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?)
    .bind(payload)
    .bind(job.map(|value| value.as_uuid()))
    .bind(job.map(|_| RECALCULATION_MAX_ATTEMPTS))
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let stored = reload_and_compare(tx, assignment, base_policy, &row).await?;
    // The capability owns the authoritative revision transition; reresolution
    // consumes that committed-in-transaction state and rolls the whole save
    // back if an active learner attempt cannot be repaired.
    super::course_policy::reresolve_post_mutation_active_attempts(
        tx,
        context,
        actor,
        assignment.course_id,
        assignment.id,
        stored.revision,
    )
    .await?;
    Ok(stored)
}

/// Executes the only structural assignment-definition replacement capability.
/// SQL owns authorization, the revision transition, and serialization with
/// first-run issuance. Every command binding remains a SQL parameter (ASVS
/// 1.2.4); browser-shaped input never reaches this capability.
pub(super) async fn replace_unissued(
    tx: &mut Transaction<'_, Postgres>,
    context: TenantContext,
    actor: UserId,
    assignment: &AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
    expected_revision: AssignmentRevision,
) -> Result<ReplaceUnissuedAssignmentDefinitionOutcome, StoreError> {
    let payload = encode(assignment, base_policy)?;
    let row = sqlx::query(
        "SELECT outcome, revision, scoring_generation, scoring_status \
         FROM ple_replace_unissued_assignment_definition_v1($1,$2,$3,$4,$5,$6)",
    )
    .bind(context.tenant_id().as_uuid())
    .bind(actor.as_uuid())
    .bind(assignment.course_id.as_uuid())
    .bind(assignment.id.as_uuid())
    .bind(i64::try_from(expected_revision.value()).map_err(|_| StoreError::Conflict)?)
    .bind(payload)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let outcome: String = row.try_get("outcome").map_err(map_sqlx_error)?;
    match outcome.as_str() {
        "replaced" => reload_and_compare(tx, assignment, base_policy, &row)
            .await
            .map(Box::new)
            .map(ReplaceUnissuedAssignmentDefinitionOutcome::Replaced),
        "issued" => Ok(ReplaceUnissuedAssignmentDefinitionOutcome::Issued),
        _ => Err(StoreError::Unavailable(
            "unissued definition capability returned an invalid outcome".to_string(),
        )),
    }
}

async fn reload_and_compare(
    tx: &mut Transaction<'_, Postgres>,
    intended: &AssignmentRecord,
    base_policy: question_model::BaseAssignmentPolicy,
    row: &PgRow,
) -> Result<StoredAssignment, StoreError> {
    let record = load_assignment(tx, intended.tenant, intended.id).await?;
    let returned_base =
        super::course_policy::load_base_policy(tx, intended.tenant, intended.id).await?;
    if record != *intended || returned_base != base_policy {
        return Err(StoreError::Unavailable(
            "assignment capability normalization mismatch".to_string(),
        ));
    }
    Ok(StoredAssignment {
        record,
        base_policy: returned_base,
        revision: AssignmentRevision::from_stored(
            row.try_get("revision").map_err(map_sqlx_error)?,
        )?,
        scoring_generation: decode_scoring_generation(row)?,
        scoring_status: decode_scoring_status(row)?,
    })
}

fn encode(
    assignment: &AssignmentRecord,
    base: question_model::BaseAssignmentPolicy,
) -> Result<Value, StoreError> {
    validate_assignment(assignment)?;
    let wire = DefinitionWire::from_domain(assignment, base)?;
    let value = serde_json::to_value(&wire).map_err(|error| {
        StoreError::InvalidRecord(format!("assignment payload encoding failed: {error}"))
    })?;
    // ASVS 1.5.2: reparse the private representation through the same closed
    // schema used by codec tests before sending it to SQL.
    serde_json::from_value::<DefinitionWire>(value.clone()).map_err(|error| {
        StoreError::InvalidRecord(format!("assignment payload wire contract failed: {error}"))
    })?;
    if serde_json::to_vec(&value)
        .map_err(|e| StoreError::InvalidRecord(format!("assignment payload encoding failed: {e}")))?
        .len()
        > MAX_PAYLOAD_BYTES
    {
        return Err(StoreError::InvalidRecord(
            "assignment definition exceeds payload ceiling".to_string(),
        ));
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionWire {
    schema_version: u8,
    title: String,
    lifecycle: LifecycleWire,
    instructions: String,
    policies: PoliciesWire,
    disclosure_policy: DisclosureWire,
    audience: AudienceWire,
    base_policy: BasePolicyWire,
    entries: Vec<EntryWire>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PoliciesWire {
    completion: CompletionWire,
    grade: GradeWire,
    continued_practice: PracticeWire,
    variation: VariationWire,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields,
    tag = "kind"
)]
enum CompletionWire {
    AnswerAll,
    AllCorrect,
    ScoreAtLeast { threshold: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields,
    tag = "kind"
)]
enum PracticeWire {
    Unlimited,
    Closed,
    Capped { max_additional_runs: u32 },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DisclosureWire {
    score: DisclosureTimingWire,
    per_item_correctness: DisclosureTimingWire,
    feedback_text: DisclosureTimingWire,
    solution: DisclosureTimingWire,
    class_statistics: DisclosureTimingWire,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields,
    tag = "kind"
)]
enum AudienceWire {
    CourseWide,
    AnyOfGroups { groups: Vec<Uuid> },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BasePolicyWire {
    available_at: Option<i64>,
    due_at: Option<i64>,
    closes_at: Option<i64>,
    late_submission: LateWire,
    deadline_behavior: DeadlineWire,
    time_limit_seconds: Option<u32>,
    attempt_limit: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields,
    tag = "kind"
)]
enum EntryWire {
    Fixed {
        id: Uuid,
        position: u32,
        problem_id: Uuid,
        version_id: Uuid,
        points_possible: String,
        delivery_state: DeliveryWire,
        scoring_mode: ScoringWire,
    },
    SelectionGroup {
        id: Uuid,
        position: u32,
        draw_count: u32,
        points_per_item: String,
        ordering: OrderingWire,
        algorithm_version: u16,
        candidates: Vec<CandidateWire>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CandidateWire {
    id: Uuid,
    position: u32,
    problem_id: Uuid,
    version_id: Uuid,
    delivery_state: DeliveryWire,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LifecycleWire {
    Draft,
    Published,
    Closed,
    Archived,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum GradeWire {
    First,
    #[serde(rename = "last")]
    Latest,
    Highest,
    InstructorSelected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum VariationWire {
    NewSeeds,
    SelectedProblemVariants,
    FullRegeneration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DisclosureTimingWire {
    DuringAttempt,
    AfterSubmit,
    AfterDue,
    AfterClose,
    Never,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DeliveryWire {
    Active,
    Retired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ScoringWire {
    Normal,
    FullCredit,
    ExtraCredit,
    Excluded,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum OrderingWire {
    CandidateOrder,
    Randomized,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum LateWire {
    Accept,
    Reject,
    MarkLate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DeadlineWire {
    AutoSubmit,
}

impl DefinitionWire {
    fn from_domain(
        assignment: &AssignmentRecord,
        base: question_model::BaseAssignmentPolicy,
    ) -> Result<Self, StoreError> {
        let completion = match assignment.policies.completion {
            CompletionRequirement::AnswerAll => CompletionWire::AnswerAll,
            CompletionRequirement::AllCorrect => CompletionWire::AllCorrect,
            CompletionRequirement::ScoreAtLeast { fraction } => CompletionWire::ScoreAtLeast {
                threshold: format_fraction(fraction)?,
            },
        };
        let audience = match &assignment.audience {
            question_model::AssignmentAudience::CourseWide => AudienceWire::CourseWide,
            question_model::AssignmentAudience::AnyOfGroups(groups) => AudienceWire::AnyOfGroups {
                groups: groups.iter().map(|group| group.as_uuid()).collect(),
            },
        };
        let entries = assignment
            .items
            .iter()
            .map(|item| EntryWire::Fixed {
                id: item.id.as_uuid(),
                position: item.position,
                problem_id: item.reference.problem.as_uuid(),
                version_id: item.reference.version.as_uuid(),
                points_possible: item.points_possible.to_string(),
                delivery_state: delivery_wire(item.delivery_state),
                scoring_mode: scoring_wire(item.scoring_mode),
            })
            .chain(assignment.selection_groups.iter().map(|group| {
                EntryWire::SelectionGroup {
                    id: group.id.as_uuid(),
                    position: group.position,
                    draw_count: group.draw_count,
                    points_per_item: group.points_per_item.to_string(),
                    ordering: ordering_wire(group.ordering),
                    algorithm_version: group.algorithm.storage_version(),
                    candidates: group
                        .candidates
                        .iter()
                        .map(|candidate| CandidateWire {
                            id: candidate.id.as_uuid(),
                            position: candidate.position,
                            problem_id: candidate.reference.problem.as_uuid(),
                            version_id: candidate.reference.version.as_uuid(),
                            delivery_state: delivery_wire(candidate.delivery_state),
                        })
                        .collect(),
                }
            }))
            .collect();
        Ok(Self {
            schema_version: 1,
            title: assignment.title.clone(),
            lifecycle: lifecycle_wire(assignment.lifecycle),
            instructions: assignment.instructions.as_str().to_string(),
            policies: PoliciesWire {
                completion,
                grade: grade_wire(assignment.policies.grade),
                continued_practice: practice_wire(assignment.policies.continued_practice),
                variation: variation_wire(assignment.policies.variation),
            },
            disclosure_policy: DisclosureWire {
                score: disclosure_wire(assignment.disclosure_policy.score),
                per_item_correctness: disclosure_wire(
                    assignment.disclosure_policy.per_item_correctness,
                ),
                feedback_text: disclosure_wire(assignment.disclosure_policy.feedback_text),
                solution: disclosure_wire(assignment.disclosure_policy.solution),
                class_statistics: disclosure_wire(assignment.disclosure_policy.class_statistics),
            },
            audience,
            base_policy: BasePolicyWire {
                available_at: base.available_at.map(|v| v.as_unix_millis()),
                due_at: base.due_at.map(|v| v.as_unix_millis()),
                closes_at: base.closes_at.map(|v| v.as_unix_millis()),
                late_submission: late_wire(base.late_submission),
                deadline_behavior: DeadlineWire::AutoSubmit,
                time_limit_seconds: base.time_limit_seconds.map(|v| v.get()),
                attempt_limit: base.attempt_limit.map(|v| v.get()),
            },
            entries,
        })
    }
}
fn format_fraction(value: f64) -> Result<String, StoreError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(StoreError::InvalidRecord(
            "completion threshold is invalid".to_string(),
        ));
    }
    let value = format!("{value:.8}");
    Ok(value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string())
}
fn lifecycle_wire(v: question_model::AssignmentLifecycle) -> LifecycleWire {
    match v {
        question_model::AssignmentLifecycle::Draft => LifecycleWire::Draft,
        question_model::AssignmentLifecycle::Published => LifecycleWire::Published,
        question_model::AssignmentLifecycle::Closed => LifecycleWire::Closed,
        question_model::AssignmentLifecycle::Archived => LifecycleWire::Archived,
    }
}
fn delivery_wire(v: question_model::AssignmentDeliveryState) -> DeliveryWire {
    match v {
        question_model::AssignmentDeliveryState::Active => DeliveryWire::Active,
        question_model::AssignmentDeliveryState::Retired => DeliveryWire::Retired,
    }
}
fn scoring_wire(v: question_model::AssignmentScoringMode) -> ScoringWire {
    match v {
        question_model::AssignmentScoringMode::Normal => ScoringWire::Normal,
        question_model::AssignmentScoringMode::FullCredit => ScoringWire::FullCredit,
        question_model::AssignmentScoringMode::ExtraCredit => ScoringWire::ExtraCredit,
        question_model::AssignmentScoringMode::Excluded => ScoringWire::Excluded,
    }
}
fn ordering_wire(v: question_model::SelectionOrdering) -> OrderingWire {
    match v {
        question_model::SelectionOrdering::CandidateOrder => OrderingWire::CandidateOrder,
        question_model::SelectionOrdering::Randomized => OrderingWire::Randomized,
    }
}
fn grade_wire(v: GradePolicy) -> GradeWire {
    match v {
        GradePolicy::First => GradeWire::First,
        GradePolicy::Latest => GradeWire::Latest,
        GradePolicy::Highest => GradeWire::Highest,
        GradePolicy::InstructorSelected => GradeWire::InstructorSelected,
    }
}
fn variation_wire(v: VariationPolicy) -> VariationWire {
    match v {
        VariationPolicy::NewSeeds => VariationWire::NewSeeds,
        VariationPolicy::SelectedProblemVariants => VariationWire::SelectedProblemVariants,
        VariationPolicy::FullRegeneration => VariationWire::FullRegeneration,
    }
}
fn practice_wire(v: ContinuedPractice) -> PracticeWire {
    match v {
        ContinuedPractice::Unlimited => PracticeWire::Unlimited,
        ContinuedPractice::Closed => PracticeWire::Closed,
        ContinuedPractice::Capped {
            max_additional_runs,
        } => PracticeWire::Capped {
            max_additional_runs,
        },
    }
}
fn disclosure_wire(v: question_model::LearnerDisclosureTiming) -> DisclosureTimingWire {
    match v {
        question_model::LearnerDisclosureTiming::DuringAttempt => {
            DisclosureTimingWire::DuringAttempt
        }
        question_model::LearnerDisclosureTiming::AfterSubmit => DisclosureTimingWire::AfterSubmit,
        question_model::LearnerDisclosureTiming::AfterDue => DisclosureTimingWire::AfterDue,
        question_model::LearnerDisclosureTiming::AfterClose => DisclosureTimingWire::AfterClose,
        question_model::LearnerDisclosureTiming::Never => DisclosureTimingWire::Never,
    }
}
fn late_wire(v: question_model::LateSubmissionPolicy) -> LateWire {
    match v {
        question_model::LateSubmissionPolicy::Accept => LateWire::Accept,
        question_model::LateSubmissionPolicy::Reject => LateWire::Reject,
        question_model::LateSubmissionPolicy::MarkLate => LateWire::MarkLate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire() -> DefinitionWire {
        DefinitionWire {
            schema_version: 1,
            title: "Protein evidence".to_string(),
            lifecycle: LifecycleWire::Draft,
            instructions: "Compare the structures.".to_string(),
            policies: PoliciesWire {
                completion: CompletionWire::ScoreAtLeast {
                    threshold: "0.75".to_string(),
                },
                grade: GradeWire::Highest,
                continued_practice: PracticeWire::Capped {
                    max_additional_runs: 2,
                },
                variation: VariationWire::NewSeeds,
            },
            disclosure_policy: DisclosureWire {
                score: DisclosureTimingWire::AfterSubmit,
                per_item_correctness: DisclosureTimingWire::AfterSubmit,
                feedback_text: DisclosureTimingWire::AfterDue,
                solution: DisclosureTimingWire::AfterClose,
                class_statistics: DisclosureTimingWire::Never,
            },
            audience: AudienceWire::CourseWide,
            base_policy: BasePolicyWire {
                available_at: Some(1_725_000_000_123),
                due_at: None,
                closes_at: None,
                late_submission: LateWire::MarkLate,
                deadline_behavior: DeadlineWire::AutoSubmit,
                time_limit_seconds: Some(3600),
                attempt_limit: Some(3),
            },
            entries: vec![EntryWire::Fixed {
                id: Uuid::nil(),
                position: 0,
                problem_id: Uuid::nil(),
                version_id: Uuid::nil(),
                points_possible: "2.125".to_string(),
                delivery_state: DeliveryWire::Active,
                scoring_mode: ScoringWire::Normal,
            }],
        }
    }

    #[test]
    fn v1_wire_uses_exact_contract_keys_enums_decimals_and_millis() {
        let value = serde_json::to_value(wire()).expect("wire serializes");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["policies"]["completion"]["kind"], "scoreAtLeast");
        assert_eq!(value["policies"]["completion"]["threshold"], "0.75");
        assert_eq!(value["basePolicy"]["availableAt"], 1_725_000_000_123_i64);
        assert_eq!(value["entries"][0]["pointsPossible"], "2.125");
        assert_eq!(value["entries"][0]["scoringMode"], "normal");
        assert!(value.get("schema_version").is_none());
    }

    #[test]
    fn v1_wire_rejects_unknown_root_and_nested_fields() {
        let mut root = serde_json::to_value(wire()).expect("wire serializes");
        root["unexpected"] = Value::Bool(true);
        assert!(serde_json::from_value::<DefinitionWire>(root).is_err());

        let mut nested = serde_json::to_value(wire()).expect("wire serializes");
        nested["basePolicy"]["unexpected"] = Value::Bool(true);
        assert!(serde_json::from_value::<DefinitionWire>(nested).is_err());
    }

    #[test]
    fn payload_ceiling_is_enforced_before_database_execution() {
        let mut oversized = wire();
        oversized.instructions = "a".repeat(MAX_PAYLOAD_BYTES);
        assert!(serde_json::to_vec(&oversized).expect("serialize").len() > MAX_PAYLOAD_BYTES);
    }

    fn assignment() -> AssignmentRecord {
        AssignmentRecord {
            id: AssignmentId::generate(),
            tenant: TenantId::generate(),
            course_id: CourseId::generate(),
            title: "Creation witness".to_string(),
            lifecycle: question_model::AssignmentLifecycle::Draft,
            instructions: question_model::AssignmentInstructions::default(),
            audience: question_model::AssignmentAudience::CourseWide,
            items: Vec::new(),
            selection_groups: Vec::new(),
            policies: question_model::run_policy::RunPolicies {
                completion: question_model::run_policy::CompletionRequirement::AnswerAll,
                grade: question_model::run_policy::GradePolicy::Highest,
                continued_practice: question_model::run_policy::ContinuedPractice::Unlimited,
                variation: question_model::run_policy::VariationPolicy::NewSeeds,
            },
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
        }
    }

    fn exact_fields(
        assignment: &AssignmentRecord,
        actor: UserId,
    ) -> AssignmentCreationWitnessFields {
        AssignmentCreationWitnessFields {
            tenant_id: assignment.tenant.as_uuid(),
            actor_id: actor.as_uuid(),
            course_id: assignment.course_id.as_uuid(),
            assignment_id: assignment.id.as_uuid(),
            term_start_date: "2026-08-24".to_string(),
            term_end_date: "2026-12-18".to_string(),
            time_zone: "America/Chicago".to_string(),
        }
    }

    fn decode_witness(
        assignment: &AssignmentRecord,
        actor: UserId,
        fields: AssignmentCreationWitnessFields,
    ) -> Result<AssignmentCreationWitness, StoreError> {
        AssignmentCreationWitness::decode_fields(
            TenantContext::from_authenticated_session(assignment.tenant),
            actor,
            assignment,
            fields,
        )
    }

    #[test]
    fn creation_witness_accepts_only_exact_bindings_and_valid_term() {
        let assignment = assignment();
        let actor = UserId::generate();
        let exact = || decode_witness(&assignment, actor, exact_fields(&assignment, actor));
        assert!(exact().is_ok());

        for changed in [0_u8, 1, 2, 3] {
            let foreign = TenantId::generate().as_uuid();
            let mut fields = exact_fields(&assignment, actor);
            match changed {
                0 => fields.tenant_id = foreign,
                1 => fields.actor_id = foreign,
                2 => fields.course_id = foreign,
                3 => fields.assignment_id = foreign,
                _ => unreachable!(),
            }
            let result = decode_witness(&assignment, actor, fields);
            assert!(result.is_err(), "changed binding {changed} is refused");
        }

        for changed in [0_u8, 1, 2] {
            let mut fields = exact_fields(&assignment, actor);
            match changed {
                0 => fields.term_start_date = "not-a-date".to_string(),
                1 => fields.term_end_date = "not-a-date".to_string(),
                2 => fields.time_zone = "Invalid/Zone".to_string(),
                _ => unreachable!(),
            }
            let result = decode_witness(&assignment, actor, fields);
            assert!(result.is_err(), "invalid term component is refused");
        }
    }
}
