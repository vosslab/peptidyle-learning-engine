//! Exact-byte codec for immutable rehearsal delivery material.
//!
//! This is deliberately the only PostgreSQL-side serialization boundary for
//! `PrefetchedPrivateExecutionV1`.  The public planner and HTTP contracts
//! cannot serialize the value, while the Store can validate and persist a
//! closed byte sequence with its raw SHA-256 witness.

use objects::Sha256Digest;
use question_model::{AssignmentId, ProblemId, QuestionSource, VersionId};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};

use super::super::*;
use crate::{IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1};

/// Store-private, locked normal-assignment source material.  It has no route
/// DTO, serialization implementation, or public constructor: the start
/// transaction consumes it to create one immutable frozen row per source.
pub(super) struct LockedNormalAssignmentSource {
    pub(super) ordinal: u32,
    pub(super) assignment_item_id: uuid::Uuid,
    pub(super) family_capability: String,
    pub(super) question: question_model::QuestionDefinition,
    pub(super) issued_snapshot: IssuedQuestionSnapshotV1,
    pub(super) private_execution: crate::PrefetchedPrivateExecutionV1,
}

/// Private arguments for the transaction-bound ordinary-source broker.
/// Grouping the route identity with the operation nonce makes the source lock
/// relationship explicit and prevents a growing positional broker boundary.
pub(super) struct LockedAssignmentSourceRequest {
    pub(super) tenant: question_model::TenantId,
    pub(super) actor: question_model::UserId,
    pub(super) course: question_model::CourseId,
    pub(super) assignment_reference: question_model::AssignmentReference,
    pub(super) revision: question_model::TeachingOperationRevision,
    pub(super) assignment: AssignmentId,
    pub(super) operation: uuid::Uuid,
    pub(super) nonce: uuid::Uuid,
}

impl LockedNormalAssignmentSource {
    /// Forces the exact bytes and all cross-field bindings to be constructed
    /// while the normal source rows remain locked.  The finalize broker writes
    /// these same values; this preflight keeps unsupported or malformed second
    /// items from creating a partial rehearsal run.
    pub(super) fn validate_for_freeze(&self) -> Result<(), StoreError> {
        crate::ensure_rehearsal_question_source_supported(&self.question)?;
        let expected_family_capability = match &self.question.source {
            QuestionSource::Native { family } => format!("native:{family}"),
            _ => {
                return Err(StoreError::Unavailable(
                    "unsupported rehearsal source family".into(),
                ));
            }
        };
        if self.family_capability != expected_family_capability {
            return Err(StoreError::Unavailable(
                "locked rehearsal source family capability is inconsistent".into(),
            ));
        }
        if self.question.problem != self.issued_snapshot.question().problem
            || self.question.version != self.issued_snapshot.question().version
        {
            return Err(StoreError::Unavailable(
                "locked rehearsal source snapshot identity is inconsistent".into(),
            ));
        }
        let _ = self.ordinal;
        let _ = self.assignment_item_id;
        let _ = self.issued_snapshot.canonical_payload_bytes()?;
        let _ = encode_private_bytes(&self.private_execution)?;
        Ok(())
    }
}

