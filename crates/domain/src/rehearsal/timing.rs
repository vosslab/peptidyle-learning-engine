//! Immutable, server-clock rehearsal timing witnesses.
//!
//! A witness binds timing to the already-frozen rehearsal material. It is
//! deliberately not serializable: persistence owns its versioned row shape,
//! while this module owns the deterministic derivation and verification rule.

use question_model::run_policy::TimingPolicy;
use question_model::{ActivityTimestamp, RehearsalEvidenceDigest};
use sha2::{Digest, Sha256};

use super::RehearsalSubjectFingerprint;

const MILLIS_PER_SECOND: i64 = 1_000;
const TIMING_WITNESS_DOMAIN: &[u8] = b"ple:rehearsal:timing-witness:v1\0";
const TIMING_WITNESS_BYTES: usize = 99;

/// Complete authoritative input for one rehearsal delivery issue cycle.
///
/// Each value comes from the locked rehearsal aggregate, frozen material, or
/// server clock; browser timestamps and current catalog state never enter this
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalTimingInputsV1 {
    pub subject_fingerprint: RehearsalSubjectFingerprint,
    pub frozen_snapshot_digest: RehearsalEvidenceDigest,
    pub timing_policy: TimingPolicy,
    /// Effective whole-subject limit in seconds, when the resolved subject has one.
    pub subject_time_limit_seconds: Option<u32>,
    pub run_started_at: ActivityTimestamp,
    pub issued_at: ActivityTimestamp,
}

/// Immutable timing facts bound to one dispatched rehearsal issue cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RehearsalTimingWitnessV1 {
    subject_fingerprint: RehearsalSubjectFingerprint,
    frozen_snapshot_digest: RehearsalEvidenceDigest,
    run_started_at: ActivityTimestamp,
    issued_at: ActivityTimestamp,
    deadline: Option<ActivityTimestamp>,
    deadline_source: Option<RehearsalDeadlineSourceV1>,
    grace_millis: Option<i64>,
}

/// The immutable authority that supplied a rehearsal deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalDeadlineSourceV1 {
    PerQuestion,
    PerAttempt,
    SubjectLimit,
}

impl RehearsalTimingWitnessV1 {
    pub const fn subject_fingerprint(self) -> RehearsalSubjectFingerprint {
        self.subject_fingerprint
    }

    pub const fn frozen_snapshot_digest(self) -> RehearsalEvidenceDigest {
        self.frozen_snapshot_digest
    }

    pub const fn run_started_at(self) -> ActivityTimestamp {
        self.run_started_at
    }

    pub const fn issued_at(self) -> ActivityTimestamp {
        self.issued_at
    }

    pub const fn deadline(self) -> Option<ActivityTimestamp> {
        self.deadline
    }

    pub const fn deadline_source(self) -> Option<RehearsalDeadlineSourceV1> {
        self.deadline_source
    }

    pub const fn grace_millis(self) -> Option<i64> {
        self.grace_millis
    }

    /// Returns fixed-width, versioned persistence bytes without serde or JSON.
    ///
    /// The byte order is subject fingerprint, snapshot digest, run start,
    /// issue time, deadline presence and value, source code, then grace presence and value.
    /// An absent value has a zero value field, so the framing is unambiguous.
    pub fn canonical_bytes(self) -> [u8; TIMING_WITNESS_BYTES] {
        let mut bytes = [0; TIMING_WITNESS_BYTES];
        bytes[0..32].copy_from_slice(&self.subject_fingerprint.as_bytes());
        bytes[32..64].copy_from_slice(&self.frozen_snapshot_digest.as_bytes());
        bytes[64..72].copy_from_slice(&self.run_started_at.as_unix_millis().to_be_bytes());
        bytes[72..80].copy_from_slice(&self.issued_at.as_unix_millis().to_be_bytes());
        write_optional_timestamp(&mut bytes[80..89], self.deadline);
        bytes[89] = deadline_source_code(self.deadline_source);
        write_optional_millis(&mut bytes[90..99], self.grace_millis);
        bytes
    }

