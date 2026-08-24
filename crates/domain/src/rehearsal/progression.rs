//! Pure verification of Store-owned rehearsal progression.
//!
//! The persisted implementation supplies this module with a deliberately
//! lossy state projection: frozen ordinals, accepted attempts, and current
//! operation links.  The verifier never chooses a catalog item.  It proves
//! that the Store's selected item is the only lawful next frozen item.

use std::collections::{BTreeMap, BTreeSet};

use question_model::{RehearsalAttemptId, RehearsalLifecycle};

/// One immutable frozen assignment position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalProgressFrozenAttempt {
    pub ordinal: u32,
    pub attempt: RehearsalAttemptId,
}

/// An unresolved state retained by the Store for one frozen attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalProgressOpenPhase {
    Prepared,
    Dispatched,
    Issued,
    SubmissionPending,
    Expired,
}

/// Links that must all name the selected frozen attempt.  `None` is allowed
/// only where a phase has not produced that artifact yet; a present link for
/// another attempt is corruption, never a cue to advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalProgressOpenAttempt {
    pub attempt: RehearsalAttemptId,
    pub phase: RehearsalProgressOpenPhase,
    pub delivery_attempt: Option<RehearsalAttemptId>,
    pub submission_claim_attempt: Option<RehearsalAttemptId>,
    pub screen_attempt: Option<RehearsalAttemptId>,
}

/// Store-hydrated facts needed to derive the only lawful route state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehearsalProgressSnapshot {
    pub lifecycle: RehearsalLifecycle,
    pub frozen: Vec<RehearsalProgressFrozenAttempt>,
    pub accepted_attempts: Vec<RehearsalAttemptId>,
    pub open_attempts: Vec<RehearsalProgressOpenAttempt>,
}

