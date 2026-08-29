//! Opaque native-key cursor for the summary-only gradebook projection.
//!
//! The transport sees one URL-safe token. Storage decodes it before querying
//! so PostgreSQL can compare the assignment/enrollment UUID tuple without a
//! text expression or a sort that defeats the page indexes.

use base64::Engine as _;
use uuid::Uuid;

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
use question_model::{AssignmentReference, CourseMembershipReference};

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
use crate::{CourseGradeSchemeRevision, GradebookFilter, RosterRevision};
use crate::{Cursor, StoreError};

const ENCODED_LENGTH: usize = 43;
const RAW_LENGTH: usize = 32;
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
const CALCULATED_RAW_LENGTH: usize = 25;
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
const CALCULATED_ENCODED_LENGTH: usize = 34;
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
const SELECTION_RAW_LENGTH: usize = 29;
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
const SELECTION_ENCODED_LENGTH: usize = 39;
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
const RUN_CHOICES_RAW_LENGTH: usize = 36;
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
const RUN_CHOICES_ENCODED_LENGTH: usize = 48;

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
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CalculatedGradebookCursor {
    pub(crate) scheme_revision: CourseGradeSchemeRevision,
    pub(crate) roster_revision: RosterRevision,
    pub(crate) filter: GradebookFilter,
    pub(crate) last_membership: CourseMembershipReference,
}

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
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

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn encode_filter(filter: GradebookFilter) -> (u8, u32) {
    match filter {
        GradebookFilter::All => (0, 0),
        GradebookFilter::Assignment(reference) => (1, reference.number()),
        GradebookFilter::Student(reference) => (2, reference.number()),
    }
}

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
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

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn invalid_calculated_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid calculated gradebook cursor".to_string())
}

