use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use learning_data_access::{
    AuthoritativeTimeStore, CatalogStore, CourseInvitationDeliveryStore, CourseItemAnalysisStore,
    CourseRecordsAccessStore, CourseRosterStore, ManualGradeExportStore, SessionStore, Store,
};
use serde::{Deserialize, Serialize};

use super::assignments::{
    add_assignment_item, create_assignment, get_assignment, get_assignment_summary,
    get_learner_assignment, remove_assignment_item, replace_assignment_item_question,
    update_assignment,
};
use super::invitation_capability::CourseInvitationIssuer;
use super::queries::{create_course, get_course, list_assignments, list_courses, list_gradebook};
use super::roster::{LocalTeachingRosterDirectory, roster_router};

pub(super) const DEFAULT_PAGE_SIZE: u16 = 50;
pub(super) const MAX_COURSE_BODY_BYTES: usize = 64 * 1_024;

/// Builds the authenticated course and assignment route group.
pub fn router<S>(store: Arc<S>) -> Router
where
    S: Store
        + CatalogStore
        + CourseItemAnalysisStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + ManualGradeExportStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
{
    router_with_invitations_and_local_teaching(store, CourseInvitationIssuer::unavailable(), None)
}

/// Builds course routes with a configured server-only invitation issuer.
pub fn router_with_invitations<S>(store: Arc<S>, issuer: CourseInvitationIssuer) -> Router
where
    S: Store
        + CatalogStore
        + CourseItemAnalysisStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + ManualGradeExportStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
{
    router_with_invitations_and_local_teaching(store, issuer, None)
}