/// The safe, Store-derived progression result. `ordinal` is internal and is
/// intentionally absent from browser transport contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalDerivedProgress {
    Next {
        attempt: RehearsalAttemptId,
        ordinal: u32,
        total: u32,
    },
    Open {
        attempt: RehearsalAttemptId,
        ordinal: u32,
        total: u32,
        phase: RehearsalProgressOpenPhase,
    },
    Completed {
        total: u32,
    },
    Terminal {
        lifecycle: RehearsalLifecycle,
        total: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalProgressError {
    EmptyFrozenInventory,
    NonContiguousOrdinal,
    DuplicateFrozenAttempt,
    AcceptedAttemptNotFrozen,
    DuplicateAcceptedAttempt,
    OpenAttemptNotFrozen,
    MultipleOpenAttempts,
    OpenAttemptAlreadyAccepted,
    FutureItemSelected,
    MismatchedOperationLink,
    ActiveLifecycleMissingProgress,
    TerminalLifecycleIncompleteCoverage,
    NonCompletedTerminalHasCompletionState,
}

/// Derives the only lawful frozen item from immutable evidence and operation
/// state.  This is intentionally pure so Memory and PostgreSQL use identical
/// corruption and sequencing rules.
pub fn derive_rehearsal_progress(
    snapshot: &RehearsalProgressSnapshot,
) -> Result<RehearsalDerivedProgress, RehearsalProgressError> {
    let total = u32::try_from(snapshot.frozen.len())
        .map_err(|_| RehearsalProgressError::NonContiguousOrdinal)?;
    if total == 0 {
        return Err(RehearsalProgressError::EmptyFrozenInventory);
    }
    let mut frozen_by_attempt = BTreeMap::new();
    for expected in 0..total {
        let frozen = snapshot
            .frozen
            .iter()
            .find(|frozen| frozen.ordinal == expected)
            .ok_or(RehearsalProgressError::NonContiguousOrdinal)?;
        if frozen_by_attempt
            .insert(frozen.attempt, frozen.ordinal)
            .is_some()
        {
            return Err(RehearsalProgressError::DuplicateFrozenAttempt);
        }
    }
    if snapshot.frozen.len() != frozen_by_attempt.len() {
        return Err(RehearsalProgressError::NonContiguousOrdinal);
    }

    let mut accepted = BTreeSet::new();
    for attempt in &snapshot.accepted_attempts {
        if !frozen_by_attempt.contains_key(attempt) {
            return Err(RehearsalProgressError::AcceptedAttemptNotFrozen);
        }
        if !accepted.insert(*attempt) {
            return Err(RehearsalProgressError::DuplicateAcceptedAttempt);
        }
    }
    let next = snapshot
        .frozen
        .iter()
        .min_by_key(|frozen| frozen.ordinal)
        .and_then(|_| {
            snapshot
                .frozen
                .iter()
                .filter(|frozen| !accepted.contains(&frozen.attempt))
                .min_by_key(|frozen| frozen.ordinal)
        });

    if snapshot.open_attempts.len() > 1 {
        return Err(RehearsalProgressError::MultipleOpenAttempts);
    }
    let open = snapshot.open_attempts.first();
    if let Some(open) = open {
        let Some(ordinal) = frozen_by_attempt.get(&open.attempt).copied() else {
            return Err(RehearsalProgressError::OpenAttemptNotFrozen);
        };
        if accepted.contains(&open.attempt) {
            return Err(RehearsalProgressError::OpenAttemptAlreadyAccepted);
        }
        if [
            open.delivery_attempt,
            open.submission_claim_attempt,
            open.screen_attempt,
        ]
        .into_iter()
        .flatten()
        .any(|linked| linked != open.attempt)
        {
            return Err(RehearsalProgressError::MismatchedOperationLink);
        }
        let Some(next) = next else {
            return Err(RehearsalProgressError::FutureItemSelected);
        };
        if next.attempt != open.attempt || ordinal != next.ordinal {
            return Err(RehearsalProgressError::FutureItemSelected);
        }
    }

    match snapshot.lifecycle {
        RehearsalLifecycle::Active => match (next, open) {
            (Some(next), Some(open)) => Ok(RehearsalDerivedProgress::Open {
                attempt: next.attempt,
                ordinal: next.ordinal,
                total,
                phase: open.phase,
            }),
            (Some(next), None) => Ok(RehearsalDerivedProgress::Next {
                attempt: next.attempt,
                ordinal: next.ordinal,
                total,
            }),
            (None, _) => Err(RehearsalProgressError::ActiveLifecycleMissingProgress),
        },
        RehearsalLifecycle::Completed => {
            if next.is_some() || open.is_some() {
                return Err(RehearsalProgressError::TerminalLifecycleIncompleteCoverage);
            }
            Ok(RehearsalDerivedProgress::Completed { total })
        }
        lifecycle => {
            if open.is_some() {
                return Err(RehearsalProgressError::NonCompletedTerminalHasCompletionState);
            }
            Ok(RehearsalDerivedProgress::Terminal { lifecycle, total })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn attempt(value: u128) -> RehearsalAttemptId {
        RehearsalAttemptId::from_uuid(Uuid::from_u128(value))
    }
    fn frozen() -> Vec<RehearsalProgressFrozenAttempt> {
        (0..4)
            .map(|ordinal| RehearsalProgressFrozenAttempt {
                ordinal,
                attempt: attempt(u128::from(ordinal) + 1),
            })
            .collect()
    }
    #[test]
    fn accepts_only_lowest_unaccepted_frozen_ordinal() {
        let snapshot = RehearsalProgressSnapshot {
            lifecycle: RehearsalLifecycle::Active,
            frozen: frozen(),
            accepted_attempts: vec![attempt(1)],
            open_attempts: vec![],
        };
        assert_eq!(
            derive_rehearsal_progress(&snapshot),
            Ok(RehearsalDerivedProgress::Next {
                attempt: attempt(2),
                ordinal: 1,
                total: 4
            })
        );
    }
    #[test]
    fn rejects_corrupt_or_out_of_order_progress() {
        let mut snapshot = RehearsalProgressSnapshot {
            lifecycle: RehearsalLifecycle::Active,
            frozen: frozen(),
            accepted_attempts: vec![attempt(1), attempt(1)],
            open_attempts: vec![],
        };
        assert_eq!(
            derive_rehearsal_progress(&snapshot),
            Err(RehearsalProgressError::DuplicateAcceptedAttempt)
        );
        snapshot.accepted_attempts = vec![attempt(1)];
        snapshot.open_attempts = vec![RehearsalProgressOpenAttempt {
            attempt: attempt(3),
            phase: RehearsalProgressOpenPhase::Issued,
            delivery_attempt: Some(attempt(3)),
            submission_claim_attempt: None,
            screen_attempt: Some(attempt(3)),
        }];
        assert_eq!(
            derive_rehearsal_progress(&snapshot),
            Err(RehearsalProgressError::FutureItemSelected)
        );
    }
    #[test]
    fn completed_requires_exact_full_coverage() {
        let mut snapshot = RehearsalProgressSnapshot {
            lifecycle: RehearsalLifecycle::Completed,
            frozen: frozen(),
            accepted_attempts: vec![attempt(1)],
            open_attempts: vec![],
        };
        assert_eq!(
            derive_rehearsal_progress(&snapshot),
            Err(RehearsalProgressError::TerminalLifecycleIncompleteCoverage)
        );
        snapshot.accepted_attempts = (1..=4).map(attempt).collect();
        assert_eq!(
            derive_rehearsal_progress(&snapshot),
            Ok(RehearsalDerivedProgress::Completed { total: 4 })
        );
    }
}
