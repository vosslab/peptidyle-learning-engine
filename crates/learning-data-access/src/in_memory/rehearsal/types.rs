//! Stored state types for the in-memory rehearsal aggregate.

use crate::StoreError;
use domain::{RehearsalClaimTransitionEvent, RehearsalEvidenceChainEntry, private_payload_digest};
use question_model::{
    AssignmentId, AssignmentReference, CourseId, CourseMembershipId, RehearsalAttemptId,
    RehearsalLifecycle, RehearsalPublicOutcome, RehearsalReference, RehearsalRunId,
    RehearsalRunReceipt, RehearsalSubmissionClaimId, TeachingOperationRevision, UserId,
};

#[derive(Debug, Clone)]
pub(in crate::in_memory) struct StoredRehearsalRun {
    pub(in crate::in_memory) id: RehearsalRunId,
    pub(in crate::in_memory) reference: RehearsalReference,
    pub(in crate::in_memory) course: CourseId,
    pub(in crate::in_memory) assignment_id: AssignmentId,
    pub(in crate::in_memory) assignment: AssignmentReference,
    pub(in crate::in_memory) owner: CourseMembershipId,
    pub(in crate::in_memory) actor: UserId,
    pub(in crate::in_memory) revision: TeachingOperationRevision,
    pub(in crate::in_memory) subject: question_model::PreviewSubject,
    pub(in crate::in_memory) fingerprint: domain::RehearsalSubjectFingerprint,
    pub(in crate::in_memory) lifecycle: RehearsalLifecycle,
    pub(in crate::in_memory) started_at: question_model::ActivityTimestamp,
    pub(in crate::in_memory) updated_at: question_model::ActivityTimestamp,
    /// Aggregate commitment advanced only by private evidence append.
    pub(in crate::in_memory) evidence_head: domain::RehearsalEvidenceHead,
}

#[derive(Debug, Clone)]
pub(in crate::in_memory) struct StoredRehearsalSubmissionReceipt {
    pub(in crate::in_memory) outcome: RehearsalPublicOutcome,
}

#[derive(Clone)]
pub(in crate::in_memory) struct StoredRehearsalDeliveryOperation {
    pub(in crate::in_memory) fingerprint: crate::RehearsalOperationDigest,
    pub(in crate::in_memory) generations: Vec<StoredRehearsalDeliveryGeneration>,
}

#[derive(Clone)]
pub(in crate::in_memory) struct StoredRehearsalDeliveryRetry {
    pub(in crate::in_memory) fingerprint: crate::RehearsalOperationDigest,
    pub(in crate::in_memory) root_key: crate::RehearsalIdempotencyKey,
    pub(in crate::in_memory) operation: crate::RehearsalOperationId,
    pub(in crate::in_memory) terminal_predecessor: bool,
}

impl std::fmt::Debug for StoredRehearsalDeliveryRetry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredRehearsalDeliveryRetry")
            .field("fingerprint", &"[DIGEST]")
            .field("operation", &self.operation)
            .finish()
    }
}

/// The durable answer-free half of immutable rehearsal material.  Its
/// checksum is retained independently so hydration never trusts an in-memory
/// object merely because it has the expected Rust type.
#[derive(Clone)]
pub(in crate::in_memory) struct StoredRehearsalFrozenSourceSnapshot {
    pub(in crate::in_memory) ordinal: usize,
    pub(in crate::in_memory) snapshot: crate::IssuedQuestionSnapshotV1,
    pub(in crate::in_memory) checksum: String,
}

impl std::fmt::Debug for StoredRehearsalFrozenSourceSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredRehearsalFrozenSourceSnapshot")
            .field("checksum", &self.checksum)
            .finish()
    }
}

/// The private sibling is deliberately separate from the answer-free map.
/// It has no Debug representation and is reachable only while constructing a
/// server-only preloaded candidate.
#[derive(Clone)]
pub(in crate::in_memory) struct StoredRehearsalFrozenPrivateExecution {
    pub(in crate::in_memory) execution: crate::PrefetchedPrivateExecutionV1,
    pub(in crate::in_memory) checksum: String,
}

