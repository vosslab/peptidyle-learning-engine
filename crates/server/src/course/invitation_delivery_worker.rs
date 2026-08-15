//! Bounded, lease-fenced course-invitation delivery dispatch.
//!
//! This worker deliberately has no browser route. It receives only a claimed
//! outbox identifier, obtains protected reissuance inputs through the Store,
//! reconstructs the deterministic secret in process, and sends outside every
//! database transaction. A lost SMTP outcome is terminally ambiguous rather
//! than a reason to send a second copy.

use std::{sync::Arc, time::Duration};

use learning_data_access::{
    ClaimedCourseInvitationDelivery, CompleteCourseInvitationDelivery,
    CourseInvitationDeliveryWorkerStore, InvitationDeliveryReissuance,
    PreparedCourseInvitationDelivery, StoreError,
};
use question_model::ActivityTimestamp;
use tracing::Instrument;

use super::{
    CourseInvitationDelivery, CourseInvitationDeliveryAttempt, CourseInvitationIssuer,
    CourseInvitationSecret,
};

const RETRY_BASE_SECONDS: i64 = 30;
const RETRY_MAX_SECONDS: i64 = 60 * 60;
/// A provider attempt cannot outlive the dedicated container's 100-second
/// stop grace or the 120-second broker lease. Timeout has unknown SMTP phase,
/// so it is always completed as ambiguous rather than retried.
const DELIVERY_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(45);

/// One safe, bounded polling result. It intentionally contains no provider,
/// recipient, lease, or invitation identifiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct InvitationDeliveryDrainReport {
    pub(crate) claimed: u32,
    pub(crate) completed: u32,
    pub(crate) fenced: u32,
}

/// Server-only worker family for the invitation outbox. The Store remains the
/// authority for claims and completion; this type owns only reissuance and the
/// external provider call.
pub(crate) struct InvitationDeliveryWorker<S> {
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    delivery: Arc<dyn CourseInvitationDelivery>,
    maximum: u16,
    lease_duration_seconds: u32,
    attempt_timeout: Duration,
}

impl<S> InvitationDeliveryWorker<S> {
    pub(crate) fn new(
        store: Arc<S>,
        issuer: CourseInvitationIssuer,
        delivery: Arc<dyn CourseInvitationDelivery>,
        maximum: u16,
        lease_duration_seconds: u32,
    ) -> Result<Self, StoreError> {
        Self::with_attempt_timeout(
            store,
            issuer,
            delivery,
            maximum,
            lease_duration_seconds,
            DELIVERY_ATTEMPT_TIMEOUT,
        )
    }

    fn with_attempt_timeout(
        store: Arc<S>,
        issuer: CourseInvitationIssuer,
        delivery: Arc<dyn CourseInvitationDelivery>,
        maximum: u16,
        lease_duration_seconds: u32,
        attempt_timeout: Duration,
    ) -> Result<Self, StoreError> {
        if maximum == 0
            || maximum > 100
            || lease_duration_seconds == 0
            || lease_duration_seconds > 900
            || attempt_timeout.is_zero()
        {
            return Err(StoreError::InvalidRecord(
                "invitation delivery worker bounds are invalid".to_string(),
            ));
        }
        Ok(Self {
            store,
            issuer,
            delivery,
            maximum,
            lease_duration_seconds,
            attempt_timeout,
        })
    }
}

