//! Immutable Question Revision acceptance facts.

/// Maximum number of Unicode scalar values in one Question Revision Reason.
pub const MAX_QUESTION_REVISION_REASON_UNICODE_SCALARS: usize = 2_000;

/// Reviewed explanation for why an immutable Question Revision was accepted.
///
/// Instructor surfaces label this value "Reason for Edit." It is distinct from
/// Question Authorship, Question Ownership, and mutable Draft Question content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionRevisionReason(String);

impl QuestionRevisionReason {
    /// Creates a trimmed, nonempty, control-free Question Revision Reason.
    pub fn new(value: String) -> Result<Self, &'static str> {
        if value.trim() != value
            || value.is_empty()
            || value.chars().count() > MAX_QUESTION_REVISION_REASON_UNICODE_SCALARS
            || value.chars().any(char::is_control)
        {
            return Err("Question Revision Reason must be reviewed and bounded");
        }
        Ok(Self(value))
    }

    /// Returns the reviewed Reason for Edit text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_revision_reason_requires_reviewed_bounded_text() {
        assert_eq!(
            QuestionRevisionReason::new("Correct the amino-acid charge".to_string())
                .expect("reviewed reason")
                .as_str(),
            "Correct the amino-acid charge"
        );
        assert!(QuestionRevisionReason::new(" leading space".to_string()).is_err());
        assert!(QuestionRevisionReason::new("line\nbreak".to_string()).is_err());
    }
}