    /// Returns the domain-separated SHA-256 commitment for exact witness bytes.
    pub fn commitment(self) -> RehearsalEvidenceDigest {
        let mut hasher = Sha256::new();
        hasher.update(TIMING_WITNESS_DOMAIN);
        hasher.update(self.canonical_bytes());
        RehearsalEvidenceDigest::from_bytes(hasher.finalize().into())
    }
}

/// A timing witness or observation that cannot safely be accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalTimingError {
    IssueBeforeRunStart,
    SubjectTimeLimitZero,
    DeadlineBeforeIssue,
    ObservedBeforeIssue,
    TimestampOverflow,
    WitnessMismatch,
}

impl std::fmt::Display for RehearsalTimingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::IssueBeforeRunStart => "rehearsal issue predates run start",
            Self::SubjectTimeLimitZero => "subject time limit must be positive",
            Self::DeadlineBeforeIssue => "selected rehearsal deadline predates issue",
            Self::ObservedBeforeIssue => "observed time predates rehearsal issue",
            Self::TimestampOverflow => "rehearsal timing calculation overflowed",
            Self::WitnessMismatch => "rehearsal timing witness does not match frozen inputs",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RehearsalTimingError {}

/// The only authoritative availability outcomes for a dispatched rehearsal item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalTimingVerdictV1 {
    Untimed,
    Open,
    GracePeriod,
    Expired,
}

/// The timing outcome that governs whether a new rehearsal delivery may dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalTimingDispatchDecisionV1 {
    Witness(RehearsalTimingWitnessV1),
    RunTimeExhausted { deadline: ActivityTimestamp },
}

/// Derives the immutable timing witness for an issue cycle.
///
/// Per-question time anchors to issue. Per-attempt and subject time anchor to
/// the run. The earlier deadline wins; a tie deliberately selects the subject
/// deadline, which has no grace.
pub fn derive_rehearsal_timing_witness(
    inputs: RehearsalTimingInputsV1,
) -> Result<RehearsalTimingWitnessV1, RehearsalTimingError> {
    match decide_rehearsal_timing_dispatch(inputs)? {
        RehearsalTimingDispatchDecisionV1::Witness(witness) => Ok(witness),
        RehearsalTimingDispatchDecisionV1::RunTimeExhausted { .. } => {
            Err(RehearsalTimingError::DeadlineBeforeIssue)
        }
    }
}

/// Decides whether current server issue time permits a new rehearsal delivery.
///
/// A per-question zero-second policy is still lawful because it anchors to the
/// issue time. A run-anchored per-attempt or subject deadline already before
/// issue represents ordinary run-time exhaustion rather than corrupt material.
pub fn decide_rehearsal_timing_dispatch(
    inputs: RehearsalTimingInputsV1,
) -> Result<RehearsalTimingDispatchDecisionV1, RehearsalTimingError> {
    let witness = derive_rehearsal_timing_witness_without_issue_check(inputs)?;
    if let (Some(deadline), Some(source)) = (witness.deadline, witness.deadline_source)
        && deadline < inputs.issued_at
        && matches!(
            source,
            RehearsalDeadlineSourceV1::PerAttempt | RehearsalDeadlineSourceV1::SubjectLimit
        )
    {
        return Ok(RehearsalTimingDispatchDecisionV1::RunTimeExhausted { deadline });
    }
    Ok(RehearsalTimingDispatchDecisionV1::Witness(witness))
}

