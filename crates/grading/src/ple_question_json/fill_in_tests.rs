use std::collections::BTreeMap;

use question_model::answer::TextResponseMatchRule;
use question_model::response::{
    QuestionResponseFormat, QuestionType, ResponseItemId, StudentResponse, StudentTextEntry,
    TextEntrySlot,
};

use super::{PleQuestionJsonError, PleQuestionJsonGradingError, PleQuestionJsonPrivateGrading};
use crate::AnswerKey;

fn evaluate(
    question_type: QuestionType,
    response_format: QuestionResponseFormat,
    answer_key: AnswerKey,
    response: StudentResponse,
) -> Result<f64, PleQuestionJsonError> {
    let checksum = "a".repeat(64);
    let grading = PleQuestionJsonPrivateGrading::new_with_key(
        checksum.clone(),
        question_type,
        &response_format,
        answer_key,
        Vec::new(),
        None,
        None,
    )?;
    Ok(grading
        .evaluate(&checksum, question_type, &response_format, &response)?
        .evaluation
        .normalized_credit())
}

fn short_text_format(match_mode: TextResponseMatchRule) -> QuestionResponseFormat {
    QuestionResponseFormat::ShortText {
        match_mode,
        max_length: 256,
    }
}

fn evaluate_short_text(
    match_mode: TextResponseMatchRule,
    accepted: &[&str],
    response: &str,
) -> Result<f64, PleQuestionJsonError> {
    evaluate(
        QuestionType::FillInBlank,
        short_text_format(match_mode),
        AnswerKey::ShortText {
            accepted: accepted.iter().map(|value| (*value).to_string()).collect(),
        },
        StudentResponse::ShortText {
            text: response.to_string(),
        },
    )
}

#[test]
fn fib_keeps_alternatives_and_literal_matching_modes() {
    assert_eq!(
        evaluate_short_text(
            TextResponseMatchRule::Exact,
            &["ATP", "adenosine triphosphate"],
            "adenosine triphosphate"
        ),
        Ok(1.0)
    );
    assert_eq!(
        evaluate_short_text(TextResponseMatchRule::CaseInsensitive, &["ATP"], "atp"),
        Ok(1.0)
    );
    assert_eq!(
        evaluate_short_text(
            TextResponseMatchRule::Normalized,
            &["adenosine triphosphate"],
            " Adenosine   Triphosphate "
        ),
        Ok(1.0)
    );
}

#[test]
fn fib_regex_uses_authored_anchors_and_case_and_rejects_blank_input() {
    assert_eq!(
        evaluate_short_text(
            TextResponseMatchRule::Regex,
            &["DNA"],
            "double-stranded DNA"
        ),
        Ok(1.0)
    );
    assert_eq!(
        evaluate_short_text(TextResponseMatchRule::Regex, &["^ATP$", "^GTP$"], "GTP"),
        Ok(1.0)
    );
    assert_eq!(
        evaluate_short_text(
            TextResponseMatchRule::Regex,
            &["^DNA$"],
            "double-stranded DNA"
        ),
        Ok(0.0)
    );
    assert_eq!(
        evaluate_short_text(TextResponseMatchRule::Regex, &["DNA"], "dna"),
        Ok(0.0)
    );
    assert_eq!(
        evaluate_short_text(TextResponseMatchRule::Regex, &["(?i)^dna$"], "DNA"),
        Ok(1.0)
    );
    assert_eq!(
        evaluate_short_text(TextResponseMatchRule::Regex, &[".*"], "  \t"),
        Ok(0.0)
    );
}

#[test]
fn fib_invalid_regex_is_a_validation_error() {
    let result = evaluate_short_text(TextResponseMatchRule::Regex, &["["], "anything");
    assert!(matches!(
        result,
        Err(PleQuestionJsonError::Grading(
            PleQuestionJsonGradingError::InvalidSource(message)
        )) if message.contains("invalid accepted-answer regular expression")
    ));
}

#[test]
fn multi_fib_scores_correct_blanks_over_all_authored_blanks() {
    let blanks = vec![
        TextEntrySlot {
            id: ResponseItemId::new("first"),
            label: Vec::new(),
            match_mode: TextResponseMatchRule::CaseInsensitive,
            max_length: 64,
        },
        TextEntrySlot {
            id: ResponseItemId::new("second"),
            label: Vec::new(),
            match_mode: TextResponseMatchRule::Regex,
            max_length: 64,
        },
        TextEntrySlot {
            id: ResponseItemId::new("third"),
            label: Vec::new(),
            match_mode: TextResponseMatchRule::Normalized,
            max_length: 64,
        },
        TextEntrySlot {
            id: ResponseItemId::new("fourth"),
            label: Vec::new(),
            match_mode: TextResponseMatchRule::Regex,
            max_length: 64,
        },
    ];
    let accepted = BTreeMap::from([
        (
            ResponseItemId::new("first"),
            vec!["ATP".to_string(), "adenosine triphosphate".to_string()],
        ),
        (ResponseItemId::new("second"), vec!["^Glu\\d$".to_string()]),
        (
            ResponseItemId::new("third"),
            vec!["glucose 6 phosphate".to_string()],
        ),
        (ResponseItemId::new("fourth"), vec!["citrate".to_string()]),
    ]);
    let response = StudentResponse::MultiBlank {
        answers: vec![
            StudentTextEntry {
                slot: ResponseItemId::new("first"),
                text: "ADENOSINE TRIPHOSPHATE".to_string(),
            },
            StudentTextEntry {
                slot: ResponseItemId::new("second"),
                text: "glucose".to_string(),
            },
            StudentTextEntry {
                slot: ResponseItemId::new("third"),
                text: " \t".to_string(),
            },
        ],
    };

    assert_eq!(
        evaluate(
            QuestionType::MultipleFillInBlank,
            QuestionResponseFormat::MultiBlank { blanks },
            AnswerKey::MultiBlank { accepted },
            response,
        ),
        Ok(0.25)
    );
}

#[test]
fn multi_fib_invalid_blank_regex_is_a_validation_error() {
    let result = evaluate(
        QuestionType::MultipleFillInBlank,
        QuestionResponseFormat::MultiBlank {
            blanks: vec![TextEntrySlot {
                id: ResponseItemId::new("first"),
                label: Vec::new(),
                match_mode: TextResponseMatchRule::Regex,
                max_length: 16,
            }],
        },
        AnswerKey::MultiBlank {
            accepted: BTreeMap::from([(ResponseItemId::new("first"), vec!["[".to_string()])]),
        },
        StudentResponse::MultiBlank {
            answers: Vec::new(),
        },
    );

    assert!(matches!(
        result,
        Err(PleQuestionJsonError::Grading(
            PleQuestionJsonGradingError::InvalidSource(message)
        )) if message.contains("invalid accepted-answer regular expression")
    ));
}
