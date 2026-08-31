//! Server-only answer checking (MOD-GRD).
//!
//! Browser-safe format validation runs first through `domain`; only a
//! structurally valid response reaches the answer-bearing comparison below.
//! The public result contains correctness and points, never the key.

use std::collections::BTreeSet;

use domain::statistics::QuestionStatisticsObservation;
use domain::validation::{ResponseFormatViolation, validate_response_format};
use question_model::answer::{NumericTolerance, TextMatchMode};
use question_model::response::{ChoiceId, HotspotRegion, ResponseDefinition, StudentResponse};
use question_model::{AttemptResult, GradingDefinition, QuestionDefinition};

use crate::AnswerKey;

/// Outcome of applying a question's declared grading mode.
#[derive(Debug, Clone, PartialEq)]
pub enum GradeOutcome {
    /// Server-graded correctness and points safe to disclose under policy.
    Graded(AttemptResult),
    /// Intentionally ungraded practice with no fabricated correctness value.
    Ungraded,
}

/// Explicit refusal to guess at malformed or backend-owned grading behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum GradingError {
    /// Browser-safe shape validation rejected the response.
    InvalidResponse(Vec<ResponseFormatViolation>),
    /// A graded question had no server-only answer key.
    MissingAnswerKey,
    /// Ungraded practice incorrectly carried answer-bearing material.
    UnexpectedAnswerKey,
    /// The response definition, submitted response, and key were not parallel variants.
    KindMismatch,
    /// Public grading or tolerance parameters were invalid.
    InvalidDefinition(String),
    /// Partial-credit rules remain owned by a capable deterministic backend.
    PartialCreditRequiresBackend,
    /// A graded file upload has no deterministic server-owned grader.
    FileUploadRequiresDeterministicGrader,
}

impl std::fmt::Display for GradingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResponse(violations) => {
                write!(
                    formatter,
                    "response has {} format violation(s)",
                    violations.len()
                )
            }
            Self::MissingAnswerKey => formatter.write_str("graded question has no answer key"),
            Self::UnexpectedAnswerKey => {
                formatter.write_str("ungraded question must not have an answer key")
            }
            Self::KindMismatch => formatter.write_str(
                "response definition, student response, and answer key kinds must agree",
            ),
            Self::InvalidDefinition(message) => {
                write!(formatter, "invalid grading definition: {message}")
            }
            Self::PartialCreditRequiresBackend => {
                formatter.write_str("partial credit requires a deterministic backend checker")
            }
            Self::FileUploadRequiresDeterministicGrader => formatter
                .write_str("graded file upload requires a deterministic server-owned grader"),
        }
    }
}

impl std::error::Error for GradingError {}

