//! Pure aggregation of selected assignment scores into one course grade.

use std::collections::HashSet;

use question_model::{
    AssignmentId, AssignmentPointValue, AssignmentScoringState, CourseGradeMode,
    CourseGradeRoundingRule, CourseGradeScheme, CourseGradeSchemeError, GradeCategory,
    GradeCategoryId,
};

/// One assignment's already-selected current score and grading configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseGradeAssignment {
    pub assignment: AssignmentId,
    pub position: u32,
    pub included: bool,
    pub category: Option<GradeCategoryId>,
    pub selected_current_score: Option<f64>,
    pub points_possible: AssignmentPointValue,
    pub assignment_scoring_state: AssignmentScoringState,
}

/// Why a course score is intentionally unavailable instead of treated as zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseGradeUnavailableReason {
    NoIncludedAssignments,
    Recalculating,
    Failed,
    EmptyAfterDrop,
    ZeroPossiblePoints,
}

/// A calculated course grade or an explicit unavailable result.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseGradeOutcome {
    pub mode: CourseGradeMode,
    pub rounded_score: Option<f64>,
    pub total_earned: Option<f64>,
    pub total_possible: Option<f64>,
    pub letter: Option<String>,
    pub dropped_assignment_ids: Vec<AssignmentId>,
    pub unavailable_reason: Option<CourseGradeUnavailableReason>,
}

/// Inputs or configuration that cannot produce a trustworthy course grade.
#[derive(Debug, Clone, PartialEq)]
pub enum CourseGradeError {
    InvalidScheme(CourseGradeSchemeError),
    DuplicateAssignment {
        assignment: AssignmentId,
    },
    InvalidSelectedScore {
        assignment: AssignmentId,
        score: f64,
    },
    MissingCategory {
        assignment: AssignmentId,
    },
    UnknownCategory {
        assignment: AssignmentId,
        category: GradeCategoryId,
    },
}

impl std::fmt::Display for CourseGradeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "course grade calculation refused: {self:?}")
    }
}
impl std::error::Error for CourseGradeError {}

/// Calculates a course result from immutable configuration and selected scores.
pub fn calculate_course_grade(
    scheme: &CourseGradeScheme,
    assignments: &[CourseGradeAssignment],
) -> Result<CourseGradeOutcome, CourseGradeError> {
    scheme.validate().map_err(CourseGradeError::InvalidScheme)?;
    validate_assignments(assignments)?;
    if !assignments.iter().any(|assignment| assignment.included) {
        return Ok(unavailable(
            scheme,
            CourseGradeUnavailableReason::NoIncludedAssignments,
        ));
    }
    if let Some(reason) = unavailable_status(assignments) {
        return Ok(unavailable(scheme, reason));
    }
    match scheme.mode {
        CourseGradeMode::TotalPoints => total_points(scheme, assignments),
        CourseGradeMode::WeightedCategories => weighted_categories(scheme, assignments),
    }
}

fn validate_assignments(assignments: &[CourseGradeAssignment]) -> Result<(), CourseGradeError> {
    let mut ids = HashSet::new();
    for assignment in assignments {
        if !ids.insert(assignment.assignment) {
            return Err(CourseGradeError::DuplicateAssignment {
                assignment: assignment.assignment,
            });
        }
        if assignment.included
            && let Some(score) = assignment.selected_current_score
            && (!score.is_finite() || !(-1000.0..=1000.0).contains(&score))
        {
            return Err(CourseGradeError::InvalidSelectedScore {
                assignment: assignment.assignment,
                score,
            });
        }
    }
    Ok(())
}

fn unavailable_status(
    assignments: &[CourseGradeAssignment],
) -> Option<CourseGradeUnavailableReason> {
    let mut recalculating = false;
    for assignment in assignments.iter().filter(|assignment| assignment.included) {
        match assignment.assignment_scoring_state {
            AssignmentScoringState::Current => {}
            AssignmentScoringState::Recalculating => recalculating = true,
            AssignmentScoringState::Failed => return Some(CourseGradeUnavailableReason::Failed),
        }
    }
    recalculating.then_some(CourseGradeUnavailableReason::Recalculating)
}

