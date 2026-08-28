//! Opaque native-key cursor for the summary-only gradebook projection.
//!
//! The transport sees one URL-safe token. Storage decodes it before querying
//! so PostgreSQL can compare the assignment/enrollment UUID tuple without a
//! text expression or a sort that defeats the page indexes.

use base64::Engine as _;
use uuid::Uuid;

use question_model::{AssignmentReference, CourseMembershipReference};

use crate::{CourseGradeSchemeRevision, Cursor, GradebookFilter, RosterRevision, StoreError};

const ENCODED_LENGTH: usize = 43;
const RAW_LENGTH: usize = 32;
const CALCULATED_RAW_LENGTH: usize = 25;
const CALCULATED_ENCODED_LENGTH: usize = 34;

/// Native UUID tuple used by the gradebook page order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GradebookCursor {
    pub(crate) assignment: Uuid,
    pub(crate) enrollment: Uuid,
}

impl GradebookCursor {
    /// Creates an opaque transport cursor from the final native tuple of a page.
    pub(crate) fn encode(self) -> Cursor {
        let mut bytes = [0_u8; RAW_LENGTH];
        bytes[..16].copy_from_slice(self.assignment.as_bytes());
        bytes[16..].copy_from_slice(self.enrollment.as_bytes());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    /// Decodes the exact fixed-width cursor accepted by this query path.
    pub(crate) fn decode(cursor: &Cursor) -> Result<Self, StoreError> {
        let token = cursor.as_str();
        if token.len() != ENCODED_LENGTH {
            return Err(invalid_cursor());
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| invalid_cursor())?;
        if bytes.len() != RAW_LENGTH {
            return Err(invalid_cursor());
        }
        let assignment = Uuid::from_slice(&bytes[..16]).map_err(|_| invalid_cursor())?;
        let enrollment = Uuid::from_slice(&bytes[16..]).map_err(|_| invalid_cursor())?;
        Ok(Self {
            assignment,
            enrollment,
        })
    }
}

fn invalid_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid gradebook cursor".to_string())
}

/// Structural continuation for the roster-first calculated Gradebook.
///
/// It contains only public structural values and is checked against the
/// current scheme, roster, and normalized filter before a later page is read.
/// The Store resolves those values under Instructor authority, so a changed or
/// forged continuation cannot widen the accessible record set (ASVS V2.2.1-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CalculatedGradebookCursor {
    pub(crate) scheme_revision: CourseGradeSchemeRevision,
    pub(crate) roster_revision: RosterRevision,
    pub(crate) filter: GradebookFilter,
    pub(crate) last_membership: CourseMembershipReference,
}

