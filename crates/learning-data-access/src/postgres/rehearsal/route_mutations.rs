//! Server-only adapters from public route identity to sealed rehearsal work.

use async_trait::async_trait;
use sqlx::Row;

use super::super::*;

pub(super) async fn resolve_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: question_model::TenantId,
    route: crate::RehearsalRouteIdentity,
) -> Result<crate::RehearsalLocator, StoreError> {
    let locator = crate::RehearsalLocator {
        actor: route.actor,
        course: route.course,
        assignment: route.assignment,
        revision: route.expected_revision,
        rehearsal: route.rehearsal,
    };
    match super::auth::prepare_operation(tx, tenant, locator).await {
        Ok(_) => Ok(locator),
        Err(StoreError::NotFound) => {
            // A stale If-Match is distinguishable only for an already-authorized
            // direct Instructor. Foreign and fenced routes retain NotFound.
            let member: Option<uuid::Uuid> = sqlx::query_scalar(
                "SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND role='instructor' AND status='active'",
            )
            .bind(tenant.as_uuid()).bind(route.course.as_uuid()).bind(route.actor.as_uuid())
            .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?;
            let current: Option<i64> = if member.is_some() {
                sqlx::query("SELECT revision FROM assignment WHERE tenant_id=$1 AND course_id=$2 AND public_id=$3")
                    .bind(tenant.as_uuid()).bind(route.course.as_uuid()).bind(i64::from(route.assignment.number()))
                    .fetch_optional(&mut **tx).await.map_err(map_sqlx_error)?
                    .map(|row| row.try_get("revision")).transpose().map_err(map_sqlx_error)?
            } else {
                None
            };
            if current.is_some_and(|revision| {
                revision != i64::try_from(route.expected_revision.value()).unwrap_or(i64::MIN)
            }) {
                Err(StoreError::Conflict)
            } else {
                Err(StoreError::NotFound)
            }
        }
        Err(error) => Err(error),
    }
}

