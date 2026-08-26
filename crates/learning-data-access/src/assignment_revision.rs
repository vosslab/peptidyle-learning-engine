//! Persistence adapters for the canonical assignment revision domain value.

use question_model::AssignmentRevision;

use crate::StoreError;

/// Rebuilds one canonical assignment revision from a PostgreSQL `BIGINT`.
#[cfg(any(feature = "postgres", test))]
pub(crate) fn assignment_revision_from_stored(
    value: i64,
) -> Result<AssignmentRevision, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(AssignmentRevision::new)
        .ok_or_else(|| StoreError::Unavailable("stored assignment revision is invalid".into()))
}

/// Converts the validated domain revision to the PostgreSQL `BIGINT` representation.
#[cfg(any(feature = "postgres", test))]
pub(crate) fn assignment_revision_to_stored(value: AssignmentRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value()).map_err(|_| {
        StoreError::InvalidRecord("assignment revision exceeds PostgreSQL BIGINT".into())
    })
}

/// Advances a revision and maps exhaustion to an actionable persistence boundary failure.
pub(crate) fn assignment_revision_checked_next(
    value: AssignmentRevision,
) -> Result<AssignmentRevision, StoreError> {
    value
        .checked_next()
        .ok_or_else(|| StoreError::Unavailable("assignment revision limit reached".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_revisions_round_trip_at_the_postgres_boundary() {
        let revision = AssignmentRevision::new(42).expect("valid revision");
        assert_eq!(assignment_revision_to_stored(revision), Ok(42));
        assert_eq!(assignment_revision_from_stored(42), Ok(revision));
    }

    #[test]
    fn stored_revisions_reject_nonpositive_values() {
        let expected = StoreError::Unavailable("stored assignment revision is invalid".into());
        assert_eq!(assignment_revision_from_stored(0), Err(expected.clone()));
        assert_eq!(assignment_revision_from_stored(-1), Err(expected));
    }

    #[test]
    fn revision_limit_is_an_actionable_persistence_boundary_failure() {
        let maximum = AssignmentRevision::new(i64::MAX as u64).expect("maximum revision");
        assert_eq!(
            assignment_revision_checked_next(maximum),
            Err(StoreError::Unavailable(
                "assignment revision limit reached".into()
            ))
        );
    }
}
