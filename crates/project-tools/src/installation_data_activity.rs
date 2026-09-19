//! Ordinary HTTP convergence for the installed Live Demo Student activity.

use std::{collections::BTreeMap, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use learning_data_access::{
    SessionLifetime, SessionStore, SessionTokenHash,
    postgres::{PostgresSessionStore, ProductionLoginProfile, lazy_pool},
};
use question_model::{
    AccountId, AssessmentAttemptId, AssessmentId, AssessmentType, CourseInstanceId,
};
use reqwest::{Method, StatusCode, header};
use serde_json::{Map, Value, json};
use server_core::auth::{self, CookieTransport, SessionConfig};
use url::Url;

use crate::installation_data::{
    LIVE_DEMO_ASSESSMENT_TITLE, LIVE_DEMO_AVERY_EMAIL, LIVE_DEMO_COURSE_LONG_NAME,
    LIVE_DEMO_COURSE_SHORT_NAME, LIVE_DEMO_ELENA_EMAIL, LIVE_DEMO_JACK_EMAIL, LIVE_DEMO_MARY_EMAIL,
};

#[path = "installation_data_activity_http.rs"]
mod http;
use http::{ProductApi, ProductResponse};
#[path = "installation_data_activity_response.rs"]
mod response;
use response::response_from_presentation;

// `live_demo.sql` assigns exactly the four PLE Question JSON Pilot entries.
const LIVE_DEMO_QUESTION_COUNT: u64 = 4;
const JACK_SAVED_RESPONSE_COUNT: u64 = 2;
const TEMPORARY_SESSION_SECONDS: u32 = 5 * 60;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_RESPONSE_BYTES: usize = 128 * 1024;

const DEMO_ACCOUNTS: [DemoAccount; 4] = [
    DemoAccount::new("elena", LIVE_DEMO_ELENA_EMAIL),
    DemoAccount::new("mary", LIVE_DEMO_MARY_EMAIL),
    DemoAccount::new("jack", LIVE_DEMO_JACK_EMAIL),
    DemoAccount::new("avery", LIVE_DEMO_AVERY_EMAIL),
];

#[derive(Clone, Copy)]
struct DemoAccount {
    name: &'static str,
    email: &'static str,
}

impl DemoAccount {
    const fn new(name: &'static str, email: &'static str) -> Self {
        Self { name, email }
    }
}

/// Converges ordinary Student work after the installation-data manifest exists.
pub(crate) fn provision() -> Result<()> {
    let database_url = required_environment("DATABASE_URL")?;
    let topology = required_environment("PLE_STORAGE_TOPOLOGY")?;
    let endpoint = browser_endpoint_from_values(|name| std::env::var(name).ok())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("creating the Live Demo activity runtime")?;
    // SQLx 0.9 initializes its lazy-pool maintenance state while constructing
    // the pool. Keep that construction inside the runtime that owns it.
    runtime.block_on(async {
        let pool = api_pool(&database_url, &topology)?;
        provision_async(pool, endpoint).await
    })
}

async fn provision_async(pool: sqlx::PgPool, endpoint: BrowserEndpoint) -> Result<()> {
    let lookup_pool = lazy_pool(&required_environment("PLE_MIGRATION_DATABASE_URL")?)
        .map_err(|_| anyhow::anyhow!("Live Demo account lookup database URL is invalid"))?;
    let sessions = PostgresSessionStore::new(pool);
    let mut temporary = TemporarySessions::issue(&sessions, &lookup_pool).await?;
    let result = converge(&endpoint, &temporary).await;
    let cleanup = temporary.revoke_all(&sessions).await;
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(_)) => Err(error.context("Live Demo temporary-session cleanup failed")),
    }
}

fn api_pool(database_url: &str, topology: &str) -> Result<sqlx::PgPool> {
    let result = if topology == "disposable-local" {
        learning_data_access::postgres::local_development_pool(
            database_url,
            ProductionLoginProfile::Api,
        )
    } else {
        learning_data_access::postgres::production_pool(database_url, ProductionLoginProfile::Api)
    };
    result.map_err(|_| anyhow::anyhow!("Live Demo API database configuration is invalid"))
}