impl std::fmt::Debug for StoredRehearsalFrozenPrivateExecution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredRehearsalFrozenPrivateExecution")
            .field("execution", &"[REDACTED]")
            .field("checksum", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(in crate::in_memory) struct StoredRehearsalStartOperation {
    pub(in crate::in_memory) fingerprint: crate::RehearsalOperationDigest,
    pub(in crate::in_memory) receipt: RehearsalRunReceipt,
}

#[derive(Clone)]
pub(in crate::in_memory) struct StoredRehearsalDeliveryGeneration {
    pub(in crate::in_memory) operation: crate::RehearsalOperationId,
    /// Store-derived from the immutable frozen item.  No caller supplies an
    /// attempt, candidate, or planner result.
    pub(in crate::in_memory) descriptor: crate::RehearsalDeliveryExecutionDescriptorV1,
    /// Append-only operation chronology.  Phase is always derived from this
    /// sequence; no mutable status flag can silently overwrite history.
    pub(in crate::in_memory) events: Vec<StoredRehearsalDeliveryEvent>,
    /// Independently committed journal projection.  It detects valid-prefix
    /// truncation of the append-only vector before any mutation can proceed.
    pub(in crate::in_memory) journal_head: objects::Sha256Digest,
    pub(in crate::in_memory) journal_count: u32,
    pub(in crate::in_memory) journal_phase: StoredRehearsalDeliveryPhase,
    pub(in crate::in_memory) screen: Option<question_model::RehearsalActiveScreenV1>,
    /// The browser must echo this exact immutable screen commitment before a
    /// submission route can mint grading work (ASVS 2.3.1, 15.4.2).
    pub(in crate::in_memory) screen_digest: Option<question_model::RehearsalPresentationDigestV1>,
    /// The exact sealed, post-dispatch issuer result.  It is separate from
    /// source material so a crash before screen completion resumes the same
    /// variant rather than regenerating against mutable implementations.
    pub(in crate::in_memory) issued_execution: Option<crate::RehearsalIssuedExecutionArtifactV1>,
    /// Immutable evidence finalized with an issued browser screen.
    pub(in crate::in_memory) frozen_binding: Option<question_model::RehearsalFrozenItemEvidence>,
    /// Server-owned, immutable availability evidence committed at dispatch.
    /// This deliberately does not reuse a learner attempt timer: rehearsal is
    /// an instructor preview over frozen material, not learner work.
    pub(in crate::in_memory) timing_witness: Option<domain::RehearsalTimingWitnessV1>,
    pub(in crate::in_memory) run_time_exhausted_deadline: Option<question_model::ActivityTimestamp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::in_memory) struct StoredRehearsalDeliveryEvent {
    pub(in crate::in_memory) sequence: u32,
    pub(in crate::in_memory) phase: StoredRehearsalDeliveryPhase,
    pub(in crate::in_memory) recorded_at: question_model::ActivityTimestamp,
    digest: objects::Sha256Digest,
}

impl StoredRehearsalDeliveryGeneration {
    pub(in crate::in_memory) fn new(
        operation: crate::RehearsalOperationId,
        descriptor: crate::RehearsalDeliveryExecutionDescriptorV1,
        now: question_model::ActivityTimestamp,
        frozen_binding: Option<question_model::RehearsalFrozenItemEvidence>,
    ) -> Self {
        let first = StoredRehearsalDeliveryEvent {
            sequence: 1,
            phase: StoredRehearsalDeliveryPhase::Prepared,
            recorded_at: now,
            digest: delivery_event_digest(None, 1, StoredRehearsalDeliveryPhase::Prepared, now),
        };
        Self {
            operation,
            descriptor,
            events: vec![first],
            journal_head: first.digest,
            journal_count: 1,
            journal_phase: first.phase,
            screen: None,
            screen_digest: None,
            issued_execution: None,
            frozen_binding,
            timing_witness: None,
            run_time_exhausted_deadline: None,
        }
    }

    /// Verifies the complete append-only chronology before returning its
    /// current state.  Cached projections are deliberately not authoritative.
    pub(in crate::in_memory) fn phase(&self) -> Result<StoredRehearsalDeliveryPhase, StoreError> {
        let Some(first) = self.events.first() else {
            return Err(StoreError::InvalidRecord(
                "delivery generation has no event history".into(),
            ));
        };
        if first.sequence != 1 || first.phase != StoredRehearsalDeliveryPhase::Prepared {
            return Err(StoreError::InvalidRecord(
                "delivery history lacks prepared genesis".into(),
            ));
        }
        let mut previous = first.phase;
        let mut previous_at = first.recorded_at;
        if first.digest
            != delivery_event_digest(None, first.sequence, first.phase, first.recorded_at)
        {
            return Err(StoreError::InvalidRecord(
                "delivery event digest is invalid".into(),
            ));
        }
        let mut previous_digest = Some(first.digest);
        for (index, event) in self.events.iter().enumerate().skip(1) {
            if event.sequence
                != u32::try_from(index + 1)
                    .map_err(|_| StoreError::InvalidRecord("delivery history is too long".into()))?
                || !previous.may_transition_to(event.phase)
                || event.recorded_at < previous_at
            {
                return Err(StoreError::InvalidRecord(
                    "delivery event history is illegal".into(),
                ));
            }
            if event.digest
                != delivery_event_digest(
                    previous_digest,
                    event.sequence,
                    event.phase,
                    event.recorded_at,
                )
            {
                return Err(StoreError::InvalidRecord(
                    "delivery event digest is invalid".into(),
                ));
            }
            previous = event.phase;
            previous_at = event.recorded_at;
            previous_digest = Some(event.digest);
        }
        if self.journal_count
            != u32::try_from(self.events.len())
                .map_err(|_| StoreError::InvalidRecord("delivery history is too long".into()))?
            || self.journal_head
                != previous_digest.ok_or_else(|| {
                    StoreError::InvalidRecord("delivery journal lacks genesis".into())
                })?
            || self.journal_phase != previous
        {
            return Err(StoreError::InvalidRecord(
                "delivery journal commitment does not match event history".into(),
            ));
        }
        if (matches!(previous, StoredRehearsalDeliveryPhase::Completed) && self.screen.is_none())
            || (self.screen.is_some()
                && !matches!(
                    previous,
                    StoredRehearsalDeliveryPhase::Completed | StoredRehearsalDeliveryPhase::Expired
                ))
            || self.screen.is_some() != self.screen_digest.is_some()
        {
            return Err(StoreError::InvalidRecord(
                "delivery projection does not match event history".into(),
            ));
        }
        if matches!(previous, StoredRehearsalDeliveryPhase::Prepared)
            && (self.timing_witness.is_some() || self.run_time_exhausted_deadline.is_some())
        {
            return Err(StoreError::InvalidRecord(
                "prepared delivery retains a later event projection".into(),
            ));
        }
        if matches!(
            previous,
            StoredRehearsalDeliveryPhase::Dispatched
                | StoredRehearsalDeliveryPhase::Completed
                | StoredRehearsalDeliveryPhase::Expired
        ) && self.timing_witness.is_none()
        {
            return Err(StoreError::InvalidRecord(
                "dispatched delivery lacks an immutable timing witness".into(),
            ));
        }
        if matches!(
            previous,
            StoredRehearsalDeliveryPhase::RunTimeExhaustedBeforeDispatch
        ) != self.run_time_exhausted_deadline.is_some()
        {
            return Err(StoreError::InvalidRecord(
                "run-time terminal projection does not match event history".into(),
            ));
        }
        Ok(previous)
    }

    pub(in crate::in_memory) fn append_phase(
        &mut self,
        phase: StoredRehearsalDeliveryPhase,
        now: question_model::ActivityTimestamp,
    ) -> Result<(), StoreError> {
        let previous = self.phase()?;
        if !previous.may_transition_to(phase) {
            return Err(StoreError::Conflict);
        }
        let sequence = u32::try_from(self.events.len() + 1)
            .map_err(|_| StoreError::InvalidRecord("delivery history is too long".into()))?;
        let event = StoredRehearsalDeliveryEvent {
            sequence,
            phase,
            recorded_at: now,
            digest: delivery_event_digest(
                self.events.last().map(|event| event.digest),
                sequence,
                phase,
                now,
            ),
        };
        self.events.push(event);
        self.journal_head = event.digest;
        self.journal_count = sequence;
        self.journal_phase = phase;
        Ok(())
    }
}

fn delivery_event_digest(
    previous: Option<objects::Sha256Digest>,
    sequence: u32,
    phase: StoredRehearsalDeliveryPhase,
    recorded_at: question_model::ActivityTimestamp,
) -> objects::Sha256Digest {
    let mut bytes = b"ple:rehearsal:delivery-event:v1\0".to_vec();
    if let Some(digest) = previous {
        bytes.extend_from_slice(digest.as_bytes());
    } else {
        bytes.extend_from_slice(&[0; 32]);
    }
    bytes.extend_from_slice(&sequence.to_be_bytes());
    bytes.push(phase.tag());
    bytes.extend_from_slice(&recorded_at.as_unix_millis().to_be_bytes());
    objects::Sha256Digest::compute(&bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::in_memory) enum StoredRehearsalDeliveryPhase {
    Prepared,
    Dispatched,
    Completed,
    Expired,
    RunTimeExhaustedBeforeDispatch,
    AbandonedBeforeDispatch,
}

impl StoredRehearsalDeliveryPhase {
    fn may_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Prepared,
                Self::Dispatched
                    | Self::RunTimeExhaustedBeforeDispatch
                    | Self::AbandonedBeforeDispatch
            ) | (Self::Dispatched, Self::Completed | Self::Expired)
                | (Self::Completed, Self::Expired)
        )
    }

    fn tag(self) -> u8 {
        match self {
            Self::Prepared => 1,
            Self::Dispatched => 2,
            Self::Completed => 3,
            Self::Expired => 4,
            Self::RunTimeExhaustedBeforeDispatch => 5,
            Self::AbandonedBeforeDispatch => 6,
        }
    }
}

