//! Rehearsal start subject resolution and creation.

use domain::{
    RehearsalGenesisContext, evidence_genesis_head, fingerprint_resolved_preview_subject,
};
use objects::Sha256Digest;
use question_model::{PreviewEvaluation, RehearsalRunId, RehearsalSubjectStart};
use serde::Deserialize;
use sqlx::Row;

use super::super::*;
use super::{auth, frozen, hydration, material};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartStructuralWitness {
    rehearsal_reference: u64,
}

/// Store-owned, durable route start.  This is the one boundary which turns a
/// public request into private aggregate identities and an immutable safe
/// receipt; handlers never manufacture a run UUID, source digest, locator, or
/// browser projection.
pub(super) async fn start_from_route(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::StartRehearsalRouteCommand,
) -> Result<crate::StartRehearsalRouteResult, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let ResolvedStartSubject { prepared, subject } =
        prepare_and_resolve_subject(&mut tx, tenant, &command).await?;
    let fingerprint = fingerprint_resolved_preview_subject(
        command.assignment,
        command.expected_revision,
        &subject,
    )
    .map_err(|_| StoreError::InvalidRecord("invalid resolved rehearsal subject".into()))?;
    let run_id = RehearsalRunId::from_uuid(crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("rehearsal ID randomness unavailable: {error}"))
    })?);
    let genesis = evidence_genesis_head(RehearsalGenesisContext {
        rehearsal: run_id,
        tenant,
        course: command.course,
        assignment: command.assignment,
        direct_instructor_membership: prepared.owner,
        revision: command.expected_revision,
        subject_fingerprint: fingerprint,
    });
    let row = sqlx::query("SELECT * FROM ple_prepare_rehearsal_start_idempotent($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(tenant.as_uuid())
        .bind(command.actor.as_uuid())
        .bind(command.course.as_uuid())
        .bind(prepared.assignment.as_uuid())
        .bind(i32::try_from(command.assignment.number()).map_err(|_| StoreError::InvalidRecord("assignment reference exceeds database range".into()))?)
        .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?)
        .bind(serde_json::to_value(&subject).map_err(|_| StoreError::InvalidRecord("rehearsal subject serialization failed".into()))?)
        .bind(fingerprint.as_bytes().to_vec())
        .bind(genesis.digest().as_bytes().to_vec())
        .bind(run_id.as_uuid())
        .bind(command.start_new_after_completion)
        .bind(command.idempotency_key.as_str())
        .bind(command.request_fingerprint.as_bytes().to_vec())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
    let kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
    if kind == "conflict" {
        return Err(StoreError::Conflict);
    }
    if kind == "replay" {
        let response: serde_json::Value =
            row.try_get("response_projection").map_err(map_sqlx_error)?;
        let receipt = serde_json::from_value(response).map_err(|_| {
            StoreError::InvalidRecord("invalid persisted rehearsal start receipt".into())
        })?;
        tx.commit().await.map_err(map_sqlx_error)?;
        return Ok(crate::StartRehearsalRouteResult {
            receipt,
            replayed: true,
        });
    }
    if kind != "apply" {
        return Err(StoreError::InvalidRecord(
            "invalid rehearsal route start result".into(),
        ));
    }
    let witness: serde_json::Value = row.try_get("structural_witness").map_err(map_sqlx_error)?;
    let witness: StartStructuralWitness = serde_json::from_value(witness)
        .map_err(|_| StoreError::InvalidRecord("invalid rehearsal start witness".into()))?;
    let rehearsal = question_model::RehearsalReference::new(witness.rehearsal_reference)
        .ok_or_else(|| StoreError::InvalidRecord("invalid rehearsal start reference".into()))?;
    let locator = crate::RehearsalLocator {
        actor: command.actor,
        course: command.course,
        assignment: command.assignment,
        revision: command.expected_revision,
        rehearsal,
    };
    // An exact idempotent replay returned above without looking at the
    // mutable assignment catalog or a private grading key.  For a new
    // operation, this broker binds and locks the complete ordinary source
    // inventory to this nonce and transaction before Rust constructs any
    // frozen bytes.  The finalizer consumes that same inventory.
    let operation: uuid::Uuid = row.try_get("operation_id").map_err(map_sqlx_error)?;
    let nonce: uuid::Uuid = row.try_get("prepare_nonce").map_err(map_sqlx_error)?;
    let locked_sources = material::resolve_locked_normal_assignment_sources(
        &mut tx,
        material::LockedAssignmentSourceRequest {
            tenant,
            actor: command.actor,
            course: command.course,
            assignment_reference: command.assignment,
            revision: command.expected_revision,
            assignment: prepared.assignment,
            operation,
            nonce,
        },
    )
    .await?;
    for locked in &locked_sources {
        locked.validate_for_freeze()?;
    }
    let source = prepared.source();
    // Freeze every real, already locked assignment item before finalizing the
    // route receipt.  These are normal `rehearsal_frozen_item`/evidence rows,
    // not an alternate rehearsal catalog.  ASVS 2.3.1 and 2.3.3.
    let mut frozen_items = Vec::with_capacity(locked_sources.len());
    for (ordinal, locked) in locked_sources.iter().enumerate() {
        let millis: i64 = sqlx::query_scalar("SELECT public.ple_rehearsal_now_millis()")
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let content = serde_json::to_vec(&locked.question).map_err(|_| {
            StoreError::InvalidRecord("cannot canonicalize locked rehearsal question".into())
        })?;
        let frozen = question_model::RehearsalFrozenItemEvidence {
            attempt: question_model::RehearsalAttemptId::from_uuid(
                crate::random_uuid::random_uuid_v4(|error| {
                    StoreError::Unavailable(format!(
                        "rehearsal attempt randomness unavailable: {error}"
                    ))
                })?,
            ),
            problem: question_model::ProblemVersionRef {
                problem: locked.question.problem,
                version: locked.question.version,
            },
            response_definition: locked.question.response.clone(),
            canonical_content_digest: question_model::RehearsalEvidenceDigest::from_bytes(
                *Sha256Digest::compute(&content).as_bytes(),
            ),
            frozen_at: question_model::ActivityTimestamp::from_unix_millis(millis),
        };
        frozen::append_in_tx(
            &mut tx,
            tenant,
            locator,
            frozen.clone(),
            operation,
            nonce,
            i32::try_from(ordinal).map_err(|_| {
                StoreError::InvalidRecord("frozen ordinal exceeds database range".into())
            })?,
        )
        .await?;
        frozen_items.push(frozen);
    }
    let receipt = hydration::load_authorized(&mut tx, tenant, locator, &source)
        .await?
        .run
        .receipt;
    let witness_digest: Vec<u8> = row
        .try_get("structural_witness_digest")
        .map_err(map_sqlx_error)?;
    material::finalize_start_freeze(
        &mut tx,
        material::FinalizeStartFreeze {
            tenant,
            operation,
            nonce,
            witness_digest,
            receipt: &receipt,
            subject: &subject,
        },
        &locked_sources,
        &frozen_items,
    )
    .await?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(crate::StartRehearsalRouteResult {
        receipt,
        replayed: false,
    })
}