fn required_environment(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} must be set"))?;
    ensure!(
        !value.is_empty() && !value.contains(['\0', '\n', '\r']),
        "{name} must not be empty"
    );
    Ok(value)
}

struct BrowserEndpoint {
    request_base: Url,
    origin_header: header::HeaderValue,
    host_header: header::HeaderValue,
}

fn browser_endpoint_from_values<F>(get: F) -> Result<BrowserEndpoint>
where
    F: Fn(&str) -> Option<String>,
{
    // ASVS V3.5: treat the external origin as untrusted configuration and
    // accept only the one canonical authority used for ordinary API calls.
    let origin = get("PLE_BROWSER_ORIGIN").context("PLE_BROWSER_ORIGIN must be set")?;
    let topology = get("PLE_STORAGE_TOPOLOGY").context("PLE_STORAGE_TOPOLOGY must be set")?;
    ensure!(
        !origin.is_empty() && !origin.contains(['\0', '\n', '\r']),
        "PLE_BROWSER_ORIGIN is invalid"
    );
    let parsed =
        Url::parse(&origin).map_err(|_| anyhow::anyhow!("PLE_BROWSER_ORIGIN is invalid"))?;
    let scheme_is_allowed =
        parsed.scheme() == "https" || (parsed.scheme() == "http" && topology == "disposable-local");
    ensure!(
        scheme_is_allowed
            && parsed.host_str().is_some()
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.path() == "/"
            && parsed.query().is_none()
            && parsed.fragment().is_none(),
        "PLE_BROWSER_ORIGIN must be a canonical HTTPS origin"
    );
    let canonical = parsed.origin().ascii_serialization();
    ensure!(
        origin.trim_end_matches('/') == canonical,
        "PLE_BROWSER_ORIGIN must be canonical"
    );
    let canonical_url = Url::parse(&format!("{canonical}/"))
        .map_err(|_| anyhow::anyhow!("PLE_BROWSER_ORIGIN is invalid"))?;
    let request_base = if topology == "disposable-local" {
        // The migrator shares Compose networking with the API but not its
        // loopback interface. The public authority remains in Origin and Host.
        Url::parse("http://api:3000/")
            .map_err(|_| anyhow::anyhow!("fixed Live Demo API base is invalid"))?
    } else {
        canonical_url
    };
    let host = parsed.host_str().context("PLE_BROWSER_ORIGIN is invalid")?;
    let authority = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    Ok(BrowserEndpoint {
        request_base,
        origin_header: header::HeaderValue::from_str(&canonical)
            .map_err(|_| anyhow::anyhow!("PLE_BROWSER_ORIGIN is invalid"))?,
        host_header: header::HeaderValue::from_str(&authority)
            .map_err(|_| anyhow::anyhow!("PLE_BROWSER_ORIGIN is invalid"))?,
    })
}

fn temporary_session_config() -> Result<SessionConfig> {
    let lifetime = SessionLifetime::from_seconds(TEMPORARY_SESSION_SECONDS)
        .context("Live Demo temporary session lifetime is invalid")?;
    Ok(SessionConfig::new(
        lifetime,
        CookieTransport::FirstPartyHttps,
    ))
}

struct TemporarySession {
    cookie: header::HeaderValue,
    token_hash: SessionTokenHash,
}

struct TemporarySessions {
    values: BTreeMap<&'static str, TemporarySession>,
}