fn total_points(
    scheme: &CourseGradeScheme,
    assignments: &[CourseGradeAssignment],
) -> Result<CourseGradeOutcome, CourseGradeError> {
    let (earned, possible_scaled) = assignments
        .iter()
        .filter(|assignment| assignment.included)
        .fold((0.0, 0_i64), |(earned, possible), assignment| {
            let points = assignment.points_possible.scaled() as f64 / 10_000.0;
            let score = assignment.selected_current_score.unwrap_or(0.0);
            if assignment.points_possible == AssignmentPointValue::ZERO {
                (earned + score, possible)
            } else {
                (
                    earned + score * points,
                    possible + assignment.points_possible.scaled(),
                )
            }
        });
    if possible_scaled == 0 {
        return Ok(unavailable(
            scheme,
            CourseGradeUnavailableReason::ZeroPossiblePoints,
        ));
    }
    Ok(finish(
        scheme,
        earned / (possible_scaled as f64 / 10_000.0),
        Some(earned),
        Some(possible_scaled as f64 / 10_000.0),
        Vec::new(),
    ))
}

fn weighted_categories(
    scheme: &CourseGradeScheme,
    assignments: &[CourseGradeAssignment],
) -> Result<CourseGradeOutcome, CourseGradeError> {
    validate_weighted_assignments(assignments, &scheme.categories)?;
    let mut dropped = Vec::new();
    let mut score = 0.0;
    for category in &scheme.categories {
        let mut members: Vec<_> = assignments
            .iter()
            .filter(|assignment| assignment.included)
            .filter(|assignment| assignment.category.as_ref() == Some(&category.id))
            .collect();
        members.sort_by(|left, right| {
            let left_score = left.selected_current_score.unwrap_or(0.0);
            let right_score = right.selected_current_score.unwrap_or(0.0);
            left_score
                .partial_cmp(&right_score)
                .expect("included scores were validated as finite")
                .then_with(|| right.position.cmp(&left.position))
                .then_with(|| left.assignment.cmp(&right.assignment))
        });
        let drop_count = usize::try_from(category.drop_lowest).expect("u32 fits usize");
        if members.is_empty() {
            return Ok(unavailable(
                scheme,
                CourseGradeUnavailableReason::ZeroPossiblePoints,
            ));
        }
        if drop_count >= members.len() {
            return Ok(unavailable(
                scheme,
                CourseGradeUnavailableReason::EmptyAfterDrop,
            ));
        }
        for assignment in members.drain(..drop_count) {
            dropped.push(assignment.assignment);
        }
        let (earned, possible) =
            members
                .iter()
                .fold((0.0, 0.0), |(earned, possible), assignment| {
                    let point_value = assignment.points_possible.scaled() as f64 / 10_000.0;
                    let selected = assignment.selected_current_score.unwrap_or(0.0);
                    if assignment.points_possible == AssignmentPointValue::ZERO {
                        (earned + selected, possible)
                    } else {
                        (earned + selected * point_value, possible + point_value)
                    }
                });
        if possible == 0.0 {
            return Ok(unavailable(
                scheme,
                CourseGradeUnavailableReason::ZeroPossiblePoints,
            ));
        }
        score += (earned / possible) * f64::from(category.weight_basis_points) / 10_000.0;
    }
    Ok(finish(scheme, score, None, None, dropped))
}

fn validate_weighted_assignments(
    assignments: &[CourseGradeAssignment],
    categories: &[GradeCategory],
) -> Result<(), CourseGradeError> {
    let known: HashSet<_> = categories.iter().map(|category| category.id).collect();
    for assignment in assignments.iter().filter(|assignment| assignment.included) {
        match assignment.category {
            None => {
                return Err(CourseGradeError::MissingCategory {
                    assignment: assignment.assignment,
                });
            }
            Some(value) if !known.contains(&value) => {
                return Err(CourseGradeError::UnknownCategory {
                    assignment: assignment.assignment,
                    category: value,
                });
            }
            Some(_) => {}
        }
    }
    Ok(())
}

fn finish(
    scheme: &CourseGradeScheme,
    score: f64,
    earned: Option<f64>,
    possible: Option<f64>,
    dropped_assignment_ids: Vec<AssignmentId>,
) -> CourseGradeOutcome {
    let rounded_score = round_score(score);
    let letter = scheme
        .letter_bands
        .iter()
        .find(|band| rounded_score >= f64::from(band.minimum_basis_points) / 10_000.0)
        .map(|band| band.label.as_str().to_owned());
    CourseGradeOutcome {
        mode: scheme.mode,
        rounded_score: Some(rounded_score),
        total_earned: earned,
        total_possible: possible,
        letter,
        dropped_assignment_ids,
        unavailable_reason: None,
    }
}

fn unavailable(
    scheme: &CourseGradeScheme,
    reason: CourseGradeUnavailableReason,
) -> CourseGradeOutcome {
    CourseGradeOutcome {
        mode: scheme.mode,
        rounded_score: None,
        total_earned: None,
        total_possible: None,
        letter: None,
        dropped_assignment_ids: Vec::new(),
        unavailable_reason: Some(reason),
    }
}