impl CalculatedGradebookCursor {
    pub(crate) fn encode(self) -> Cursor {
        let mut bytes = [0_u8; CALCULATED_RAW_LENGTH];
        bytes[..8].copy_from_slice(&self.scheme_revision.value().to_be_bytes());
        bytes[8..16].copy_from_slice(&self.roster_revision.value().to_be_bytes());
        let (kind, reference) = encode_filter(self.filter);
        bytes[16] = kind;
        bytes[17..21].copy_from_slice(&reference.to_be_bytes());
        bytes[21..25].copy_from_slice(&self.last_membership.number().to_be_bytes());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn decode(cursor: &Cursor) -> Result<Self, StoreError> {
        if cursor.as_str().len() != CALCULATED_ENCODED_LENGTH {
            return Err(invalid_calculated_cursor());
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(cursor.as_str())
            .map_err(|_| invalid_calculated_cursor())?;
        if bytes.len() != CALCULATED_RAW_LENGTH {
            return Err(invalid_calculated_cursor());
        }
        let scheme_revision = CourseGradeSchemeRevision::from_u64(u64::from_be_bytes(
            bytes[..8].try_into().expect("fixed-width scheme revision"),
        ))
        .map_err(|_| invalid_calculated_cursor())?;
        let roster_revision = RosterRevision::from_stored(
            i64::try_from(u64::from_be_bytes(
                bytes[8..16]
                    .try_into()
                    .expect("fixed-width roster revision"),
            ))
            .map_err(|_| invalid_calculated_cursor())?,
        )
        .map_err(|_| invalid_calculated_cursor())?;
        let reference = u32::from_be_bytes(
            bytes[17..21]
                .try_into()
                .expect("fixed-width filter reference"),
        );
        let filter = decode_filter(bytes[16], reference)?;
        let last_membership = CourseMembershipReference::new(u64::from(u32::from_be_bytes(
            bytes[21..25]
                .try_into()
                .expect("fixed-width membership reference"),
        )))
        .ok_or_else(invalid_calculated_cursor)?;
        Ok(Self {
            scheme_revision,
            roster_revision,
            filter,
            last_membership,
        })
    }
}

fn encode_filter(filter: GradebookFilter) -> (u8, u32) {
    match filter {
        GradebookFilter::All => (0, 0),
        GradebookFilter::Assignment(reference) => (1, reference.number()),
        GradebookFilter::Student(reference) => (2, reference.number()),
    }
}

fn decode_filter(kind: u8, reference: u32) -> Result<GradebookFilter, StoreError> {
    match (kind, reference) {
        (0, 0) => Ok(GradebookFilter::All),
        (1, _) => AssignmentReference::new(u64::from(reference))
            .map(GradebookFilter::Assignment)
            .ok_or_else(invalid_calculated_cursor),
        (2, _) => CourseMembershipReference::new(u64::from(reference))
            .map(GradebookFilter::Student)
            .ok_or_else(invalid_calculated_cursor),
        _ => Err(invalid_calculated_cursor()),
    }
}

fn invalid_calculated_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid calculated gradebook cursor".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips_without_exposing_uuid_boundaries() {
        let original = GradebookCursor {
            assignment: Uuid::from_u128(1),
            enrollment: Uuid::from_u128(2),
        };
        let encoded = original.encode();

        assert_eq!(encoded.as_str().len(), ENCODED_LENGTH);
        assert!(!encoded.as_str().contains('/'));
        assert_eq!(GradebookCursor::decode(&encoded), Ok(original));
    }

    #[test]
    fn malformed_cursor_is_rejected_before_a_query_is_built() {
        assert!(matches!(
            Cursor::parse(String::new()),
            Err(crate::PaginationError::EmptyCursor)
        ));
        for token in [
            "not-a-gradebook-cursor",
            "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
        ] {
            let cursor = Cursor::parse(token.to_string()).expect("nonempty generic cursor");
            assert!(matches!(
                GradebookCursor::decode(&cursor),
                Err(StoreError::InvalidRecord(message)) if message == "invalid gradebook cursor"
            ));
        }
    }

    #[test]
    fn calculated_cursor_binds_structural_revisions_filter_and_roster_position() {
        let original = CalculatedGradebookCursor {
            scheme_revision: CourseGradeSchemeRevision::from_u64(7).expect("revision"),
            roster_revision: RosterRevision::from_stored(11).expect("revision"),
            filter: GradebookFilter::Student(
                CourseMembershipReference::new(13).expect("membership reference"),
            ),
            last_membership: CourseMembershipReference::new(17).expect("membership reference"),
        };
        let encoded = original.encode();

        assert_eq!(encoded.as_str().len(), CALCULATED_ENCODED_LENGTH);
        assert!(!encoded.as_str().contains('/'));
        assert_eq!(CalculatedGradebookCursor::decode(&encoded), Ok(original));
    }

    #[test]
    fn calculated_cursor_rejects_unknown_filter_and_nonpositive_membership() {
        let mut bytes = [0_u8; CALCULATED_RAW_LENGTH];
        bytes[..8].copy_from_slice(&1_u64.to_be_bytes());
        bytes[8..16].copy_from_slice(&1_u64.to_be_bytes());
        bytes[16] = 9;
        bytes[21..25].copy_from_slice(&1_u32.to_be_bytes());
        let cursor =
            Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes));
        assert!(matches!(
            CalculatedGradebookCursor::decode(&cursor),
            Err(StoreError::InvalidRecord(message)) if message == "invalid calculated gradebook cursor"
        ));

        bytes[16] = 0;
        bytes[21..25].copy_from_slice(&0_u32.to_be_bytes());
        let cursor =
            Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes));
        assert!(matches!(
            CalculatedGradebookCursor::decode(&cursor),
            Err(StoreError::InvalidRecord(message)) if message == "invalid calculated gradebook cursor"
        ));
    }
}