/// Couples a replayable browser candidate to the broker's locked start
/// witness.  Synthetic subjects are identity-free, while a derived subject
/// must be traced to exactly one prior derived-preview audit and then checked
/// again after the broker locks that membership.  Rust uses only parameterized,
/// answer-free reads; SQL remains the mutation and lock authority.  ASVS
/// 1.2.4, 2.2.1, 2.3.1, 2.3.3, 8.2.2, 8.3.1, 8.4.1.
async fn prepare_and_resolve_subject(
    tx: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    command: &crate::StartRehearsalRouteCommand,
) -> Result<ResolvedStartSubject, StoreError> {
    match &command.subject {
        RehearsalSubjectStart::Synthetic { request } => {
            let prepared = auth::prepare_start(
                tx,
                tenant,
                command.actor,
                command.course,
                command.assignment,
                command.expected_revision,
                None,
            )
            .await?;
            let evaluation = super::super::preview_plane::resolve_synthetic_preview_read_only(
                tx,
                tenant,
                command.course,
                question_model::SyntheticPreviewSubjectRequest {
                    assignment: command.assignment,
                    revision: command.expected_revision,
                    selected_moment: request.selected_moment.clone(),
                    groups: request.groups.clone(),
                    modifiers: request.modifiers.clone(),
                },
            )
            .await?;
            let PreviewEvaluation::Allowed { subject, .. } = evaluation.evaluation else {
                return Err(StoreError::NotFound);
            };
            Ok(ResolvedStartSubject { prepared, subject })
        }
        RehearsalSubjectStart::Derived { candidate } => {
            let matched = find_unique_derived_membership(tx, tenant, command, candidate).await?;
            let prepared = auth::prepare_start(
                tx,
                tenant,
                command.actor,
                command.course,
                command.assignment,
                command.expected_revision,
                Some(matched.membership),
            )
            .await?;
            if prepared.derived_membership != Some(matched.membership) {
                return Err(StoreError::NotFound);
            }
            let resolved =
                super::super::preview_plane::resolve_derived_preview_by_membership_read_only_bound(
                    tx,
                    tenant,
                    command.course,
                    command.assignment,
                    command.expected_revision,
                    matched.reference,
                    candidate.selected_moment.clone(),
                )
                .await?;
            let PreviewEvaluation::Allowed { subject, .. } = resolved.result.evaluation else {
                return Err(StoreError::NotFound);
            };
            if resolved.membership != matched.membership || subject != *candidate {
                return Err(StoreError::NotFound);
            }
            Ok(ResolvedStartSubject { prepared, subject })
        }
    }
}

