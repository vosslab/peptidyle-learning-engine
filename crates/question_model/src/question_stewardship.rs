//! Browser-safe vocabulary for Published Question stewardship activity.
//!
//! This module names the activity that a Question Watch can eventually
//! subscribe to. It deliberately contains neither a watch subscription nor a
//! watcher identity: those are private, account-authorized persistence facts
//! owned by the later star and watch stores. The vocabulary is safe to project
//! only after that authorization boundary has selected a recipient.

use serde::{Deserialize, Serialize};

use crate::question_library::{QuestionId, QuestionRevisionTuple};

/// A browser-safe stewardship activity concerning a Published Question.
///
/// This is an activity vocabulary, not a delivery record. In particular, it
/// does not disclose whether any Instructor watched the Question or received a
/// notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionStewardshipEvent {
    /// A new immutable revision joined the Question's published lineage.
    RevisionPublished {
        question_revision_tuple: QuestionRevisionTuple,
    },
    /// A published fork records its distinct Question lineage and source.
    ForkPublished {
        source_question: QuestionId,
        fork_revision_tuple: QuestionRevisionTuple,
    },
    /// An improvement thread has activity for the named Question lineage.
    ImprovementThreadActivity { question: QuestionId },
    /// A maintained impact notice concerns the named Question lineage.
    ImpactNotice { question: QuestionId },
}

impl QuestionStewardshipEvent {
    /// Returns the published Question lineage whose stewardship changed.
    pub fn question(&self) -> &QuestionId {
        match self {
            Self::RevisionPublished {
                question_revision_tuple,
            } => &question_revision_tuple.question_id,
            Self::ForkPublished {
                source_question, ..
            } => source_question,
            Self::ImprovementThreadActivity { question } | Self::ImpactNotice { question } => {
                question
            }
        }
    }
}