/// Grades one structurally valid response without exposing its answer key.
///
/// Ungraded questions return [`GradeOutcome::Ungraded`] and require no key.
/// All-or-nothing questions use the shared checkers below. Partial credit is
/// refused until a capable deterministic adapter supplies the
/// actual pedagogical rule; this module does not invent one from a boolean
/// `partialCredit` flag.
///
/// # Errors
///
/// Returns [`GradingError`] for response-format violations, missing or
/// mismatched keys, invalid public parameters, backend-owned partial credit,
/// or other backend-owned grading behavior. A graded file upload returns a
/// typed deterministic-capability refusal after its response has passed shape
/// validation; this checker never fabricates a numeric grade.
pub fn grade(
    question: &QuestionDefinition,
    response: &StudentResponse,
    key: Option<&AnswerKey>,
) -> Result<GradeOutcome, GradingError> {
    let report = validate_response_format(&question.response, response);
    if !report.is_valid() {
        return Err(GradingError::InvalidResponse(report.violations));
    }

    let points = match question.grading {
        GradingDefinition::Ungraded => {
            return if key.is_none() {
                Ok(GradeOutcome::Ungraded)
            } else {
                Err(GradingError::UnexpectedAnswerKey)
            };
        }
        GradingDefinition::AllOrNothing { points } => validated_points(points)?,
        GradingDefinition::PartialCredit { points } => {
            let _ = validated_points(points)?;
            return Err(GradingError::PartialCreditRequiresBackend);
        }
    };
    if matches!(
        (&question.response, response),
        (
            ResponseDefinition::FileUpload { .. },
            StudentResponse::FileUpload { .. },
        )
    ) {
        return Err(GradingError::FileUploadRequiresDeterministicGrader);
    }
    let key = key.ok_or(GradingError::MissingAnswerKey)?;
    let correct = answer_is_correct(&question.response, response, key)?;
    Ok(GradeOutcome::Graded(AttemptResult {
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
    question: &QuestionDefinition,
    response: &StudentResponse,
    outcome: &GradeOutcome,
) -> Result<Option<QuestionStatisticsObservation>, GradingError> {
    let report = validate_response_format(&question.response, response);
    if !report.is_valid() {
        return Err(GradingError::InvalidResponse(report.violations));
    }
    let GradeOutcome::Graded(result) = outcome else {
        return Ok(None);
    };
    let selections = match (&question.response, response) {
        (
            ResponseDefinition::MultipleChoice { .. },
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
    definition: &ResponseDefinition,
    response: &StudentResponse,
    key: &AnswerKey,
) -> Result<bool, GradingError> {
    match (definition, response, key) {
        (
            ResponseDefinition::Numeric { tolerance, .. },
            StudentResponse::Numeric { value },
            AnswerKey::Numeric { expected },
        ) => numeric_is_correct(*value, *expected, tolerance),
        (
            ResponseDefinition::MultipleChoice { choices, .. },
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
            ResponseDefinition::ShortText { match_mode, .. },
            StudentResponse::ShortText { text },
            AnswerKey::ShortText { accepted },
        ) => Ok(text_is_correct(text, accepted, *match_mode)),
        (
            ResponseDefinition::MultiBlank { blanks },
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
            ResponseDefinition::Matching { prompts, choices },
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
            ResponseDefinition::Ordering { items },
            StudentResponse::Ordering { order },
            AnswerKey::Ordering { correct },
        ) => {
            let available: BTreeSet<ChoiceId> = items.iter().map(|item| item.id.clone()).collect();
            let keyed: BTreeSet<ChoiceId> = correct.iter().cloned().collect();
            if keyed.len() != correct.len() || keyed != available {
                return Err(GradingError::InvalidDefinition(
                    "ordering key must contain every available item exactly once".to_string(),
                ));
            }
            Ok(order == correct)
        }
        (
            ResponseDefinition::Hotspot { regions, .. },
            StudentResponse::Hotspot { points },
            AnswerKey::Hotspot { correct },
        ) => {
            let available: BTreeSet<_> = regions.iter().map(|region| region.id.clone()).collect();
            if !correct.is_subset(&available) {
                return Err(GradingError::InvalidDefinition(
                    "hotspot key names an unavailable region".to_string(),
                ));
            }
            let selected = points
                .iter()
                .map(|point| {
                    regions
                        .iter()
                        .find(|region| region_contains(region, point.x, point.y))
                        .expect("format validation proved each point maps to one region")
                        .id
                        .clone()
                })
                .collect::<BTreeSet<_>>();
            Ok(selected == *correct)
        }
        _ => Err(GradingError::KindMismatch),
    }
}

fn region_contains(region: &HotspotRegion, x: u16, y: u16) -> bool {
    let right = u32::from(region.x) + u32::from(region.width);
    let bottom = u32::from(region.y) + u32::from(region.height);
    u32::from(x) >= u32::from(region.x)
        && u32::from(x) <= right
        && u32::from(y) >= u32::from(region.y)
        && u32::from(y) <= bottom
}

fn numeric_is_correct(
    actual: f64,
    expected: f64,
    tolerance: &NumericTolerance,
) -> Result<bool, GradingError> {
    if !expected.is_finite() {
        return Err(GradingError::InvalidDefinition(
            "numeric key must be finite".to_string(),
        ));
    }
    match tolerance {
        NumericTolerance::Exact => Ok(actual == expected),
        NumericTolerance::Absolute { epsilon } => {
            validate_nonnegative_finite("absolute epsilon", *epsilon)?;
            Ok((actual - expected).abs() <= *epsilon)
        }
        NumericTolerance::Relative { fraction } => {
            validate_nonnegative_finite("relative fraction", *fraction)?;
            Ok((actual - expected).abs() <= expected.abs() * *fraction)
        }
        NumericTolerance::SignificantFigures { digits } => {
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

fn text_is_correct(actual: &str, accepted: &[String], mode: TextMatchMode) -> bool {
    match mode {
        TextMatchMode::Exact => accepted.iter().any(|expected| actual == expected),
        TextMatchMode::CaseInsensitive => accepted
            .iter()
            .any(|expected| actual.to_lowercase() == expected.to_lowercase()),
        TextMatchMode::Normalized => {
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
    use question_model::answer::SelectionCardinality;
    use question_model::assignment_activity_rules::{AttemptPolicy, TimingPolicy};
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ChoiceOption;
    use question_model::taxonomy::License;
    use question_model::{
        QuestionId, QuestionMetadata, QuestionSource, QuestionVersionNumber, WorkspaceId,
    };
    use uuid::Uuid;

    fn choice(id: &str) -> ChoiceOption {
        ChoiceOption {
            id: ChoiceId::new(id),
            body: Vec::new(),
        }
    }

    fn question(response: ResponseDefinition, grading: GradingDefinition) -> QuestionDefinition {
        QuestionDefinition {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
            version_number: QuestionVersionNumber::new(2).expect("positive version"),
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            source: QuestionSource::Native {
                family: "grading-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Fixture".to_string(),
            }],
            response,
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading,
            metadata: QuestionMetadata {
                title: "Grading fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    fn all_or_nothing(response: ResponseDefinition) -> QuestionDefinition {
        question(response, GradingDefinition::AllOrNothing { points: 2.0 })
    }

    #[test]
    fn numeric_tolerances_use_inclusive_boundaries() {
        let cases = [
            (NumericTolerance::Exact, 10.0, 10.0, true),
            (
                NumericTolerance::Absolute { epsilon: 0.5 },
                10.5,
                10.0,
                true,
            ),
            (
                NumericTolerance::Relative { fraction: 0.1 },
                -11.0,
                -10.0,
                true,
            ),
            (
                NumericTolerance::SignificantFigures { digits: 3 },
                1_234.0,
                1_230.0,
                true,
            ),
            (
                NumericTolerance::SignificantFigures { digits: 3 },
                1_219.0,
                1_230.0,
                false,
            ),
        ];

        for (tolerance, actual, expected, correct) in cases {
            let result = grade(
                &all_or_nothing(ResponseDefinition::Numeric {
                    tolerance,
                    unit: None,
                }),
                &StudentResponse::Numeric { value: actual },
                Some(&AnswerKey::Numeric { expected }),
            )
            .expect("numeric case should grade");
            assert_eq!(
                result,
                GradeOutcome::Graded(AttemptResult {
                    correct,
                    points_earned: if correct { 2.0 } else { 0.0 },
                    points_possible: 2.0,
                })
            );
        }
    }

    #[test]
    fn choice_text_and_ordering_use_their_declared_comparisons() {
        let choice_question = all_or_nothing(ResponseDefinition::MultipleChoice {
            choices: vec![choice("a"), choice("b"), choice("c")],
            selection: SelectionCardinality::Exactly { count: 2 },
        });
        assert!(matches!(
            grade(
                &choice_question,
                &StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("c"), ChoiceId::new("a")],
                },
                Some(&AnswerKey::MultipleChoice {
                    correct: BTreeSet::from([ChoiceId::new("a"), ChoiceId::new("c")]),
                }),
            ),
            Ok(GradeOutcome::Graded(AttemptResult { correct: true, .. }))
        ));

        let text_question = all_or_nothing(ResponseDefinition::ShortText {
            match_mode: TextMatchMode::Normalized,
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
            Ok(GradeOutcome::Graded(AttemptResult { correct: true, .. }))
        ));

        let ordering_question = all_or_nothing(ResponseDefinition::Ordering {
            items: vec![choice("first"), choice("second")],
        });
        assert!(matches!(
            grade(
                &ordering_question,
                &StudentResponse::Ordering {
                    order: vec![ChoiceId::new("first"), ChoiceId::new("second")],
                },
                Some(&AnswerKey::Ordering {
                    correct: vec![ChoiceId::new("first"), ChoiceId::new("second")],
                }),
            ),
            Ok(GradeOutcome::Graded(AttemptResult { correct: true, .. }))
        ));
    }

    #[test]
    fn accepted_multiple_choice_grade_yields_only_its_eligible_choice_counts() {
        let question = all_or_nothing(ResponseDefinition::MultipleChoice {
            choices: vec![choice("a"), choice("b"), choice("c")],
            selection: SelectionCardinality::Exactly { count: 2 },
        });
        let response = StudentResponse::MultipleChoice {
            selected: vec![ChoiceId::new("a"), ChoiceId::new("c")],
        };
        let outcome = grade(
            &question,
            &response,
            Some(&AnswerKey::MultipleChoice {
                correct: BTreeSet::from([ChoiceId::new("a")]),
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
                .map(ChoiceId::as_str)
                .collect::<Vec<_>>(),
            vec!["a", "c"]
        );
    }

    #[test]
    fn malformed_backend_inputs_fail_instead_of_guessing() {
        let numeric = all_or_nothing(ResponseDefinition::Numeric {
            tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
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
                ResponseFormatViolation::ResponseKindMismatch,
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
            ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            GradingDefinition::AllOrNothing { points: -1.0 },
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

        let invalid_tolerance = all_or_nothing(ResponseDefinition::Numeric {
            tolerance: NumericTolerance::SignificantFigures { digits: 0 },
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
    fn ungraded_partial_credit_and_file_upload_capabilities_are_explicit() {
        let ungraded = question(
            ResponseDefinition::ShortText {
                match_mode: TextMatchMode::Exact,
                max_length: 10,
            },
            GradingDefinition::Ungraded,
        );
        assert_eq!(
            grade(
                &ungraded,
                &StudentResponse::ShortText {
                    text: "practice".to_string(),
                },
                None,
            ),
            Ok(GradeOutcome::Ungraded)
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
            ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            GradingDefinition::PartialCredit { points: 2.0 },
        );
        assert_eq!(
            grade(
                &partial,
                &StudentResponse::Numeric { value: 1.0 },
                Some(&AnswerKey::Numeric { expected: 1.0 }),
            ),
            Err(GradingError::PartialCreditRequiresBackend)
        );

        let upload = all_or_nothing(ResponseDefinition::FileUpload {
            max_bytes: 1_000,
            accepted_extensions: vec!["pdf".to_string()],
        });
        assert_eq!(
            grade(
                &upload,
                &StudentResponse::FileUpload {
                    object_key: "workspace/object".to_string(),
                },
                None,
            ),
            Err(GradingError::FileUploadRequiresDeterministicGrader)
        );
    }
}