struct ResolvedStartSubject {
    prepared: auth::StartWitness,
    subject: question_model::PreviewSubject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DerivedMembershipCandidate {
    membership: question_model::CourseMembershipId,
    reference: question_model::CourseMembershipReference,
}

async fn find_unique_derived_membership(
    tx: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    command: &crate::StartRehearsalRouteCommand,
    candidate: &question_model::PreviewSubject,
) -> Result<DerivedMembershipCandidate, StoreError> {
    domain::validate_subject_binding(command.assignment, command.expected_revision, candidate)
        .map_err(|_| StoreError::NotFound)?;
    let assignment_id = sqlx::query_scalar::<_, sqlx::types::Uuid>(
        "SELECT assignment_id FROM assignment WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3 AND revision=$4",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(i64::from(command.assignment.number()))
    .bind(i64::try_from(command.expected_revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error)?
    .ok_or(StoreError::NotFound)?;
    let memberships = sqlx::query(
        "SELECT DISTINCT member.course_membership_id, member.public_id FROM audit_event audit JOIN course_member member ON member.tenant_id=audit.tenant_id AND member.course_id=audit.course_id AND member.course_membership_id=audit.target_id WHERE audit.tenant_id=$1 AND audit.actor_id=$2 AND audit.course_id=$3 AND audit.action='preview.subject.derived' AND audit.target_kind='course_membership' AND audit.payload->>'assignmentId'=$4 AND member.role='student' AND member.status='active' ORDER BY member.public_id, member.course_membership_id",
    )
    .bind(tenant.as_uuid())
    .bind(command.actor.as_uuid())
    .bind(command.course.as_uuid())
    .bind(assignment_id.to_string())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let mut matches = Vec::new();
    for row in memberships {
        let membership = question_model::CourseMembershipId::from_uuid(
            row.try_get("course_membership_id")
                .map_err(map_sqlx_error)?,
        );
        let value: i64 = row.try_get("public_id").map_err(map_sqlx_error)?;
        let reference =
            question_model::CourseMembershipReference::new(u64::try_from(value).map_err(|_| {
                StoreError::Unavailable("stored membership reference is invalid".into())
            })?)
            .ok_or_else(|| {
                StoreError::Unavailable("stored membership reference is invalid".into())
            })?;
        let resolved =
            super::super::preview_plane::resolve_derived_preview_by_membership_read_only_bound(
                tx,
                tenant,
                command.course,
                command.assignment,
                command.expected_revision,
                reference,
                candidate.selected_moment.clone(),
            )
            .await?;
        if let PreviewEvaluation::Allowed { subject, .. } = resolved.result.evaluation
            && resolved.membership == membership
            && subject == *candidate
        {
            matches.push(DerivedMembershipCandidate {
                membership,
                reference,
            });
        }
    }
    exact_one_derived_candidate(matches).ok_or(StoreError::NotFound)
}

fn exact_one_derived_candidate(
    matches: Vec<DerivedMembershipCandidate>,
) -> Option<DerivedMembershipCandidate> {
    (matches.len() == 1)
        .then(|| matches.into_iter().next())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(reference: u64) -> DerivedMembershipCandidate {
        DerivedMembershipCandidate {
            membership: question_model::CourseMembershipId::from_uuid(
                sqlx::types::Uuid::from_u128(u128::from(reference)),
            ),
            reference: question_model::CourseMembershipReference::new(reference)
                .expect("positive public membership reference"),
        }
    }

    #[test]
    fn derived_start_requires_one_and_only_one_audited_candidate() {
        let only = candidate(1);
        assert_eq!(exact_one_derived_candidate(vec![only]), Some(only));
        assert_eq!(exact_one_derived_candidate(Vec::new()), None);
        assert_eq!(
            exact_one_derived_candidate(vec![candidate(1), candidate(2)]),
            None,
            "two learners with the same visible candidate are ambiguous"
        );
    }

    #[test]
    fn derived_start_is_bound_before_fingerprint_and_uses_plain_resolvers() {
        let source = include_str!("start.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source before unit tests");
        let prepare_call = production
            .find("prepare_and_resolve_subject(&mut tx, tenant, &command)")
            .expect("start prepares and resolves its subject");
        let fingerprint = production
            .find("let fingerprint =")
            .expect("start fingerprints only a resolved subject");
        assert!(prepare_call < fingerprint);
        let helper_start = production
            .find("async fn prepare_and_resolve_subject")
            .expect("derived binding helper");
        let helper_end = production[helper_start..]
            .find("struct ResolvedStartSubject")
            .map(|offset| helper_start + offset)
            .expect("derived binding helper boundary");
        let helper = &production[helper_start..helper_end];
        let discovery = helper
            .find("let matched = find_unique_derived_membership")
            .expect("derived start discovers audited candidates");
        let preparation = helper
            .find("Some(matched.membership)")
            .expect("derived start supplies the internal membership to the broker");
        let recheck = helper
            .find("resolved.membership != matched.membership")
            .expect("derived start rechecks the broker-locked public reference");
        assert!(discovery < preparation && preparation < recheck);
        assert!(
            !production.contains("FOR UPDATE")
                && !production.contains("FOR SHARE")
                && !production.contains("FOR KEY SHARE"),
            "start delegates all source locking to the broker capability"
        );
    }

    #[test]
    fn bound_read_only_resolver_has_no_application_lock_clause() {
        let source = include_str!("../preview_plane.rs");
        let start = source
            .find("pub(super) async fn resolve_derived_preview_by_membership_read_only_bound")
            .expect("bound plain resolver");
        let end = source[start..]
            .find("/// Current-fact resolution result")
            .map(|offset| start + offset)
            .expect("resolver boundary");
        let resolver = &source[start..end];
        assert!(
            !resolver.contains("FOR UPDATE")
                && !resolver.contains("FOR SHARE")
                && !resolver.contains("FOR KEY SHARE"),
            "broker preparation, not the application role, owns source locks"
        );
    }
}