/// Structural continuation for a bounded Gradebook Student selection.
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GradebookSelectionCursor {
    pub(crate) scheme_revision: CourseGradeSchemeRevision,
    pub(crate) roster_revision: RosterRevision,
    pub(crate) assignment: AssignmentReference,
    pub(crate) operation: Option<question_model::GradingOperationReference>,
    pub(crate) last_membership: CourseMembershipReference,
}

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
impl GradebookSelectionCursor {
    pub(crate) fn encode(self) -> Cursor {
        let mut bytes = [0_u8; SELECTION_RAW_LENGTH];
        bytes[..8].copy_from_slice(&self.scheme_revision.value().to_be_bytes());
        bytes[8..16].copy_from_slice(&self.roster_revision.value().to_be_bytes());
        bytes[16..20].copy_from_slice(&self.assignment.number().to_be_bytes());
        bytes[20..24].copy_from_slice(
            &self
                .operation
                .map_or(0, |value| value.number())
                .to_be_bytes(),
        );
        bytes[24..28].copy_from_slice(&self.last_membership.number().to_be_bytes());
        bytes[28] = u8::from(self.operation.is_some());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn decode(cursor: &Cursor) -> Result<Self, StoreError> {
        let bytes = decode_fixed_cursor(cursor, SELECTION_ENCODED_LENGTH, SELECTION_RAW_LENGTH)?;
        let scheme_revision = decode_scheme_revision(&bytes[..8])?;
        let roster_revision = decode_roster_revision(&bytes[8..16])?;
        let assignment = decode_assignment_reference(&bytes[16..20])?;
        let operation_number =
            u32::from_be_bytes(bytes[20..24].try_into().expect("fixed operation"));
        let operation = match (bytes[28], operation_number) {
            (0, 0) => None,
            (1, number) => question_model::GradingOperationReference::new(u64::from(number))
                .map(Some)
                .ok_or_else(invalid_selection_cursor)?,
            _ => return Err(invalid_selection_cursor()),
        };
        let last_membership = decode_membership_reference(&bytes[24..28])?;
        Ok(Self {
            scheme_revision,
            roster_revision,
            assignment,
            operation,
            last_membership,
        })
    }
}

/// Structural continuation for the bounded submitted-run chooser.
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubmittedRunChoicesCursor {
    pub(crate) roster_revision: RosterRevision,
    pub(crate) membership: CourseMembershipReference,
    pub(crate) assignment: AssignmentReference,
    pub(crate) operation: Option<question_model::GradingOperationReference>,
    pub(crate) submitted_at_millis: i64,
    pub(crate) last_run: question_model::RunReference,
}

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
impl SubmittedRunChoicesCursor {
    pub(crate) fn encode(self) -> Cursor {
        let mut bytes = [0_u8; RUN_CHOICES_RAW_LENGTH];
        bytes[..8].copy_from_slice(&self.roster_revision.value().to_be_bytes());
        bytes[8..12].copy_from_slice(&self.membership.number().to_be_bytes());
        bytes[12..16].copy_from_slice(&self.assignment.number().to_be_bytes());
        bytes[16..20].copy_from_slice(
            &self
                .operation
                .map_or(0, |value| value.number())
                .to_be_bytes(),
        );
        bytes[20..28].copy_from_slice(&self.submitted_at_millis.to_be_bytes());
        bytes[28..32].copy_from_slice(&self.last_run.number().to_be_bytes());
        bytes[32] = u8::from(self.operation.is_some());
        Cursor::from_stable_key(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn decode(cursor: &Cursor) -> Result<Self, StoreError> {
        let bytes =
            decode_fixed_cursor(cursor, RUN_CHOICES_ENCODED_LENGTH, RUN_CHOICES_RAW_LENGTH)?;
        if bytes[33..].iter().any(|value| *value != 0) {
            return Err(invalid_run_choices_cursor());
        }
        let operation_number =
            u32::from_be_bytes(bytes[16..20].try_into().expect("fixed operation"));
        let operation = match (bytes[32], operation_number) {
            (0, 0) => None,
            (1, number) => question_model::GradingOperationReference::new(u64::from(number))
                .map(Some)
                .ok_or_else(invalid_run_choices_cursor)?,
            _ => return Err(invalid_run_choices_cursor()),
        };
        let last_run = question_model::RunReference::new(u64::from(u32::from_be_bytes(
            bytes[28..32].try_into().expect("fixed run"),
        )))
        .ok_or_else(invalid_run_choices_cursor)?;
        Ok(Self {
            roster_revision: decode_roster_revision(&bytes[..8])
                .map_err(|_| invalid_run_choices_cursor())?,
            membership: decode_membership_reference(&bytes[8..12])
                .map_err(|_| invalid_run_choices_cursor())?,
            assignment: decode_assignment_reference(&bytes[12..16])
                .map_err(|_| invalid_run_choices_cursor())?,
            operation,
            submitted_at_millis: i64::from_be_bytes(
                bytes[20..28].try_into().expect("fixed timestamp"),
            ),
            last_run,
        })
    }
}

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn decode_fixed_cursor(
    cursor: &Cursor,
    encoded_length: usize,
    raw_length: usize,
) -> Result<Vec<u8>, StoreError> {
    if cursor.as_str().len() != encoded_length {
        return Err(invalid_selection_cursor());
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor.as_str())
        .map_err(|_| invalid_selection_cursor())?;
    if bytes.len() != raw_length {
        return Err(invalid_selection_cursor());
    }
    Ok(bytes)
}

#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn decode_scheme_revision(bytes: &[u8]) -> Result<CourseGradeSchemeRevision, StoreError> {
    CourseGradeSchemeRevision::from_u64(u64::from_be_bytes(
        bytes.try_into().expect("fixed revision"),
    ))
    .map_err(|_| invalid_selection_cursor())
}
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn decode_roster_revision(bytes: &[u8]) -> Result<RosterRevision, StoreError> {
    RosterRevision::from_stored(
        i64::try_from(u64::from_be_bytes(
            bytes.try_into().expect("fixed revision"),
        ))
        .map_err(|_| invalid_selection_cursor())?,
    )
    .map_err(|_| invalid_selection_cursor())
}
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn decode_assignment_reference(bytes: &[u8]) -> Result<AssignmentReference, StoreError> {
    AssignmentReference::new(u64::from(u32::from_be_bytes(
        bytes.try_into().expect("fixed assignment"),
    )))
    .ok_or_else(invalid_selection_cursor)
}
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn decode_membership_reference(bytes: &[u8]) -> Result<CourseMembershipReference, StoreError> {
    CourseMembershipReference::new(u64::from(u32::from_be_bytes(
        bytes.try_into().expect("fixed membership"),
    )))
    .ok_or_else(invalid_selection_cursor)
}
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn invalid_selection_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid gradebook selection cursor".to_string())
}
#[cfg(any(test, feature = "test-support", feature = "postgres"))]
fn invalid_run_choices_cursor() -> StoreError {
    StoreError::InvalidRecord("invalid submitted run choices cursor".to_string())
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

    #[cfg(feature = "test-support")]
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

    #[cfg(feature = "test-support")]
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
