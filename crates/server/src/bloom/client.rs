//! JEV's typed-choice contract using BPW rubric version 2, copied verbatim.

use std::time::Duration;

use reqwest::{Client, header};
use serde_json::{Value, json};

const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub(super) struct JevClient {
    http: Client,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Failure {
    pub reason: &'static str,
    pub retry_after: Duration,
}

impl Failure {
    pub fn temporary(reason: &'static str) -> Self {
        Self {
            reason,
            retry_after: Duration::from_secs(60),
        }
    }

    pub fn persistent(reason: &'static str) -> Self {
        Self {
            reason,
            retry_after: Duration::from_secs(3600),
        }
    }
}

impl JevClient {
    pub fn new(key: &str) -> Result<Self, Failure> {
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {}", key.trim()))
            .map_err(|_| Failure::persistent("invalid_credentials"))?;
        authorization.set_sensitive(true);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, authorization);
        let http = Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| Failure::persistent("client_configuration"))?;
        Ok(Self { http })
    }

    pub async fn classify(&self, state: Value) -> Result<(String, String), Failure> {
        let payload = payload(state);
        let mut response = self
            .http
            .post(ENDPOINT)
            .json(&payload)
            .send()
            .await
            .map_err(|_| Failure::temporary("jev_transport"))?;
        if !response.status().is_success() {
            let mut failure = match response.status().as_u16() {
                429 => Failure::temporary("jev_rate_limit"),
                500..=599 => Failure::temporary("jev_unavailable"),
                401 | 403 => Failure::persistent("jev_credentials"),
                _ => Failure::persistent("jev_request_rejected"),
            };
            if let Some(seconds) = response
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
            {
                failure.retry_after = failure.retry_after.max(Duration::from_secs(seconds));
            }
            return Err(failure);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Failure::temporary("jev_response_transport"))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(Failure::persistent("jev_response_too_large"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let response: Value = serde_json::from_slice(&bytes)
            .map_err(|_| Failure::persistent("jev_invalid_response"))?;
        validate(&response, &payload)
    }
}

fn payload(state: Value) -> Value {
    // Source: BPW bioproblems_site/bloom_rubric.py, commit 4d5deb6142d2834b3a9fef97ab8a459ae52c3edb.
    // Criteria remain verbatim; only the supplied evidence scope changes.
    let mut rubric: Value = serde_json::from_str(include_str!("rubric.json"))
        .expect("embedded BPW rubric is valid JSON");
    for dimension in ["cognitive", "knowledge"] {
        let instructions = rubric["questions"][dimension]["instructions"]
            .as_str()
            .expect("embedded BPW rubric has instruction strings for both dimensions");
        rubric["questions"][dimension]["instructions"] = Value::String(
            instructions
                .replace("across this biology question bank", "in this Question")
                .replace(
                    "; YAML describes authored variants",
                    "; the evidence describes the supplied variation",
                ),
        );
    }
    json!({"model":rubric["model"], "state":state, "questions":rubric["questions"]})
}

fn validate(response: &Value, request: &Value) -> Result<(String, String), Failure> {
    let invalid = || Failure::persistent("jev_invalid_response");
    if response["model"] != request["model"] {
        return Err(invalid());
    }
    let mut labels = Vec::new();
    for dimension in ["cognitive", "knowledge"] {
        let criteria = request["questions"][dimension]["criteria"]
            .as_object()
            .ok_or_else(invalid)?;
        let answer = &response["answers"][dimension];
        let label = answer["choice"].as_str().ok_or_else(invalid)?;
        if !criteria.contains_key(label) {
            return Err(invalid());
        }
        let probabilities = answer["probabilities"].as_object().ok_or_else(invalid)?;
        if probabilities.len() != criteria.len() {
            return Err(invalid());
        }
        let mut sum = 0.0;
        for key in criteria.keys() {
            let probability = probabilities
                .get(key)
                .and_then(Value::as_f64)
                .ok_or_else(invalid)?;
            if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
                return Err(invalid());
            }
            sum += probability;
        }
        if (sum - 1.0).abs() > 0.02 {
            return Err(invalid());
        }
        labels.push(if dimension == "knowledge" {
            format!("{label} Knowledge")
        } else {
            label.to_string()
        });
    }
    Ok((labels.remove(0), labels.remove(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_both_choices_and_complete_distributions() {
        let request = payload(json!({"prompt":"test"}));
        let mut response = json!({"model":request["model"], "answers":{}});
        for dimension in ["cognitive", "knowledge"] {
            let criteria = request["questions"][dimension]["criteria"]
                .as_object()
                .unwrap();
            let selected = criteria.keys().next().unwrap();
            let distribution: serde_json::Map<String, Value> = criteria
                .keys()
                .map(|key| (key.clone(), json!(if key == selected { 1.0 } else { 0.0 })))
                .collect();
            response["answers"][dimension] =
                json!({"choice":selected,"probabilities":distribution});
        }
        assert_eq!(
            validate(&response, &request).unwrap(),
            ("Analyze".to_string(), "Conceptual Knowledge".to_string())
        );
        let valid = response.clone();
        response["answers"]["knowledge"]["probabilities"] = json!({"Conceptual":1.0});
        assert!(validate(&response, &request).is_err());
        response = valid;

        response["answers"]["knowledge"]["choice"] = json!("invented");
        assert!(validate(&response, &request).is_err());
    }
}