/// Builds the local-teaching course router with a server-owned learner directory.
/// Production composition always passes `None`, so this route cannot be mounted
/// outside the paired local authentication mode.
pub(crate) fn router_with_invitations_and_local_teaching<S>(
    store: Arc<S>,
    issuer: CourseInvitationIssuer,
    local_teaching_roster: Option<Arc<LocalTeachingRosterDirectory>>,
) -> Router
where
    S: Store
        + CatalogStore
        + CourseItemAnalysisStore
        + CourseRecordsAccessStore
        + CourseRosterStore
        + CourseInvitationDeliveryStore
        + ManualGradeExportStore
        + SessionStore
        + AuthoritativeTimeStore
        + 'static,
{
    let course_routes = Router::new()
        .route(
            "/api/courses",
            get(list_courses::<S>).post(create_course::<S>),
        )
        .route(
            "/api/courses/{course}/assignments",
            get(list_assignments::<S>).post(create_assignment::<S>),
        )
        .route("/api/courses/{course}/gradebook", get(list_gradebook::<S>))
        .route("/api/courses/{course}", get(get_course::<S>))
        .route("/api/assignments/{assignment}", get(get_assignment::<S>))
        .route(
            "/api/assignments/{assignment}/learner",
            get(get_learner_assignment::<S>),
        )
        .route(
            "/api/assignments/{assignment}/summary",
            get(get_assignment_summary::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}",
            put(update_assignment::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/items",
            post(add_assignment_item::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/items/{item}",
            delete(remove_assignment_item::<S>),
        )
        .route(
            "/api/courses/{course}/assignments/{assignment}/items/{item}/question",
            put(replace_assignment_item_question::<S>),
        )
        .layer(DefaultBodyLimit::max(MAX_COURSE_BODY_BYTES))
        .with_state(CourseRouteState {
            store: Arc::clone(&store),
        });
    course_routes.merge(roster_router(store, issuer, local_teaching_roster))
}

pub(super) struct CourseRouteState<S> {
    pub(super) store: Arc<S>,
}

impl<S> Clone for CourseRouteState<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CourseQuery {
    pub(super) cursor: Option<String>,
    pub(super) page_size: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateCourseRequest {
    pub(super) title: String,
    pub(super) term: question_model::CourseTerm,
}

pub(super) enum CreateCourseDecodeError {
    Invalid,
    Term(question_model::CourseTermValidationFailure),
}

pub(super) fn decode_course_create_request(
    value: serde_json::Value,
) -> Result<CreateCourseRequest, CreateCourseDecodeError> {
    let record = value.as_object().ok_or(CreateCourseDecodeError::Invalid)?;
    if record
        .keys()
        .any(|field| field != "title" && field != "term")
    {
        return Err(CreateCourseDecodeError::Invalid);
    }
    let term_value = record.get("term").ok_or_else(|| {
        CreateCourseDecodeError::Term(course_term_failure(
            question_model::CourseTermField::Term,
            question_model::CourseTermFailureReason::Required,
        ))
    })?;
    let term_record = term_value.as_object().ok_or_else(|| {
        CreateCourseDecodeError::Term(course_term_failure(
            question_model::CourseTermField::Term,
            question_model::CourseTermFailureReason::Required,
        ))
    })?;
    if term_record
        .keys()
        .any(|field| field != "startDate" && field != "endDate" && field != "timeZone")
    {
        return Err(CreateCourseDecodeError::Invalid);
    }
    let title = record
        .get("title")
        .and_then(serde_json::Value::as_str)
        .ok_or(CreateCourseDecodeError::Invalid)?;
    if title.trim().is_empty() {
        return Err(CreateCourseDecodeError::Invalid);
    }
    let start_date = required_term_string(
        term_record,
        "startDate",
        question_model::CourseTermField::StartDate,
    )?;
    let end_date = required_term_string(
        term_record,
        "endDate",
        question_model::CourseTermField::EndDate,
    )?;
    let time_zone = required_term_string(
        term_record,
        "timeZone",
        question_model::CourseTermField::TimeZone,
    )?;
    let term = question_model::CourseTerm::from_parts(start_date, end_date, time_zone).map_err(
        |error| {
            let (field, reason) = match error {
                question_model::CourseTermError::StartDate => (
                    question_model::CourseTermField::StartDate,
                    question_model::CourseTermFailureReason::InvalidCalendarDate,
                ),
                question_model::CourseTermError::EndDate => (
                    question_model::CourseTermField::EndDate,
                    question_model::CourseTermFailureReason::InvalidCalendarDate,
                ),
                question_model::CourseTermError::EndBeforeStart => (
                    question_model::CourseTermField::EndDate,
                    question_model::CourseTermFailureReason::EndBeforeStart,
                ),
                question_model::CourseTermError::TimeZone => (
                    question_model::CourseTermField::TimeZone,
                    question_model::CourseTermFailureReason::UnknownIanaTimeZone,
                ),
            };
            CreateCourseDecodeError::Term(course_term_failure(field, reason))
        },
    )?;
    Ok(CreateCourseRequest {
        title: title.to_string(),
        term,
    })
}

fn required_term_string<'a>(
    record: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
    field: question_model::CourseTermField,
) -> Result<&'a str, CreateCourseDecodeError> {
    let Some(value) = record.get(name) else {
        return Err(CreateCourseDecodeError::Term(course_term_failure(
            field,
            question_model::CourseTermFailureReason::Required,
        )));
    };
    value.as_str().ok_or_else(|| {
        let reason = match field {
            question_model::CourseTermField::StartDate
            | question_model::CourseTermField::EndDate => {
                question_model::CourseTermFailureReason::InvalidCalendarDate
            }
            question_model::CourseTermField::TimeZone => {
                question_model::CourseTermFailureReason::UnknownIanaTimeZone
            }
            question_model::CourseTermField::Term => {
                question_model::CourseTermFailureReason::Required
            }
        };
        CreateCourseDecodeError::Term(course_term_failure(field, reason))
    })
}

fn course_term_failure(
    field: question_model::CourseTermField,
    reason: question_model::CourseTermFailureReason,
) -> question_model::CourseTermValidationFailure {
    let message = match (field, reason) {
        (question_model::CourseTermField::Term, _) => "Enter the course term dates and time zone.",
        (_, question_model::CourseTermFailureReason::Required) => match field {
            question_model::CourseTermField::StartDate => "Enter a course start date.",
            question_model::CourseTermField::EndDate => "Enter a course end date.",
            question_model::CourseTermField::TimeZone => "Enter an IANA time zone.",
            question_model::CourseTermField::Term => "Enter the course term dates and time zone.",
        },
        (_, question_model::CourseTermFailureReason::InvalidCalendarDate) => {
            "Enter a valid date in YYYY-MM-DD format."
        }
        (_, question_model::CourseTermFailureReason::EndBeforeStart) => {
            "Choose an end date on or after the start date."
        }
        (_, question_model::CourseTermFailureReason::UnknownIanaTimeZone) => {
            "Choose a valid IANA time zone such as America/Chicago."
        }
    };
    question_model::CourseTermValidationFailure {
        error: question_model::CourseTermFailureCode::CourseTermInvalid,
        field,
        reason,
        message: message.to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub(super) struct CreateAssignmentRequest {
    pub(super) title: String,
    pub(super) question_ids: Vec<question_model::QuestionId>,
    pub(super) disclosure_policy: question_model::LearnerDisclosurePolicy,
    pub(super) policies: question_model::RunPolicies,
    /// Whole-run timing is always an explicit instructor decision. `null`
    /// within the object deliberately means Untimed.
    pub(super) assignment_timing: question_model::AssignmentRunTiming,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateAssignmentRequest {
    pub(super) title: String,
    pub(super) items: Vec<AssignmentItemUpdateRequest>,
    pub(super) disclosure_policy: question_model::LearnerDisclosurePolicy,
    pub(super) policies: question_model::RunPolicies,
    pub(super) assignment_timing: question_model::AssignmentRunTiming,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AssignmentItemUpdateRequest {
    pub(super) id: question_model::AssignmentItemId,
    pub(super) question_id: question_model::QuestionId,
    pub(super) position: u32,
    pub(super) points_possible: question_model::PointValue,
    pub(super) delivery_state: question_model::AssignmentDeliveryState,
    pub(super) scoring_mode: question_model::AssignmentScoringMode,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AddAssignmentItemRequest {
    pub(super) question_id: question_model::QuestionId,
    pub(super) position: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReplaceAssignmentItemQuestionRequest {
    pub(super) question_id: question_model::QuestionId,
}

/// Rejects unknown fields at every level by comparing the request to the
/// canonical wire form of the typed model, mirroring the workspace boundary.
pub(super) fn strict_assignment_request<T>(value: serde_json::Value) -> Result<T, ()>
where
    T: serde::de::DeserializeOwned + Serialize,
{
    let request = serde_json::from_value(value.clone()).map_err(|_| ())?;
    if serde_json::to_value(&request).map_err(|_| ())? == value {
        Ok(request)
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explicit_request() -> serde_json::Value {
        serde_json::to_value(CreateAssignmentRequest {
            title: "Mastery".to_string(),
            question_ids: Vec::new(),
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: question_model::RunPolicies {
                completion: question_model::CompletionRequirement::AllCorrect,
                grade: question_model::GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
            assignment_timing: question_model::AssignmentRunTiming::default(),
        })
        .expect("request fixture serializes")
    }

    #[test]
    fn assignment_timing_requires_an_object_with_nullable_member() {
        let explicit = strict_assignment_request::<CreateAssignmentRequest>(explicit_request())
            .expect("explicit null member is an untimed editor choice");
        assert_eq!(
            explicit.assignment_timing,
            question_model::AssignmentRunTiming::default()
        );

        let mut omitted = explicit_request();
        omitted
            .as_object_mut()
            .expect("request object")
            .remove("assignmentTiming");
        assert!(strict_assignment_request::<CreateAssignmentRequest>(omitted).is_err());

        let mut invalid = explicit_request();
        invalid["assignmentTiming"] = serde_json::Value::Null;
        assert!(strict_assignment_request::<CreateAssignmentRequest>(invalid).is_err());
    }

    #[test]
    fn assignment_disclosure_policy_is_required_and_rejects_unknown_members() {
        let mut omitted = explicit_request();
        omitted
            .as_object_mut()
            .expect("request object")
            .remove("disclosurePolicy");
        assert!(strict_assignment_request::<CreateAssignmentRequest>(omitted).is_err());

        let mut unknown = explicit_request();
        unknown["disclosurePolicy"]["surprise"] = serde_json::json!("never");
        assert!(strict_assignment_request::<CreateAssignmentRequest>(unknown).is_err());

        let update = serde_json::json!({
            "title": "Mastery",
            "items": [],
            "disclosurePolicy": question_model::LearnerDisclosurePolicy::default(),
            "policies": question_model::RunPolicies {
                completion: question_model::CompletionRequirement::AllCorrect,
                grade: question_model::GradePolicy::Highest,
                continued_practice: question_model::ContinuedPractice::Unlimited,
                variation: question_model::VariationPolicy::NewSeeds,
            },
            "assignmentTiming": question_model::AssignmentRunTiming::default(),
        });
        assert!(strict_assignment_request::<UpdateAssignmentRequest>(update.clone()).is_ok());
        let mut omitted_update = update;
        omitted_update
            .as_object_mut()
            .expect("update request object")
            .remove("disclosurePolicy");
        assert!(strict_assignment_request::<UpdateAssignmentRequest>(omitted_update).is_err());
    }
}
