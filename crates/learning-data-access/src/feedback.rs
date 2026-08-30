//! Private teaching-feedback records and their bounded validation.

use crate::{AssignmentRecord, Page, StoreError};
use domain::disclosure_policy::StudentDisclosureDecision;
use domain::effective_assignment_policy::EffectiveAssignmentPolicy;
use objects::Sha256Digest;
use question_model::envelope::ContentBlock;
use question_model::{
    ActivityTimestamp, AssignmentRun, AttemptResult, FeedbackContent, QuestionAttemptId,
    ScoringStatus, StudentAssignmentSummary, StudentResponse, UserId,
};

/// One Student-safe summary and its score freshness read atomically by storage.
/// It deliberately has no serialization or debug representation.
#[derive(Clone, PartialEq)]
pub struct StudentAssignmentSummarySnapshot {
    pub summary: StudentAssignmentSummary,
    pub scoring_status: ScoringStatus,
}

/// Private feedback retained beside the first grade.
///
/// The content is deliberately neither serde nor debug printable. Store
/// backends encode its closed `ContentBlock` representation only into their
/// private persistence table and must return the original stored content on a
/// matching idempotent replay.
#[derive(Clone, PartialEq, Eq)]
pub struct AttemptFeedbackRecord {
    content: FeedbackContent,
    content_sha256: Sha256Digest,
}

/// Immutable audit receipt for an instructor feedback action.
///
/// This carries no feedback content and never changes Student disclosure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackReleaseRecord {
    pub attempt: QuestionAttemptId,
    pub released_by: UserId,
    pub released_at: ActivityTimestamp,
}

/// Private, current Student-disclosure evaluation input.
///
/// This boundary deliberately contains only assignment-owned disclosure
/// policy, the effective policy read from the current sealed S3 receipt, and
/// authoritative temporal facts. It is neither serializable nor debug
/// printable and cannot carry a question definition, answer key, provider
/// material, or an entitlement decision. S5 authorization is completed before
/// a store returns it.
#[derive(Clone, PartialEq, Eq)]
pub struct StudentDisclosureInput {
    assignment_policy: question_model::StudentDisclosurePolicy,
    effective_policy: EffectiveAssignmentPolicy,
    evaluated_at: ActivityTimestamp,
    submitted_at: Option<ActivityTimestamp>,
}

impl StudentDisclosureInput {
    pub(crate) fn new(
        assignment_policy: question_model::StudentDisclosurePolicy,
        effective_policy: EffectiveAssignmentPolicy,
        evaluated_at: ActivityTimestamp,
        submitted_at: Option<ActivityTimestamp>,
    ) -> Self {
        Self {
            assignment_policy,
            effective_policy,
            evaluated_at,
            submitted_at,
        }
    }

    /// Evaluates every public field from the one current server-side source.
    pub fn decision(&self) -> StudentDisclosureDecision {
        domain::disclosure_policy::evaluate_allowed_student_disclosure(
            &self.effective_policy,
            self.assignment_policy,
            self.evaluated_at,
            self.submitted_at,
        )
    }
}

/// Private, bounded input for server-side feedback redaction on a run summary.
///
/// This is intentionally neither serializable nor debug printable. The route
/// turns it into a public DTO only after applying the current disclosure
/// decision. The current effective policy is read from the sealed S3 receipt;
/// the projection then applies the current assignment disclosure policy and
/// authoritative time. A feedback-release audit record never unlocks content.
#[derive(Clone, PartialEq)]
pub struct RunSummaryOutcomeInput {
    pub attempt: QuestionAttemptId,
    pub assignment_position: u32,
    pub submitted_at: Option<ActivityTimestamp>,
    pub response: Option<StudentResponse>,
    pub result: Option<AttemptResult>,
    pub disclosure: StudentDisclosureInput,
    pub feedback: Option<AttemptFeedbackRecord>,
}

