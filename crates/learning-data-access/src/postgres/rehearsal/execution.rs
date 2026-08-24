//! Grader-only issue-or-resume persistence for one rehearsal generation.
//!
//! The source rows are immutable inputs. This adapter alone commits the
//! exact canonical issue/render result (ASVS 2.3.1 and 2.3.3).

use sqlx::Row;

use super::super::*;
use super::material;

/// Rebuild the exact frozen response-schema witness from sealed source
/// material.  The timestamp is deliberately not used by response validation;
/// it is still supplied as a stable value because the domain witness is a
/// complete frozen-item value rather than a partial schema shortcut.
fn frozen_for_submission(
    attempt: question_model::RehearsalAttemptId,
    snapshot: &crate::IssuedQuestionSnapshotV1,
    canonical_content_digest: [u8; 32],
) -> question_model::RehearsalFrozenItemEvidence {
    question_model::RehearsalFrozenItemEvidence {
        attempt,
        problem: question_model::ProblemVersionRef {
            problem: snapshot.question().problem,
            version: snapshot.question().version,
        },
        response_definition: snapshot.question().response.clone(),
        canonical_content_digest: question_model::RehearsalEvidenceDigest::from_bytes(
            canonical_content_digest,
        ),
        frozen_at: question_model::ActivityTimestamp::from_unix_millis(0),
    }
}

fn submission_execution_from_row(
    context: TenantContext,
    route: crate::RehearsalRouteIdentity,
    row: &sqlx::postgres::PgRow,
) -> Result<
    (
        crate::SealedRehearsalGradingParts,
        crate::SealedRehearsalSubmissionCompletion,
    ),
    StoreError,
