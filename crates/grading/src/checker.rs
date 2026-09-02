//! Server-only answer checking (MOD-GRD).
//!
//! Browser-safe format validation runs first through `domain`; only a
//! structurally valid response reaches the answer-bearing comparison below.
//! The public result contains correctness and points, never the key.

use std::collections::BTreeSet;

use domain::statistics::QuestionStatisticsObservation;
use domain::validation::{StudentResponseFormatIssue, validate_response_format};
use question_model::answer::{NumericResponseTolerance, TextResponseMatchRule};
use question_model::response::{QuestionResponseFormat, ResponseItemReference, StudentResponse};
use question_model::{GradingResult, QuestionGradingRule, QuestionRevision};

use crate::AnswerKey;

/// Outcome of applying a question's declared grading mode.
#[derive(Debug, Clone, PartialEq)]
pub enum QuestionGradingOutcome {
    /// Server-graded correctness and points safe to disclose under policy.
    Graded(GradingResult),
    /// Intentionally ungraded practice with no fabricated correctness value.
    Ungraded,
}

/// Explicit refusal to guess at malformed or backend-owned grading behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum GradingError {
    /// Browser-safe shape validation rejected the response.
    InvalidResponse(Vec<StudentResponseFormatIssue>),
    /// A graded question had no server-only answer key.
    MissingAnswerKey,
    /// Ungraded practice incorrectly carried answer-bearing material.
    UnexpectedAnswerKey,
    /// The Question Response Format, submitted response, and key were not parallel variants.
    KindMismatch,
    /// Public grading or tolerance parameters were invalid.
    InvalidDefinition(String),
    /// Partial-credit rules remain owned by a capable deterministic backend.
    PartialCreditRequiresBackend,
}

impl std::fmt::Display for GradingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResponse(issues) => {
                write!(formatter, "response has {} format issue(s)", issues.len())
            }
            Self::MissingAnswerKey => formatter.write_str("graded question has no answer key"),
            Self::UnexpectedAnswerKey => {
                formatter.write_str("ungraded question must not have an answer key")
            }
            Self::KindMismatch => formatter.write_str(
                "Question Response Format, Student Response, and Answer Key kinds must agree",
            ),
            Self::InvalidDefinition(message) => {
                write!(formatter, "invalid Question Grading Rule: {message}")
            }
            Self::PartialCreditRequiresBackend => {
                formatter.write_str("partial credit requires a deterministic backend checker")
            }
        }
    }
}

impl std::error::Error for GradingError {}

/// Grades one structurally valid response without exposing its answer key.
///
/// Ungraded questions return [`QuestionGradingOutcome::Ungraded`] and require no key.
/// All-or-nothing questions use the shared checkers below. Partial credit is
/// refused until a capable deterministic adapter supplies the
/// actual pedagogical rule; this module does not invent one from a boolean
/// `partialCredit` flag.
///
/// # Errors
///
/// Returns [`GradingError`] for response-format issues, missing or
/// mismatched keys, invalid public parameters, backend-owned partial credit,
/// or other backend-owned grading behavior. This checker never fabricates a
/// numeric grade.
pub fn grade(
    question: &QuestionRevision,
    response: &StudentResponse,
    key: Option<&AnswerKey>,
) -> Result<QuestionGradingOutcome, GradingError> {
    let check = validate_response_format(&question.response, response);
    if !check.is_valid() {
        return Err(GradingError::InvalidResponse(check.issues));
    }

    let points = match question.grading {
        QuestionGradingRule::Ungraded => {
            return if key.is_none() {
                Ok(QuestionGradingOutcome::Ungraded)
            } else {
                Err(GradingError::UnexpectedAnswerKey)
            };
        }
        QuestionGradingRule::AllOrNothing { points } => validated_points(points)?,
        QuestionGradingRule::PartialCredit { points } => {
            let _ = validated_points(points)?;
            return Err(GradingError::PartialCreditRequiresBackend);
        }
    };
    let key = key.ok_or(GradingError::MissingAnswerKey)?;
    let correct = answer_is_correct(&question.response, response, key)?;
    Ok(QuestionGradingOutcome::Graded(GradingResult {
        correct,
        points_earned: if correct { points } else { 0.0 },
        points_possible: points,
    }))
}

