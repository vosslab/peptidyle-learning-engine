//! MOD-DOMAIN: attempt state, runs, timing, generation, and validation.
//!
//! This crate reaches `question_model` and nothing else, so it has no clock
//! and no database. Time and storage arrive as parameters, which is what lets
//! the same code run on the server and in the browser through `wasm_bridge`
//! and makes the seed-parity test meaningful.

/// Attempt state machine (MOD-STATE).
pub mod attempt;
/// Completion derivation within a run (MOD-STATE).
pub mod completion;
/// Pure course-grade aggregation from selected assignment scores.
pub mod course_grade;
/// Pure evaluation of assignment-owned learner disclosure policy (WP-PROF-S4).
pub mod disclosure_policy;
/// Key-free deterministic workspace-draft prompt preview (MOD-WASM).
pub mod draft_preview;
/// Pure current assignment-policy resolution after S5 entitlement.
pub mod effective_assignment_policy;
/// Pure current-entitlement evaluation. Persistence supplies normalized facts;
/// this module never reads a roster, database, clock, or browser token.
pub mod entitlement;
/// Seeded question generation (MOD-GEN).
pub mod generator;
/// Current tenant-owned course item-analysis projections (MOD-STATS).
pub mod item_analysis;
/// Assignment configuration validation (MOD-CAP).
pub mod policy;
/// Pure non-mutating S5 -> S3 -> S4 preview composition (WP-PROF-T3).
pub mod preview_plane;
/// Pure instructor-owned rehearsal lifecycle and evidence-chain rules (WP-PROF-T4).
pub mod rehearsal;
/// Continued-practice eligibility and shared run-model errors (MOD-RUN).
pub mod run;
/// Completed-run score selection and summary projection (MOD-SCORE).
pub mod scoring;
/// Retention-safe anonymous question-statistics aggregation (MOD-STATS).
pub mod statistics;
/// Pure group-membership and co-instructor authority validation (WP-PROF-T2).
pub mod teaching_authority;
/// Timer verdict for time-limited attempts (MOD-TIME).
pub mod timing;
/// Browser-safe student-response format validation (MOD-GRD boundary).
pub mod validation;

pub use crate::course_grade::{
    CourseGradeAssignment, CourseGradeError, CourseGradeOutcome, CourseGradeUnavailableReason,
    calculate_course_grade,
};
pub use crate::rehearsal::{
    DispatchedClaimHandle, PreparedClaimHandle, RehearsalClaimCompletionError,
    RehearsalClaimCompletionMaterial, RehearsalClaimCompletionProofError, RehearsalClaimGeneration,
    RehearsalClaimHandleError, RehearsalClaimHydrationError, RehearsalClaimReclaimError,
    RehearsalClaimRoot, RehearsalClaimRootVerificationError, RehearsalClaimSubmissionInput,
    RehearsalClaimTransitionEvent, RehearsalDeadlineSourceV1, RehearsalEvidenceChainEntry,
    RehearsalEvidenceHead, RehearsalEvidencePayload, RehearsalFrozenInventoryEntry,
    RehearsalGenesisContext, RehearsalIntegrityError, RehearsalInventoryError,
    RehearsalLifecycleSnapshot, RehearsalPersistedClaimRoot, RehearsalPreDispatchAbandonReason,
    RehearsalPreDispatchAbandonment, RehearsalStartDecision, RehearsalSubjectFingerprint,
    RehearsalSubmissionClaimDecision, RehearsalSubmissionClaimPhase,
    RehearsalSubmissionClaimSnapshot, RehearsalSubmissionClaimState,
    RehearsalSubmissionRequestFingerprint, RehearsalTerminalTransition,
    RehearsalTimingDispatchDecisionV1, RehearsalTimingError, RehearsalTimingInputsV1,
    RehearsalTimingVerdictV1, RehearsalTimingWitnessV1, RehearsalValidatedSubmissionEvidence,
    RehearsalValidatedSubmissionRequest, VerifiedRehearsalAcceptedEvidenceOwner,
    VerifiedRehearsalClaimCompletionProof, abandon_rehearsal_submission_before_dispatch,
    apply_terminal_transition, decide_rehearsal_timing_dispatch, decide_start,
    decide_submission_claim, derive_rehearsal_timing_witness, evidence_entry_digest,
    evidence_genesis_digest, evidence_genesis_head, fingerprint_resolved_preview_subject,
    frozen_response_schema_digest, hydrate_claim_history, mark_rehearsal_submission_dispatched,
    private_payload_digest, rehearsal_accepted_evidence_owner,
    rehearsal_claim_submission_input_fingerprint, rehearsal_retry_is_available,
    rehearsal_submission_request_fingerprint, rehearsal_timing_verdict, validate_claim_completion,
    validate_subject_binding, verify_evidence_chain, verify_rehearsal_claim_completion_proof,
    verify_rehearsal_inventory, verify_rehearsal_timing_witness,
};
pub use crate::teaching_authority::{
    CoInstructorInvitationAcceptance, CoInstructorInvitationError, DirectInstructorMembership,
    InstructorAuthority, InstructorMembershipRemovalError, accept_co_instructor_invitation,
    evaluate_course_instructor_authority, evaluate_multiple_membership, invitation_state,
    refuse_final_instructor_removal, validate_instructor_approval,
};