> {
    let attempt = question_model::RehearsalAttemptId::from_uuid(
        row.try_get("attempt_id").map_err(map_sqlx_error)?,
    );
    let operation = crate::RehearsalOperationId::from_uuid(
        row.try_get("delivery_operation_id")
            .map_err(map_sqlx_error)?,
    );
    let descriptor = crate::RehearsalDeliveryExecutionDescriptorV1::decode_persisted(
        &row.try_get("execution_descriptor")
            .map_err(map_sqlx_error)?,
    )?;
    if descriptor.attempt() != attempt {
        return Err(StoreError::Unavailable(
            "sealed rehearsal submission work has a foreign attempt".into(),
        ));
    }
    let snapshot_bytes: Vec<u8> = row
        .try_get("issued_snapshot_bytes")
        .map_err(map_sqlx_error)?;
    let snapshot_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("issued_snapshot_sha256")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal snapshot digest is invalid".into())
        })?;
    let private_bytes: Vec<u8> = row
        .try_get("private_execution_bytes")
        .map_err(map_sqlx_error)?;
    let private_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("private_execution_sha256")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal private digest is invalid".into())
        })?;
    let artifact_bytes: Vec<u8> = row.try_get("artifact_bytes").map_err(map_sqlx_error)?;
    let artifact_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("artifact_sha256")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal artifact digest is invalid".into())
        })?;
    let snapshot =
        crate::IssuedQuestionSnapshotV1::decode_checked_bytes(&snapshot_bytes, &snapshot_digest)?;
    let private_execution = material::decode_private_bytes(&private_bytes, &private_digest)?;
    let artifact = crate::RehearsalIssuedExecutionArtifactV1::from_persisted_bytes(artifact_bytes)?;
    if *objects::Sha256Digest::compute(artifact.bytes()).as_bytes() != artifact_digest {
        return Err(StoreError::Unavailable(
            "sealed rehearsal artifact checksum mismatch".into(),
        ));
    }
    let issue_work = crate::SealedRehearsalDeliveryIssueWork::new(
        operation,
        descriptor,
        snapshot.clone(),
        private_execution.clone(),
        crate::RehearsalOperationDigest::from_bytes(private_digest),
    );
    let execution = artifact.decode_for_work(&issue_work)?;
    let screen = execution.active_screen()?;
    let expected_digest: Vec<u8> = row
        .try_get("issued_screen_digest")
        .map_err(map_sqlx_error)?;
    let screen_commitment = screen.commitment().map_err(|_| StoreError::Conflict)?;
    if screen_commitment.as_bytes() != expected_digest.as_slice() {
        return Err(StoreError::Conflict);
    }
    let canonical_content_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("canonical_content_digest")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal content digest is invalid".into())
        })?;
    let frozen = frozen_for_submission(attempt, &snapshot, canonical_content_digest);
    let submission_input: serde_json::Value =
        row.try_get("submission_input").map_err(map_sqlx_error)?;
    let run = question_model::RehearsalRunId::from_uuid(
        row.try_get("rehearsal_run_id").map_err(map_sqlx_error)?,
    );
    let claim = question_model::RehearsalSubmissionClaimId::from_uuid(
        row.try_get("claim_id").map_err(map_sqlx_error)?,
    );
    let fingerprint: Vec<u8> = row.try_get("request_fingerprint").map_err(map_sqlx_error)?;
    let subject_fingerprint: Vec<u8> =
        row.try_get("subject_fingerprint").map_err(map_sqlx_error)?;
    let owner = question_model::CourseMembershipId::from_uuid(
        row.try_get("direct_instructor_membership_id")
            .map_err(map_sqlx_error)?,
    );
    let assignment = question_model::AssignmentReference::new(
        u64::try_from(
            row.try_get::<i32, _>("assignment_reference")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal assignment reference is invalid".into())
        })?,
    )
    .ok_or_else(|| {
        StoreError::Unavailable("sealed rehearsal assignment reference is invalid".into())
    })?;
    let revision = question_model::TeachingOperationRevision::new(
        u64::try_from(
            row.try_get::<i64, _>("assignment_revision")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::Unavailable("sealed rehearsal revision is invalid".into()))?,
    )
    .ok_or_else(|| StoreError::Unavailable("sealed rehearsal revision is invalid".into()))?;
    let genesis = domain::RehearsalGenesisContext {
        rehearsal: run,
        tenant: question_model::TenantId::from_uuid(
            row.try_get("tenant_id").map_err(map_sqlx_error)?,
        ),
        course: question_model::CourseId::from_uuid(
            row.try_get("course_id").map_err(map_sqlx_error)?,
        ),
        assignment,
        direct_instructor_membership: owner,
        revision,
        subject_fingerprint: domain::rehearsal::persistence::restore_subject_fingerprint(
            &subject_fingerprint,
        )
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal subject fingerprint is invalid".into())
        })?,
    };
    let persisted = domain::rehearsal::persistence::decode_persisted_claim_root_with_screen(
        run,
        claim,
        &fingerprint,
        &submission_input,
        &frozen,
        attempt,
        Some(&screen),
    )
    .map_err(|_| StoreError::Conflict)?;
    let root = domain::RehearsalClaimRoot::verify_persisted(genesis, &frozen, persisted)
        .map_err(|_| StoreError::Conflict)?;
    let domain::RehearsalClaimSubmissionInput::Rendered(rendered) = root.submission_input() else {
        return Err(StoreError::Conflict);
    };
    if rendered.presentation_commitment() != screen_commitment {
        return Err(StoreError::Conflict);
    }
    let grading = execution.into_grading_parts(rendered.response())?;
    // Revalidate after the only rendered-ID translation seam before an
    // adapter sees the durable answer (ASVS V2.2.1/V2.3.1).
    let durable_request = domain::RehearsalValidatedSubmissionRequest::try_from_frozen_attempt(
        &frozen,
        attempt,
        grading.response().clone(),
    )
    .map_err(|_| StoreError::Conflict)?;
    let generation = row
        .try_get::<i32, _>("claim_generation")
        .map_err(map_sqlx_error)?;
    let generation = u32::try_from(generation)
        .map_err(|_| StoreError::Unavailable("sealed rehearsal generation is invalid".into()))?;
    let head_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("evidence_head_digest")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| StoreError::Unavailable("sealed rehearsal evidence head is invalid".into()))?;
    let head_length = u32::try_from(
        row.try_get::<i64, _>("evidence_length")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::Unavailable("sealed rehearsal evidence length is invalid".into()))?;
    Ok((
        grading,
        crate::SealedRehearsalSubmissionCompletion::new(
            crate::SealedRehearsalSubmissionCompletionParts {
                context,
                route,
                handle: domain::rehearsal::restore_sealed_dispatched_claim_handle(
                    run,
                    claim,
                    root.fingerprint(),
                    question_model::RehearsalGradeOperationId::from_uuid(
                        row.try_get("claim_operation_id").map_err(map_sqlx_error)?,
                    ),
                    domain::RehearsalClaimGeneration::from_persisted(generation).ok_or_else(
                        || StoreError::Unavailable("sealed rehearsal generation is invalid".into()),
                    )?,
                ),
                root,
                attempt,
                frozen,
                expected_evidence_head: domain::RehearsalEvidenceHead::from_persisted(
                    question_model::RehearsalEvidenceDigest::from_bytes(head_digest),
                    head_length,
                ),
                presentation_commitment: screen_commitment,
                durable_request,
            },
        ),
    ))
}