/// Locks the exact active ordinary assignment source set in deterministic
/// `(position, assignment_item_id)` order.  This is deliberately an internal
/// Store capability; the browser never receives the catalog payload, private
/// flat key, assignment-item UUID, or raw source identity.
pub(super) async fn resolve_locked_normal_assignment_sources(
    tx: &mut Transaction<'_, Postgres>,
    request: LockedAssignmentSourceRequest,
) -> Result<Vec<LockedNormalAssignmentSource>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM public.ple_prepare_rehearsal_start_sources($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(request.tenant.as_uuid())
    .bind(request.actor.as_uuid())
    .bind(request.course.as_uuid())
    .bind(
        i32::try_from(request.assignment_reference.number()).map_err(|_| {
            StoreError::InvalidRecord("assignment reference exceeds database range".into())
        })?,
    )
    .bind(i64::try_from(request.revision.value()).map_err(|_| {
        StoreError::InvalidRecord("teaching revision exceeds database range".into())
    })?)
    .bind(request.operation)
    .bind(request.nonce)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    if rows.is_empty() {
        return Err(StoreError::Conflict);
    }
    let mut result = Vec::with_capacity(rows.len());
    for (ordinal, row) in rows.into_iter().enumerate() {
        let returned_assignment: uuid::Uuid =
            row.try_get("assignment_id").map_err(map_sqlx_error)?;
        if returned_assignment != request.assignment.as_uuid() {
            return Err(StoreError::Unavailable(
                "locked rehearsal source returned a different assignment".into(),
            ));
        }
        let problem = ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?);
        let version = VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?);
        let family_capability: String = row.try_get("family_capability").map_err(map_sqlx_error)?;
        let record: crate::PublishedProblemRecord =
            serde_json::from_value(row.try_get("payload").map_err(map_sqlx_error)?).map_err(
                |_| StoreError::Unavailable("locked rehearsal source payload is invalid".into()),
            )?;
        if record.problem != problem || record.version != version {
            return Err(StoreError::Unavailable(
                "locked rehearsal source identity disagrees with its payload".into(),
            ));
        }
        crate::ensure_rehearsal_question_source_supported(&record.question)?;
        let (witness, private_execution) = match &record.question.source {
            QuestionSource::Native { family }
                if grading::flat_question::is_flat_question_family(family) =>
            {
                let key_row = sqlx::query(
                    "SELECT * FROM public.ple_prepare_rehearsal_flat_grading_material($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                )
                .bind(request.tenant.as_uuid())
                .bind(request.actor.as_uuid())
                .bind(request.course.as_uuid())
                .bind(i32::try_from(request.assignment_reference.number()).map_err(|_| {
                    StoreError::InvalidRecord("assignment reference exceeds database range".into())
                })?)
                .bind(i64::try_from(request.revision.value()).map_err(|_| {
                    StoreError::InvalidRecord("teaching revision exceeds database range".into())
                })?)
                .bind(problem.as_uuid())
                .bind(version.as_uuid())
                .bind(request.operation)
                .bind(request.nonce)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_sqlx_error)?
                .ok_or_else(|| {
                    StoreError::Unavailable(
                        "locked flat rehearsal source lacks its sealed grading authority".into(),
                    )
                })?;
                let grading: crate::FlatQuestionGradingPayload =
                    serde_json::from_value(key_row.try_get("key_payload").map_err(map_sqlx_error)?)
                        .map_err(|_| {
                            StoreError::Unavailable(
                                "locked flat rehearsal grading is invalid".into(),
                            )
                        })?;
                let contract =
                    crate::IssuedFlatGradingContract::new(record.question.clone(), grading)?;
                (
                    IssuedQuestionFamilyWitnessV1::Flat {},
                    crate::PrefetchedPrivateExecutionV1 {
                        flat_grading: Some(contract),
                        webwork_replay: None,
                        webwork_grading: None,
                        qti_grading: None,
                    },
                )
            }
            QuestionSource::Native { .. } => {
                // A native witness may be empty only for a definition with no
                // asset reference at all.  The rehearsal source broker does
                // not yet expose the published physical rendition registry,
                // so a referenced asset is a positive unsupported source
                // rather than fabricated empty provenance.
                if contains_asset_reference(&record.question)? {
                    return Err(StoreError::Unavailable(
                        "native rehearsal source requires immutable asset bindings".into(),
                    ));
                }
                (
                    IssuedQuestionFamilyWitnessV1::Native {
                        physical_asset_bindings: Vec::new(),
                    },
                    crate::PrefetchedPrivateExecutionV1 {
                        flat_grading: None,
                        webwork_replay: None,
                        webwork_grading: None,
                        qti_grading: None,
                    },
                )
            }
            _ => unreachable!("source-family gate accepts only native sources"),
        };
        result.push(LockedNormalAssignmentSource {
            ordinal: u32::try_from(ordinal)
                .map_err(|_| StoreError::Unavailable("rehearsal source count overflow".into()))?,
            assignment_item_id: row.try_get("assignment_item_id").map_err(map_sqlx_error)?,
            family_capability,
            issued_snapshot: IssuedQuestionSnapshotV1::new(record.question.clone(), witness)?,
            question: record.question,
            private_execution,
        });
    }
    Ok(result)
}