/// Private run-summary material returned in one authorized store read.
///
/// It deliberately carries no question definition, source, provenance, key,
/// envelope, or provider data. `practice_allowed` is advisory presentation
/// state; `start_or_resume_run` remains the authoritative transition.
#[derive(Clone, PartialEq)]
pub struct RunSummaryPageInput {
    pub run: AssignmentRun,
    pub assignment: AssignmentRecord,
    pub summary: StudentAssignmentSummary,
    pub scoring_status: ScoringStatus,
    pub practice_allowed: bool,
    pub outcomes: Page<RunSummaryOutcomeInput>,
}

/// Trusted command for an instructor-initiated feedback disclosure release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseAttemptFeedbackCommand {
    pub actor: UserId,
    pub attempt: QuestionAttemptId,
}

impl AttemptFeedbackRecord {
    pub fn content(&self) -> &FeedbackContent {
        &self.content
    }

    pub fn content_sha256(&self) -> &Sha256Digest {
        &self.content_sha256
    }
}

/// Validates and canonicalizes private feedback before any submission state is
/// changed. The closed block model admits only structural content that the
/// renderer can safely escape or resolve through its own positive allowlist.
pub fn private_feedback_record(
    content: FeedbackContent,
) -> Result<AttemptFeedbackRecord, StoreError> {
    let mut budget = FeedbackBudget::default();
    validate_feedback_blocks(content.hint.as_deref(), "hint", &mut budget)?;
    validate_feedback_blocks(
        content.correct_response.as_deref(),
        "correct_response",
        &mut budget,
    )?;
    validate_feedback_blocks(content.rationale.as_deref(), "rationale", &mut budget)?;
    let encoded = canonical_feedback_bytes(&content)?;
    Ok(AttemptFeedbackRecord {
        content,
        content_sha256: Sha256Digest::compute(&encoded),
    })
}

/// Encodes the private fixed feedback tuple through `ple-canonical-json-v1`.
///
/// The tuple is intentionally local so `FeedbackContent` never needs serde
/// derives merely for persistence. Receipt and automated-grading writers use
/// this exact source/projection/digest value instead of rebuilding the tuple.
pub(crate) fn canonical_feedback_json_v1(
    content: &FeedbackContent,
) -> Result<crate::canonical_json::CanonicalJsonV1, StoreError> {
    crate::canonical_json::canonical_json_bytes_v1(
        "feedback",
        &(
            content.hint.as_deref(),
            content.correct_response.as_deref(),
            content.rationale.as_deref(),
        ),
    )
}

/// Compatibility source-byte view for established private feedback callers.
pub(crate) fn canonical_feedback_bytes(content: &FeedbackContent) -> Result<Vec<u8>, StoreError> {
    Ok(canonical_feedback_json_v1(content)?.source.into_bytes())
}

#[derive(Default)]
struct FeedbackBudget {
    blocks: usize,
    bytes: usize,
}