fn work_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<crate::SealedRehearsalDeliveryIssueWork, StoreError> {
    let operation = crate::RehearsalOperationId::from_uuid(
        row.try_get("issued_operation_id").map_err(map_sqlx_error)?,
    );
    let descriptor = crate::RehearsalDeliveryExecutionDescriptorV1::decode_persisted(
        &row.try_get("execution_descriptor")
            .map_err(map_sqlx_error)?,
    )?;
    let snapshot_bytes: Vec<u8> = row
        .try_get("issued_snapshot_bytes")
        .map_err(map_sqlx_error)?;
    let snapshot_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("issued_snapshot_sha256")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal snapshot digest is invalid".into())
        })?;
    let private_bytes: Vec<u8> = row
        .try_get("private_execution_bytes")
        .map_err(map_sqlx_error)?;
    let private_digest: [u8; 32] = row
        .try_get::<Vec<u8>, _>("private_execution_sha256")
        .map_err(map_sqlx_error)?
        .try_into()
        .map_err(|_| {
            StoreError::Unavailable("sealed rehearsal private digest is invalid".into())
        })?;
    let issued_snapshot =
        crate::IssuedQuestionSnapshotV1::decode_checked_bytes(&snapshot_bytes, &snapshot_digest)?;
    let private_execution = material::decode_private_bytes(&private_bytes, &private_digest)?;
    if issued_snapshot.question().problem != descriptor.problem().problem
        || issued_snapshot.question().version != descriptor.problem().version
        || issued_snapshot.question().response != *descriptor.response_definition()
    {
        return Err(StoreError::Unavailable(
            "sealed rehearsal issue work disagrees with its descriptor".into(),
        ));
    }
    Ok(crate::SealedRehearsalDeliveryIssueWork::new(
        operation,
        descriptor,
        issued_snapshot,
        private_execution,
        crate::RehearsalOperationDigest::from_bytes(private_digest),
    ))
}

