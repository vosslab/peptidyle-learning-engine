//! Receipt-led repair contracts for B2-owned derived and index projections.

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use serde::de;

use super::{CurriculumAdoptionReceiptBinding, bounded::deserialize_repaired_projections};
use crate::{AssignmentReference, MAX_ASSIGNMENT_ORDERED_ENTRIES};

/// Closed server-owned request to reconcile one completed B2 operation.
///
/// The opaque receipt binding locates completed evidence; Store implementations
/// reauthorize the current actor and require the matching immutable receipt,
/// baseline, and envelope before repairing any derived projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReconcileCurriculumAdoptionCommand {
    /// Public opaque binding returned with the completed B2 operation.
    pub receipt: CurriculumAdoptionReceiptBinding,
}

/// One B2-owned derived or current-index projection repaired from immutable evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CurriculumAdoptionRepairedProjection {
    /// Current assignment import baseline/envelope lookup for one destination assignment.
    AssignmentImportCurrent {
        /// Destination assignment whose current import lookup was rebuilt.
        assignment: AssignmentReference,
    },
}

/// Nonempty bounded set of B2-owned derived projections repaired atomically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<CurriculumAdoptionRepairedProjection>")]
pub struct CurriculumAdoptionRepairedProjections(Vec<CurriculumAdoptionRepairedProjection>);

impl<'de> Deserialize<'de> for CurriculumAdoptionRepairedProjections {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let projections = deserialize_repaired_projections(deserializer)?;
        Self::new(projections).map_err(de::Error::custom)
    }
}

impl CurriculumAdoptionRepairedProjections {
    /// Validates the distinct assignment import projections repaired by one receipt replay.
    pub fn new(
        projections: Vec<CurriculumAdoptionRepairedProjection>,
    ) -> Result<Self, CurriculumAdoptionRepairedProjectionsError> {
        if projections.is_empty() || projections.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(CurriculumAdoptionRepairedProjectionsError);
        }
        let mut assignments = BTreeSet::new();
        if projections.iter().any(|projection| match projection {
            CurriculumAdoptionRepairedProjection::AssignmentImportCurrent { assignment } => {
                !assignments.insert(*assignment)
            }
        }) {
            return Err(CurriculumAdoptionRepairedProjectionsError);
        }
        Ok(Self(projections))
    }

    /// Returns each repaired B2-owned derived projection in stable operation order.
    pub fn as_slice(&self) -> &[CurriculumAdoptionRepairedProjection] {
        &self.0
    }
}

impl TryFrom<Vec<CurriculumAdoptionRepairedProjection>> for CurriculumAdoptionRepairedProjections {
    type Error = CurriculumAdoptionRepairedProjectionsError;

    fn try_from(value: Vec<CurriculumAdoptionRepairedProjection>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CurriculumAdoptionRepairedProjections> for Vec<CurriculumAdoptionRepairedProjection> {
    fn from(value: CurriculumAdoptionRepairedProjections) -> Self {
        value.0
    }
}

/// Reconciliation named no projection, too many projections, or one assignment twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumAdoptionRepairedProjectionsError;

impl std::fmt::Display for CurriculumAdoptionRepairedProjectionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("repaired curriculum adoption projections are invalid")
    }
}

impl std::error::Error for CurriculumAdoptionRepairedProjectionsError {}

/// Closed reconciliation result for one completed B2 receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CurriculumAdoptionReconciliationResult {
    /// Every B2-owned derived and current-index projection already matches immutable evidence.
    AlreadyConsistent {
        /// Completed operation selected by the opaque receipt binding.
        receipt: CurriculumAdoptionReceiptBinding,
    },
    /// The Store rebuilt one derived/current-index projection from immutable evidence.
    Repaired {
        /// Completed operation selected by the opaque receipt binding.
        receipt: CurriculumAdoptionReceiptBinding,
        /// Derived/current-index projections rebuilt without changing authoritative records.
        projections: CurriculumAdoptionRepairedProjections,
    },
}
