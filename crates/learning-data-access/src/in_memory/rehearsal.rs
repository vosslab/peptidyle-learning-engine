//! In-memory implementation of the isolated WP-PROF-T4 rehearsal aggregate.
//!
//! The modules follow durable ownership boundaries: route lifecycle, frozen
//! material, delivery operations, route mutations, start admission, and
//! read-side integrity are independently maintainable without a second
//! rehearsal implementation.

use async_trait::async_trait;
use domain::{
    DispatchedClaimHandle, PreparedClaimHandle, RehearsalClaimRoot, RehearsalClaimSubmissionInput,
    RehearsalClaimTransitionEvent, RehearsalEvidenceChainEntry, RehearsalEvidencePayload,
    RehearsalLifecycleSnapshot, RehearsalPersistedClaimRoot, RehearsalSubmissionClaimDecision,
    RehearsalSubmissionClaimPhase, RehearsalSubmissionClaimSnapshot, RehearsalTerminalTransition,
    RehearsalValidatedSubmissionRequest, abandon_rehearsal_submission_before_dispatch,
    decide_start, decide_submission_claim, evidence_entry_digest,
    fingerprint_resolved_preview_subject, hydrate_claim_history,
    mark_rehearsal_submission_dispatched, private_payload_digest,
    rehearsal_claim_submission_input_fingerprint, validate_claim_completion,
    verify_rehearsal_claim_completion_proof,
};
use question_model::{
    AssignmentId, AssignmentReference, CourseId, CourseMembershipId, CourseMembershipReference,
    PreviewEvaluation, RehearsalAttemptId, RehearsalEvidenceKind, RehearsalEvidenceRecord,
    RehearsalGradeOperationId, RehearsalLifecycle, RehearsalReference, RehearsalRunId,
    RehearsalRunReceipt, RehearsalSubjectStart, RehearsalSubmissionClaimId,
    TeachingOperationRevision, TenantId, UserId,
};

use super::rehearsal_integrity::{
    genesis, transition_locked, verify_rehearsal_aggregate, verify_run,
};
use super::*;
use crate::{
    RehearsalDeliveryPreDispatchCompensationStore, RehearsalOperationStore,
    RehearsalPreDispatchCompensationStore,
};

mod delivery;
mod material;
mod mutations;
mod read;
mod route;
mod start;
mod types;

pub(super) use super::rehearsal_integrity::invalidate_assignment_rehearsals;
pub(super) use delivery::{
    decode_rehearsal_private_checksum, durable_request_for_claim_completion,
    reconcile_delivery_expiry_locked,
};
#[cfg(feature = "test-support")]
pub(super) use mutations::transition_by_locator;
pub(super) use mutations::{
    RouteClaimDeliveryBindingInput, route_claim_delivery_binding,
    route_claim_initial_grade_operation,
};
pub(super) use read::{
    active_current, append_evidence, authorize_assignment, authorize_locator, authorized_run,
    claim_key_for_handle, fresh_uuid, hydrate_claim, invalid_claim_history, invalid_claim_root,
    next_claim_sequence, next_evidence_entry, next_reference, receipt, resolve_subject_locked,
    revision_is_current, same_dispatched_handle, same_prepared_handle,
};
pub(super) use route::complete_sealed_submission_locked;
pub(super) use start::{
    canonical_rehearsal_question_content_digest, deterministic_rehearsal_seed,
    freeze_route_start_material, missing_rehearsal_material, private_execution_checksum,
    start_locked,
};
pub(in crate::in_memory) use types::{
    StoredRehearsalClaim, StoredRehearsalDeliveryGeneration, StoredRehearsalDeliveryOperation,
    StoredRehearsalDeliveryPhase, StoredRehearsalDeliveryRetry, StoredRehearsalEvidence,
    StoredRehearsalFrozenPrivateExecution, StoredRehearsalFrozenSourceSnapshot, StoredRehearsalRun,
    StoredRehearsalStartOperation, StoredRehearsalSubmissionReceipt,
};