fn derive_rehearsal_timing_witness_without_issue_check(
    inputs: RehearsalTimingInputsV1,
) -> Result<RehearsalTimingWitnessV1, RehearsalTimingError> {
    if inputs.issued_at < inputs.run_started_at {
        return Err(RehearsalTimingError::IssueBeforeRunStart);
    }

    let (question_deadline, question_source, question_grace_millis) = match inputs.timing_policy {
        TimingPolicy::Untimed => (None, None, None),
        TimingPolicy::PerQuestion {
            seconds,
            grace_seconds,
        } => (
            Some(add_seconds(inputs.issued_at, seconds)?),
            Some(RehearsalDeadlineSourceV1::PerQuestion),
            Some(seconds_to_millis(grace_seconds)),
        ),
        TimingPolicy::PerAttempt {
            seconds,
            grace_seconds,
        } => (
            Some(add_seconds(inputs.run_started_at, seconds)?),
            Some(RehearsalDeadlineSourceV1::PerAttempt),
            Some(seconds_to_millis(grace_seconds)),
        ),
    };
    let subject_deadline = match inputs.subject_time_limit_seconds {
        None => None,
        Some(0) => return Err(RehearsalTimingError::SubjectTimeLimitZero),
        Some(seconds) => Some(add_seconds(inputs.run_started_at, seconds)?),
    };

    let (deadline, deadline_source, grace_millis) = match (question_deadline, subject_deadline) {
        (None, None) => (None, None, None),
        (Some(question), None) => (Some(question), question_source, question_grace_millis),
        (None, Some(subject)) => (
            Some(subject),
            Some(RehearsalDeadlineSourceV1::SubjectLimit),
            Some(0),
        ),
        (Some(question), Some(subject)) if question < subject => {
            (Some(question), question_source, question_grace_millis)
        }
        (Some(_), Some(subject)) => (
            Some(subject),
            Some(RehearsalDeadlineSourceV1::SubjectLimit),
            Some(0),
        ),
    };

    if let (Some(deadline), Some(grace_millis)) = (deadline, grace_millis) {
        let _ = add_millis(deadline, grace_millis)?;
    }

    Ok(RehearsalTimingWitnessV1 {
        subject_fingerprint: inputs.subject_fingerprint,
        frozen_snapshot_digest: inputs.frozen_snapshot_digest,
        run_started_at: inputs.run_started_at,
        issued_at: inputs.issued_at,
        deadline,
        deadline_source,
        grace_millis,
    })
}

/// Re-derives and compares a witness before any timing-dependent transition.
pub fn verify_rehearsal_timing_witness(
    inputs: RehearsalTimingInputsV1,
    witness: RehearsalTimingWitnessV1,
) -> Result<(), RehearsalTimingError> {
    if derive_rehearsal_timing_witness(inputs)? == witness {
        Ok(())
    } else {
        Err(RehearsalTimingError::WitnessMismatch)
    }
}

/// Evaluates a verified witness at a server-recorded observation time.
///
/// Both the selected deadline and deadline-plus-grace boundary are inclusive.
pub fn rehearsal_timing_verdict(
    witness: RehearsalTimingWitnessV1,
    observed_at: ActivityTimestamp,
) -> Result<RehearsalTimingVerdictV1, RehearsalTimingError> {
    if observed_at < witness.issued_at {
        return Err(RehearsalTimingError::ObservedBeforeIssue);
    }
    let Some(deadline) = witness.deadline else {
        return Ok(RehearsalTimingVerdictV1::Untimed);
    };
    if observed_at <= deadline {
        return Ok(RehearsalTimingVerdictV1::Open);
    }
    let grace_deadline = add_millis(deadline, witness.grace_millis.unwrap_or(0))?;
    if observed_at <= grace_deadline {
        Ok(RehearsalTimingVerdictV1::GracePeriod)
    } else {
        Ok(RehearsalTimingVerdictV1::Expired)
    }
}

/// Returns whether the immutable issue cycle may receive a lawful retry.
///
/// Only a per-question deadline opens retry after it has expired. A caller
/// still supplies the server observation time; it never alters the witness's
/// deadline source.
pub fn rehearsal_retry_is_available(
    witness: RehearsalTimingWitnessV1,
    observed_at: ActivityTimestamp,
) -> Result<bool, RehearsalTimingError> {
    Ok(matches!(
        (
            witness.deadline_source,
            rehearsal_timing_verdict(witness, observed_at)?
        ),
        (
            Some(RehearsalDeadlineSourceV1::PerQuestion),
            RehearsalTimingVerdictV1::Expired
        )
    ))
}

fn add_seconds(
    timestamp: ActivityTimestamp,
    seconds: u32,
) -> Result<ActivityTimestamp, RehearsalTimingError> {
    add_millis(timestamp, seconds_to_millis(seconds))
}

fn seconds_to_millis(seconds: u32) -> i64 {
    i64::from(seconds) * MILLIS_PER_SECOND
}

