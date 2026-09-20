//! Browser-safe contracts for the no-write Instructor Student View.
//!
//! The initial response is a bounded manifest rather than an eager rendering
//! of every Question. A separate one-Question read reauthorizes the current
//! Assessment Edit Number and reuses the ordinary answer-free Student Question
//! presentation contract.

use std::num::NonZeroU32;

use serde::Serialize;

use crate::{
    AccountTimeZone, AssessmentEditNumber, AssessmentInstructions, AssessmentStatus,
    AssessmentTitle, LateWorkRule, QuestionRevisionTuple, Timestamp,
};

/// Answer-free, non-mutating Instructor Student View manifest for one current Assessment.
///
/// ASVS 8.1.1, 8.1.2, and 8.3.1: the trusted server admits this projection only
/// through the authenticated Instructor's direct Course relationship. The DTO
/// exposes neither a write operation nor Student-specific fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorStudentView {
    /// Exact current authored Assessment state used for this projection.
    pub assessment_edit_number: AssessmentEditNumber,
    /// Current stable Assessment lifecycle.
    pub status: AssessmentStatus,
    /// Student-facing Assessment title.
    pub title: AssessmentTitle,
    /// Student-facing instructions.
    pub instructions: AssessmentInstructions,
    /// Authenticated Instructor's IANA zone for presenting delivery facts.
    pub display_time_zone: AccountTimeZone,
    /// Server-derived base delivery facts, without Student progress or actions.
    pub delivery: InstructorStudentViewDelivery,
    /// Current entries in authored order, preserving unavailable positions.
    pub entries: Vec<InstructorStudentViewEntry>,
}

/// Instructor-base delivery facts shown as Student-facing Assessment policy.
///
/// These facts describe Assessment policy, never a particular Student's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorStudentViewDelivery {
    #[serde(rename = "available_at")]
    pub available_at: Option<Timestamp>,
    #[serde(rename = "due_at")]
    pub due_at: Option<Timestamp>,
    #[serde(rename = "closes_at")]
    pub closes_at: Option<Timestamp>,
    #[serde(rename = "assessment_attempt_time_limit_seconds")]
    pub assessment_attempt_time_limit_seconds: Option<u32>,
    #[serde(rename = "attempt_limit")]
    pub attempt_limit: Option<u32>,
    #[serde(rename = "late_work_rule")]
    pub late_work_rule: LateWorkRule,
}

/// One current Assessment Entry in the ordered Student View manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InstructorStudentViewEntry {
    /// One fixed Question or the selected Questions from one Question Pool.
    Presented {
        /// Zero-based position in the current authored Assessment Entry order.
        authored_position: u32,
        /// Presented Questions in their projected preview order.
        questions: Vec<InstructorStudentViewQuestion>,
    },
    /// One current authored entry excluded from Student delivery.
    NotShown {
        /// Zero-based position in the current authored Assessment Entry order.
        authored_position: u32,
        reason: InstructorStudentViewNotShownReason,
    },
}

/// One presented Question in Instructor Student View: position plus its Question Revision Tuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructorStudentViewQuestion {
    /// One-based position in the flattened presented-Question sequence.
    pub position: NonZeroU32,
    pub question_revision_tuple: QuestionRevisionTuple,
}

/// Closed Student-safe reason that an authored entry is not shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstructorStudentViewNotShownReason {
    /// The current Assessment Entry is unavailable for delivery.
    AssessmentEntryUnavailable,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{QuestionId, QuestionRevisionNumber};

    fn question_revision_tuple() -> QuestionRevisionTuple {
        QuestionRevisionTuple {
            question_id: "0000-T00N".parse::<QuestionId>().expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(1).expect("Question Revision"),
        }
    }

    #[test]
    fn presented_entry_serializes_only_public_exact_question_locators() {
        let question_id = "0000-T00N".parse::<QuestionId>().expect("Question ID");
        let entry = InstructorStudentViewEntry::Presented {
            authored_position: 0,
            questions: vec![InstructorStudentViewQuestion {
                position: NonZeroU32::new(1).expect("positive position"),
                question_revision_tuple: question_revision_tuple(),
            }],
        };
        assert_eq!(
            serde_json::to_value(entry).expect("manifest entry serializes"),
            serde_json::json!({
                "kind": "presented",
                "authoredPosition": 0,
                "questions": [{
                    "position": 1,
                    "questionRevisionTuple": {
                        "questionId": question_id.to_string(),
                        "revisionNumber": 1
                    }
                }]
            })
        );
    }

    #[test]
    fn instructor_student_view_serializes_assessment_edit_number() {
        let view = InstructorStudentView {
            assessment_edit_number: "3".parse().expect("Assessment Edit Number"),
            status: AssessmentStatus::Unreleased,
            title: crate::AssessmentTitle::try_new("Quiz".to_string()).expect("title"),
            instructions: AssessmentInstructions::default(),
            display_time_zone: AccountTimeZone::parse("America/Chicago").expect("zone"),
            delivery: InstructorStudentViewDelivery {
                available_at: None,
                due_at: None,
                closes_at: None,
                assessment_attempt_time_limit_seconds: None,
                attempt_limit: None,
                late_work_rule: LateWorkRule::Reject,
            },
            entries: Vec::new(),
        };
        let wire = serde_json::to_value(&view).expect("manifest serializes");
        assert_eq!(wire["assessmentEditNumber"], "3");
        assert!(wire.get("editNumber").is_none());
    }
}
