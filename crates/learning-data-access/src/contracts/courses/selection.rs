//! Deterministic item-pool selection for assignment runs and previews.

use super::*;
use question_model::AssignmentSelectionCandidate;

type RankedCandidate<'a> = (u64, &'a AssignmentSelectionCandidate);

/// Freezes current fixed items and deterministic group selections for one new run.
///
/// A successful draw is persisted with the new run. Resume paths read those
/// immutable rows and never invoke this algorithm again.
pub(crate) fn select_assignment_run_items(
    assignment: &AssignmentRecord,
    run: &AssignmentRun,
) -> Result<Vec<AssignmentRunItem>, StoreError> {
    enum Source<'a> {
        Fixed(&'a AssignmentItem),
        Group(&'a AssignmentSelectionGroup),
    }
    let mut sources = assignment
        .active_items()
        .map(|item| (item.position, Source::Fixed(item)))
        .chain(
            assignment
                .selection_groups
                .iter()
                .map(|group| (group.position, Source::Group(group))),
        )
        .collect::<Vec<_>>();
    sources.sort_by_key(|(position, _)| *position);
    let mut selected = Vec::new();
    for (source_position, source) in sources {
        match source {
            Source::Fixed(item) => {
                selected.push((item.id, source_position, item.reference, None, None))
            }
            Source::Group(group) => {
                let basis = assignment
                    .policies
                    .variation
                    .pool_draw_basis(assignment.id, run, group.id)
                    .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
                let (seed, candidates) = select_assignment_group_candidates(group, basis)?;
                for candidate in candidates {
                    selected.push((
                        candidate.id,
                        source_position,
                        candidate.reference,
                        Some(group.id),
                        Some(seed),
                    ));
                }
            }
        }
    }
    selected
        .into_iter()
        .enumerate()
        .map(
            |(
                issued_position,
                (assignment_item, source_position, reference, selection_group, selection_seed),
            )| {
                Ok(AssignmentRunItem {
                    run: run.id,
                    assignment_item,
                    source_position,
                    issued_position: u32::try_from(issued_position).map_err(|_| {
                        StoreError::InvalidRecord("too many selected run items".to_string())
                    })?,
                    reference,
                    selection_group,
                    selection_seed,
                })
            },
        )
        .collect()
}

/// Selects one group's candidates in final delivery order without creating
/// learner activity. The preview path reuses this exact pure operation.
pub(crate) fn select_assignment_group_candidates(
    group: &AssignmentSelectionGroup,
    basis: PoolDrawBasis,
) -> Result<(u64, Vec<&AssignmentSelectionCandidate>), StoreError> {
    match group.algorithm {
        PoolDrawAlgorithm::V1 => {
            let (seed, ranked) = select_pool_draw_v1(group, basis)?;
            Ok((
                seed,
                ranked.into_iter().map(|(_, candidate)| candidate).collect(),
            ))
        }
    }
}

fn select_pool_draw_v1(
    group: &AssignmentSelectionGroup,
    basis: PoolDrawBasis,
) -> Result<(u64, Vec<RankedCandidate<'_>>), StoreError> {
    let seed = assignment_selection_seed_v1(basis, group.algorithm);
    let mut candidates = group
        .candidates
        .iter()
        .filter(|candidate| candidate.delivery_state == AssignmentDeliveryState::Active)
        .map(|candidate| (assignment_selection_rank_v1(seed, candidate.id), candidate))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(rank, candidate)| (*rank, candidate.id));
    candidates.truncate(
        usize::try_from(group.draw_count).map_err(|_| {
            StoreError::InvalidRecord("selection draw count is too large".to_string())
        })?,
    );
    if group.ordering == SelectionOrdering::CandidateOrder {
        candidates.sort_by_key(|(_, candidate)| (candidate.position, candidate.id));
    }
    Ok((seed, candidates))
}

fn assignment_selection_seed_v1(basis: PoolDrawBasis, algorithm: PoolDrawAlgorithm) -> u64 {
    let mut bytes = Vec::with_capacity(90);
    bytes.extend_from_slice(b"ple-pool-draw-v1\\0");
    bytes.extend_from_slice(&algorithm.storage_version().to_be_bytes());
    match basis {
        PoolDrawBasis::StableEnrollment {
            enrollment,
            assignment,
            group,
        } => {
            bytes.extend_from_slice(b"stable-enrollment\\0");
            bytes.extend_from_slice(enrollment.as_uuid().as_bytes());
            bytes.extend_from_slice(assignment.as_uuid().as_bytes());
            bytes.extend_from_slice(group.as_uuid().as_bytes());
        }
        PoolDrawBasis::RegeneratedRun {
            run,
            assignment,
            group,
        } => {
            bytes.extend_from_slice(b"regenerated-run\\0");
            bytes.extend_from_slice(run.as_uuid().as_bytes());
            bytes.extend_from_slice(assignment.as_uuid().as_bytes());
            bytes.extend_from_slice(group.as_uuid().as_bytes());
        }
        PoolDrawBasis::Preview {
            assignment,
            group,
            nonce,
        } => {
            bytes.extend_from_slice(b"preview\\0");
            bytes.extend_from_slice(assignment.as_uuid().as_bytes());
            bytes.extend_from_slice(group.as_uuid().as_bytes());
            bytes.extend_from_slice(&nonce.as_bytes());
        }
    }
    let digest = Sha256Digest::compute(&bytes);
    let mut seed = [0_u8; 8];
    seed.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_be_bytes(seed) & 9_007_199_254_740_991
}

fn assignment_selection_rank_v1(seed: u64, candidate: AssignmentItemId) -> u64 {
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&seed.to_be_bytes());
    bytes.extend_from_slice(candidate.as_uuid().as_bytes());
    let digest = Sha256Digest::compute(&bytes);
    let mut rank = [0_u8; 8];
    rank.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_be_bytes(rank)
}