/// Reject native content with any logical asset reference until the source
/// broker can capture physical publication bindings.  The closed model uses
/// `asset` only for logical asset references; recursive inspection keeps this
/// defensive boundary correct as response families gain nested content.
fn contains_asset_reference(
    question: &question_model::QuestionDefinition,
) -> Result<bool, StoreError> {
    fn contains(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Array(values) => values.iter().any(contains),
            serde_json::Value::Object(values) => {
                values.contains_key("asset") || values.values().any(contains)
            }
            _ => false,
        }
    }
    serde_json::to_value(question)
        .map(|value| contains(&value))
        .map_err(|_| {
            StoreError::Unavailable("native rehearsal source cannot inspect assets".into())
        })
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistedPrivateExecutionV1 {
    flat_grading: Option<crate::IssuedFlatGradingContract>,
    webwork_replay: Option<crate::WebworkReplayMappingV1>,
    webwork_grading: Option<crate::IssuedWebworkGradingContract>,
    qti_grading: Option<crate::IssuedQtiGradingContractV1>,
}

/// Serializes a closed private execution witness once, so the value stored in
/// PostgreSQL and the value hashed by Rust are the same bytes.  ASVS 1.5.2,
/// 2.2.1, and 2.3.3: callers validate the corresponding issued snapshot
/// before this function is reached; later readers use `decode_private_bytes`.
pub(super) fn encode_private_bytes(
    execution: &crate::PrefetchedPrivateExecutionV1,
) -> Result<(Vec<u8>, Sha256Digest), StoreError> {
    let persisted = PersistedPrivateExecutionV1 {
        flat_grading: execution.flat_grading.clone(),
        webwork_replay: execution.webwork_replay.clone(),
        webwork_grading: execution.webwork_grading.clone(),
        qti_grading: execution.qti_grading.clone(),
    };
    let bytes = serde_json::to_vec(&persisted).map_err(|_| {
        StoreError::InvalidRecord("rehearsal private execution serialization failed".into())
    })?;
    if bytes.len() > 512 * 1024 {
        return Err(StoreError::InvalidRecord(
            "rehearsal private execution exceeds its bounded persistence size".into(),
        ));
    }
    Ok((bytes.clone(), Sha256Digest::compute(&bytes)))
}

/// Validates raw stored bytes before deserializing a closed private contract.
pub(super) fn decode_private_bytes(
    bytes: &[u8],
    expected_digest: &[u8; 32],
) -> Result<crate::PrefetchedPrivateExecutionV1, StoreError> {
    if bytes.len() > 512 * 1024 || *Sha256Digest::compute(bytes).as_bytes() != *expected_digest {
        return Err(StoreError::Unavailable(
            "stored rehearsal private execution checksum mismatch".into(),
        ));
    }
    let persisted: PersistedPrivateExecutionV1 = serde_json::from_slice(bytes).map_err(|_| {
        StoreError::Unavailable("stored rehearsal private execution is invalid".into())
    })?;
    let canonical = serde_json::to_vec(&persisted).map_err(|_| {
        StoreError::Unavailable("stored rehearsal private execution is invalid".into())
    })?;
    if canonical != bytes {
        return Err(StoreError::Unavailable(
            "stored rehearsal private execution contains unknown fields".into(),
        ));
    }
    Ok(crate::PrefetchedPrivateExecutionV1 {
        flat_grading: persisted.flat_grading,
        webwork_replay: persisted.webwork_replay,
        webwork_grading: persisted.webwork_grading,
        qti_grading: persisted.qti_grading,
    })
}

