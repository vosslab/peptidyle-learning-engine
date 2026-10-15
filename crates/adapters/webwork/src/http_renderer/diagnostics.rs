//! Author-only projection of the renderer's PG lint result.

use serde::Serialize;
use serde_json::Value;

use crate::renderer_contract::RendererFailure;

/// Diagnostics for one source and seed, without renderer HTML or private state.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebworkSourceDiagnostics {
    pub error_flag: bool,
    pub pg_warn: Vec<String>,
}

pub(super) fn project(value: &Value) -> Result<WebworkSourceDiagnostics, RendererFailure> {
    let flags = value
        .get("flags")
        .and_then(Value::as_object)
        .ok_or_else(|| RendererFailure::InvalidOutput("renderer omitted flags".into()))?;
    // Successful PG renders may omit error_flag, as in the upstream lint helper.
    let flag = flags.get("error_flag").unwrap_or(&Value::Null);
    let error_flag = match flag {
        Value::Null | Value::Bool(false) => false,
        Value::Number(number) => number.as_i64() != Some(0),
        Value::String(text) => !text.is_empty() && text != "0",
        _ => true,
    };
    let debug = value
        .get("debug")
        .and_then(Value::as_object)
        .ok_or_else(|| RendererFailure::InvalidOutput("renderer omitted PG diagnostics".into()))?;
    let mut pg_warn = match debug.get("pg_warn") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(messages)) => messages
            .iter()
            .filter(|message| !message.is_null())
            .map(|message| message.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| RendererFailure::InvalidOutput("invalid PG warnings".into()))?,
        _ => return Err(RendererFailure::InvalidOutput("invalid PG warnings".into())),
    };
    // ASVS 16.5.1: expose only PG diagnostics to the authorized author, never
    // the renderer envelope, HTML, grading data, or private JWT state.
    if let Some(tokens) = value.get("JWT").and_then(Value::as_object) {
        for token in tokens
            .values()
            .filter_map(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            for message in &mut pg_warn {
                *message = message.replace(token, "[REDACTED]");
            }
        }
    }
    pg_warn.retain(|message| !message.trim().is_empty());
    Ok(WebworkSourceDiagnostics {
        error_flag,
        pg_warn,
    })
}

#[cfg(test)]
mod tests {
    use super::project;
    use serde_json::json;

    #[test]
    fn diagnostics_preserve_warnings_without_promoting_them_to_errors() {
        let mut response = json!({
            "flags": {"error_flag": 0},
            "debug": {"pg_warn": ["line 7: warning", "private-token"], "internal": "private"},
            "JWT": {"problem": "private-token"}, "renderedHTML": "private"
        });
        let diagnostics = project(&response).unwrap();
        assert!(!diagnostics.error_flag);
        assert_eq!(diagnostics.pg_warn, ["line 7: warning", "[REDACTED]"]);
        assert_eq!(
            serde_json::to_value(diagnostics).unwrap(),
            json!({
                "errorFlag": false, "pgWarn": ["line 7: warning", "[REDACTED]"]
            })
        );
        response["flags"]["error_flag"] = json!(1);
        assert!(project(&response).unwrap().error_flag);
        response["flags"] = json!({});
        assert!(!project(&response).unwrap().error_flag);
        response["flags"] = json!(null);
        assert!(project(&response).is_err());
    }
}
