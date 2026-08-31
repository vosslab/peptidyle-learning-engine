//! Authenticated Assignment Attempt start contract.
//!
//! The service prepares exact released Question pins and Question Pool
//! selections from server-held records. This boundary mints durable record
//! identities and persists the complete start atomically; browser input never
//! supplies an Attempt number, released revision, score, or private table row.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    AssignmentAttemptId, AssignmentEntryId, AssignmentId, QuestionPoolCandidateId,
    QuestionPoolSelectedCandidate, QuestionPoolSelectionId, QuestionRevisionReference,
    StudentRecordId,
};

use crate::{SessionTokenHash, StoreError};

/// Exact server-selected candidates for one Question Pool Assignment Entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedQuestionPoolSelection {
    /// Exact Question Pool Assignment Entry in the released Assignment Revision.
    pub question_pool_entry: AssignmentEntryId,
    /// Earlier same-Student Selection whose exact candidates are retained.
    pub reused_from_question_pool_selection: Option<QuestionPoolSelectionId>,
    /// Exact selected candidates in their frozen delivery order.
    pub selected_candidates: Vec<QuestionPoolSelectedCandidate>,
}

/// One exact Question to issue for a new Assignment Attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedIssuedQuestion {
    /// The released Assignment Entry pins one exact fixed Question Revision.
    FixedQuestion {
        /// Exact fixed Assignment Entry.
        assignment_entry: AssignmentEntryId,
        /// Exact pinned Question Revision.
        reference: QuestionRevisionReference,
    },
    /// One candidate selected from a released Question Pool Assignment Entry.
    QuestionPoolCandidate {
        /// Exact Question Pool Assignment Entry.
        assignment_entry: AssignmentEntryId,
        /// Position in [`AssignmentAttemptStart::question_pool_selections`].
        question_pool_selection_index: usize,
        /// Exact selected candidate in that Question Pool.
        question_pool_candidate: QuestionPoolCandidateId,
        /// Exact pinned Question Revision.
        reference: QuestionRevisionReference,
    },
}

/// Server-prepared input for starting or resuming one Assignment Attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentAttemptStart {
    /// Exact Student Record that will own the Attempt.
    pub student_record: StudentRecordId,
    /// Exact released Assignment to start.
    pub assignment: AssignmentId,
    /// One prepared Selection for each released Question Pool Assignment Entry.
    pub question_pool_selections: Vec<PreparedQuestionPoolSelection>,
    /// Fixed and pooled Questions in their intended issued order.
    pub issued_questions: Vec<PreparedIssuedQuestion>,
}

impl AssignmentAttemptStart {
    /// Refuses malformed server preparation before a database transaction starts.
    pub fn validate(&self) -> Result<(), StoreError> {
        let mut entries = BTreeSet::new();
        for selection in &self.question_pool_selections {
            if selection.selected_candidates.is_empty() {
                return Err(StoreError::InvalidRecord(
                    "a Question Pool Selection must retain at least one candidate".to_string(),
                ));
            }
            if !entries.insert(selection.question_pool_entry.as_uuid()) {
                return Err(StoreError::InvalidRecord(
                    "an Assignment Attempt has at most one Question Pool Selection per Assignment Entry"
                        .to_string(),
                ));
            }
        }
        let mut issued_pool_candidates = BTreeSet::new();
        for question in &self.issued_questions {
            let PreparedIssuedQuestion::QuestionPoolCandidate {
                assignment_entry,
                question_pool_selection_index,
                question_pool_candidate,
                ..
            } = question
            else {
                continue;
            };
            let Some(selection) = self
                .question_pool_selections
                .get(*question_pool_selection_index)
            else {
                return Err(StoreError::InvalidRecord(
                    "a pooled Issued Question must name a prepared Question Pool Selection"
                        .to_string(),
                ));
            };
            if selection.question_pool_entry != *assignment_entry
                || !selection
                    .selected_candidates
                    .iter()
                    .any(|candidate| candidate.candidate == *question_pool_candidate)
            {
                return Err(StoreError::InvalidRecord(
                    "a pooled Issued Question must match its prepared Selection".to_string(),
                ));
            }
            if !issued_pool_candidates.insert((
                assignment_entry.as_uuid(),
                question_pool_candidate.as_uuid(),
            )) {
                return Err(StoreError::InvalidRecord(
                    "a selected Question Pool candidate may be issued once".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Result of atomically starting or resuming one Assignment Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentAttemptStartResult {
    /// Durable Assignment Attempt identity.
    pub assignment_attempt: AssignmentAttemptId,
    /// One-based sequence for this Student Record and Assignment.
    pub attempt_number: u32,
    /// Whether an existing unfinished Attempt was resumed without re-selection.
    pub resumed: bool,
}

/// Authenticated persistence boundary for starting or resuming Student work.
#[async_trait]
pub trait AssignmentAttemptStore: Send + Sync {
    /// Starts one authorized new Attempt or returns its existing unfinished Attempt.
    async fn start_assignment_attempt(
        &self,
        session_token_hash: SessionTokenHash,
        start: AssignmentAttemptStart,
    ) -> Result<AssignmentAttemptStartResult, StoreError>;
}

#[cfg(test)]
mod tests {
    use question_model::{QuestionId, QuestionRevisionNumber};
    use uuid::Uuid;

    use super::*;

    fn reference() -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: "123-4567".parse::<QuestionId>().expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
        }
    }

    #[test]
    fn prepared_pooled_question_must_match_its_server_prepared_selection() {
        let entry = AssignmentEntryId::from_uuid(Uuid::from_u128(2));
        let start = AssignmentAttemptStart {
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(1)),
            assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
            question_pool_selections: vec![PreparedQuestionPoolSelection {
                question_pool_entry: entry,
                reused_from_question_pool_selection: None,
                selected_candidates: vec![QuestionPoolSelectedCandidate {
                    candidate: QuestionPoolCandidateId::from_uuid(Uuid::from_u128(4)),
                    reference: reference(),
                }],
            }],
            issued_questions: vec![PreparedIssuedQuestion::QuestionPoolCandidate {
                assignment_entry: entry,
                question_pool_selection_index: 0,
                question_pool_candidate: QuestionPoolCandidateId::from_uuid(Uuid::from_u128(5)),
                reference: reference(),
            }],
        };

        assert!(matches!(
            start.validate(),
            Err(StoreError::InvalidRecord(_))
        ));
    }
}
