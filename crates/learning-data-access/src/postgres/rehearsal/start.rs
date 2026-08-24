//! Rehearsal start subject resolution and creation.

use domain::{
    RehearsalGenesisContext, RehearsalLifecycleSnapshot, RehearsalStartDecision, decide_start,
    evidence_genesis_head, fingerprint_resolved_preview_subject,
};
use question_model::{PreviewEvaluation, RehearsalRunId, RehearsalSubjectStart};

use super::super::*;
use super::{auth, hydration};

pub(super) async fn start(
    store: &PostgresStore,
    context: TenantContext,
    command: crate::StartRehearsalCommand,
) -> Result<question_model::RehearsalRunReceipt, StoreError> {
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let ResolvedStartSubject { prepared, subject } =
        prepare_and_resolve_subject(&mut tx, tenant, &command).await?;
    let source = prepared.source();
    let fingerprint =
        fingerprint_resolved_preview_subject(command.assignment, command.revision, &subject)
            .map_err(|_| StoreError::InvalidRecord("invalid resolved rehearsal subject".into()))?;
    // Lock and fully verify the latest owner aggregate before asking the
    // capability to resume or replace an active run.  ASVS 2.3.3: a source
    // lock plus canonical aggregate proof makes the state transition atomic.
    let prior_locator = prepared.latest_reference.zip(prepared.latest_revision);
    // Retain the exact, canonically hydrated owner aggregate as the optimistic
    // witness for the one live start capability.  Do not return from Rust for
    // resume or completed-state refusal: the broker is the sole mutation
    // authority and must make every persisted start decision.  ASVS 2.3.3,
    // 2.3.4.
    let (expected_latest_run, prior_receipt, decision) = if let Some((reference, prior_revision)) =
        prior_locator
    {
        let locator = crate::RehearsalLocator {
            actor: command.actor,
            course: command.course,
            assignment: command.assignment,
            revision: prior_revision,
            rehearsal: reference,
        };
        let prior = hydration::load_authorized(&mut tx, tenant, locator, &source).await?;
        (
            Some(
                prepared
                    .latest_run
                    .ok_or_else(|| {
                        StoreError::InvalidRecord("missing prepared latest rehearsal run".into())
                    })?
                    .as_uuid(),
            ),
            Some(prior.run.receipt.clone()),
            decide_start(
                Some(RehearsalLifecycleSnapshot {
                    lifecycle: prior.run.receipt.lifecycle,
                    revision: prior.run.receipt.revision,
                    subject_fingerprint: prior.run.subject_fingerprint,
                }),
                command.revision,
                fingerprint,
                command.start_new_after_completion,
            ),
        )
    } else {
        (
            None,
            None,
            decide_start(
                None,
                command.revision,
                fingerprint,
                command.start_new_after_completion,
            ),
        )
    };
    let run_id = RehearsalRunId::from_uuid(crate::random_uuid::random_uuid_v4(|error| {
        StoreError::Unavailable(format!("rehearsal ID randomness unavailable: {error}"))
    })?);
    let genesis = evidence_genesis_head(RehearsalGenesisContext {
        rehearsal: run_id,
        tenant,
        course: command.course,
        assignment: command.assignment,
        direct_instructor_membership: source.owner,
        revision: command.revision,
        subject_fingerprint: fingerprint,
    });
    let reference: Option<i64> =
        // The fixed-shape capability holds the durable workflow decision;
        // every value is separately bound (ASVS 1.2.4).
        sqlx::query_scalar(
            "SELECT ple_rehearsal_start($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
            .bind(tenant.as_uuid())
            .bind(command.actor.as_uuid())
            .bind(command.course.as_uuid())
            .bind(source.assignment.as_uuid())
            .bind(i32::try_from(command.assignment.number()).map_err(|_| {
                StoreError::InvalidRecord("assignment reference exceeds database range".into())
            })?)
            .bind(i64::try_from(command.revision.value()).map_err(|_| {
                StoreError::InvalidRecord("teaching revision exceeds database range".into())
            })?)
            .bind(serde_json::to_value(&subject).map_err(|_| {
                StoreError::InvalidRecord("rehearsal subject serialization failed".into())
            })?)
            .bind(fingerprint.as_bytes().to_vec())
            .bind(genesis.digest().as_bytes().to_vec())
            .bind(run_id.as_uuid())
            .bind(command.start_new_after_completion)
            .bind(expected_latest_run)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
    let reference = reference
        .and_then(|v| question_model::RehearsalReference::new(u64::try_from(v).ok()?))
        .ok_or(StoreError::Conflict)?;
    let locator = crate::RehearsalLocator {
        actor: command.actor,
        course: command.course,
        assignment: command.assignment,
        revision: command.revision,
        rehearsal: reference,
    };
    let receipt = hydration::load_authorized(&mut tx, tenant, locator, &source)
        .await?
        .run
        .receipt;
    require_capability_outcome(decision, prior_receipt.as_ref(), &receipt)?;
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(receipt)
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
    command: &crate::StartRehearsalCommand,
) -> Result<ResolvedStartSubject, StoreError> {
    match &command.subject {
        RehearsalSubjectStart::Synthetic { request } => {
            let prepared = auth::prepare_start(
                tx,
                tenant,
                command.actor,
                command.course,
                command.assignment,
                command.revision,
                None,
            )
            .await?;
            let evaluation = super::super::preview_plane::resolve_synthetic_preview_read_only(
                tx,
                tenant,
                command.course,
                question_model::SyntheticPreviewSubjectRequest {
                    assignment: command.assignment,
                    revision: command.revision,
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
                command.revision,
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
                    command.revision,
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

/// Reject a broker result which does not agree with the aggregate verified
/// under the source locks immediately before the call.  This is a consistency
/// assertion, not an alternate authorization or mutation path: a successful
/// result is always the durable SQL capability outcome (ASVS 2.3.1, 2.3.3).
fn require_capability_outcome(
    decision: RehearsalStartDecision,
    prior: Option<&question_model::RehearsalRunReceipt>,
    receipt: &question_model::RehearsalRunReceipt,
) -> Result<(), StoreError> {
    let matches = match decision {
        RehearsalStartDecision::Resume => prior == Some(receipt),
        RehearsalStartDecision::Create | RehearsalStartDecision::DiscardByNewSubjectThenCreate => {
            receipt.lifecycle.is_active()
                && prior.is_none_or(|prior_receipt| prior_receipt.rehearsal != receipt.rehearsal)
        }
        RehearsalStartDecision::RequireExplicitRestart
        | RehearsalStartDecision::DiscardStaleRevision => false,
    };
    matches.then_some(()).ok_or_else(|| {
        StoreError::InvalidRecord(
            "rehearsal start capability outcome disagrees with aggregate".into(),
        )
    })
}

async fn find_unique_derived_membership(
    tx: &mut Transaction<'_, Postgres>,
    tenant: question_model::TenantId,
    command: &crate::StartRehearsalCommand,
    candidate: &question_model::PreviewSubject,
) -> Result<DerivedMembershipCandidate, StoreError> {
    domain::validate_subject_binding(command.assignment, command.revision, candidate)
        .map_err(|_| StoreError::NotFound)?;
    let assignment_id = sqlx::query_scalar::<_, sqlx::types::Uuid>(
        "SELECT assignment_id FROM assignment WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3 AND revision=$4",
    )
    .bind(tenant.as_uuid())
    .bind(command.course.as_uuid())
    .bind(i64::from(command.assignment.number()))
    .bind(i64::try_from(command.revision.value()).map_err(|_| StoreError::InvalidRecord("teaching revision exceeds database range".into()))?)
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
                command.revision,
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
            .find("#[allow(clippy::too_many_arguments)]")
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
