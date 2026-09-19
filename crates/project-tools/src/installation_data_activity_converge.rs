//! Course, Assessment, and attempt convergence for the Live Demo activity.

use anyhow::{Context, Result, bail, ensure};
use question_model::{AssessmentAttemptId, AssessmentId, AssessmentType, CourseInstanceId};
use reqwest::{Method, StatusCode};
use serde_json::{Map, Value, json};

use super::http::ProductApi;
use super::response::response_from_presentation;
use super::{
    AttemptState, BrowserEndpoint, DemoGraph, JACK_SAVED_RESPONSE_COUNT,
    LIVE_DEMO_ASSESSMENT_TITLE, LIVE_DEMO_COURSE_LONG_NAME, LIVE_DEMO_COURSE_SHORT_NAME,
    LIVE_DEMO_QUESTION_COUNT, TemporarySession, TemporarySessions, closed_array_field,
    closed_object, closed_object_with_optional, expect_status, public_id,
};

pub(super) async fn converge(
    endpoint: &BrowserEndpoint,
    sessions: &TemporarySessions,
) -> Result<()> {
    let api = ProductApi::new(endpoint)?;
    let graph = resolve_graph(&api, sessions.session("elena")?).await?;
    ensure_avery_is_startable(&api, sessions.session("avery")?, &graph).await?;

    let mary = student_attempt_state(&api, sessions.session("mary")?, &graph).await?;
    let mary_attempt =
        prepare_attempt(&api, sessions.session("mary")?, &graph, mary, "Mary").await?;
    if let Some(attempt) = mary_attempt {
        save_responses(
            &api,
            sessions.session("mary")?,
            &attempt,
            LIVE_DEMO_QUESTION_COUNT,
        )
        .await?;
        submit_attempt(&api, sessions.session("mary")?, &attempt).await?;
    }

    let jack = student_attempt_state(&api, sessions.session("jack")?, &graph).await?;
    ensure!(
        jack != AttemptState::Completed,
        "Live Demo Jack Assessment Attempt cannot be converged"
    );
    let jack_attempt = prepare_attempt(&api, sessions.session("jack")?, &graph, jack, "Jack")
        .await?
        .context("Live Demo Jack Assessment Attempt is unavailable")?;
    save_responses(
        &api,
        sessions.session("jack")?,
        &jack_attempt,
        JACK_SAVED_RESPONSE_COUNT,
    )
    .await?;

    verify_complete_activity(&api, sessions, &graph, &jack_attempt).await
}

