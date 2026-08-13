use super::*;

#[cfg(feature = "postgres")]
pub(super) fn course_membership_role_name(role: CourseMembershipRole) -> &'static str {
    match role {
        CourseMembershipRole::Student => "student",
        CourseMembershipRole::Instructor => "instructor",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn assignment_delivery_state_name(
    state: question_model::AssignmentDeliveryState,
) -> &'static str {
    match state {
        question_model::AssignmentDeliveryState::Active => "active",
        question_model::AssignmentDeliveryState::Retired => "retired",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn completion_policy_columns(
    policy: CompletionRequirement,
) -> (&'static str, Option<String>) {
    match policy {
        CompletionRequirement::AnswerAll => ("answer_all", None),
        CompletionRequirement::AllCorrect => ("all_correct", None),
        CompletionRequirement::ScoreAtLeast { fraction } => {
            ("score_at_least", Some(fraction.to_string()))
        }
    }
}

#[cfg(feature = "postgres")]
pub(super) fn grade_policy_name(policy: GradePolicy) -> &'static str {
    match policy {
        GradePolicy::First => "first",
        GradePolicy::Latest => "last",
        GradePolicy::Highest => "highest",
        GradePolicy::InstructorSelected => "instructor_selected",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn continued_practice_columns(
    policy: ContinuedPractice,
) -> Result<(&'static str, Option<i32>), StoreError> {
    match policy {
        ContinuedPractice::Unlimited => Ok(("unlimited", None)),
        ContinuedPractice::Closed => Ok(("closed", None)),
        ContinuedPractice::Capped {
            max_additional_runs,
        } => Ok((
            "capped",
            Some(i32::try_from(max_additional_runs).map_err(|_| {
                StoreError::InvalidRecord("continued-practice limit is too large".to_string())
            })?),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn variation_policy_name(policy: VariationPolicy) -> &'static str {
    match policy {
        VariationPolicy::NewSeeds => "new_seeds",
        VariationPolicy::SelectedProblemVariants => "selected_problem_variants",
        VariationPolicy::FullRegeneration => "full_regeneration",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn assignment_scoring_mode_name(
    mode: question_model::AssignmentScoringMode,
) -> &'static str {
    match mode {
        question_model::AssignmentScoringMode::Normal => "normal",
        question_model::AssignmentScoringMode::FullCredit => "full_credit",
        question_model::AssignmentScoringMode::ExtraCredit => "extra_credit",
        question_model::AssignmentScoringMode::Excluded => "excluded",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn selection_ordering_name(ordering: question_model::SelectionOrdering) -> &'static str {
    match ordering {
        question_model::SelectionOrdering::CandidateOrder => "candidate_order",
        question_model::SelectionOrdering::Randomized => "randomized",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_assignment_delivery_state(
    value: &str,
) -> Result<AssignmentDeliveryState, StoreError> {
    match value {
        "active" => Ok(AssignmentDeliveryState::Active),
        "retired" => Ok(AssignmentDeliveryState::Retired),
        _ => Err(invalid_stored_assignment_value("delivery state", value)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_assignment_scoring_mode(
    value: &str,
) -> Result<AssignmentScoringMode, StoreError> {
    match value {
        "normal" => Ok(AssignmentScoringMode::Normal),
        "full_credit" => Ok(AssignmentScoringMode::FullCredit),
        "extra_credit" => Ok(AssignmentScoringMode::ExtraCredit),
        "excluded" => Ok(AssignmentScoringMode::Excluded),
        _ => Err(invalid_stored_assignment_value("scoring mode", value)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_selection_ordering(value: &str) -> Result<SelectionOrdering, StoreError> {
    match value {
        "candidate_order" => Ok(SelectionOrdering::CandidateOrder),
        "randomized" => Ok(SelectionOrdering::Randomized),
        _ => Err(invalid_stored_assignment_value("selection ordering", value)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_completion_policy(
    policy: &str,
    threshold: Option<String>,
) -> Result<CompletionRequirement, StoreError> {
    match (policy, threshold) {
        ("answer_all", None) => Ok(CompletionRequirement::AnswerAll),
        ("all_correct", None) => Ok(CompletionRequirement::AllCorrect),
        ("score_at_least", Some(value)) => {
            let fraction = value
                .parse::<f64>()
                .map_err(|_| invalid_stored_assignment_value("completion threshold", &value))?;
            Ok(CompletionRequirement::ScoreAtLeast { fraction })
        }
        _ => Err(invalid_stored_assignment_value("completion policy", policy)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_grade_policy(value: &str) -> Result<GradePolicy, StoreError> {
    match value {
        "first" => Ok(GradePolicy::First),
        "last" => Ok(GradePolicy::Latest),
        "highest" => Ok(GradePolicy::Highest),
        "instructor_selected" => Ok(GradePolicy::InstructorSelected),
        _ => Err(invalid_stored_assignment_value(
            "attempt selection policy",
            value,
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_continued_practice(
    policy: &str,
    limit: Option<i32>,
) -> Result<ContinuedPractice, StoreError> {
    match (policy, limit) {
        ("unlimited", None) => Ok(ContinuedPractice::Unlimited),
        ("closed", None) => Ok(ContinuedPractice::Closed),
        ("capped", Some(limit)) => Ok(ContinuedPractice::Capped {
            max_additional_runs: u32::try_from(limit).map_err(|_| {
                invalid_stored_assignment_value("continued-practice limit", &limit.to_string())
            })?,
        }),
        _ => Err(invalid_stored_assignment_value(
            "continued-practice policy",
            policy,
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_variation_policy(value: &str) -> Result<VariationPolicy, StoreError> {
    match value {
        "new_seeds" => Ok(VariationPolicy::NewSeeds),
        "selected_problem_variants" => Ok(VariationPolicy::SelectedProblemVariants),
        "full_regeneration" => Ok(VariationPolicy::FullRegeneration),
        _ => Err(invalid_stored_assignment_value("variation policy", value)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn invalid_stored_assignment_value(field: &str, value: &str) -> StoreError {
    StoreError::Unavailable(format!("stored assignment {field} is invalid: {value}"))
}

#[cfg(feature = "postgres")]
pub(super) fn decode_scoring_generation(row: &PgRow) -> Result<ScoringGeneration, StoreError> {
    let value: i64 = row.try_get("scoring_generation").map_err(map_sqlx_error)?;
    u64::try_from(value)
        .ok()
        .and_then(ScoringGeneration::new)
        .ok_or_else(|| invalid_stored_assignment_value("scoring generation", &value.to_string()))
}

#[cfg(feature = "postgres")]
pub(super) fn decode_scoring_status(row: &PgRow) -> Result<ScoringStatus, StoreError> {
    let value: String = row.try_get("scoring_status").map_err(map_sqlx_error)?;
    match value.as_str() {
        "current" => Ok(ScoringStatus::Current),
        "recalculating" => Ok(ScoringStatus::Recalculating),
        "failed" => Ok(ScoringStatus::Failed),
        _ => Err(invalid_stored_assignment_value("scoring status", &value)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_course_membership_role(
    value: &str,
) -> Result<CourseMembershipRole, StoreError> {
    match value {
        "student" => Ok(CourseMembershipRole::Student),
        "instructor" => Ok(CourseMembershipRole::Instructor),
        _ => Err(StoreError::Unavailable(format!(
            "stored course membership role is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_question_backend(value: &str) -> Result<QuestionBackend, StoreError> {
    match value {
        "native" => Ok(QuestionBackend::Native),
        "webwork" => Ok(QuestionBackend::Webwork),
        "qti" => Ok(QuestionBackend::Qti),
        "h5p" => Ok(QuestionBackend::H5p),
        "imathas" => Ok(QuestionBackend::Imathas),
        _ => Err(StoreError::Unavailable(format!(
            "stored question backend is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn publication_scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Institution => "institution",
        PublicationScope::Public => "public",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_publication_scope(value: &str) -> Result<PublicationScope, StoreError> {
    match value {
        "institution" => Ok(PublicationScope::Institution),
        "public" => Ok(PublicationScope::Public),
        _ => Err(StoreError::Unavailable(format!(
            "stored publication scope is invalid: {value}"
        ))),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn catalog_lifecycle_parts(
    lifecycle: &CatalogLifecycle,
) -> (&'static str, Option<&str>) {
    match lifecycle {
        CatalogLifecycle::Published => ("published", None),
        CatalogLifecycle::Deprecated { reason } => ("deprecated", Some(reason.as_str())),
        CatalogLifecycle::Archived { reason } => ("archived", Some(reason.as_str())),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn parse_catalog_lifecycle(
    lifecycle: &str,
    reason: Option<String>,
) -> Result<CatalogLifecycle, StoreError> {
    match (lifecycle, reason) {
        ("published", None) => Ok(CatalogLifecycle::Published),
        ("deprecated", Some(reason)) => Ok(CatalogLifecycle::Deprecated {
            reason: validated_deprecation_reason(reason)?,
        }),
        ("archived", Some(reason)) => Ok(CatalogLifecycle::Archived {
            reason: validated_deprecation_reason(reason)?,
        }),
        _ => Err(StoreError::Unavailable(
            "stored catalog lifecycle and reason disagree".to_string(),
        )),
    }
}
