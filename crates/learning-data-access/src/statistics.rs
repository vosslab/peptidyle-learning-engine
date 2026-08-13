//! Server-only derivation of retention-safe question-statistics contributions.

use std::collections::BTreeMap;

use domain::statistics::{CollapsedQuestionObservation, MAX_DURATION_SECONDS};
use objects::Sha256Digest;
use question_model::{AssignmentRunItem, AttemptResult, ProblemVersionRef, QuestionAttempt};

use crate::StoreError;

/// One identity-free contribution for an immutable problem version.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StatisticsContribution {
    pub(crate) reference: ProblemVersionRef,
    pub(crate) observation: CollapsedQuestionObservation,
    pub(crate) checksum: Sha256Digest,
}

/// Derives one collapsed observation per exact version in a newly completed
/// run. This reads only published identities, points, and server timestamps;
/// response, feedback, source, provider, and grading-key material never enter
/// the aggregation boundary.
pub(crate) fn derive_statistics_contributions(
    run_items: &[AssignmentRunItem],
    final_results: &[Option<AttemptResult>],
    attempts: &[QuestionAttempt],
) -> Result<Vec<StatisticsContribution>, StoreError> {
    let mut items = run_items.iter().collect::<Vec<_>>();
    items.sort_by_key(|item| item.issued_position);
    if items
        .iter()
        .enumerate()
        .any(|(position, item)| usize::try_from(item.issued_position).ok() != Some(position))
    {
        return Err(StoreError::InvalidRecord(
            "statistics require contiguous immutable run items".to_string(),
        ));
    }
    if final_results.len() != items.len() || final_results.iter().any(Option::is_none) {
        return Err(StoreError::InvalidRecord(
            "statistics require one final result per delivered position".to_string(),
        ));
    }
    let mut groups = BTreeMap::<ProblemVersionRef, GroupAccumulator>::new();
    for (item, result) in items.iter().copied().zip(final_results) {
        let reference = item.reference;
        let result = result.expect("missing result rejected before statistics derivation");
        crate::validate_attempt_result(result)?;
        let group = groups.entry(reference).or_default();
        group.positions += 1;
        group.earned += result.points_earned;
        group.possible += result.points_possible;
    }
    for attempt in attempts {
        let position = usize::try_from(attempt.assignment_position).map_err(|_| {
            StoreError::InvalidRecord("statistics attempt position is invalid".to_string())
        })?;
        let reference = items
            .get(position)
            .map(|item| item.reference)
            .ok_or_else(|| {
                StoreError::InvalidRecord("statistics attempt position is invalid".to_string())
            })?;
        if attempt.problem != reference.problem || attempt.question_version != reference.version {
            return Err(StoreError::InvalidRecord(
                "statistics attempt identity disagrees with its assignment position".to_string(),
            ));
        }
        let Some(submitted_at) = attempt.timer.submitted_at else {
            continue;
        };
        let group = groups
            .get_mut(&reference)
            .expect("assignment establishes group");
        group.attempts = group.attempts.checked_add(1).ok_or_else(|| {
            StoreError::InvalidRecord("statistics attempt count overflows".to_string())
        })?;
        let elapsed = submitted_at
            .as_unix_millis()
            .checked_sub(attempt.timer.issued_at.as_unix_millis())
            .ok_or_else(|| {
                StoreError::InvalidRecord("statistics attempt time is invalid".to_string())
            })?;
        let elapsed = u64::try_from(elapsed).map_err(|_| {
            StoreError::InvalidRecord("statistics attempt time is invalid".to_string())
        })?;
        group.duration_millis = group
            .duration_millis
            .saturating_add(elapsed)
            .min(MAX_DURATION_SECONDS.saturating_mul(1_000));
    }
    let groups = groups.into_iter().collect::<Vec<_>>();
    let mut prefix = vec![(0.0, 0.0); groups.len() + 1];
    for (index, (_, group)) in groups.iter().enumerate() {
        prefix[index + 1] = (
            prefix[index].0 + group.earned,
            prefix[index].1 + group.possible,
        );
    }
    let mut suffix = vec![(0.0, 0.0); groups.len() + 1];
    for index in (0..groups.len()).rev() {
        let group = &groups[index].1;
        suffix[index] = (
            suffix[index + 1].0 + group.earned,
            suffix[index + 1].1 + group.possible,
        );
    }
    let mut contributions = Vec::with_capacity(groups.len());
    for (index, (reference, group)) in groups.into_iter().enumerate() {
        let question_score = normalized_score(group.earned, group.possible)?;
        let rest_score = (group.positions < items.len())
            .then(|| {
                normalized_score(
                    prefix[index].0 + suffix[index + 1].0,
                    prefix[index].1 + suffix[index + 1].1,
                )
            })
            .transpose()?;
        let attempts_count = group.attempts;
        let duration_seconds = group
            .duration_millis
            .div_ceil(1_000)
            .min(MAX_DURATION_SECONDS);
        let observation = CollapsedQuestionObservation::new(
            question_score,
            attempts_count,
            duration_seconds,
            rest_score,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        contributions.push(StatisticsContribution {
            reference,
            observation,
            checksum: contribution_checksum(reference, observation),
        });
    }
    Ok(contributions)
}

#[derive(Default)]
struct GroupAccumulator {
    positions: usize,
    earned: f64,
    possible: f64,
    attempts: u64,
    duration_millis: u64,
}

fn normalized_score(earned: f64, possible: f64) -> Result<f64, StoreError> {
    if !earned.is_finite() || !possible.is_finite() || possible <= 0.0 {
        return Err(StoreError::InvalidRecord(
            "statistics points are invalid".to_string(),
        ));
    }
    let score = earned / possible;
    if !score.is_finite() || !(0.0..=1.0).contains(&score) {
        return Err(StoreError::InvalidRecord(
            "statistics normalized score is invalid".to_string(),
        ));
    }
    Ok(if score == 0.0 { 0.0 } else { score })
}

fn contribution_checksum(
    reference: ProblemVersionRef,
    observation: CollapsedQuestionObservation,
) -> Sha256Digest {
    let mut bytes = Vec::with_capacity(96);
    bytes.extend_from_slice(b"ple-question-statistics-v1");
    bytes.extend_from_slice(reference.problem.as_uuid().as_bytes());
    bytes.extend_from_slice(reference.version.as_uuid().as_bytes());
    bytes.extend_from_slice(&observation.normalized_score().to_bits().to_be_bytes());
    bytes.extend_from_slice(&observation.attempts().to_be_bytes());
    bytes.extend_from_slice(&observation.duration_seconds().to_be_bytes());
    match observation.rest_score() {
        Some(score) => {
            bytes.push(1);
            bytes.extend_from_slice(&score.to_bits().to_be_bytes());
        }
        None => bytes.push(0),
    }
    Sha256Digest::compute(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        ActivityTimestamp, AssignmentItemId, AttemptProvenance, AttemptTimerRecord,
        ImplementationVersion, ProblemId, QuestionAttemptId, RunId, StudentResponse, TenantId,
        VersionId,
    };
    use uuid::Uuid;

    fn id(number: u128) -> Uuid {
        Uuid::from_u128(number)
    }

    fn reference(number: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(id(number)),
            version: VersionId::from_uuid(id(1_000 + number)),
        }
    }

    fn run_items(problems: Vec<ProblemVersionRef>) -> Vec<AssignmentRunItem> {
        problems
            .into_iter()
            .enumerate()
            .map(|(position, reference)| AssignmentRunItem {
                run: RunId::from_uuid(id(2_004)),
                assignment_item: AssignmentItemId::from_uuid(id(3_000 + position as u128)),
                source_position: u32::try_from(position).expect("fixture position fits"),
                issued_position: u32::try_from(position).expect("fixture position fits"),
                reference,
                selection_group: None,
                selection_seed: None,
            })
            .collect()
    }

    fn result(earned: f64, possible: f64) -> Option<AttemptResult> {
        Some(AttemptResult {
            correct: earned == possible,
            points_earned: earned,
            points_possible: possible,
        })
    }

    fn attempt(
        number: u128,
        reference: ProblemVersionRef,
        position: u32,
        elapsed_millis: Option<i64>,
    ) -> QuestionAttempt {
        let attempt_result = AttemptResult {
            correct: true,
            points_earned: 1.0,
            points_possible: 1.0,
        };
        QuestionAttempt {
            id: QuestionAttemptId::from_uuid(id(3_000 + number)),
            tenant: TenantId::from_uuid(id(2_002)),
            run: RunId::from_uuid(id(2_004)),
            problem: reference.problem,
            question_version: reference.version,
            assignment_position: position,
            seed: number as u64,
            parameter_hash: format!("parameters-{number}"),
            response: elapsed_millis.map(|_| StudentResponse::Numeric { value: 1.0 }),
            status: if elapsed_millis.is_some() {
                question_model::AttemptStatus::Submitted
            } else {
                question_model::AttemptStatus::InProgress
            },
            result: elapsed_millis.map(|_| attempt_result),
            timer: AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(10_000),
                deadline: None,
                submitted_at: elapsed_millis
                    .map(|elapsed| ActivityTimestamp::from_unix_millis(10_000 + elapsed)),
            },
            provenance: AttemptProvenance {
                adapter: ImplementationVersion {
                    id: "native".to_string(),
                    version: "1".to_string(),
                },
                renderer: None,
                generator: None,
                source_artifact: None,
                asset_objects: Vec::new(),
                grading: ImplementationVersion {
                    id: "numeric".to_string(),
                    version: "1".to_string(),
                },
                rendered_question_sha256: format!("rendered-{number}"),
            },
            issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
        }
    }

    fn contribution(
        contributions: &[StatisticsContribution],
        reference: ProblemVersionRef,
    ) -> StatisticsContribution {
        contributions
            .iter()
            .copied()
            .find(|contribution| contribution.reference == reference)
            .expect("fixture contribution")
    }

    #[test]
    fn duplicate_positions_collapse_retries_and_bounded_total_time() {
        let a = reference(1);
        let b = reference(2);
        let run_items = run_items(vec![a, b, a]);
        let results = vec![result(1.0, 2.0), result(1.0, 4.0), result(2.0, 2.0)];
        let attempts = vec![
            attempt(1, a, 0, Some(1_500)),
            attempt(2, a, 0, Some(2_500)),
            attempt(3, b, 1, Some(1_000)),
            attempt(4, a, 2, Some(100_000_000)),
            attempt(5, a, 2, None),
        ];

        let derived = derive_statistics_contributions(&run_items, &results, &attempts)
            .expect("valid collapsed contributions");
        assert_eq!(derived.len(), 2);
        let a_observation = contribution(&derived, a).observation;
        assert_eq!(a_observation.normalized_score(), 0.75);
        assert_eq!(a_observation.rest_score(), Some(0.25));
        assert_eq!(a_observation.attempts(), 3);
        assert_eq!(a_observation.duration_seconds(), MAX_DURATION_SECONDS);
        let b_observation = contribution(&derived, b).observation;
        assert_eq!(b_observation.normalized_score(), 0.25);
        assert_eq!(b_observation.rest_score(), Some(0.75));
        assert_eq!(b_observation.attempts(), 1);
        assert_eq!(b_observation.duration_seconds(), 1);
        assert_eq!(
            derive_statistics_contributions(&run_items, &results, &attempts)
                .expect("deterministic replay"),
            derived,
        );
    }

    #[test]
    fn rest_score_excludes_the_current_group_without_subtractive_cancellation() {
        let a = reference(10);
        let b = reference(11);
        let run_items = run_items(vec![a, b, a]);
        let results = vec![
            result(2.5e307, 5.0e307),
            result(0.0, 1.0),
            result(2.5e307, 5.0e307),
        ];
        let attempts = vec![
            attempt(10, a, 0, Some(1)),
            attempt(11, b, 1, Some(1)),
            attempt(12, a, 2, Some(1)),
        ];

        let derived = derive_statistics_contributions(&run_items, &results, &attempts)
            .expect("tiny rest group remains representable");
        assert_eq!(
            contribution(&derived, a).observation.rest_score(),
            Some(0.0)
        );
        assert_eq!(
            contribution(&derived, b).observation.rest_score(),
            Some(0.5)
        );
    }

    #[test]
    fn one_version_has_no_rest_score_and_canonicalizes_negative_zero() {
        let a = reference(20);
        let run_items = run_items(vec![a]);
        let derived = derive_statistics_contributions(
            &run_items,
            &[result(-0.0, 1.0)],
            &[attempt(20, a, 0, Some(1_001))],
        )
        .expect("one-question observation");
        let observation = derived[0].observation;
        assert_eq!(observation.normalized_score().to_bits(), 0.0_f64.to_bits());
        assert_eq!(observation.rest_score(), None);
        assert_eq!(observation.duration_seconds(), 2);
    }

    #[test]
    fn malformed_results_identities_and_timestamps_are_refused() {
        let a = reference(30);
        let b = reference(31);
        let run_items = run_items(vec![a]);
        assert!(matches!(
            derive_statistics_contributions(
                &run_items,
                &[result(2.0, 1.0)],
                &[attempt(30, a, 0, Some(1))]
            ),
            Err(StoreError::InvalidRecord(_))
        ));

        let mut wrong_identity = attempt(31, a, 0, Some(1));
        wrong_identity.problem = b.problem;
        assert!(matches!(
            derive_statistics_contributions(&run_items, &[result(1.0, 1.0)], &[wrong_identity]),
            Err(StoreError::InvalidRecord(_))
        ));

        assert!(matches!(
            derive_statistics_contributions(
                &run_items,
                &[result(1.0, 1.0)],
                &[attempt(32, a, 0, Some(-1))]
            ),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}