impl<S> InvitationDeliveryWorker<S>
where
    S: CourseInvitationDeliveryWorkerStore + 'static,
{
    /// Drains one claimed batch. An unconfigured provider does not claim work:
    /// retaining it as pending lets a correctly configured worker act later.
    pub(crate) async fn drain_once(&self) -> Result<InvitationDeliveryDrainReport, StoreError> {
        if !self.delivery.is_configured() {
            return Ok(InvitationDeliveryDrainReport::default());
        }
        let claimed = self
            .store
            .claim_due_course_invitation_deliveries(self.maximum, self.lease_duration_seconds)
            .await?;
        let mut report = InvitationDeliveryDrainReport {
            claimed: u32::try_from(claimed.len()).map_err(|_| StoreError::Conflict)?,
            ..InvitationDeliveryDrainReport::default()
        };
        for claim in claimed {
            self.dispatch_claim(claim, &mut report).await?;
        }
        Ok(report)
    }

    async fn dispatch_claim(
        &self,
        claim: ClaimedCourseInvitationDelivery,
        report: &mut InvitationDeliveryDrainReport,
    ) -> Result<(), StoreError> {
        let Some(prepared) = self
            .store
            .prepare_course_invitation_delivery(claim.delivery.id, claim.lease)
            .await?
        else {
            report.fenced = report.fenced.checked_add(1).ok_or(StoreError::Conflict)?;
            return Ok(());
        };
        if prepared.delivery != claim.delivery.id || prepared.lease != claim.lease {
            count_completion(
                report,
                self.complete(&claim, CompleteCourseInvitationDelivery::PermanentFailed)
                    .await?,
            )?;
            return Ok(());
        }
        let Some((email, secret)) = reissue(&self.issuer, &prepared) else {
            count_completion(
                report,
                self.complete(&claim, CompleteCourseInvitationDelivery::PermanentFailed)
                    .await?,
            )?;
            return Ok(());
        };
        // `prepare_course_invitation_delivery` is the Store's atomic
        // dispatch-start fence: it rechecks the invitation and lease before
        // returning the protected inputs. Once it succeeds, any expired
        // completion is ambiguous rather than eligible for an automatic
        // resend.
        if !self
            .store
            .revalidate_course_invitation_delivery_lease(claim.delivery.id, claim.lease)
            .await?
        {
            report.fenced = report.fenced.checked_add(1).ok_or(StoreError::Conflict)?;
            return Ok(());
        }
        // This is a durable worker correlation, not a browser request span.
        // `delivery_id` is the server-minted opaque outbox identifier; no
        // recipient, secret, lease, or provider response enters tracing.
        let attempt_span = tracing::info_span!(
            "invitation_delivery_attempt",
            event_family = "course_invitation_delivery",
            delivery_id = %claim.delivery.id.as_uuid(),
        );
        let attempt = match tokio::time::timeout(
            self.attempt_timeout,
            self.delivery
                .attempt_course_invitation(&email, &secret)
                .instrument(attempt_span),
        )
        .await
        {
            Ok(attempt) => attempt,
            Err(_) => CourseInvitationDeliveryAttempt::Ambiguous,
        };
        let completion = match attempt {
            CourseInvitationDeliveryAttempt::AcceptedByProvider => {
                // SMTP offers no stable submission receipt or documented
                // provider idempotency key, so retain neither value.
                CompleteCourseInvitationDelivery::AcceptedByProvider
            }
            CourseInvitationDeliveryAttempt::RetryableFailure => {
                CompleteCourseInvitationDelivery::RetryableFailed {
                    next_attempt_at: retry_at(&claim),
                }
            }
            CourseInvitationDeliveryAttempt::PermanentFailure => {
                CompleteCourseInvitationDelivery::PermanentFailed
            }
            CourseInvitationDeliveryAttempt::Ambiguous => {
                CompleteCourseInvitationDelivery::Ambiguous
            }
        };
        count_completion(report, self.complete(&claim, completion).await?)?;
        Ok(())
    }

    async fn complete(
        &self,
        claim: &ClaimedCourseInvitationDelivery,
        completion: CompleteCourseInvitationDelivery,
    ) -> Result<bool, StoreError> {
        self.store
            .complete_course_invitation_delivery(claim.delivery.id, claim.lease, completion)
            .await
    }
}

fn count_completion(
    report: &mut InvitationDeliveryDrainReport,
    completed: bool,
) -> Result<(), StoreError> {
    let target = if completed {
        &mut report.completed
    } else {
        &mut report.fenced
    };
    *target = target.checked_add(1).ok_or(StoreError::Conflict)?;
    Ok(())
}