fn add_millis(
    timestamp: ActivityTimestamp,
    millis: i64,
) -> Result<ActivityTimestamp, RehearsalTimingError> {
    timestamp
        .as_unix_millis()
        .checked_add(millis)
        .map(ActivityTimestamp::from_unix_millis)
        .ok_or(RehearsalTimingError::TimestampOverflow)
}

fn write_optional_timestamp(destination: &mut [u8], value: Option<ActivityTimestamp>) {
    destination[0] = u8::from(value.is_some());
    if let Some(value) = value {
        destination[1..9].copy_from_slice(&value.as_unix_millis().to_be_bytes());
    }
}

fn write_optional_millis(destination: &mut [u8], value: Option<i64>) {
    destination[0] = u8::from(value.is_some());
    if let Some(value) = value {
        destination[1..9].copy_from_slice(&value.to_be_bytes());
    }
}

const fn deadline_source_code(source: Option<RehearsalDeadlineSourceV1>) -> u8 {
    match source {
        None => 0,
        Some(RehearsalDeadlineSourceV1::PerQuestion) => 1,
        Some(RehearsalDeadlineSourceV1::PerAttempt) => 2,
        Some(RehearsalDeadlineSourceV1::SubjectLimit) => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(value: i64) -> ActivityTimestamp {
        ActivityTimestamp::from_unix_millis(value)
    }

    fn inputs(policy: TimingPolicy, subject_limit: Option<u32>) -> RehearsalTimingInputsV1 {
        RehearsalTimingInputsV1 {
            subject_fingerprint: RehearsalSubjectFingerprint([7; 32]),
            frozen_snapshot_digest: RehearsalEvidenceDigest::from_bytes([9; 32]),
            timing_policy: policy,
            subject_time_limit_seconds: subject_limit,
            run_started_at: timestamp(1_000),
            issued_at: timestamp(2_000),
        }
    }

    #[test]
    fn derives_the_earliest_deadline_and_its_lawful_grace() {
        let cases = [
            (
                "untimed without subject limit",
                inputs(TimingPolicy::Untimed, None),
                None,
                None,
                None,
            ),
            (
                "per question anchors to issue",
                inputs(
                    TimingPolicy::PerQuestion {
                        seconds: 4,
                        grace_seconds: 3,
                    },
                    None,
                ),
                Some(6_000),
                Some(RehearsalDeadlineSourceV1::PerQuestion),
                Some(3_000),
            ),
            (
                "per attempt anchors to run",
                inputs(
                    TimingPolicy::PerAttempt {
                        seconds: 4,
                        grace_seconds: 3,
                    },
                    None,
                ),
                Some(5_000),
                Some(RehearsalDeadlineSourceV1::PerAttempt),
                Some(3_000),
            ),
            (
                "earlier subject limit removes question grace",
                inputs(
                    TimingPolicy::PerQuestion {
                        seconds: 8,
                        grace_seconds: 3,
                    },
                    Some(4),
                ),
                Some(5_000),
                Some(RehearsalDeadlineSourceV1::SubjectLimit),
                Some(0),
            ),
            (
                "subject wins an exact tie",
                inputs(
                    TimingPolicy::PerQuestion {
                        seconds: 3,
                        grace_seconds: 9,
                    },
                    Some(4),
                ),
                Some(5_000),
                Some(RehearsalDeadlineSourceV1::SubjectLimit),
                Some(0),
            ),
        ];

        for (name, input, deadline, source, grace) in cases {
            let witness = derive_rehearsal_timing_witness(input).expect(name);
            assert_eq!(
                witness.deadline().map(|value| value.as_unix_millis()),
                deadline,
                "{name}"
            );
            assert_eq!(witness.deadline_source(), source, "{name}");
            assert_eq!(witness.grace_millis(), grace, "{name}");
            assert_eq!(
                verify_rehearsal_timing_witness(input, witness),
                Ok(()),
                "{name}"
            );
        }
    }

    #[test]
    fn verdict_boundaries_are_inclusive_and_server_owned() {
        let witness = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerQuestion {
                seconds: 4,
                grace_seconds: 2,
            },
            None,
        ))
        .expect("valid witness");
        let cases = [
            (2_000, RehearsalTimingVerdictV1::Open),
            (6_000, RehearsalTimingVerdictV1::Open),
            (6_001, RehearsalTimingVerdictV1::GracePeriod),
            (8_000, RehearsalTimingVerdictV1::GracePeriod),
            (8_001, RehearsalTimingVerdictV1::Expired),
        ];
        for (observed_at, expected) in cases {
            assert_eq!(
                rehearsal_timing_verdict(witness, timestamp(observed_at)),
                Ok(expected)
            );
        }
    }

    #[test]
    fn canonical_witness_bytes_and_commitment_have_a_stable_vector() {
        let witness = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerQuestion {
                seconds: 4,
                grace_seconds: 2,
            },
            None,
        ))
        .expect("valid witness");
        let mut expected = [0_u8; TIMING_WITNESS_BYTES];
        expected[0..32].fill(7);
        expected[32..64].fill(9);
        expected[64..72].copy_from_slice(&1_000_i64.to_be_bytes());
        expected[72..80].copy_from_slice(&2_000_i64.to_be_bytes());
        expected[80] = 1;
        expected[81..89].copy_from_slice(&6_000_i64.to_be_bytes());
        expected[89] = 1;
        expected[90] = 1;
        expected[91..99].copy_from_slice(&2_000_i64.to_be_bytes());

        assert_eq!(witness.canonical_bytes(), expected);
        assert_eq!(
            witness.commitment().to_hex(),
            "285d4c0bba98360fdb6bc32a0d1253ef610506e3f0e94d56805ce2c81106822f"
        );
    }

    #[test]
    fn commitment_changes_when_any_bound_timing_fact_changes() {
        let baseline = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerQuestion {
                seconds: 4,
                grace_seconds: 2,
            },
            None,
        ))
        .expect("valid baseline");
        let changed_fingerprint = derive_rehearsal_timing_witness(RehearsalTimingInputsV1 {
            subject_fingerprint: RehearsalSubjectFingerprint([8; 32]),
            ..inputs(
                TimingPolicy::PerQuestion {
                    seconds: 4,
                    grace_seconds: 2,
                },
                None,
            )
        })
        .expect("valid changed fingerprint");
        let changed_grace = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerQuestion {
                seconds: 4,
                grace_seconds: 3,
            },
            None,
        ))
        .expect("valid changed grace");
        let changed_source = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerAttempt {
                seconds: 5,
                grace_seconds: 2,
            },
            None,
        ))
        .expect("valid changed source");

        assert_ne!(baseline.commitment(), changed_fingerprint.commitment());
        assert_ne!(baseline.commitment(), changed_grace.commitment());
        assert_eq!(baseline.deadline(), changed_source.deadline());
        assert_eq!(baseline.grace_millis(), changed_source.grace_millis());
        assert_ne!(baseline.deadline_source(), changed_source.deadline_source());
        assert_ne!(baseline.commitment(), changed_source.commitment());
    }

    #[test]
    fn retry_is_limited_to_expired_per_question_cycles() {
        let per_question = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerQuestion {
                seconds: 4,
                grace_seconds: 0,
            },
            None,
        ))
        .expect("valid per-question witness");
        let per_attempt = derive_rehearsal_timing_witness(inputs(
            TimingPolicy::PerAttempt {
                seconds: 4,
                grace_seconds: 0,
            },
            None,
        ))
        .expect("valid per-attempt witness");
        assert_eq!(
            rehearsal_retry_is_available(per_question, timestamp(6_001)),
            Ok(true)
        );
        assert_eq!(
            rehearsal_retry_is_available(per_question, timestamp(6_000)),
            Ok(false)
        );
        assert_eq!(
            rehearsal_retry_is_available(per_attempt, timestamp(6_001)),
            Ok(false)
        );
    }

    #[test]
    fn dispatch_distinguishes_run_time_exhaustion_from_a_lawful_immediate_question() {
        let cases = [
            (
                "per-attempt exhausted before issue",
                RehearsalTimingInputsV1 {
                    timing_policy: TimingPolicy::PerAttempt {
                        seconds: 1,
                        grace_seconds: 0,
                    },
                    issued_at: timestamp(2_001),
                    ..inputs(TimingPolicy::Untimed, None)
                },
                RehearsalTimingDispatchDecisionV1::RunTimeExhausted {
                    deadline: timestamp(2_000),
                },
            ),
            (
                "subject limit exhausted before issue",
                RehearsalTimingInputsV1 {
                    subject_time_limit_seconds: Some(1),
                    issued_at: timestamp(2_001),
                    ..inputs(TimingPolicy::Untimed, None)
                },
                RehearsalTimingDispatchDecisionV1::RunTimeExhausted {
                    deadline: timestamp(2_000),
                },
            ),
            (
                "issue at run deadline wins the dispatch race",
                RehearsalTimingInputsV1 {
                    timing_policy: TimingPolicy::PerAttempt {
                        seconds: 1,
                        grace_seconds: 0,
                    },
                    issued_at: timestamp(2_000),
                    ..inputs(TimingPolicy::Untimed, None)
                },
                RehearsalTimingDispatchDecisionV1::Witness(
                    derive_rehearsal_timing_witness(inputs(
                        TimingPolicy::PerAttempt {
                            seconds: 1,
                            grace_seconds: 0,
                        },
                        None,
                    ))
                    .expect("reference witness"),
                ),
            ),
            (
                "immediate per-question timing remains a witness",
                RehearsalTimingInputsV1 {
                    timing_policy: TimingPolicy::PerQuestion {
                        seconds: 0,
                        grace_seconds: 0,
                    },
                    ..inputs(TimingPolicy::Untimed, None)
                },
                RehearsalTimingDispatchDecisionV1::Witness(
                    derive_rehearsal_timing_witness(inputs(
                        TimingPolicy::PerQuestion {
                            seconds: 0,
                            grace_seconds: 0,
                        },
                        None,
                    ))
                    .expect("immediate witness"),
                ),
            ),
        ];
        for (name, input, expected) in cases {
            assert_eq!(
                decide_rehearsal_timing_dispatch(input),
                Ok(expected),
                "{name}"
            );
        }
    }

    #[test]
    fn invalid_inputs_and_mismatched_witnesses_fail_closed() {
        let valid = inputs(TimingPolicy::Untimed, None);
        let witness = derive_rehearsal_timing_witness(valid).expect("valid witness");
        let cases = [
            (
                "issue before run",
                RehearsalTimingInputsV1 {
                    issued_at: timestamp(999),
                    ..valid
                },
                RehearsalTimingError::IssueBeforeRunStart,
            ),
            (
                "zero subject limit",
                RehearsalTimingInputsV1 {
                    subject_time_limit_seconds: Some(0),
                    ..valid
                },
                RehearsalTimingError::SubjectTimeLimitZero,
            ),
            (
                "selected deadline before issue",
                RehearsalTimingInputsV1 {
                    timing_policy: TimingPolicy::PerAttempt {
                        seconds: 0,
                        grace_seconds: 0,
                    },
                    ..valid
                },
                RehearsalTimingError::DeadlineBeforeIssue,
            ),
            (
                "overflow",
                RehearsalTimingInputsV1 {
                    run_started_at: timestamp(i64::MAX),
                    issued_at: timestamp(i64::MAX),
                    timing_policy: TimingPolicy::PerQuestion {
                        seconds: 1,
                        grace_seconds: 0,
                    },
                    ..valid
                },
                RehearsalTimingError::TimestampOverflow,
            ),
        ];
        for (name, input, expected) in cases {
            assert_eq!(
                derive_rehearsal_timing_witness(input),
                Err(expected),
                "{name}"
            );
        }

        let changed = RehearsalTimingInputsV1 {
            frozen_snapshot_digest: RehearsalEvidenceDigest::from_bytes([10; 32]),
            ..valid
        };
        assert_eq!(
            verify_rehearsal_timing_witness(changed, witness),
            Err(RehearsalTimingError::WitnessMismatch)
        );
        assert_eq!(
            rehearsal_timing_verdict(witness, timestamp(1_999)),
            Err(RehearsalTimingError::ObservedBeforeIssue)
        );
    }
}