#[async_trait]
impl crate::RehearsalRouteMutationStore for PostgresStore {
    async fn claim_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: crate::ClaimRehearsalDeliveryRouteCommand,
    ) -> Result<crate::RehearsalDeliveryClaimResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, command.route).await?;
        let result = super::operations::claim_delivery_in_tx(
            &mut tx,
            tenant,
            crate::RehearsalDeliveryRequest {
                locator,
                idempotency_key: command.idempotency_key,
                request_fingerprint: command.request_fingerprint,
            },
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn reconcile_rehearsal_delivery_expiry_from_route(
        &self,
        context: TenantContext,
        command: crate::ReconcileRehearsalDeliveryExpiryRouteCommand,
    ) -> Result<crate::RehearsalDeliveryTimingResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let route = command.route;
        let row =
            sqlx::query("SELECT * FROM ple_reconcile_rehearsal_delivery_expiry($1,$2,$3,$4,$5,$6)")
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
                .map_err(map_sqlx_error)?
                .ok_or(StoreError::NotFound)?;
        let verdict = match row
            .try_get::<String, _>("verdict")
            .map_err(map_sqlx_error)?
            .as_str()
        {
            "untimed" => domain::RehearsalTimingVerdictV1::Untimed,
            "open" => domain::RehearsalTimingVerdictV1::Open,
            "gracePeriod" => domain::RehearsalTimingVerdictV1::GracePeriod,
            "expired" => domain::RehearsalTimingVerdictV1::Expired,
            _ => {
                return Err(StoreError::Unavailable(
                    "invalid persisted rehearsal timing verdict".into(),
                ));
            }
        };
        let time = |name| -> Result<Option<question_model::ActivityTimestamp>, StoreError> {
            row.try_get::<Option<i64>, _>(name)
                .map_err(map_sqlx_error)
                .map(|value| value.map(question_model::ActivityTimestamp::from_unix_millis))
        };
        let result = crate::RehearsalDeliveryTimingResult {
            verdict,
            deadline: time("deadline_at_millis")?,
            expires_at: time("expires_at_millis")?,
            retry_disposition: match row
                .try_get::<String, _>("retry_disposition")
                .map_err(map_sqlx_error)?
                .as_str()
            {
                "notApplicable" => crate::RehearsalDeliveryRetryDisposition::NotApplicable,
                "available" => crate::RehearsalDeliveryRetryDisposition::Available,
                "runTimeExhausted" => crate::RehearsalDeliveryRetryDisposition::RunTimeExhausted,
                _ => {
                    return Err(StoreError::Unavailable(
                        "invalid persisted rehearsal retry disposition".into(),
                    ));
                }
            },
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn retry_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: crate::RetryRehearsalDeliveryRouteCommand,
    ) -> Result<crate::RetryRehearsalDeliveryResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, command.route).await?;
        let row = sqlx::query(
            "SELECT * FROM ple_prepare_rehearsal_delivery_retry($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(tenant.as_uuid())
        .bind(command.route.actor.as_uuid())
        .bind(command.route.course.as_uuid())
        .bind(
            i32::try_from(command.route.assignment.number()).map_err(|_| {
                StoreError::InvalidRecord("assignment reference exceeds database range".into())
            })?,
        )
        .bind(
            i64::try_from(command.route.expected_revision.value()).map_err(|_| {
                StoreError::InvalidRecord("teaching revision exceeds database range".into())
            })?,
        )
        .bind(i64::from(command.route.rehearsal.number()))
        .bind(command.idempotency_key.as_str())
        .bind(command.request_fingerprint.as_bytes().to_vec())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let kind: String = row.try_get("result_kind").map_err(map_sqlx_error)?;
        let result = match kind.as_str() {
            "prepared" => {
                let operation = crate::RehearsalOperationId::from_uuid(
                    row.try_get("operation_id").map_err(map_sqlx_error)?,
                );
                let descriptor = crate::RehearsalDeliveryExecutionDescriptorV1::decode_persisted(
                    &row.try_get("execution_descriptor")
                        .map_err(map_sqlx_error)?,
                )?;
                crate::RetryRehearsalDeliveryResult::Prepared {
                    prepared: crate::PreparedRehearsalDelivery::mint(
                        locator, operation, descriptor,
                    ),
                }
            }
            "pending" => {
                let operation = crate::RehearsalOperationId::from_uuid(
                    row.try_get("operation_id").map_err(map_sqlx_error)?,
                );
                crate::RetryRehearsalDeliveryResult::Pending {
                    dispatched: crate::DispatchedRehearsalDelivery::mint(locator, operation),
                }
            }
            "replay" => {
                crate::RetryRehearsalDeliveryResult::Replay(super::operations::active_screen(
                    row.try_get("screen_projection").map_err(map_sqlx_error)?,
                    row.try_get("screen_digest").map_err(map_sqlx_error)?,
                )?)
            }
            "runTimeExhausted" => crate::RetryRehearsalDeliveryResult::RunTimeExhausted {
                deadline: question_model::ActivityTimestamp::from_unix_millis(
                    row.try_get("deadline_at_millis").map_err(map_sqlx_error)?,
                ),
            },
            "conflict" => crate::RetryRehearsalDeliveryResult::Conflict,
            _ => {
                return Err(StoreError::Unavailable(
                    "invalid rehearsal retry result".into(),
                ));
            }
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn mark_rehearsal_delivery_dispatched_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        prepared: crate::PreparedRehearsalDelivery,
    ) -> Result<crate::RehearsalDeliveryDispatchResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, route).await?;
        if prepared.locator() != locator {
            return Err(StoreError::NotFound);
        }
        let result = super::operations::mark_dispatched_in_tx(&mut tx, tenant, prepared).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn complete_rehearsal_delivery_from_route(
        &self,
        context: TenantContext,
        command: crate::CompleteRehearsalDeliveryRouteCommand,
    ) -> Result<question_model::RehearsalActiveScreenV1, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, command.route).await?;
        if command.dispatched.locator() != locator {
            return Err(StoreError::NotFound);
        }
        let result = super::operations::complete_delivery_in_tx(
            &mut tx,
            tenant,
            crate::RehearsalDeliveryCompletionCommand {
                dispatched: command.dispatched,
                screen: command.screen,
            },
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn abandon_rehearsal_delivery_before_dispatch_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        prepared: crate::PreparedRehearsalDelivery,
        reason: crate::RehearsalDeliveryPreDispatchAbandonReason,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, route).await?;
        if prepared.locator() != locator {
            return Err(StoreError::NotFound);
        }
        super::operations::abandon_delivery_in_tx(&mut tx, tenant, prepared, reason).await?;
        tx.commit().await.map_err(map_sqlx_error)
    }
    async fn claim_rehearsal_submission_from_route(
        &self,
        context: TenantContext,
        command: crate::ClaimRehearsalSubmissionRouteCommand,
    ) -> Result<crate::RehearsalSubmissionClaimResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, command.route).await?;
        // The route-scoped preparation capability validates the entire public
        // route, reconciles expiry under the same aggregate lock, and returns
        // the immutable issued presentation evidence in this transaction.
        // It is not a selectable attempt API: the following claim broker
        // independently derives the only issued tuple before it appends any
        // immutable claim state (ASVS 1.2.4, 2.2.1, 2.3.1, 15.4.2).
        let prepared = sqlx::query(
            "SELECT decision,attempt_id,outcome_projection,receipt_digest,screen_projection,screen_digest FROM ple_prepare_rehearsal_route_submission($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(tenant.as_uuid())
        .bind(command.route.actor.as_uuid())
        .bind(command.route.course.as_uuid())
        .bind(
            i32::try_from(command.route.assignment.number()).map_err(|_| {
                StoreError::InvalidRecord("assignment reference exceeds database range".into())
            })?,
        )
        .bind(
            i64::try_from(command.route.expected_revision.value()).map_err(|_| {
                StoreError::InvalidRecord("teaching revision exceeds database range".into())
            })?,
        )
        .bind(i64::from(command.route.rehearsal.number()))
        .bind(command.presentation_digest.as_str())
        .bind(command.idempotency_key.as_str())
        .bind(serde_json::to_value(&command.response).map_err(|error| {
            StoreError::InvalidRecord(format!("invalid rehearsal submission response: {error}"))
        })?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Conflict)?;
        let decision: String = prepared.try_get("decision").map_err(map_sqlx_error)?;
        match decision.as_str() {
            "conflict" => return Err(StoreError::Conflict),
            "invalid" => {
                return Err(StoreError::InvalidRecord(
                    "invalid persisted rehearsal submission claim".into(),
                ));
            }
            "pending" | "replay" | "new" | "reclaim" => {}
            _ => {
                return Err(StoreError::InvalidRecord(
                    "invalid rehearsal submission preparation decision".into(),
                ));
            }
        }
        let (screen, presentation_digest) = super::operations::active_screen_with_commitment(
            prepared
                .try_get("screen_projection")
                .map_err(map_sqlx_error)?,
            prepared.try_get("screen_digest").map_err(map_sqlx_error)?,
        )?;
        if presentation_digest.public_token() != command.presentation_digest {
            return Err(StoreError::Conflict);
        }
        // The source-owned SQL check is independent of the Rust screen
        // decoder.  It binds the full issued digest, not the browser token,
        // before the claim broker may append any immutable state.
        let admitted: bool = sqlx::query_scalar(
            "SELECT public.ple_validate_rehearsal_rendered_submission_v1($1,$2,$3)",
        )
        .bind(
            serde_json::to_value(&screen)
                .map_err(|_| StoreError::InvalidRecord("invalid issued rehearsal screen".into()))?,
        )
        .bind(presentation_digest.as_bytes().to_vec())
        .bind(serde_json::to_value(&command.response).map_err(|_| {
            StoreError::InvalidRecord("invalid rehearsal submission response".into())
        })?)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !admitted {
            return Err(StoreError::Conflict);
        }
        match decision.as_str() {
            "pending" => {
                tx.commit().await.map_err(map_sqlx_error)?;
                return Ok(crate::RehearsalSubmissionClaimResult::Pending);
            }
            "replay" => {
                let projection: serde_json::Value = prepared
                    .try_get("outcome_projection")
                    .map_err(map_sqlx_error)?;
                let stored_digest: Vec<u8> =
                    prepared.try_get("receipt_digest").map_err(map_sqlx_error)?;
                let outcome =
                    domain::rehearsal::persistence::decode_persisted_rehearsal_receipt(&projection)
                        .map_err(|_| {
                            StoreError::InvalidRecord("invalid persisted rehearsal receipt".into())
                        })?;
                let expected =
                    domain::rehearsal::persistence::persisted_rehearsal_receipt_digest(&outcome);
                if stored_digest.as_slice() != expected.as_bytes() {
                    return Err(StoreError::InvalidRecord(
                        "rehearsal replay receipt digest mismatch".into(),
                    ));
                }
                tx.commit().await.map_err(map_sqlx_error)?;
                return Ok(crate::RehearsalSubmissionClaimResult::Replay(
                    crate::RehearsalSubmissionReceipt {
                        outcome,
                        replayed: true,
                    },
                ));
            }
            "new" | "reclaim" => {}
            _ => unreachable!("validated rehearsal submission preparation decision"),
        }
        let attempt: uuid::Uuid = prepared.try_get("attempt_id").map_err(map_sqlx_error)?;
        let result = super::claims::claim_from_route_in_tx(
            &mut tx,
            tenant,
            crate::ClaimRehearsalSubmissionCommand {
                locator,
                attempt: question_model::RehearsalAttemptId::from_uuid(attempt),
                response: command.response,
                idempotency_key: command.idempotency_key,
            },
            screen,
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn mark_rehearsal_submission_dispatched_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        handle: domain::PreparedClaimHandle,
    ) -> Result<domain::DispatchedClaimHandle, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, route).await?;
        let result = super::claims::mark_dispatched_in_tx(
            &mut tx,
            tenant,
            crate::MarkRehearsalSubmissionDispatchedCommand { locator, handle },
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn dispatch_rehearsal_submission_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        idempotency_key: crate::RehearsalIdempotencyKey,
    ) -> Result<domain::DispatchedClaimHandle, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, route).await?;
        let binding_is_valid: bool = sqlx::query_scalar(
            "SELECT ple_verify_rehearsal_claim_delivery_binding_from_route($1,$2,$3,$4,$5,$6,$7)",
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
        .bind(idempotency_key.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !binding_is_valid {
            return Err(StoreError::Conflict);
        }
        // The source-owned broker locks the claim event stream and makes the
        // Prepared -> dispatched transition exactly once (ASVS 2.3.1/2.3.3).
        let kind: String =
            sqlx::query_scalar("SELECT ple_rehearsal_route_dispatch_claim($1,$2,$3,$4,$5,$6,$7)")
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
                .bind(idempotency_key.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        if kind != "dispatched" {
            return Err(match kind.as_str() {
                "notFound" => StoreError::NotFound,
                "conflict" | "terminal" => StoreError::Conflict,
                _ => StoreError::InvalidRecord("invalid rehearsal dispatch broker result".into()),
            });
        }
        let source = super::auth::prepare_operation(&mut tx, tenant, locator)
            .await?
            .source();
        let aggregate =
            super::hydration::load_authorized(&mut tx, tenant, locator, &source).await?;
        let claim = aggregate
            .claim_by_key(idempotency_key.as_str())
            .ok_or(StoreError::NotFound)?;
        if claim.snapshot.state() != domain::RehearsalSubmissionClaimState::GradingDispatched {
            return Err(StoreError::Conflict);
        }
        let handle = domain::rehearsal::restore_sealed_dispatched_claim_handle(
            claim.snapshot.rehearsal(),
            claim.snapshot.claim(),
            claim.snapshot.fingerprint(),
            claim.snapshot.operation(),
            claim.snapshot.generation(),
        );
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(handle)
    }
    async fn abandon_rehearsal_submission_before_dispatch_from_route(
        &self,
        context: TenantContext,
        route: crate::RehearsalRouteIdentity,
        handle: domain::PreparedClaimHandle,
        reason: domain::RehearsalPreDispatchAbandonReason,
    ) -> Result<(), StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, route).await?;
        super::claims::abandon_in_tx(
            &mut tx,
            tenant,
            crate::AbandonRehearsalSubmissionBeforeDispatchCommand {
                locator,
                handle,
                reason,
            },
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)
    }
    async fn discard_rehearsal_from_route(
        &self,
        context: TenantContext,
        command: crate::DiscardRehearsalRouteCommand,
    ) -> Result<crate::RehearsalIdempotentProjectionResult, StoreError> {
        let tenant = context.tenant_id();
        let mut tx = self.begin_tenant(context).await?;
        let locator = resolve_in_tx(&mut tx, tenant, command.route).await?;
        let result = super::operations::discard_in_tx(
            &mut tx,
            tenant,
            crate::RehearsalDiscardOperationCommand {
                locator,
                idempotency_key: command.idempotency_key,
                request_fingerprint: command.request_fingerprint,
                response: command.response,
                response_digest: command.response_digest,
            },
        )
        .await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}