#[async_trait::async_trait]
impl<S> crate::worker::runtime::BoundedWorkerDispatch for InvitationDeliveryWorker<S>
where
    S: CourseInvitationDeliveryWorkerStore + 'static,
{
    async fn drain_once(&self) -> Result<u32, StoreError> {
        let report = InvitationDeliveryWorker::drain_once(self).await?;
        Ok(report.completed + report.fenced)
    }
}

fn reissue(
    issuer: &CourseInvitationIssuer,
    prepared: &PreparedCourseInvitationDelivery,
) -> Option<(
    learning_data_access::AuthenticationEmail,
    CourseInvitationSecret,
)> {
    let email = learning_data_access::AuthenticationEmail::parse(&prepared.delivery_email).ok()?;
    let secret = match &prepared.reissuance {
        InvitationDeliveryReissuance::Single {
            tenant,
            course,
            roster_id,
            idempotency_key,
        } => issuer
            .issue(*tenant, *course, &email, roster_id, idempotency_key)
            .ok()?,
        InvitationDeliveryReissuance::Import {
            tenant,
            course,
            import,
            row_number,
            commit_idempotency_key,
        } => {
            issuer
                .issue_import(
                    *tenant,
                    *course,
                    *import,
                    *row_number,
                    commit_idempotency_key,
                )
                .ok()?
                .0
        }
    };
    (secret.hash() == prepared.expected_token_hash).then_some((email, secret))
}

