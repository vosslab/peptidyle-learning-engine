//! Ordinary API adoption and release of imported Biochemistry teaching content.

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use learning_data_access::SessionTokenHash;
use question_model::{AssessmentEditNumber, BlueprintCourseId, CourseInstanceId};
use reqwest::{Method, StatusCode, header};
use serde_json::{Value, json};

use super::{
    BrowserEndpoint, LIVE_DEMO_COURSE_LONG_NAME, LIVE_DEMO_COURSE_SHORT_NAME, TemporarySession,
    browser_endpoint_from_values, expect_status, http::ProductApi, public_id,
};

const LIVE_DEMO_TERM_END_DATE: &str = "2026-12-11";

/// Adopts the complete imported Blueprint through the same endpoint used by an Instructor.
pub(crate) fn adopt_blueprint(cookie: &str, blueprint: &BlueprintCourseId) -> Result<String> {
    let endpoint = browser_endpoint_from_values(|name| std::env::var(name).ok())?;
    let token: [u8; 32] = URL_SAFE_NO_PAD
        .decode(
            cookie
                .strip_prefix("__Host-ple_session=")
                .context("import session cookie is invalid")?,
        )?
        .try_into()
        .map_err(|_| anyhow::anyhow!("import session cookie is invalid"))?;
    let session = TemporarySession {
        cookie: header::HeaderValue::from_str(cookie)
            .context("import session cookie is invalid")?,
        token_hash: SessionTokenHash::compute(&token),
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(adopt(&endpoint, &session, blueprint))
}

async fn adopt(
    endpoint: &BrowserEndpoint,
    session: &TemporarySession,
    blueprint: &BlueprintCourseId,
) -> Result<String> {
    let api = ProductApi::new(endpoint)?;
    let view = expect_status(
        api.request(
            session,
            "Blueprint discovery",
            Method::GET,
            &format!("/api/course-blueprints/{blueprint}"),
            None,
        )
        .await?,
        StatusCode::OK,
        "Blueprint discovery",
    )?;
    let title = representative_assessment(&api, session, blueprint, &view).await?;
    let courses = expect_status(
        api.request(
            session,
            "Course discovery",
            Method::GET,
            "/api/course-instances",
            None,
        )
        .await?,
        StatusCode::OK,
        "Course discovery",
    )?;
    let items = courses["items"]
        .as_array()
        .context("Course discovery is invalid")?;
    let existing = items
        .iter()
        .filter(|course| course["shortName"].as_str() == Some(LIVE_DEMO_COURSE_SHORT_NAME))
        .collect::<Vec<_>>();
    ensure!(existing.len() <= 1, "Live Demo Course is ambiguous");
    let course = if let Some(existing) = existing.first() {
        ensure!(
            existing["longName"].as_str() == Some(LIVE_DEMO_COURSE_LONG_NAME),
            "Live Demo Course names conflict"
        );
        let id = public_id::<CourseInstanceId>(existing.get("id"), "Course")?;
        let existing_view = expect_status(
            api.request(
                session,
                "Course origin",
                Method::GET,
                &format!("/api/course-instances/{id}"),
                None,
            )
            .await?,
            StatusCode::OK,
            "Course origin",
        )?;
        ensure!(
            existing_view["blueprintOrigin"]["adoptedBlueprintCourseRevisionTuple"]
                == view["current_revision_tuple"],
            "Live Demo Course is already adopted from a different Blueprint Revision"
        );
        id
    } else {
        let created = expect_status(api.request(session, "Course adoption", Method::POST,
            "/api/course-instances", Some(json!({
                "classification": view["classification"],
                "source": {"kind":"adopted", "blueprintCourseRevisionTuple":view["current_revision_tuple"]},
                "shortName":LIVE_DEMO_COURSE_SHORT_NAME, "longName":LIVE_DEMO_COURSE_LONG_NAME,
                "term":{"startDate":"2026-08-24", "endDate":LIVE_DEMO_TERM_END_DATE},
            }))).await?, StatusCode::CREATED, "Course adoption")?;
        public_id::<CourseInstanceId>(created["courseInstance"].get("id"), "Course")?
    };
    let assessments = expect_status(
        api.request(
            session,
            "Assessment discovery",
            Method::GET,
            &format!("/api/course-instances/{course}/assessments"),
            None,
        )
        .await?,
        StatusCode::OK,
        "Assessment discovery",
    )?;
    let selected = assessments
        .as_array()
        .context("Assessment discovery is invalid")?
        .iter()
        .filter(|assessment| assessment["title"].as_str() == Some(title.as_str()))
        .collect::<Vec<_>>();
    ensure!(
        selected.len() == 1,
        "Representative Live Demo Assessment is missing or ambiguous"
    );
    let assessment = selected[0];
    if assessment["status"].as_str() == Some("unreleased") {
        let id = public_id::<question_model::AssessmentId>(assessment.get("id"), "Assessment")?;
        let mut edit: AssessmentEditNumber =
            serde_json::from_value(assessment["assessmentEditNumber"].clone())
                .context("Assessment edit number is invalid")?;
        if assessment["dueAt"].is_null() {
            // ASVS 2.3.1: schedule the adopted Assessment through its ordinary
            // edit operation, then release with the returned canonical Edit Number.
            let scheduled = expect_status(
                api.request_with_edit(
                    session,
                    "Assessment schedule",
                    Method::PUT,
                    &format!("/api/course-instances/{course}/assessments/{id}/inline"),
                    Some(json!({
                        "title": title,
                        "dueAt": format!("{LIVE_DEMO_TERM_END_DATE}T23:59:00.000"),
                    })),
                    Some(edit.value()),
                )
                .await?,
                StatusCode::OK,
                "Assessment schedule",
            )?;
            edit = serde_json::from_value(scheduled["assessmentEditNumber"].clone())
                .context("Assessment schedule edit number is invalid")?;
        }
        expect_status(
            api.request_with_edit(
                session,
                "Assessment release",
                Method::POST,
                &format!("/api/course-instances/{course}/assessments/{id}/release"),
                None,
                Some(edit.value()),
            )
            .await?,
            StatusCode::OK,
            "Assessment release",
        )?;
    } else {
        ensure!(
            assessment["status"].as_str() == Some("released"),
            "Representative Assessment is not available"
        );
    }
    Ok(course)
}

/// Select ordinary native content so fictional work uses answer-free PLE controls.
async fn representative_assessment(
    api: &ProductApi,
    session: &TemporarySession,
    blueprint: &BlueprintCourseId,
    view: &Value,
) -> Result<String> {
    let modules = view["modules"]
        .as_array()
        .context("Blueprint Modules are unavailable")?;
    for module in modules {
        for assessment in module["assessments"]
            .as_array()
            .context("Blueprint Assessments are unavailable")?
        {
            let content = &assessment["content"];
            let entries = content["entries"]
                .as_array()
                .context("Blueprint entries are unavailable")?;
            if entries.is_empty() {
                continue;
            }
            let mut native = true;
            for entry in entries {
                if entry["kind"].as_str() != Some("pool") {
                    native = false;
                    break;
                }
                let pool = entry["question_pool_id"]
                    .as_str()
                    .context("Blueprint Pool ID is unavailable")?;
                let assessment_id = assessment["blueprint_assessment_id"]
                    .as_str()
                    .context("Blueprint Assessment ID is unavailable")?;
                let members = expect_status(api.request(session, "Pool discovery", Method::GET,
                    &format!("/api/course-blueprints/{blueprint}/assessments/{assessment_id}/pools/{pool}/members"),
                    None).await?, StatusCode::OK, "Pool discovery")?;
                if members["backend"].as_str() != Some("ple") {
                    native = false;
                    break;
                }
            }
            if native {
                return content["title"]
                    .as_str()
                    .map(str::to_owned)
                    .context("Assessment title is unavailable");
            }
        }
    }
    bail!("Biochemistry Blueprint has no native Assessment for representative Student work")
}