#[async_trait::async_trait]
impl crate::SealedRehearsalDeliveryExecutionStore for crate::postgres::PostgresGraderStore {
    async fn prepare_or_resume_issued_execution(
        &self,
        context: TenantContext,
        dispatched: &crate::DispatchedRehearsalDelivery,
    ) -> Result<crate::SealedRehearsalDeliveryIssuePreparation, StoreError> {
        let mut transaction = self.begin_sealed_reader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_prepare_or_resume_rehearsal_issued_execution($1,$2)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(dispatched.operation().as_uuid())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let work = work_from_row(&row)?;
        if work.operation() != dispatched.operation() {
            return Err(StoreError::Unavailable(
                "sealed rehearsal issue work has a foreign operation".into(),
            ));
        }
        let result_kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
        let result = match result_kind.as_str() {
            "work" => crate::SealedRehearsalDeliveryIssuePreparation::IssueWork(Box::new(work)),
            "existing" => {
                let artifact = crate::RehearsalIssuedExecutionArtifactV1::from_persisted_bytes(
                    row.try_get("artifact_bytes").map_err(map_sqlx_error)?,
                )?;
                let digest: [u8; 32] = row
                    .try_get::<Vec<u8>, _>("artifact_sha256")
                    .map_err(map_sqlx_error)?
                    .try_into()
                    .map_err(|_| {
                        StoreError::Unavailable(
                            "issued rehearsal artifact digest is invalid".into(),
                        )
                    })?;
                if *objects::Sha256Digest::compute(artifact.bytes()).as_bytes() != digest {
                    return Err(StoreError::Unavailable(
                        "issued rehearsal artifact checksum mismatch".into(),
                    ));
                }
                crate::SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(Box::new(
                    artifact.decode_for_work(&work)?,
                ))
            }
            _ => {
                return Err(StoreError::Unavailable(
                    "invalid sealed rehearsal issue preparation".into(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn commit_issued_execution(
        &self,
        context: TenantContext,
        work: crate::SealedRehearsalDeliveryIssueWork,
        artifact: crate::RehearsalIssuedExecutionArtifactV1,
    ) -> Result<crate::SealedRehearsalDeliveryExecution, StoreError> {
        let execution = artifact.decode_for_work(&work)?;
        let digest = objects::Sha256Digest::compute(artifact.bytes());
        let mut transaction = self.begin_sealed_reader_tenant(context).await?;
        let result: String = sqlx::query_scalar(
            "SELECT public.ple_commit_sealed_rehearsal_issued_execution($1,$2,$3,$4)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(work.operation().as_uuid())
        .bind(artifact.bytes())
        .bind(digest.as_bytes().to_vec())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if !matches!(result.as_str(), "committed" | "replay") {
            return Err(StoreError::Conflict);
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(execution)
    }

    async fn prepare_sealed_rehearsal_delivery_execution(
        &self,
        context: TenantContext,
        dispatched: &crate::DispatchedRehearsalDelivery,
    ) -> Result<crate::SealedRehearsalDeliveryExecution, StoreError> {
        match self
            .prepare_or_resume_issued_execution(context, dispatched)
            .await?
        {
            crate::SealedRehearsalDeliveryIssuePreparation::ExistingArtifact(execution) => {
                Ok(*execution)
            }
            crate::SealedRehearsalDeliveryIssuePreparation::IssueWork(_) => {
                Err(StoreError::Conflict)
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::SealedRehearsalSubmissionExecutionStore for crate::postgres::PostgresGraderStore {
    async fn prepare_or_resume_sealed_rehearsal_submission_execution(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        idempotency_key: crate::RehearsalSubmissionIdempotencyKey,
    ) -> Result<crate::SealedRehearsalSubmissionExecutionPreparation, StoreError> {
        let mut transaction = self.begin_sealed_reader_tenant(context).await?;
        let row = sqlx::query(
            "SELECT * FROM public.ple_prepare_or_resume_sealed_rehearsal_submission($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(route.actor.as_uuid())
        .bind(route.course.as_uuid())
        .bind(i32::try_from(route.assignment.number()).map_err(|_| {
            StoreError::InvalidRecord("assignment reference exceeds database range".into())
        })?)
        .bind(i64::try_from(route.expected_revision.value()).map_err(|_| {
            StoreError::InvalidRecord("teaching revision exceeds database range".into())
        })?)
        .bind(i64::from(route.rehearsal.number()))
        .bind(idempotency_key.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
        let result = match kind.as_str() {
            "pending" => crate::SealedRehearsalSubmissionExecutionPreparation::PendingPreparation,
            "receipt" => {
                let projection: serde_json::Value =
                    row.try_get("outcome_projection").map_err(map_sqlx_error)?;
                let receipt =
                    domain::rehearsal::persistence::decode_persisted_rehearsal_receipt(&projection)
                        .map_err(|_| {
                            StoreError::Unavailable("sealed rehearsal receipt is invalid".into())
                        })?;
                let digest: Vec<u8> = row.try_get("receipt_digest").map_err(map_sqlx_error)?;
                if digest.as_slice()
                    != domain::rehearsal::persistence::persisted_rehearsal_receipt_digest(&receipt)
                        .as_bytes()
                {
                    return Err(StoreError::Unavailable(
                        "sealed rehearsal receipt digest is invalid".into(),
                    ));
                }
                crate::SealedRehearsalSubmissionExecutionPreparation::Receipt(
                    crate::RehearsalSubmissionReceipt {
                        outcome: receipt,
                        replayed: true,
                    },
                )
            }
            "work" => {
                let (grading, completion) = submission_execution_from_row(context, route, &row)?;
                crate::SealedRehearsalSubmissionExecutionPreparation::Work(Box::new(
                    crate::SealedRehearsalSubmissionExecutionWork::new(grading, completion),
                ))
            }
            _ => {
                return Err(StoreError::Unavailable(
                    "invalid sealed rehearsal preparation".into(),
                ));
            }
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn complete_sealed_rehearsal_submission_execution(
        &self,
        context: TenantContext,
        completion: crate::SealedRehearsalSubmissionCompletion,
        grading: question_model::RehearsalPrivateGradingResult,
    ) -> Result<crate::RehearsalSubmissionReceipt, StoreError> {
        let (
            capability_context,
            route,
            handle,
            root,
            _attempt,
            frozen,
            head,
            presentation_commitment,
            durable_request,
        ) = completion.into_internal_parts();
        if capability_context != context {
            return Err(StoreError::NotFound);
        }
        let mut transaction = self.begin_sealed_reader_tenant(context).await?;
        let millis: i64 = sqlx::query_scalar("SELECT public.ple_rehearsal_now_millis()")
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let accepted_at = question_model::ActivityTimestamp::from_unix_millis(millis);
        let evidence = domain::RehearsalValidatedSubmissionEvidence::try_complete_with_claim_input(
            &root,
            durable_request,
            &frozen,
            grading,
            accepted_at,
        )
        .map_err(|_| StoreError::Conflict)?;
        let payload = domain::RehearsalEvidencePayload::AcceptedSubmission(evidence);
        let sequence = head.length().checked_add(1).ok_or_else(|| {
            StoreError::Unavailable("rehearsal evidence sequence exhausted".into())
        })?;
        let entry_digest = domain::evidence_entry_digest(
            sequence,
            payload.kind(),
            head.digest(),
            domain::private_payload_digest(&payload),
            accepted_at,
        );
        let question_model::RehearsalPrivateGradingResult::Graded { feedback, .. } = match &payload
        {
            domain::RehearsalEvidencePayload::AcceptedSubmission(value) => value.result(),
            domain::RehearsalEvidencePayload::FrozenItem(_) => unreachable!("accepted payload"),
        };
        let outcome = question_model::RehearsalPublicOutcome::Submitted {
            feedback: feedback.clone(),
        };
        let receipt_digest =
            domain::rehearsal::persistence::persisted_rehearsal_receipt_digest(&outcome);
        let row = sqlx::query(
            "SELECT * FROM public.ple_complete_sealed_rehearsal_submission($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
        )
        .bind(context.tenant_id().as_uuid())
        .bind(route.actor.as_uuid())
        .bind(route.course.as_uuid())
        .bind(i32::try_from(route.assignment.number()).map_err(|_| {
            StoreError::InvalidRecord("assignment reference exceeds database range".into())
        })?)
        .bind(i64::try_from(route.expected_revision.value()).map_err(|_| {
            StoreError::InvalidRecord("teaching revision exceeds database range".into())
        })?)
        .bind(i64::from(route.rehearsal.number()))
        .bind(handle.claim().as_uuid())
        .bind(handle.operation().as_uuid())
        .bind(i32::try_from(handle.generation().value()).map_err(|_| {
            StoreError::InvalidRecord("claim generation exceeds database range".into())
        })?)
        .bind(handle.fingerprint().as_bytes().to_vec())
        .bind(presentation_commitment.as_bytes().to_vec())
        .bind(head.digest().as_bytes().to_vec())
        .bind(i64::from(head.length()))
        .bind(entry_digest.as_bytes().to_vec())
        .bind(domain::rehearsal::persistence::encode_evidence_payload(&payload))
        .bind(domain::private_payload_digest(&payload).as_bytes().to_vec())
        .bind(millis)
        .bind(domain::rehearsal::persistence::encode_persisted_rehearsal_receipt(&outcome))
        .bind(receipt_digest.as_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
        if !matches!(kind.as_str(), "completed" | "receipt") {
            return Err(StoreError::Conflict);
        }
        let projection: serde_json::Value =
            row.try_get("outcome_projection").map_err(map_sqlx_error)?;
        let persisted =
            domain::rehearsal::persistence::decode_persisted_rehearsal_receipt(&projection)
                .map_err(|_| {
                    StoreError::Unavailable("sealed rehearsal receipt is invalid".into())
                })?;
        let stored_digest: Vec<u8> = row.try_get("receipt_digest").map_err(map_sqlx_error)?;
        if stored_digest.as_slice()
            != domain::rehearsal::persistence::persisted_rehearsal_receipt_digest(&persisted)
                .as_bytes()
        {
            return Err(StoreError::Unavailable(
                "sealed rehearsal receipt digest is invalid".into(),
            ));
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::RehearsalSubmissionReceipt {
            outcome: persisted,
            replayed: kind == "receipt",
        })
    }
}