fn retry_at(claim: &ClaimedCourseInvitationDelivery) -> ActivityTimestamp {
    let attempt = claim.delivery.attempt_count.saturating_sub(1).min(16);
    let exponent = 1_i64.checked_shl(attempt).unwrap_or(i64::MAX);
    let delay = RETRY_BASE_SECONDS
        .saturating_mul(exponent)
        .min(RETRY_MAX_SECONDS);
    let base = claim
        .delivery
        .last_attempt_at
        .unwrap_or(claim.delivery.next_attempt_at)
        .as_unix_millis();
    ActivityTimestamp::from_unix_millis(base.saturating_add(delay.saturating_mul(1000)))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use learning_data_access::{
        CourseInvitationDelivery as CourseInvitationDeliveryRecord, CourseInvitationDeliveryId,
        CourseInvitationDeliveryLeaseId, CourseInvitationDeliveryOutcomeCode,
        CourseInvitationDeliveryState, CourseInvitationSecretHash, CourseRosterId,
        RosterIdempotencyKey,
    };
    use question_model::{CourseId, TenantId};
    use uuid::Uuid;

    use super::*;

    struct FakeStore {
        claim: Mutex<Vec<ClaimedCourseInvitationDelivery>>,
        prepared: Mutex<Option<PreparedCourseInvitationDelivery>>,
        completions: Mutex<Vec<CompleteCourseInvitationDelivery>>,
        revalidated: Mutex<bool>,
        completion_result: Mutex<bool>,
    }

    #[async_trait]
    impl CourseInvitationDeliveryWorkerStore for FakeStore {
        async fn prepare_course_invitation_delivery(
            &self,
            _: CourseInvitationDeliveryId,
            _: CourseInvitationDeliveryLeaseId,
        ) -> Result<Option<PreparedCourseInvitationDelivery>, StoreError> {
            Ok(self.prepared.lock().expect("prepared lock").take())
        }

        async fn claim_due_course_invitation_deliveries(
            &self,
            _: u16,
            _: u32,
        ) -> Result<Vec<ClaimedCourseInvitationDelivery>, StoreError> {
            Ok(std::mem::take(&mut *self.claim.lock().expect("claim lock")))
        }

        async fn complete_course_invitation_delivery(
            &self,
            _: CourseInvitationDeliveryId,
            _: CourseInvitationDeliveryLeaseId,
            completion: CompleteCourseInvitationDelivery,
        ) -> Result<bool, StoreError> {
            self.completions
                .lock()
                .expect("completion lock")
                .push(completion);
            Ok(*self
                .completion_result
                .lock()
                .expect("completion result lock"))
        }

        async fn revalidate_course_invitation_delivery_lease(
            &self,
            _: CourseInvitationDeliveryId,
            _: CourseInvitationDeliveryLeaseId,
        ) -> Result<bool, StoreError> {
            Ok(*self.revalidated.lock().expect("revalidation lock"))
        }
    }

    struct FakeDelivery {
        configured: bool,
        attempt: CourseInvitationDeliveryAttempt,
        sends: Mutex<u32>,
    }

    #[async_trait]
    impl CourseInvitationDelivery for FakeDelivery {
        fn is_configured(&self) -> bool {
            self.configured
        }

        async fn attempt_course_invitation(
            &self,
            _: &learning_data_access::AuthenticationEmail,
            _: &CourseInvitationSecret,
        ) -> CourseInvitationDeliveryAttempt {
            *self.sends.lock().expect("send lock") += 1;
            self.attempt
        }
    }

    struct HangingDelivery;

    #[async_trait]
    impl CourseInvitationDelivery for HangingDelivery {
        fn is_configured(&self) -> bool {
            true
        }

        async fn attempt_course_invitation(
            &self,
            _: &learning_data_access::AuthenticationEmail,
            _: &CourseInvitationSecret,
        ) -> CourseInvitationDeliveryAttempt {
            std::future::pending().await
        }
    }

    fn claimed() -> ClaimedCourseInvitationDelivery {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let invitation = learning_data_access::CourseInvitationId::from_uuid(Uuid::from_u128(3));
        let id = CourseInvitationDeliveryId::from_uuid(Uuid::from_u128(4));
        let lease = CourseInvitationDeliveryLeaseId::from_uuid(Uuid::from_u128(5));
        ClaimedCourseInvitationDelivery {
            lease,
            delivery: CourseInvitationDeliveryRecord {
                tenant,
                course,
                invitation,
                id,
                state: CourseInvitationDeliveryState::Pending,
                attempt_count: 1,
                next_attempt_at: ActivityTimestamp::from_unix_millis(10_000),
                last_attempt_at: Some(ActivityTimestamp::from_unix_millis(10_000)),
                lease: Some(lease),
                lease_expires_at: Some(ActivityTimestamp::from_unix_millis(20_000)),
                dispatch_started_at: None,
                outcome_code: Some(CourseInvitationDeliveryOutcomeCode::TemporaryFailure),
                created_at: ActivityTimestamp::from_unix_millis(0),
                updated_at: ActivityTimestamp::from_unix_millis(10_000),
                accepted_at: None,
                terminal_at: None,
            },
        }
    }

    fn prepared(claim: &ClaimedCourseInvitationDelivery) -> PreparedCourseInvitationDelivery {
        let issuer = CourseInvitationIssuer::from_server_secret([7; 32]);
        let email = learning_data_access::AuthenticationEmail::parse("learner@example.edu")
            .expect("fixture email");
        let roster = CourseRosterId::parse("900000001").expect("fixture roster ID");
        let key = RosterIdempotencyKey::parse("fixture-invitation").expect("fixture key");
        let secret = issuer
            .issue(
                claim.delivery.tenant,
                claim.delivery.course,
                &email,
                &roster,
                &key,
            )
            .expect("fixture secret");
        PreparedCourseInvitationDelivery {
            delivery: claim.delivery.id,
            lease: claim.lease,
            delivery_email: email.delivery().to_string(),
            expected_token_hash: secret.hash(),
            reissuance: InvitationDeliveryReissuance::Single {
                tenant: claim.delivery.tenant,
                course: claim.delivery.course,
                roster_id: roster,
                idempotency_key: key,
            },
        }
    }

    #[tokio::test]
    async fn unconfigured_worker_leaves_due_delivery_unclaimed() {
        let claim = claimed();
        let store = Arc::new(FakeStore {
            claim: Mutex::new(vec![claim]),
            prepared: Mutex::new(None),
            completions: Mutex::new(Vec::new()),
            revalidated: Mutex::new(true),
            completion_result: Mutex::new(true),
        });
        let delivery = Arc::new(FakeDelivery {
            configured: false,
            attempt: CourseInvitationDeliveryAttempt::AcceptedByProvider,
            sends: Mutex::new(0),
        });
        let worker = InvitationDeliveryWorker::new(
            Arc::clone(&store),
            CourseInvitationIssuer::from_server_secret([7; 32]),
            Arc::clone(&delivery) as Arc<dyn CourseInvitationDelivery>,
            1,
            60,
        )
        .expect("worker bounds");
        assert_eq!(
            worker.drain_once().await.expect("drain"),
            InvitationDeliveryDrainReport::default()
        );
        assert_eq!(store.claim.lock().expect("claim lock").len(), 1);
        assert_eq!(*delivery.sends.lock().expect("send lock"), 0);
    }

    #[tokio::test]
    async fn prepared_claim_maps_only_closed_outcomes_and_never_logs_inputs() {
        for (attempt, expected) in [
            (
                CourseInvitationDeliveryAttempt::AcceptedByProvider,
                CompleteCourseInvitationDelivery::AcceptedByProvider,
            ),
            (
                CourseInvitationDeliveryAttempt::PermanentFailure,
                CompleteCourseInvitationDelivery::PermanentFailed,
            ),
            (
                CourseInvitationDeliveryAttempt::Ambiguous,
                CompleteCourseInvitationDelivery::Ambiguous,
            ),
        ] {
            let claim = claimed();
            let store = Arc::new(FakeStore {
                claim: Mutex::new(vec![claim.clone()]),
                prepared: Mutex::new(Some(prepared(&claim))),
                completions: Mutex::new(Vec::new()),
                revalidated: Mutex::new(true),
                completion_result: Mutex::new(true),
            });
            let delivery = Arc::new(FakeDelivery {
                configured: true,
                attempt,
                sends: Mutex::new(0),
            });
            let worker = InvitationDeliveryWorker::new(
                Arc::clone(&store),
                CourseInvitationIssuer::from_server_secret([7; 32]),
                Arc::clone(&delivery) as Arc<dyn CourseInvitationDelivery>,
                1,
                60,
            )
            .expect("worker bounds");
            worker.drain_once().await.expect("drain");
            assert_eq!(*delivery.sends.lock().expect("send lock"), 1);
            assert_eq!(
                store
                    .completions
                    .lock()
                    .expect("completion lock")
                    .as_slice(),
                &[expected]
            );
        }
    }

    #[tokio::test]
    async fn stale_or_hash_mismatched_claim_never_reaches_provider() {
        let claim = claimed();
        let stale_store = Arc::new(FakeStore {
            claim: Mutex::new(vec![claim.clone()]),
            prepared: Mutex::new(None),
            completions: Mutex::new(Vec::new()),
            revalidated: Mutex::new(true),
            completion_result: Mutex::new(true),
        });
        let delivery = Arc::new(FakeDelivery {
            configured: true,
            attempt: CourseInvitationDeliveryAttempt::AcceptedByProvider,
            sends: Mutex::new(0),
        });
        let worker = InvitationDeliveryWorker::new(
            Arc::clone(&stale_store),
            CourseInvitationIssuer::from_server_secret([7; 32]),
            Arc::clone(&delivery) as Arc<dyn CourseInvitationDelivery>,
            1,
            60,
        )
        .expect("worker bounds");
        worker.drain_once().await.expect("stale drain");
        assert_eq!(*delivery.sends.lock().expect("send lock"), 0);

        let mut mismatch = prepared(&claim);
        mismatch.expected_token_hash = CourseInvitationSecretHash::compute(b"different");
        let mismatch_store = Arc::new(FakeStore {
            claim: Mutex::new(vec![claim]),
            prepared: Mutex::new(Some(mismatch)),
            completions: Mutex::new(Vec::new()),
            revalidated: Mutex::new(true),
            completion_result: Mutex::new(true),
        });
        let mismatch_worker = InvitationDeliveryWorker::new(
            Arc::clone(&mismatch_store),
            CourseInvitationIssuer::from_server_secret([7; 32]),
            Arc::clone(&delivery) as Arc<dyn CourseInvitationDelivery>,
            1,
            60,
        )
        .expect("worker bounds");
        mismatch_worker.drain_once().await.expect("mismatch drain");
        assert_eq!(*delivery.sends.lock().expect("send lock"), 0);
        assert_eq!(
            mismatch_store
                .completions
                .lock()
                .expect("completion lock")
                .as_slice(),
            &[CompleteCourseInvitationDelivery::PermanentFailed]
        );
    }

    #[tokio::test]
    async fn revoked_or_expired_after_prepare_never_reaches_provider() {
        let claim = claimed();
        let store = Arc::new(FakeStore {
            claim: Mutex::new(vec![claim.clone()]),
            prepared: Mutex::new(Some(prepared(&claim))),
            completions: Mutex::new(Vec::new()),
            revalidated: Mutex::new(false),
            completion_result: Mutex::new(true),
        });
        let delivery = Arc::new(FakeDelivery {
            configured: true,
            attempt: CourseInvitationDeliveryAttempt::AcceptedByProvider,
            sends: Mutex::new(0),
        });
        let worker = InvitationDeliveryWorker::new(
            Arc::clone(&store),
            CourseInvitationIssuer::from_server_secret([7; 32]),
            Arc::clone(&delivery) as Arc<dyn CourseInvitationDelivery>,
            1,
            60,
        )
        .expect("worker bounds");

        assert_eq!(
            worker.drain_once().await.expect("drain"),
            InvitationDeliveryDrainReport {
                claimed: 1,
                completed: 0,
                fenced: 1,
            }
        );
        assert_eq!(*delivery.sends.lock().expect("send lock"), 0);
        assert!(
            store
                .completions
                .lock()
                .expect("completion lock")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn stale_completion_is_counted_as_fenced() {
        let claim = claimed();
        let store = Arc::new(FakeStore {
            claim: Mutex::new(vec![claim.clone()]),
            prepared: Mutex::new(Some(prepared(&claim))),
            completions: Mutex::new(Vec::new()),
            revalidated: Mutex::new(true),
            completion_result: Mutex::new(false),
        });
        let delivery = Arc::new(FakeDelivery {
            configured: true,
            attempt: CourseInvitationDeliveryAttempt::PermanentFailure,
            sends: Mutex::new(0),
        });
        let worker = InvitationDeliveryWorker::new(
            Arc::clone(&store),
            CourseInvitationIssuer::from_server_secret([7; 32]),
            Arc::clone(&delivery) as Arc<dyn CourseInvitationDelivery>,
            1,
            60,
        )
        .expect("worker bounds");

        assert_eq!(
            worker.drain_once().await.expect("drain"),
            InvitationDeliveryDrainReport {
                claimed: 1,
                completed: 0,
                fenced: 1,
            }
        );
        assert_eq!(*delivery.sends.lock().expect("send lock"), 1);
        assert_eq!(
            store
                .completions
                .lock()
                .expect("completion lock")
                .as_slice(),
            &[CompleteCourseInvitationDelivery::PermanentFailed]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn provider_timeout_is_ambiguous_and_finishes_within_shutdown_budget() {
        let claim = claimed();
        let store = Arc::new(FakeStore {
            claim: Mutex::new(vec![claim.clone()]),
            prepared: Mutex::new(Some(prepared(&claim))),
            completions: Mutex::new(Vec::new()),
            revalidated: Mutex::new(true),
            completion_result: Mutex::new(true),
        });
        let worker = Arc::new(
            InvitationDeliveryWorker::with_attempt_timeout(
                Arc::clone(&store),
                CourseInvitationIssuer::from_server_secret([7; 32]),
                Arc::new(HangingDelivery),
                1,
                60,
                Duration::from_millis(1),
            )
            .expect("worker bounds"),
        );
        let pass = tokio::spawn({
            let worker = Arc::clone(&worker);
            async move { worker.drain_once().await.expect("drain") }
        });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(1)).await;
        assert_eq!(
            pass.await.expect("pass task"),
            InvitationDeliveryDrainReport {
                claimed: 1,
                completed: 1,
                fenced: 0,
            }
        );
        assert_eq!(
            store
                .completions
                .lock()
                .expect("completion lock")
                .as_slice(),
            &[CompleteCourseInvitationDelivery::Ambiguous]
        );
    }
}