impl TemporarySessions {
    async fn issue(store: &PostgresSessionStore, lookup_pool: &sqlx::PgPool) -> Result<Self> {
        let mut accounts = Vec::with_capacity(DEMO_ACCOUNTS.len());
        for account in DEMO_ACCOUNTS {
            let account_id = account_id_for_email(lookup_pool, account.email).await?;
            accounts.push((account, account_id));
        }
        let config = temporary_session_config()?;
        let mut values = BTreeMap::new();
        for (account, account_id) in accounts {
            let issued = match auth::issue_session(store, account_id, config).await {
                Ok(value) => value,
                Err(_) => {
                    ensure!(
                        revoke_issued_sessions(store, values.values()).await,
                        "Live Demo temporary-session cleanup failed"
                    );
                    bail!("Live Demo temporary-session issuance failed");
                }
            };
            let cookie = match canonical_cookie_header(&issued.set_cookie) {
                Ok(value) => value,
                Err(error) => {
                    let issued_revoked = revoke_session_hash(store, issued.record.token_hash).await;
                    let existing_revoked = revoke_issued_sessions(store, values.values()).await;
                    ensure!(
                        issued_revoked && existing_revoked,
                        "Live Demo temporary-session cleanup failed"
                    );
                    return Err(error);
                }
            };
            values.insert(
                account.name,
                TemporarySession {
                    cookie,
                    token_hash: issued.record.token_hash,
                },
            );
        }
        Ok(Self { values })
    }

    fn session(&self, name: &'static str) -> Result<&TemporarySession> {
        self.values
            .get(name)
            .context("Live Demo temporary session is unavailable")
    }

    async fn revoke_all(&mut self, store: &PostgresSessionStore) -> Result<()> {
        let values = std::mem::take(&mut self.values);
        let mut failed = false;
        for session in values.values() {
            if !revoke_session(store, session).await {
                failed = true;
            }
        }
        ensure!(!failed, "Live Demo temporary-session cleanup failed");
        Ok(())
    }
}

async fn revoke_issued_sessions<'a>(
    store: &PostgresSessionStore,
    sessions: impl Iterator<Item = &'a TemporarySession>,
) -> bool {
    let mut revoked = true;
    for session in sessions {
        revoked &= revoke_session(store, session).await;
    }
    revoked
}

async fn revoke_session(store: &PostgresSessionStore, session: &TemporarySession) -> bool {
    auth::revoke_session(store, session.cookie.to_str().ok())
        .await
        .is_ok()
        || revoke_session_hash(store, session.token_hash).await
}

async fn revoke_session_hash(store: &PostgresSessionStore, token_hash: SessionTokenHash) -> bool {
    store.revoke_session(token_hash).await.is_ok()
}

async fn account_id_for_email(pool: &sqlx::PgPool, email: &str) -> Result<AccountId> {
    let mut transaction = pool
        .begin()
        .await
        .context("opening Live Demo account lookup")?;
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .context("assuming Live Demo account lookup authority")?;
    let account_id: Option<String> = sqlx::query_scalar(
        "SELECT email.account_id::text \
         FROM ple_private.account_authentication_email AS email \
         WHERE email.normalized_email = $1",
    )
    .bind(email)
    .fetch_optional(&mut *transaction)
    .await
    .context("looking up the Live Demo account")?;
    transaction
        .commit()
        .await
        .context("committing Live Demo account lookup")?;
    let account_id = account_id.context("Live Demo account is unavailable")?;
    AccountId::new(account_id).map_err(|_| anyhow::anyhow!("Live Demo account ID is invalid"))
}

fn canonical_cookie_header(set_cookie: &str) -> Result<header::HeaderValue> {
    let pair = cookie_pair(set_cookie).context("Live Demo session cookie is invalid")?;
    ensure!(
        pair.starts_with("__Host-ple_session=") && pair.len() > "__Host-ple_session=".len(),
        "Live Demo session cookie is invalid"
    );
    header::HeaderValue::from_str(pair)
        .map_err(|_| anyhow::anyhow!("Live Demo session cookie is invalid"))
}

fn cookie_pair(set_cookie: &str) -> Option<&str> {
    let pair = set_cookie.split(';').next()?;
    (!pair.contains(['\0', '\n', '\r'])).then_some(pair)
}