/// Reduces one accepted server grade to the exact global-statistics observation.
///
/// Only a validated `MultipleChoice` response contributes eligible choice
/// selections. Other supported response formats still contribute one accepted
/// grade and its correctness without inventing a choice-count interpretation.
pub fn question_statistics_observation(
    question: &QuestionRevision,
    response: &StudentResponse,
    outcome: &QuestionGradingOutcome,
) -> Result<Option<QuestionStatisticsObservation>, GradingError> {
    let check = validate_response_format(&question.response, response);
    if !check.is_valid() {
        return Err(GradingError::InvalidResponse(check.issues));
    }
    let QuestionGradingOutcome::Graded(result) = outcome else {
        return Ok(None);
    };
    let selections = match (&question.response, response) {
        (
            QuestionResponseFormat::MultipleChoice { .. },
            StudentResponse::MultipleChoice { selected },
        ) => selected.clone(),
        _ => Vec::new(),
    };
    QuestionStatisticsObservation::new(result.correct, selections)
        .map(Some)
        .map_err(|error| GradingError::InvalidDefinition(error.to_string()))
}

fn validated_points(points: f64) -> Result<f64, GradingError> {
    if points.is_finite() && points >= 0.0 {
        Ok(points)
    } else {
        Err(GradingError::InvalidDefinition(
            "points must be finite and nonnegative".to_string(),
        ))
    }
}

fn answer_is_correct(
    definition: &QuestionResponseFormat,
    response: &StudentResponse,
    key: &AnswerKey,
) -> Result<bool, GradingError> {
    match (definition, response, key) {
        (
            QuestionResponseFormat::Numeric { tolerance, .. },
            StudentResponse::Numeric { value },
            AnswerKey::Numeric { expected },
        ) => numeric_is_correct(*value, *expected, tolerance),
        (
            QuestionResponseFormat::MultipleChoice { choices, .. },
            StudentResponse::MultipleChoice { selected },
            AnswerKey::MultipleChoice { correct },
        ) => {
            let available: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            if !correct.is_subset(&available) {
                return Err(GradingError::InvalidDefinition(
                    "multiple-choice key names an unavailable choice".to_string(),
                ));
            }
            Ok(selected.iter().cloned().collect::<BTreeSet<_>>() == *correct)
        }
        (
            QuestionResponseFormat::ShortText { match_mode, .. },
            StudentResponse::ShortText { text },
            AnswerKey::ShortText { accepted },
        ) => Ok(text_is_correct(text, accepted, *match_mode)),
        (
            QuestionResponseFormat::MultiBlank { blanks },
            StudentResponse::MultiBlank { answers },
            AnswerKey::MultiBlank { accepted },
        ) => {
            if accepted.len() != blanks.len()
                || blanks.iter().any(|blank| !accepted.contains_key(&blank.id))
            {
                return Err(GradingError::InvalidDefinition(
                    "multi-blank key must name every available slot exactly once".to_string(),
                ));
            }
            Ok(answers.iter().all(|answer| {
                let blank = blanks
                    .iter()
                    .find(|blank| blank.id == answer.slot)
                    .expect("format validation proved the slot set");
                accepted.get(&answer.slot).is_some_and(|accepted| {
                    text_is_correct(&answer.text, accepted, blank.match_mode)
                })
            }))
        }
        (
            QuestionResponseFormat::Matching { prompts, choices },
            StudentResponse::Matching { matches },
            AnswerKey::Matching { correct },
        ) => {
            let prompt_ids: BTreeSet<_> = prompts.iter().map(|prompt| prompt.id.clone()).collect();
            let choice_ids: BTreeSet<_> = choices.iter().map(|choice| choice.id.clone()).collect();
            if correct.len() != prompts.len()
                || correct.keys().cloned().collect::<BTreeSet<_>>() != prompt_ids
                || correct.values().any(|choice| !choice_ids.contains(choice))
            {
                return Err(GradingError::InvalidDefinition(
                    "matching key must bind every prompt to an available choice".to_string(),
                ));
            }
            Ok(matches
                .iter()
                .all(|pair| correct.get(&pair.prompt) == Some(&pair.choice)))
        }
        (
            QuestionResponseFormat::Ordering { items },
            StudentResponse::Ordering { order },
            AnswerKey::Ordering { correct },
        ) => {
            let available: BTreeSet<ResponseItemReference> =
                items.iter().map(|item| item.id.clone()).collect();
            let keyed: BTreeSet<ResponseItemReference> = correct.iter().cloned().collect();
            if keyed.len() != correct.len() || keyed != available {
                return Err(GradingError::InvalidDefinition(
                    "ordering key must contain every available item exactly once".to_string(),
                ));
            }
            Ok(order == correct)
        }
        (
            QuestionResponseFormat::Hotspot { regions, .. },
            StudentResponse::Hotspot { selections },
            AnswerKey::Hotspot { correct },
        ) => {
            let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
            if !correct.is_subset(&available) {
                return Err(GradingError::InvalidDefinition(
                    "hotspot key names an unavailable region".to_string(),
                ));
            }
            let selected = selections
                .iter()
                .map(|selection| selection.region.clone())
                .collect::<BTreeSet<_>>();
            Ok(selected == *correct)
        }
        _ => Err(GradingError::KindMismatch),
    }
}