async fn resolve_graph(api: &ProductApi, instructor: &TemporarySession) -> Result<DemoGraph> {
    let courses = expect_status(
        api.request(
            instructor,
            "Course discovery",
            Method::GET,
            "/api/course-instances",
            None,
        )
        .await?,
        StatusCode::OK,
        "Course discovery",
    )?;
    let course_items = closed_array_field(&courses, &["items", "nextCursor"], "items", "Course")?;
    let courses = course_items
        .iter()
        .map(|item| {
            let object = closed_object(
                item,
                &[
                    "classification",
                    "lifecycleState",
                    "courseEditNumber",
                    "id",
                    "shortName",
                    "longName",
                    "term",
                    "theme",
                ],
                "Course",
            )?;
            Ok((object.get("shortName").and_then(Value::as_str)
                == Some(LIVE_DEMO_COURSE_SHORT_NAME)
                && object.get("longName").and_then(Value::as_str)
                    == Some(LIVE_DEMO_COURSE_LONG_NAME))
            .then_some(object))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    ensure!(
        courses.len() == 1,
        "Live Demo Course is missing or ambiguous"
    );
    let course = public_id::<CourseInstanceId>(courses[0].get("id"), "Course")?;

    let assessments = expect_status(
        api.request(
            instructor,
            "Assessment discovery",
            Method::GET,
            &format!("/api/course-instances/{course}/assessments"),
            None,
        )
        .await?,
        StatusCode::OK,
        "Assessment discovery",
    )?;
    let assessment_items = assessments
        .as_array()
        .context("Live Demo Assessment projection is invalid")?;
    let assessments = assessment_items
        .iter()
        .map(|item| {
            let object = closed_object(
                item,
                &[
                    "id",
                    "assessmentType",
                    "title",
                    "dueAt",
                    "displayTimeZone",
                    "status",
                    "editNumber",
                ],
                "Assessment",
            )?;
            Ok((object.get("assessmentType").and_then(Value::as_str)
                == Some(AssessmentType::PracticeQuestionAssignment.as_str())
                && object.get("title").and_then(Value::as_str) == Some(LIVE_DEMO_ASSESSMENT_TITLE)
                && object.get("status").and_then(Value::as_str) == Some("released"))
            .then_some(object))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    ensure!(
        assessments.len() == 1,
        "Live Demo Assessment is missing or ambiguous"
    );
    let assessment = public_id::<AssessmentId>(assessments[0].get("id"), "Assessment")?;
    Ok(DemoGraph { course, assessment })
}

async fn student_attempt_state(
    api: &ProductApi,
    student: &TemporarySession,
    graph: &DemoGraph,
) -> Result<AttemptState> {
    let landing = expect_status(
        api.request(
            student,
            "Student Assessment landing",
            Method::GET,
            &format!("/api/course-instances/{}/assessment-landing", graph.course),
            None,
        )
        .await?,
        StatusCode::OK,
        "Student Assessment landing",
    )?;
    let assessments = closed_array_field(&landing, &["assessments"], "assessments", "Student")?;
    let matches = assessments
        .iter()
        .filter_map(|item| {
            let object = closed_object_with_optional(
                item,
                &[
                    "id",
                    "title",
                    "assessmentType",
                    "decision",
                    "assessmentAttemptNumber",
                    "assessmentAttemptCompletion",
                    "canResumeAssessmentAttempt",
                    "gradedQuestionCount",
                    "savedQuestionCount",
                    "questionCount",
                ],
                &["assessmentScore"],
                "Student Assessment",
            )
            .ok()?;
            (object.get("id")?.as_str() == Some(graph.assessment.as_str())).then_some(object)
        })
        .collect::<Vec<_>>();
    ensure!(
        matches.len() == 1,
        "Live Demo Student Assessment is missing or ambiguous"
    );
    let assessment = matches[0];
    ensure!(
        assessment.get("questionCount").and_then(Value::as_u64) == Some(LIVE_DEMO_QUESTION_COUNT),
        "Live Demo Student Assessment question count is invalid"
    );
    match (
        assessment.get("assessmentAttemptNumber"),
        assessment.get("assessmentAttemptCompletion"),
    ) {
        (Some(Value::Null), Some(Value::Null)) => Ok(AttemptState::NotStarted),
        (Some(number), Some(Value::String(completion)))
            if number.as_u64().is_some_and(|value| value > 0) && completion == "inProgress" =>
        {
            Ok(AttemptState::InProgress)
        }
        (Some(number), Some(Value::String(completion)))
            if number.as_u64().is_some_and(|value| value > 0) && completion == "completed" =>
        {
            Ok(AttemptState::Completed)
        }
        _ => bail!("Live Demo Student Assessment state is invalid"),
    }
}

async fn ensure_avery_is_startable(
    api: &ProductApi,
    avery: &TemporarySession,
    graph: &DemoGraph,
) -> Result<()> {
    ensure!(
        student_attempt_state(api, avery, graph).await? == AttemptState::NotStarted,
        "Live Demo Avery Assessment Attempt cannot be converged"
    );
    let access = assessment_access(api, avery, graph).await?;
    ensure!(
        access
            .get("decision")
            .and_then(Value::as_object)
            .and_then(|decision| decision.get("startDecision"))
            .and_then(Value::as_str)
            == Some("may_start")
            && access.get("activeAssessmentAttempt") == Some(&Value::Null),
        "Live Demo Avery Assessment is not startable"
    );
    Ok(())
}

async fn assessment_access(
    api: &ProductApi,
    student: &TemporarySession,
    graph: &DemoGraph,
) -> Result<Map<String, Value>> {
    let value = expect_status(
        api.request(
            student,
            "Assessment access",
            Method::GET,
            &format!(
                "/api/course-instances/{}/assessments/{}/access",
                graph.course, graph.assessment
            ),
            None,
        )
        .await?,
        StatusCode::OK,
        "Assessment access",
    )?;
    let object = closed_object(
        &value,
        &[
            "decision",
            "activeAssessmentAttempt",
            "title",
            "assessmentType",
            "questionCount",
            "pointsPossible",
            "previousAttempts",
        ],
        "Assessment access",
    )?;
    Ok(object.clone())
}

async fn prepare_attempt(
    api: &ProductApi,
    student: &TemporarySession,
    graph: &DemoGraph,
    state: AttemptState,
    student_name: &'static str,
) -> Result<Option<String>> {
    if state == AttemptState::Completed {
        return Ok(None);
    }
    if state == AttemptState::NotStarted {
        let access = assessment_access(api, student, graph).await?;
        ensure!(
            access
                .get("decision")
                .and_then(Value::as_object)
                .and_then(|decision| decision.get("startDecision"))
                .and_then(Value::as_str)
                == Some("may_start")
                && access.get("activeAssessmentAttempt") == Some(&Value::Null),
            "Live Demo {student_name} Assessment is not startable"
        );
    }
    let started = expect_status(
        api.request(
            student,
            "Assessment start",
            Method::POST,
            &format!(
                "/api/course-instances/{}/assessments/{}/start",
                graph.course, graph.assessment
            ),
            Some(json!({})),
        )
        .await?,
        StatusCode::CREATED,
        "Assessment start",
    )?;
    let object = closed_object(
        &started,
        &[
            "assessmentAttempt",
            "assessment",
            "attemptNumber",
            "resumed",
            "title",
            "instructions",
            "questions",
        ],
        "Assessment start",
    )?;
    ensure!(
        object.get("assessment").and_then(Value::as_str) == Some(graph.assessment.as_str())
            && object
                .get("attemptNumber")
                .and_then(Value::as_u64)
                .is_some_and(|value| value > 0)
            && object.get("resumed").and_then(Value::as_bool)
                == Some(state == AttemptState::InProgress)
            && object
                .get("questions")
                .and_then(Value::as_array)
                .is_some_and(|items| items.len() == LIVE_DEMO_QUESTION_COUNT as usize),
        "Live Demo {student_name} Assessment Attempt is invalid"
    );
    public_id::<AssessmentAttemptId>(object.get("assessmentAttempt"), "Assessment Attempt")
        .map(Some)
}

async fn save_responses(
    api: &ProductApi,
    student: &TemporarySession,
    attempt: &str,
    count: u64,
) -> Result<()> {
    for position in 1..=count {
        let presentation = selected_presentation(api, student, attempt, position).await?;
        let response = response_from_presentation(&presentation)?;
        let receipt = expect_status(
            api.request(
                student,
                "Student response save",
                Method::PUT,
                &format!("/api/assessment-attempts/{attempt}/responses/{position}"),
                Some(json!({"response": response})),
            )
            .await?,
            StatusCode::OK,
            "Student response save",
        )?;
        ensure!(
            receipt
                == json!({
                    "assessmentAttempt": attempt,
                    "position": position,
                    "responseState": "saved"
                }),
            "Live Demo Student response save receipt is invalid"
        );
    }
    Ok(())
}

async fn selected_presentation(
    api: &ProductApi,
    student: &TemporarySession,
    attempt: &str,
    position: u64,
) -> Result<Value> {
    let value = expect_status(
        api.request(
            student,
            "Student presentation",
            Method::GET,
            &format!("/api/assessment-attempts/{attempt}/student-question?position={position}"),
            None,
        )
        .await?,
        StatusCode::OK,
        "Student presentation",
    )?;
    let object = closed_object(
        &value,
        &["position", "presentation", "savedResponse"],
        "Student presentation",
    )?;
    ensure!(
        object.get("position").and_then(Value::as_u64) == Some(position)
            && object.contains_key("savedResponse"),
        "Live Demo Student presentation is invalid"
    );
    let presentation = object
        .get("presentation")
        .context("Live Demo Student presentation is invalid")?;
    let presentation = closed_object(
        presentation,
        &["questionRevisionTuple", "prompt", "response"],
        "Student presentation",
    )?;
    presentation
        .get("response")
        .cloned()
        .context("Live Demo Student presentation is invalid")
}

async fn submit_attempt(api: &ProductApi, student: &TemporarySession, attempt: &str) -> Result<()> {
    let receipt = expect_status(
        api.request(
            student,
            "Assessment submission",
            Method::POST,
            &format!("/api/assessment-attempts/{attempt}/submission"),
            Some(json!({})),
        )
        .await?,
        StatusCode::OK,
        "Assessment submission",
    )?;
    let receipt = receipt
        .as_object()
        .context("Live Demo Assessment submission receipt is invalid")?;
    ensure!(
        receipt.keys().map(String::as_str).collect::<Vec<_>>()
            == ["assessmentAttempt", "submissionState"],
        "Live Demo Assessment submission receipt is not closed"
    );
    ensure!(
        receipt.get("assessmentAttempt").and_then(Value::as_str) == Some(attempt)
            && receipt.get("submissionState").and_then(Value::as_str) == Some("submitted"),
        "Live Demo Assessment submission receipt is invalid"
    );
    Ok(())
}

async fn verify_complete_activity(
    api: &ProductApi,
    sessions: &TemporarySessions,
    graph: &DemoGraph,
    jack_attempt: &str,
) -> Result<()> {
    ensure!(
        student_attempt_state(api, sessions.session("mary")?, graph).await?
            == AttemptState::Completed,
        "Live Demo Mary Assessment Attempt is not completed"
    );
    ensure!(
        student_attempt_state(api, sessions.session("jack")?, graph).await?
            == AttemptState::InProgress,
        "Live Demo Jack Assessment Attempt is not open"
    );
    ensure_avery_is_startable(api, sessions.session("avery")?, graph).await?;
    let progress = expect_status(
        api.request(
            sessions.session("jack")?,
            "Student progress",
            Method::GET,
            &format!("/api/assessment-attempts/{jack_attempt}/student-progress"),
            None,
        )
        .await?,
        StatusCode::OK,
        "Student progress",
    )?;
    let positions = closed_array_field(
        &progress,
        &[
            "assessmentAttempt",
            "questionCount",
            "recommendedPosition",
            "positions",
        ],
        "positions",
        "Student progress",
    )?;
    ensure!(
        progress.get("assessmentAttempt").and_then(Value::as_str) == Some(jack_attempt)
            && progress.get("questionCount").and_then(Value::as_u64)
                == Some(LIVE_DEMO_QUESTION_COUNT)
            && positions.len() == LIVE_DEMO_QUESTION_COUNT as usize,
        "Live Demo Jack Student progress is invalid"
    );
    let saved = positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let object =
                closed_object(position, &["position", "responseState"], "Student progress")?;
            ensure!(
                object.get("position").and_then(Value::as_u64) == Some(index as u64 + 1),
                "Live Demo Jack Student progress is invalid"
            );
            Ok(object.get("responseState").and_then(Value::as_str) == Some("saved"))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|saved| *saved)
        .count();
    ensure!(
        saved == JACK_SAVED_RESPONSE_COUNT as usize,
        "Live Demo Jack saved work did not converge"
    );
    Ok(())
}