fn validate_feedback_blocks(
    blocks: Option<&[ContentBlock]>,
    field: &str,
    budget: &mut FeedbackBudget,
) -> Result<(), StoreError> {
    const MAX_TOTAL_BLOCKS: usize = 64;
    const MAX_TOTAL_BYTES: usize = 64 * 1024;
    const MAX_TABLE_COLUMNS: usize = 64;
    const MAX_TABLE_ROWS: usize = 256;
    let Some(blocks) = blocks else {
        return Ok(());
    };
    let encoded = serde_json::to_vec(blocks).map_err(|error| {
        StoreError::InvalidRecord(format!("feedback {field} encoding failed: {error}"))
    })?;
    budget.blocks = budget.blocks.saturating_add(blocks.len());
    budget.bytes = budget.bytes.saturating_add(encoded.len());
    if budget.blocks > MAX_TOTAL_BLOCKS {
        return Err(StoreError::InvalidRecord(
            "feedback has too many blocks".to_string(),
        ));
    }
    if budget.bytes > MAX_TOTAL_BYTES {
        return Err(StoreError::InvalidRecord(
            "feedback is too large".to_string(),
        ));
    }
    for block in blocks {
        match block {
            // Literal text, code, and table cells are inert data. The
            // renderer owns escaping/sanitization; Store must not impose a
            // brittle content blacklist.
            ContentBlock::Text { .. } | ContentBlock::Math { .. } => {}
            ContentBlock::Image { asset, .. } => validate_feedback_asset_checksum(&asset.checksum)?,
            ContentBlock::Code { language, .. } => validate_feedback_language(language)?,
            ContentBlock::Table { headers, rows, .. } => {
                if headers.is_empty() || headers.len() > MAX_TABLE_COLUMNS {
                    return Err(StoreError::InvalidRecord(format!(
                        "feedback {field} table has an invalid column count"
                    )));
                }
                if rows.len() > MAX_TABLE_ROWS || rows.iter().any(|row| row.len() != headers.len())
                {
                    return Err(StoreError::InvalidRecord(format!(
                        "feedback {field} table rows do not match its headers"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_feedback_asset_checksum(checksum: &str) -> Result<(), StoreError> {
    if checksum.len() != 64
        || !checksum
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(StoreError::InvalidRecord(
            "feedback image has an invalid asset checksum".to_string(),
        ));
    }
    Ok(())
}

fn validate_feedback_language(language: &str) -> Result<(), StoreError> {
    if language.is_empty()
        || language.len() > 64
        || !language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'_'))
    {
        return Err(StoreError::InvalidRecord(
            "feedback code has an invalid language tag".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::AssetId;

    #[test]
    fn private_feedback_accepts_inert_teaching_text() {
        let content = FeedbackContent {
            hint: Some(vec![
                ContentBlock::Text {
                    markdown: "For this protein comparison, x < 5 is meaningful.".to_string(),
                },
                ContentBlock::Code {
                    language: "python".to_string(),
                    source: "if score > 0: continue".to_string(),
                },
                ContentBlock::Text {
                    markdown: "Literal reference: https://example.invalid/teaching".to_string(),
                },
            ]),
            ..FeedbackContent::default()
        };
        assert!(private_feedback_record(content).is_ok());
    }

    #[test]
    fn private_feedback_rejects_malformed_structure_and_oversized_content() {
        let malformed_table = FeedbackContent {
            hint: Some(vec![ContentBlock::Table {
                headers: vec!["residue".to_string(), "charge".to_string()],
                rows: vec![vec!["Lys".to_string()]],
                description: "amino-acid comparison".to_string(),
            }]),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(malformed_table),
            Err(StoreError::InvalidRecord(_))
        ));

        let bad_image = FeedbackContent {
            hint: Some(vec![ContentBlock::Image {
                asset: question_model::envelope::AssetRef {
                    asset: AssetId::from_uuid(uuid::Uuid::nil()),
                    checksum: "not-a-sha256".to_string(),
                },
                description: "a peptide diagram".to_string(),
            }]),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(bad_image),
            Err(StoreError::InvalidRecord(_))
        ));

        let oversized = FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "x".repeat(64 * 1024),
            }]),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(oversized),
            Err(StoreError::InvalidRecord(_))
        ));

        let too_many = FeedbackContent {
            hint: Some(
                (0..65)
                    .map(|_| ContentBlock::Text {
                        markdown: "bounded".to_string(),
                    })
                    .collect(),
            ),
            ..FeedbackContent::default()
        };
        assert!(matches!(
            private_feedback_record(too_many),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn private_feedback_digest_is_stable_for_exact_content() {
        let content = FeedbackContent {
            hint: Some(vec![ContentBlock::Text {
                markdown: "Check your sign.".to_string(),
            }]),
            ..FeedbackContent::default()
        };
        let left = private_feedback_record(content.clone()).expect("valid private feedback");
        let right = private_feedback_record(content).expect("valid private feedback");
        assert_eq!(left.content_sha256(), right.content_sha256());
        assert!(left == right);
    }
}