fn expect_status(
    response: ProductResponse,
    expected: StatusCode,
    operation: &'static str,
) -> Result<Value> {
    ensure!(
        response.status == expected,
        "Live Demo {operation} returned HTTP {}",
        response.status
    );
    response
        .body
        .context("Live Demo successful response body is unavailable")
}

#[derive(Clone)]
struct DemoGraph {
    course: String,
    assessment: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AttemptState {
    NotStarted,
    InProgress,
    Completed,
}

async fn converge(endpoint: &BrowserEndpoint, sessions: &TemporarySessions) -> Result<()> {
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
        &["questionRevision", "prompt", "response"],
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

fn closed_array_field<'a>(
    value: &'a Value,
    fields: &[&str],
    field: &str,
    label: &'static str,
) -> Result<&'a Vec<Value>> {
    closed_object(value, fields, label)?
        .get(field)
        .and_then(Value::as_array)
        .context("Live Demo projection is invalid")
}

pub(super) fn closed_object<'a>(
    value: &'a Value,
    fields: &[&str],
    label: &'static str,
) -> Result<&'a Map<String, Value>> {
    closed_object_with_optional(value, fields, &[], label)
}

fn closed_object_with_optional<'a>(
    value: &'a Value,
    required: &[&str],
    optional: &[&str],
    label: &'static str,
) -> Result<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .with_context(|| format!("Live Demo {label} projection is invalid"))?;
    ensure!(
        required.iter().all(|field| object.contains_key(*field))
            && object.len() >= required.len()
            && object.len() <= required.len() + optional.len()
            && object
                .keys()
                .all(|field| required.contains(&field.as_str())
                    || optional.contains(&field.as_str())),
        "Live Demo {label} projection is invalid"
    );
    Ok(object)
}

fn public_id<Reference>(value: Option<&Value>, label: &'static str) -> Result<String>
where
    Reference: std::str::FromStr,
{
    // ASVS 2.2.1: validate each API projection with its canonical domain parser.
    let value = value
        .and_then(Value::as_str)
        .context("Live Demo public ID is invalid")?;
    value
        .parse::<Reference>()
        .map_err(|_| anyhow::anyhow!("Live Demo {label} reference is invalid"))?;
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_endpoint_requires_https_except_for_disposable_local() {
        let production = browser_endpoint_from_values(|name| match name {
            "PLE_BROWSER_ORIGIN" => Some("https://demo.example:8443".to_owned()),
            "PLE_STORAGE_TOPOLOGY" => Some("production".to_owned()),
            _ => None,
        })
        .unwrap();
        assert_eq!(
            production.request_base.as_str(),
            "https://demo.example:8443/"
        );
        assert!(
            browser_endpoint_from_values(|name| match name {
                "PLE_BROWSER_ORIGIN" => Some("http://127.0.0.1:8080".to_owned()),
                "PLE_STORAGE_TOPOLOGY" => Some("production".to_owned()),
                _ => None,
            })
            .is_err()
        );
        let disposable = browser_endpoint_from_values(|name| match name {
            "PLE_BROWSER_ORIGIN" => Some("https://localhost:55001".to_owned()),
            "PLE_STORAGE_TOPOLOGY" => Some("disposable-local".to_owned()),
            _ => None,
        })
        .unwrap();
        assert_eq!(disposable.request_base.as_str(), "http://api:3000/");
        assert_eq!(
            disposable.origin_header.to_str().unwrap(),
            "https://localhost:55001"
        );
        assert_eq!(disposable.host_header.to_str().unwrap(), "localhost:55001");
        assert!(
            browser_endpoint_from_values(|name| match name {
                "PLE_BROWSER_ORIGIN" => Some("http://127.0.0.1:8080".to_owned()),
                "PLE_STORAGE_TOPOLOGY" => Some("disposable-local".to_owned()),
                _ => None,
            })
            .is_ok()
        );
    }
}