/// Verifies the sealed, immutable material set through a one-bit route
/// capability. The broker returns no candidate identity, source payload,
/// snapshot, or private grading material. It authorizes the ordinary route
/// and recomputes the frozen inventory commitments inside PostgreSQL; later
/// claim and grading brokers independently consume the sealed material.
pub(super) async fn verify_from_route(
    store: &PostgresStore,
    context: crate::TenantContext,
    command: crate::VerifyRehearsalDeliveryMaterialRouteCommand,
) -> Result<(), StoreError> {
    let route = command.route;
    let tenant = context.tenant_id();
    let mut tx = store.begin_tenant(context).await?;
    let material_valid: Option<bool> = sqlx::query_scalar(
        "SELECT material_valid FROM public.ple_verify_rehearsal_delivery_material_from_route($1,$2,$3,$4,$5,$6)",
    )
    .bind(tenant.as_uuid())
    .bind(route.actor.as_uuid())
    .bind(route.course.as_uuid())
    .bind(i32::try_from(route.assignment.number()).map_err(|_| {
        StoreError::InvalidRecord("assignment reference exceeds database range".into())
    })?)
    .bind(i64::try_from(route.expected_revision.value()).map_err(|_| {
        StoreError::InvalidRecord("teaching revision exceeds database range".into())
    })?)
    .bind(i64::from(route.rehearsal.number()))
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sqlx_error)?;
    match material_valid {
        None => return Err(StoreError::NotFound),
        Some(false) => {
            return Err(StoreError::Unavailable(
                "frozen rehearsal material failed integrity verification".into(),
            ));
        }
        Some(true) => {}
    }
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(())
}

/// Finalizes the prepared start only after every ordinary frozen item has been
/// appended in this transaction.  The payload columns are parallel, typed
/// arrays rather than a generic JSON plan: each byte sequence is validated
/// and hashed once by Rust before PostgreSQL checks the complete ordered set.
pub(super) struct FinalizeStartFreeze<'a> {
    pub(super) tenant: question_model::TenantId,
    pub(super) operation: uuid::Uuid,
    pub(super) nonce: uuid::Uuid,
    pub(super) witness_digest: Vec<u8>,
    pub(super) receipt: &'a question_model::RehearsalRunReceipt,
    pub(super) subject: &'a question_model::PreviewSubject,
}

