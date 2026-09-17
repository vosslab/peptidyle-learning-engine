//! Authenticated Assessment Attempt start contract.
//!
//! The service prepares exact released Question pins and Question Pool
//! selections from server-held records. This boundary mints durable record
//! identities and persists the complete start atomically; browser input never
//! supplies an Attempt number, released revision, score, or private table row.

use std::collections::BTreeSet;

use async_trait::async_trait;
use question_model::{
    AssessmentAttemptId, AssessmentEntryId, AssessmentId, PoolRevisionMemberReference,
    QuestionBackend, QuestionPoolRevisionReference, QuestionPoolSelectedItem,
    QuestionRevisionReference, StudentRecordId,
};

use crate::{SessionTokenHash, StoreError};

/// Exact server-selected Question Pool Items for one Question Pool Assessment Entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedQuestionPoolSelection {
    /// Exact Question Pool Assessment Entry in the current released Assessment.
    pub question_pool_assessment_entry: AssessmentEntryId,
    /// Exact immutable Assessment-owned fork Pool Revision selected from.
    pub question_pool_revision: QuestionPoolRevisionReference,
    /// Exact selected Question Pool Items in their frozen delivery order.
    pub selected_items: Vec<QuestionPoolSelectedItem>,
}

/// One exact Question to issue for a new Assessment Attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedIssuedQuestion {
    /// The current released Assessment Entry pins one exact fixed Question Revision.
    FixedQuestion {
        /// Exact fixed Assessment Entry.
        assessment_entry: AssessmentEntryId,
        /// Exact pinned Question Revision.
        reference: QuestionRevisionReference,
        /// Authoritative backend of the pinned revision.
        backend: QuestionBackend,
    },
    /// One Question Pool Item selected from a current released Question Pool Assessment Entry.
    QuestionPoolItem {
        /// Exact Question Pool Assessment Entry.
        assessment_entry: AssessmentEntryId,
        /// Position in [`AssessmentAttemptStart::question_pool_selections`].
        question_pool_selection_index: usize,
        /// Exact selected immutable Pool Revision member.
        pool_revision_member: PoolRevisionMemberReference,
        /// Exact pinned Question Revision.
        reference: QuestionRevisionReference,
        /// Authoritative backend of the pinned revision.
        backend: QuestionBackend,
    },
}

/// Server-prepared input for starting or resuming one Assessment Attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentAttemptStart {
    /// Exact Student Record that will own the Attempt.
    pub student_record: StudentRecordId,
    /// Exact released Assessment to start.
    pub assessment: AssessmentId,
    /// One prepared Selection for each current released Question Pool Assessment Entry.
    pub question_pool_selections: Vec<PreparedQuestionPoolSelection>,
    /// Fixed and pooled Questions in their intended issued order.
    pub issued_questions: Vec<PreparedIssuedQuestion>,
}