impl std::fmt::Debug for StoredRehearsalDeliveryOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredRehearsalDeliveryOperation")
            .field("generations", &self.generations.len())
            .field(
                "latest_operation",
                &self.generations.last().map(|entry| entry.operation),
            )
            .field(
                "latest_phase",
                &self.generations.last().and_then(|entry| entry.phase().ok()),
            )
            .field(
                "completed",
                &self
                    .generations
                    .last()
                    .and_then(|entry| entry.screen.as_ref())
                    .is_some(),
            )
            .finish()
    }
}

#[derive(Clone)]
pub(in crate::in_memory) struct StoredRehearsalClaim {
    /// Cloneable canonical persistence material. Rendered input is decoded
    /// only through the authenticated issued screen during hydration; Memory
    /// never keeps a reusable rendered-to-durable mapping.
    pub(in crate::in_memory) claim: RehearsalSubmissionClaimId,
    pub(in crate::in_memory) fingerprint: domain::RehearsalSubmissionRequestFingerprint,
    pub(in crate::in_memory) attempt: RehearsalAttemptId,
    pub(in crate::in_memory) submission_input: serde_json::Value,
    pub(in crate::in_memory) events: Vec<RehearsalClaimTransitionEvent>,
    pub(in crate::in_memory) receipt: Option<StoredRehearsalSubmissionReceipt>,
    /// Immutable route-claim binding to the exact issued screen generation.
    /// Generic test-support claims deliberately leave this absent.
    pub(in crate::in_memory) route_delivery: Option<StoredRehearsalClaimDeliveryBinding>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::in_memory) struct StoredRehearsalClaimDeliveryBinding {
    pub(in crate::in_memory) operation: crate::RehearsalOperationId,
    pub(in crate::in_memory) screen_digest: question_model::RehearsalPresentationDigestV1,
    /// Private canonical digest binding this exact claim root to one issued
    /// delivery generation.  It prevents a same-attempt retry substitution.
    pub(in crate::in_memory) binding_digest: crate::RehearsalOperationDigest,
}

impl std::fmt::Debug for StoredRehearsalClaim {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StoredRehearsalClaim")
            .field("claim", &self.claim)
            .field("fingerprint", &self.fingerprint.to_hex())
            .field("events", &self.events)
            .field("receipt", &self.receipt)
            .finish()
    }
}

#[derive(Clone, Default)]
pub(in crate::in_memory) struct StoredRehearsalEvidence(
    pub(in crate::in_memory) Vec<RehearsalEvidenceChainEntry>,
);

impl std::fmt::Debug for StoredRehearsalEvidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.0.iter().map(|entry| {
                (
                    entry.record.clone(),
                    private_payload_digest(&entry.payload).to_hex(),
                )
            }))
            .finish()
    }
}