pub(super) async fn finalize_start_freeze(
    tx: &mut Transaction<'_, Postgres>,
    command: FinalizeStartFreeze<'_>,
    sources: &[LockedNormalAssignmentSource],
    frozen: &[question_model::RehearsalFrozenItemEvidence],
) -> Result<(), StoreError> {
    if sources.len() != frozen.len() || sources.is_empty() {
        return Err(StoreError::Conflict);
    }
    let mut attempts = Vec::with_capacity(sources.len());
    let mut assignment_items = Vec::with_capacity(sources.len());
    let mut problems = Vec::with_capacity(sources.len());
    let mut versions = Vec::with_capacity(sources.len());
    let mut responses = Vec::with_capacity(sources.len());
    let mut response_digests = Vec::with_capacity(sources.len());
    let mut content_digests = Vec::with_capacity(sources.len());
    let mut seeds = Vec::with_capacity(sources.len());
    let mut algorithms = Vec::with_capacity(sources.len());
    let mut snapshots = Vec::with_capacity(sources.len());
    let mut snapshot_digests = Vec::with_capacity(sources.len());
    let mut private = Vec::with_capacity(sources.len());
    let mut private_digests = Vec::with_capacity(sources.len());
    let mut timing_kinds = Vec::with_capacity(sources.len());
    let mut timing_seconds = Vec::with_capacity(sources.len());
    let mut timing_grace_seconds = Vec::with_capacity(sources.len());
    // Versioned source-set bytes are shared with the finalizer.  They bind the
    // assignment revision and every ordered source scalar before the immutable
    // material header is committed; no PostgreSQL JSON rendering participates.
    let mut source_digest_input = Vec::with_capacity(10 + sources.len() * 128);
    source_digest_input.extend_from_slice(&2_u16.to_be_bytes());
    source_digest_input.extend_from_slice(
        &i64::try_from(command.receipt.revision.value())
            .map_err(|_| {
                StoreError::InvalidRecord("teaching revision exceeds database range".into())
            })?
            .to_be_bytes(),
    );
    for (source, item) in sources.iter().zip(frozen) {
        source.validate_for_freeze()?;
        if source.question.problem != item.problem.problem
            || source.question.version != item.problem.version
            || source.question.response != item.response_definition
        {
            return Err(StoreError::Unavailable(
                "locked rehearsal source no longer matches frozen item".into(),
            ));
        }
        let (snapshot, snapshot_digest) = source.issued_snapshot.canonical_payload_bytes()?;
        let (execution, execution_digest) = encode_private_bytes(&source.private_execution)?;
        attempts.push(item.attempt.as_uuid());
        assignment_items.push(source.assignment_item_id);
        problems.push(item.problem.problem.as_uuid());
        versions.push(item.problem.version.as_uuid());
        responses.push(
            serde_json::to_value(&item.response_definition).map_err(|_| {
                StoreError::InvalidRecord("rehearsal response serialization failed".into())
            })?,
        );
        response_digests.push(
            domain::frozen_response_schema_digest(&item.response_definition)
                .as_bytes()
                .to_vec(),
        );
        content_digests.push(item.canonical_content_digest.as_bytes().to_vec());
        // V1 has no randomized rehearsal generator yet; retain explicit,
        // stable source values so future algorithms extend this row shape.
        seeds.push(0_i64);
        algorithms.push(1_i32);
        source_digest_input.extend_from_slice(&source.ordinal.to_be_bytes());
        source_digest_input.extend_from_slice(source.assignment_item_id.as_bytes());
        source_digest_input.extend_from_slice(item.problem.problem.as_uuid().as_bytes());
        source_digest_input.extend_from_slice(item.problem.version.as_uuid().as_bytes());
        source_digest_input.extend_from_slice(response_digests.last().expect("just pushed"));
        source_digest_input.extend_from_slice(content_digests.last().expect("just pushed"));
        let family_bytes = source.family_capability.as_bytes();
        source_digest_input.extend_from_slice(
            &u32::try_from(family_bytes.len())
                .map_err(|_| StoreError::Unavailable("family capability is too large".into()))?
                .to_be_bytes(),
        );
        source_digest_input.extend_from_slice(family_bytes);
        source_digest_input.extend_from_slice(&0_i64.to_be_bytes());
        source_digest_input.extend_from_slice(&1_i32.to_be_bytes());
        snapshots.push(snapshot);
        snapshot_digests.push(snapshot_digest.as_bytes().to_vec());
        private.push(execution);
        private_digests.push(execution_digest.as_bytes().to_vec());
        match source.question.timing_policy {
            question_model::run_policy::TimingPolicy::Untimed => {
                timing_kinds.push("untimed");
                timing_seconds.push(None);
                timing_grace_seconds.push(None);
            }
            question_model::run_policy::TimingPolicy::PerQuestion {
                seconds,
                grace_seconds,
            } => {
                timing_kinds.push("perQuestion");
                timing_seconds.push(Some(i32::try_from(seconds).map_err(|_| {
                    StoreError::InvalidRecord("rehearsal timing exceeds database range".into())
                })?));
                timing_grace_seconds.push(Some(i32::try_from(grace_seconds).map_err(|_| {
                    StoreError::InvalidRecord("rehearsal grace exceeds database range".into())
                })?));
            }
            question_model::run_policy::TimingPolicy::PerAttempt {
                seconds,
                grace_seconds,
            } => {
                timing_kinds.push("perAttempt");
                timing_seconds.push(Some(i32::try_from(seconds).map_err(|_| {
                    StoreError::InvalidRecord("rehearsal timing exceeds database range".into())
                })?));
                timing_grace_seconds.push(Some(i32::try_from(grace_seconds).map_err(|_| {
                    StoreError::InvalidRecord("rehearsal grace exceeds database range".into())
                })?));
            }
        }
        let timing_kind = timing_kinds
            .last()
            .expect("timing kind was pushed")
            .as_bytes();
        source_digest_input.extend_from_slice(&(timing_kind.len() as u32).to_be_bytes());
        source_digest_input.extend_from_slice(timing_kind);
        for value in [
            timing_seconds.last().expect("timing seconds was pushed"),
            timing_grace_seconds
                .last()
                .expect("timing grace was pushed"),
        ] {
            source_digest_input.push(u8::from(value.is_some()));
            source_digest_input.extend_from_slice(&value.unwrap_or_default().to_be_bytes());
        }
    }
    let response = serde_json::to_value(command.receipt)
        .map_err(|_| StoreError::InvalidRecord("rehearsal receipt serialization failed".into()))?;
    let response_bytes = serde_json::to_vec(&response)
        .map_err(|_| StoreError::InvalidRecord("rehearsal receipt encoding failed".into()))?;
    let response_digest = Sha256Digest::compute(&response_bytes);
    let subject_limit = command
        .subject
        .policy
        .time_limit_seconds()
        .value
        .map(|value| {
            i32::try_from(value).map_err(|_| {
                StoreError::InvalidRecord("subject time limit exceeds database range".into())
            })
        })
        .transpose()?;
    // The resolved subject limit is part of the frozen source-set commitment,
    // including its explicit null tag.  A later policy edit cannot alter a
    // run-anchored deadline without invalidating the immutable material set.
    source_digest_input.push(u8::from(subject_limit.is_some()));
    source_digest_input.extend_from_slice(&subject_limit.unwrap_or_default().to_be_bytes());
    let source_set_digest = Sha256Digest::compute(&source_digest_input);
    let completed = sqlx::query("SELECT * FROM public.ple_finalize_rehearsal_start_freeze($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)")
        .bind(command.tenant.as_uuid()).bind(command.operation).bind(command.nonce).bind(command.witness_digest)
        .bind(source_set_digest.as_bytes().to_vec()).bind(i32::try_from(sources.len()).map_err(|_| StoreError::Unavailable("rehearsal source count overflow".into()))?)
        .bind(assignment_items).bind(attempts).bind(problems).bind(versions).bind(responses).bind(response_digests).bind(content_digests)
        .bind(seeds).bind(algorithms).bind(snapshots).bind(snapshot_digests).bind(private).bind(private_digests)
        .bind(timing_kinds).bind(timing_seconds).bind(timing_grace_seconds).bind(subject_limit)
        .bind(response_bytes).bind(response_digest.as_bytes().to_vec())
        .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?.ok_or(StoreError::Conflict)?;
    let persisted: serde_json::Value = completed
        .try_get("response_projection")
        .map_err(map_sqlx_error)?;
    if persisted
        != serde_json::to_value(command.receipt).map_err(|_| {
            StoreError::InvalidRecord("rehearsal receipt serialization failed".into())
        })?
    {
        return Err(StoreError::Unavailable(
            "rehearsal start receipt changed before material finalization".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_bytes_reject_tampering_and_unknown_keys() {
        let execution = crate::PrefetchedPrivateExecutionV1 {
            flat_grading: None,
            webwork_replay: None,
            webwork_grading: None,
            qti_grading: None,
        };
        let (bytes, digest) = encode_private_bytes(&execution).expect("encode");
        assert!(decode_private_bytes(&bytes, digest.as_bytes()).is_ok());
        let mut changed = bytes.clone();
        changed.push(b' ');
        assert!(decode_private_bytes(&changed, digest.as_bytes()).is_err());
        let unknown = br#"{\"flatGrading\":null,\"webworkReplay\":null,\"webworkGrading\":null,\"qtiGrading\":null,\"extra\":true}"#;
        let digest = Sha256Digest::compute(unknown);
        assert!(decode_private_bytes(unknown, digest.as_bytes()).is_err());
    }
}