fn round_score(value: f64) -> f64 {
    match CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero {
        CourseGradeRoundingRule::FourDecimalPlacesHalfAwayFromZero => {
            let scaled = value * 10_000.0;
            let rounded = if scaled.is_sign_negative() {
                (scaled - 0.5).ceil()
            } else {
                (scaled + 0.5).floor()
            };
            rounded / 10_000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{GradeCategoryTitle, LetterBand, LetterBandLabel};
    use uuid::Uuid;

    fn assignment(
        id: u128,
        position: u32,
        points: u32,
        score: Option<f64>,
    ) -> CourseGradeAssignment {
        CourseGradeAssignment {
            assignment: AssignmentId::from_uuid(Uuid::from_u128(id)),
            position,
            included: true,
            category: None,
            selected_current_score: score,
            points_possible: AssignmentPointValue::from_whole(points),
            assignment_scoring_state: AssignmentScoringState::Current,
        }
    }
    fn category(id: u128, position: u32, drop_lowest: u32) -> GradeCategory {
        GradeCategory {
            id: GradeCategoryId::from_uuid(Uuid::from_u128(id)),
            title: GradeCategoryTitle::new("Lab").expect("title"),
            position,
            weight_basis_points: 10_000,
            drop_lowest,
        }
    }
    fn total() -> CourseGradeScheme {
        CourseGradeScheme {
            mode: CourseGradeMode::TotalPoints,
            rounding: CourseGradeRoundingRule::default(),
            categories: Vec::new(),
            letter_bands: vec![LetterBand {
                label: LetterBandLabel::new("A").expect("label"),
                minimum_basis_points: 9000,
            }],
        }
    }
    fn weighted(drop_lowest: u32) -> CourseGradeScheme {
        CourseGradeScheme {
            mode: CourseGradeMode::WeightedCategories,
            rounding: CourseGradeRoundingRule::default(),
            categories: vec![category(11, 0, drop_lowest)],
            letter_bands: Vec::new(),
        }
    }
    fn lab(id: u128, position: u32, points: u32, score: Option<f64>) -> CourseGradeAssignment {
        let mut item = assignment(id, position, points, score);
        item.category = Some(GradeCategoryId::from_uuid(Uuid::from_u128(11)));
        item
    }

    #[test]
    fn total_points_uses_unequal_points_and_missing_score_is_zero() {
        let result = calculate_course_grade(
            &total(),
            &[assignment(1, 0, 10, Some(1.0)), assignment(2, 1, 90, None)],
        )
        .expect("grade");
        assert_eq!(result.rounded_score, Some(0.1));
    }
    #[test]
    fn excluded_nonfinite_and_out_of_range_scores_cannot_break_a_grade() {
        let mut ignored_nan = assignment(2, 1, 100, Some(f64::NAN));
        ignored_nan.included = false;
        let mut ignored_large = assignment(3, 2, 100, Some(1000.1));
        ignored_large.included = false;
        let result = calculate_course_grade(
            &total(),
            &[assignment(1, 0, 100, Some(1.2)), ignored_nan, ignored_large],
        )
        .expect("grade");
        assert_eq!(result.rounded_score, Some(1.2));
    }
    #[test]
    fn selected_score_range_accepts_both_boundaries() {
        let positive = calculate_course_grade(&total(), &[assignment(1, 0, 1, Some(1000.0))])
            .expect("positive boundary");
        let negative = calculate_course_grade(&total(), &[assignment(2, 0, 1, Some(-1000.0))])
            .expect("negative boundary");
        assert_eq!(positive.rounded_score, Some(1000.0));
        assert_eq!(negative.rounded_score, Some(-1000.0));
    }
    #[test]
    fn zero_possible_assignment_is_extra_credit_only() {
        let result = calculate_course_grade(
            &total(),
            &[
                assignment(1, 0, 100, Some(1.0)),
                assignment(2, 1, 0, Some(5.0)),
            ],
        )
        .expect("grade");
        assert_eq!(result.rounded_score, Some(1.05));
    }
    #[test]
    fn total_points_without_included_or_possible_points_are_unavailable() {
        let mut excluded = assignment(1, 0, 1, Some(1.0));
        excluded.included = false;
        let no_included = calculate_course_grade(&total(), &[excluded]).expect("outcome");
        assert_eq!(
            no_included.unavailable_reason,
            Some(CourseGradeUnavailableReason::NoIncludedAssignments)
        );
        let zero_possible =
            calculate_course_grade(&total(), &[assignment(2, 0, 0, Some(5.0))]).expect("outcome");
        assert_eq!(
            zero_possible.unavailable_reason,
            Some(CourseGradeUnavailableReason::ZeroPossiblePoints)
        );
    }
    #[test]
    fn weighted_categories_are_point_weighted_and_exact() {
        let result = calculate_course_grade(
            &weighted(0),
            &[lab(1, 0, 10, Some(1.0)), lab(2, 1, 90, Some(0.0))],
        )
        .expect("grade");
        assert_eq!(result.rounded_score, Some(0.1));
    }
    #[test]
    fn drop_ties_use_later_position_then_assignment_id() {
        let later = calculate_course_grade(
            &weighted(1),
            &[lab(2, 0, 10, Some(0.5)), lab(1, 1, 10, Some(0.5))],
        )
        .expect("grade");
        assert_eq!(
            later.dropped_assignment_ids,
            vec![AssignmentId::from_uuid(Uuid::from_u128(1))]
        );
        let id = calculate_course_grade(
            &weighted(1),
            &[lab(2, 0, 10, Some(0.5)), lab(1, 0, 10, Some(0.5))],
        )
        .expect("grade");
        assert_eq!(
            id.dropped_assignment_ids,
            vec![AssignmentId::from_uuid(Uuid::from_u128(1))]
        );
    }
    #[test]
    fn missing_category_and_noncurrent_state_do_not_become_zero() {
        let error = calculate_course_grade(&weighted(0), &[assignment(1, 0, 1, Some(1.0))]);
        assert!(matches!(
            error,
            Err(CourseGradeError::MissingCategory { .. })
        ));
        let mut item = assignment(2, 0, 1, Some(1.0));
        item.assignment_scoring_state = AssignmentScoringState::Recalculating;
        let result = calculate_course_grade(&total(), &[item.clone()]).expect("outcome");
        assert_eq!(
            result.unavailable_reason,
            Some(CourseGradeUnavailableReason::Recalculating)
        );
        item.assignment_scoring_state = AssignmentScoringState::Failed;
        let failed = calculate_course_grade(&total(), &[item]).expect("outcome");
        assert_eq!(
            failed.unavailable_reason,
            Some(CourseGradeUnavailableReason::Failed)
        );
    }
    #[test]
    fn unknown_category_is_an_observable_error() {
        let mut item = assignment(1, 0, 1, Some(1.0));
        item.category = Some(GradeCategoryId::from_uuid(Uuid::from_u128(12)));
        assert!(matches!(
            calculate_course_grade(&weighted(0), &[item]),
            Err(CourseGradeError::UnknownCategory { .. })
        ));
    }
    #[test]
    fn empty_after_drop_is_unavailable() {
        let result =
            calculate_course_grade(&weighted(1), &[lab(1, 0, 1, Some(1.0))]).expect("outcome");
        assert_eq!(
            result.unavailable_reason,
            Some(CourseGradeUnavailableReason::EmptyAfterDrop)
        );
    }
    #[test]
    fn empty_category_with_drop_rule_is_unavailable_without_panicking() {
        let mut scheme = weighted(1);
        scheme.categories[0].weight_basis_points = 5_000;
        scheme.categories.push(GradeCategory {
            id: GradeCategoryId::from_uuid(Uuid::from_u128(12)),
            title: GradeCategoryTitle::new("Exam").expect("title"),
            position: 1,
            weight_basis_points: 5_000,
            drop_lowest: 0,
        });
        let mut exam = assignment(1, 0, 100, Some(1.0));
        exam.category = Some(GradeCategoryId::from_uuid(Uuid::from_u128(12)));

        let result = calculate_course_grade(&scheme, &[exam]).expect("outcome");
        assert_eq!(
            result.unavailable_reason,
            Some(CourseGradeUnavailableReason::ZeroPossiblePoints)
        );
    }
    #[test]
    fn rounding_is_final_only_and_bands_use_rounded_score() {
        let mut scheme = total();
        scheme.letter_bands[0].minimum_basis_points = 3334;
        let result =
            calculate_course_grade(&scheme, &[assignment(1, 0, 3, Some(0.33335))]).expect("grade");
        assert_eq!(
            (result.rounded_score, result.letter),
            (Some(0.3334), Some("A".to_string()))
        );
    }
    #[test]
    fn negative_half_boundary_rounds_away_from_zero() {
        let result = calculate_course_grade(&total(), &[assignment(1, 0, 1, Some(-0.33335))])
            .expect("grade");
        assert_eq!(result.rounded_score, Some(-0.3334));
    }
}