fn numeric_is_correct(
    actual: f64,
    expected: f64,
    tolerance: &NumericResponseTolerance,
) -> Result<bool, GradingError> {
    if !expected.is_finite() {
        return Err(GradingError::InvalidDefinition(
            "numeric key must be finite".to_string(),
        ));
    }
    match tolerance {
        NumericResponseTolerance::Exact => Ok(actual == expected),
        NumericResponseTolerance::Absolute { epsilon } => {
            validate_nonnegative_finite("absolute epsilon", *epsilon)?;
            Ok((actual - expected).abs() <= *epsilon)
        }
        NumericResponseTolerance::Relative { fraction } => {
            validate_nonnegative_finite("relative fraction", *fraction)?;
            Ok((actual - expected).abs() <= expected.abs() * *fraction)
        }
        NumericResponseTolerance::SignificantFigures { digits } => {
            if *digits == 0 {
                return Err(GradingError::InvalidDefinition(
                    "significant figures must be at least one".to_string(),
                ));
            }
            Ok(round_significant(actual, *digits) == round_significant(expected, *digits))
        }
    }
}

fn validate_nonnegative_finite(name: &str, value: f64) -> Result<(), GradingError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(GradingError::InvalidDefinition(format!(
            "{name} must be finite and nonnegative"
        )))
    }
}

fn round_significant(value: f64, digits: u8) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let exponent = value.abs().log10().floor();
    let scale = 10_f64.powf(f64::from(digits) - 1.0 - exponent);
    if scale.is_finite() && scale != 0.0 {
        (value * scale).round() / scale
    } else {
        value
    }
}

fn text_is_correct(actual: &str, accepted: &[String], mode: TextResponseMatchRule) -> bool {
    match mode {
        TextResponseMatchRule::Exact => accepted.iter().any(|expected| actual == expected),
        TextResponseMatchRule::CaseInsensitive => accepted
            .iter()
            .any(|expected| actual.to_lowercase() == expected.to_lowercase()),
        TextResponseMatchRule::Normalized => {
            let actual = normalized_text(actual);
            accepted
                .iter()
                .any(|expected| actual == normalized_text(expected))
        }
    }
}