impl AssessmentAttemptStart {
    /// Refuses malformed server preparation before a database transaction starts.
    pub fn validate(&self) -> Result<(), StoreError> {
        let mut entries = BTreeSet::new();
        for selection in &self.question_pool_selections {
            if selection.selected_items.is_empty() {
                return Err(StoreError::InvalidRecord(
                    "a Question Pool Selection must retain at least one Question Pool Item"
                        .to_string(),
                ));
            }
            if !entries.insert(selection.question_pool_assessment_entry.as_uuid()) {
                return Err(StoreError::InvalidRecord(
                    "an Assessment Attempt has at most one Question Pool Selection per Assessment Entry"
                        .to_string(),
                ));
            }
        }
        let mut issued_question_pool_items = BTreeSet::new();
        for question in &self.issued_questions {
            let backend = match question {
                PreparedIssuedQuestion::FixedQuestion { backend, .. }
                | PreparedIssuedQuestion::QuestionPoolItem { backend, .. } => backend,
            };
            if !backend.is_supported_for_production() {
                return Err(StoreError::InvalidRecord(
                    "Question Backend is unavailable for new Assessment work".to_string(),
                ));
            }
            let PreparedIssuedQuestion::QuestionPoolItem {
                assessment_entry,
                question_pool_selection_index,
                pool_revision_member,
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
            if selection.question_pool_assessment_entry != *assessment_entry
                || !selection
                    .selected_items
                    .iter()
                    .any(|item| item.pool_revision_member == *pool_revision_member)
            {
                return Err(StoreError::InvalidRecord(
                    "a pooled Issued Question must match its prepared Selection".to_string(),
                ));
            }
            if !issued_question_pool_items
                .insert((assessment_entry.as_uuid(), pool_revision_member.clone()))
            {
                return Err(StoreError::InvalidRecord(
                    "a selected Question Pool Item may be issued once".to_string(),
                ));
            }
        }
        for selection in &self.question_pool_selections {
            for selected_item in &selection.selected_items {
                let issued_count = self
                    .issued_questions
                    .iter()
                    .filter(|question| {
                        matches!(
                            question,
                            PreparedIssuedQuestion::QuestionPoolItem {
                                assessment_entry,
                                pool_revision_member,
                                reference,
                                ..
                            } if assessment_entry == &selection.question_pool_assessment_entry
                                && pool_revision_member == &selected_item.pool_revision_member
                                && reference == &selected_item.reference
                        )
                    })
                    .count();
                if issued_count != 1 {
                    return Err(StoreError::InvalidRecord(
                        "each selected Question Pool Item must produce exactly one matching Issued Question"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Result of atomically starting or resuming one Assessment Attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssessmentAttemptStartResult {
    /// Durable Assessment Attempt identity.
    pub assessment_attempt: AssessmentAttemptId,
    /// One-based sequence for this Student Record and Assessment.
    pub attempt_number: u32,
    /// Whether an existing unfinished Attempt was resumed without re-selection.
    pub resumed: bool,
}

/// Authenticated persistence boundary for starting or resuming Student work.
#[async_trait]
pub trait AssessmentAttemptStore: Send + Sync {
    /// Starts one authorized new Attempt or returns its existing unfinished Attempt.
    async fn start_assessment_attempt(
        &self,
        session_token_hash: SessionTokenHash,
        start: AssessmentAttemptStart,
    ) -> Result<AssessmentAttemptStartResult, StoreError>;
}

#[cfg(test)]
mod tests {
    use question_model::{
        QuestionId, QuestionPoolRevisionNumber, QuestionPoolRevisionReference,
        QuestionRevisionNumber,
    };
    use uuid::Uuid;

    use super::*;

    fn reference() -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: "1234-H567".parse::<QuestionId>().expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("positive revision"),
        }
    }

    fn pool_revision() -> QuestionPoolRevisionReference {
        QuestionPoolRevisionReference {
            question_pool_id: "7654-Z321".parse().expect("Pool ID"),
            revision_number: QuestionPoolRevisionNumber::new(1).expect("positive Pool Revision"),
        }
    }

    fn pool_revision_member(member_position: u32) -> PoolRevisionMemberReference {
        PoolRevisionMemberReference {
            question_pool_revision: pool_revision(),
            member_position,
        }
    }

    #[test]
    fn prepared_pooled_question_must_match_its_server_prepared_selection() {
        let entry = AssessmentEntryId::from_uuid(Uuid::from_u128(2));
        let start = AssessmentAttemptStart {
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(1)),
            assessment: AssessmentId::from_uuid(Uuid::from_u128(3)),
            question_pool_selections: vec![PreparedQuestionPoolSelection {
                question_pool_assessment_entry: entry,
                question_pool_revision: pool_revision(),
                selected_items: vec![QuestionPoolSelectedItem {
                    pool_revision_member: pool_revision_member(4),
                    reference: reference(),
                }],
            }],
            issued_questions: vec![PreparedIssuedQuestion::QuestionPoolItem {
                assessment_entry: entry,
                question_pool_selection_index: 0,
                pool_revision_member: pool_revision_member(5),
                reference: reference(),
                backend: QuestionBackend::Ple,
            }],
        };

        assert!(matches!(
            start.validate(),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn each_selected_question_pool_item_must_be_issued_once() {
        let entry = AssessmentEntryId::from_uuid(Uuid::from_u128(2));
        let start = AssessmentAttemptStart {
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(1)),
            assessment: AssessmentId::from_uuid(Uuid::from_u128(3)),
            question_pool_selections: vec![PreparedQuestionPoolSelection {
                question_pool_assessment_entry: entry,
                question_pool_revision: pool_revision(),
                selected_items: vec![
                    QuestionPoolSelectedItem {
                        pool_revision_member: pool_revision_member(4),
                        reference: reference(),
                    },
                    QuestionPoolSelectedItem {
                        pool_revision_member: pool_revision_member(5),
                        reference: reference(),
                    },
                ],
            }],
            issued_questions: vec![PreparedIssuedQuestion::QuestionPoolItem {
                assessment_entry: entry,
                question_pool_selection_index: 0,
                pool_revision_member: pool_revision_member(4),
                reference: reference(),
                backend: QuestionBackend::Ple,
            }],
        };

        assert!(matches!(
            start.validate(),
            Err(StoreError::InvalidRecord(message))
                if message == "each selected Question Pool Item must produce exactly one matching Issued Question"
        ));
    }

    #[test]
    fn deferred_backend_cannot_start_new_assessment_work() {
        let start = AssessmentAttemptStart {
            student_record: StudentRecordId::from_uuid(Uuid::from_u128(1)),
            assessment: AssessmentId::from_uuid(Uuid::from_u128(2)),
            question_pool_selections: Vec::new(),
            issued_questions: vec![PreparedIssuedQuestion::FixedQuestion {
                assessment_entry: AssessmentEntryId::from_uuid(Uuid::from_u128(3)),
                reference: reference(),
                backend: QuestionBackend::Imathas,
            }],
        };

        assert!(matches!(
            start.validate(),
            Err(StoreError::InvalidRecord(message))
                if message == "Question Backend is unavailable for new Assessment work"
        ));
    }
}