fn normalized_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::answer::ResponseSelectionRule;
    use question_model::assignment_activity_rules::{
        QuestionAttemptLimit, QuestionAttemptTimeLimit,
    };
    use question_model::classification::QuestionLicense;
    use question_model::envelope::QuestionContentBlock;
    use question_model::generation::QuestionVariationRule;
    use question_model::response::{OrderingItem, QuestionChoice, QuestionType};
    use question_model::{
        QuestionBackendLocator, QuestionFormat, QuestionId, QuestionMetadata,
        QuestionRevisionNumber, WorkspaceId,
    };
    use uuid::Uuid;

    fn question_choice(id: &str) -> QuestionChoice {
        QuestionChoice {
            id: ResponseItemReference::new(id),
            body: Vec::new(),
        }
    }

    fn ordering_item(id: &str) -> OrderingItem {
        OrderingItem {
            id: ResponseItemReference::new(id),
            body: Vec::new(),
        }
    }

    fn question(
        response: QuestionResponseFormat,
        grading: QuestionGradingRule,
    ) -> QuestionRevision {
        QuestionRevision {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
            revision_number: QuestionRevisionNumber::new(2).expect("positive version"),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            backend_locator: QuestionBackendLocator::Ple,
            question_format: QuestionFormat::PleAlgorithmic,
            prompt: vec![QuestionContentBlock::Text {
                markdown: "Fixture".to_string(),
            }],
            response,
            question_type: QuestionType::MultipleChoice,
            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
            question_variation_rule: QuestionVariationRule::Static,
            grading,
            metadata: QuestionMetadata {
                title: "Grading fixture".to_string(),
                question_description: "Instructor-facing grading fixture summary.".to_string(),
                tags: Vec::new(),
                classifications: Vec::new(),
                question_license: Some(QuestionLicense::CcBySa4_0),
                question_citation: None,
                language: "en-US".to_string(),
            },
        }
    }

    fn all_or_nothing(response: QuestionResponseFormat) -> QuestionRevision {
        question(response, QuestionGradingRule::AllOrNothing { points: 2.0 })
    }

    #[test]
    fn numeric_tolerances_use_inclusive_boundaries() {
        let cases = [
            (NumericResponseTolerance::Exact, 10.0, 10.0, true),
            (
                NumericResponseTolerance::Absolute { epsilon: 0.5 },
                10.5,
                10.0,
                true,
            ),
            (
                NumericResponseTolerance::Relative { fraction: 0.1 },
                -11.0,
                -10.0,
                true,
            ),
            (
                NumericResponseTolerance::SignificantFigures { digits: 3 },
                1_234.0,
                1_230.0,
                true,
            ),
            (
                NumericResponseTolerance::SignificantFigures { digits: 3 },
                1_219.0,
                1_230.0,
                false,
            ),
        ];

        for (tolerance, actual, expected, correct) in cases {
            let result = grade(
                &all_or_nothing(QuestionResponseFormat::Numeric {
                    tolerance,
                    unit: None,
                }),
                &StudentResponse::Numeric { value: actual },
                Some(&AnswerKey::Numeric { expected }),
            )
            .expect("numeric case should grade");
            assert_eq!(
                result,
                QuestionGradingOutcome::Graded(GradingResult {
                    correct,
                    points_earned: if correct { 2.0 } else { 0.0 },
                    points_possible: 2.0,
                })
            );
        }
    }

    #[test]
    fn choice_text_and_ordering_use_their_declared_comparisons() {
        let choice_question = all_or_nothing(QuestionResponseFormat::MultipleChoice {
            choices: vec![
                question_choice("a"),
                question_choice("b"),
                question_choice("c"),
            ],
            selection: ResponseSelectionRule::Exactly { count: 2 },
        });
        assert!(matches!(
            grade(
                &choice_question,
                &StudentResponse::MultipleChoice {
                    selected: vec![
                        ResponseItemReference::new("c"),
                        ResponseItemReference::new("a")
                    ],
                },
                Some(&AnswerKey::MultipleChoice {
                    correct: BTreeSet::from([
                        ResponseItemReference::new("a"),
                        ResponseItemReference::new("c")
                    ]),
                }),
            ),
            Ok(QuestionGradingOutcome::Graded(GradingResult {
                correct: true,
                ..
            }))
        ));

        let text_question = all_or_nothing(QuestionResponseFormat::ShortText {
            match_mode: TextResponseMatchRule::Normalized,
            max_length: 40,
        });
        assert!(matches!(
            grade(
                &text_question,
                &StudentResponse::ShortText {
                    text: "  PEPTIDE\n bond ".to_string(),
                },
                Some(&AnswerKey::ShortText {
                    accepted: vec!["peptide bond".to_string()],
                }),
            ),
            Ok(QuestionGradingOutcome::Graded(GradingResult {
                correct: true,
                ..
            }))
        ));

        let ordering_question = all_or_nothing(QuestionResponseFormat::Ordering {
            items: vec![ordering_item("first"), ordering_item("second")],
        });
        assert!(matches!(
            grade(
                &ordering_question,
                &StudentResponse::Ordering {
                    order: vec![
                        ResponseItemReference::new("first"),
                        ResponseItemReference::new("second")
                    ],
                },
                Some(&AnswerKey::Ordering {
                    correct: vec![
                        ResponseItemReference::new("first"),
                        ResponseItemReference::new("second")
                    ],
                }),
            ),
            Ok(QuestionGradingOutcome::Graded(GradingResult {
                correct: true,
                ..
            }))
        ));
    }

    #[test]
    fn accepted_multiple_choice_grade_yields_only_its_eligible_choice_counts() {
        let question = all_or_nothing(QuestionResponseFormat::MultipleChoice {
            choices: vec![
                question_choice("a"),
                question_choice("b"),
                question_choice("c"),
            ],
            selection: ResponseSelectionRule::Exactly { count: 2 },
        });
        let response = StudentResponse::MultipleChoice {
            selected: vec![
                ResponseItemReference::new("a"),
                ResponseItemReference::new("c"),
            ],
        };
        let outcome = grade(
            &question,
            &response,
            Some(&AnswerKey::MultipleChoice {
                correct: BTreeSet::from([ResponseItemReference::new("a")]),
            }),
        )
        .expect("validated selection grades");

        let observation = question_statistics_observation(&question, &response, &outcome)
            .expect("accepted grade yields statistics evidence")
            .expect("graded outcome contributes evidence");
        assert!(!observation.correct());
        assert_eq!(
            observation
                .eligible_choice_selections()
                .map(ResponseItemReference::as_str)
                .collect::<Vec<_>>(),
            vec!["a", "c"]
        );
    }

    #[test]
    fn malformed_backend_inputs_fail_instead_of_guessing() {
        let numeric = all_or_nothing(QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::Absolute { epsilon: 0.1 },
            unit: None,
        });
        assert_eq!(
            grade(
                &numeric,
                &StudentResponse::ShortText {
                    text: "10".to_string(),
                },
                Some(&AnswerKey::Numeric { expected: 10.0 }),
            ),
            Err(GradingError::InvalidResponse(vec![
                StudentResponseFormatIssue::ResponseKindMismatch,
            ]))
        );
        assert_eq!(
            grade(
                &numeric,
                &StudentResponse::Numeric { value: 10.0 },
                Some(&AnswerKey::ShortText {
                    accepted: vec!["10".to_string()],
                }),
            ),
            Err(GradingError::KindMismatch)
        );
        assert_eq!(
            grade(&numeric, &StudentResponse::Numeric { value: 10.0 }, None),
            Err(GradingError::MissingAnswerKey)
        );
    }

    #[test]
    fn invalid_public_grading_parameters_are_rejected() {
        let negative_points = question(
            QuestionResponseFormat::Numeric {
                tolerance: NumericResponseTolerance::Exact,
                unit: None,
            },
            QuestionGradingRule::AllOrNothing { points: -1.0 },
        );
        assert_eq!(
            grade(
                &negative_points,
                &StudentResponse::Numeric { value: 1.0 },
                Some(&AnswerKey::Numeric { expected: 1.0 }),
            ),
            Err(GradingError::InvalidDefinition(
                "points must be finite and nonnegative".to_string(),
            ))
        );

        let invalid_tolerance = all_or_nothing(QuestionResponseFormat::Numeric {
            tolerance: NumericResponseTolerance::SignificantFigures { digits: 0 },
            unit: None,
        });
        assert_eq!(
            grade(
                &invalid_tolerance,
                &StudentResponse::Numeric { value: 1.0 },
                Some(&AnswerKey::Numeric { expected: 1.0 }),
            ),
            Err(GradingError::InvalidDefinition(
                "significant figures must be at least one".to_string(),
            ))
        );
    }

    #[test]
    fn ungraded_and_partial_credit_capabilities_are_explicit() {
        let ungraded = question(
            QuestionResponseFormat::ShortText {
                match_mode: TextResponseMatchRule::Exact,
                max_length: 10,
            },
            QuestionGradingRule::Ungraded,
        );
        assert_eq!(
            grade(
                &ungraded,
                &StudentResponse::ShortText {
                    text: "practice".to_string(),
                },
                None,
            ),
            Ok(QuestionGradingOutcome::Ungraded)
        );
        assert_eq!(
            grade(
                &ungraded,
                &StudentResponse::ShortText {
                    text: "practice".to_string(),
                },
                Some(&AnswerKey::ShortText {
                    accepted: vec!["practice".to_string()],
                }),
            ),
            Err(GradingError::UnexpectedAnswerKey)
        );

        let partial = question(
            QuestionResponseFormat::Numeric {
                tolerance: NumericResponseTolerance::Exact,
                unit: None,
            },
            QuestionGradingRule::PartialCredit { points: 2.0 },
        );
        assert_eq!(
            grade(
                &partial,
                &StudentResponse::Numeric { value: 1.0 },
                Some(&AnswerKey::Numeric { expected: 1.0 }),
            ),
            Err(GradingError::PartialCreditRequiresBackend)
        );
    }
}
