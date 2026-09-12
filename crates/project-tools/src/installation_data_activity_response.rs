//! Answer-free response derivation for the fixed Live Demo Questions.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use super::closed_object;

pub(super) fn response_from_presentation(response: &Value) -> Result<Value> {
    let object = response
        .as_object()
        .context("Live Demo Student presentation response is invalid")?;
    match object.get("kind").and_then(Value::as_str) {
        Some("singleChoice") => single_choice_response(response, object),
        Some("matching") => matching_response(response, object),
        _ => bail!("Live Demo Pilot Question has an unsupported presentation kind"),
    }
}

fn single_choice_response(
    response: &Value,
    object: &serde_json::Map<String, Value>,
) -> Result<Value> {
    closed_object(response, &["kind", "choices"], "single-choice presentation")?;
    let choices = object
        .get("choices")
        .and_then(Value::as_array)
        .filter(|choices| !choices.is_empty())
        .context("Live Demo single-choice presentation is invalid")?;
    let id = item_ids(choices, "single-choice presentation")?
        .first()
        .copied()
        .context("Live Demo single-choice presentation is invalid")?;
    Ok(json!({"kind": "multipleChoice", "selected": [id]}))
}

fn matching_response(response: &Value, object: &serde_json::Map<String, Value>) -> Result<Value> {
    closed_object(
        response,
        &["kind", "prompts", "choices", "reuseChoices"],
        "matching presentation",
    )?;
    let prompts = nonempty_array(object.get("prompts"))?;
    let choices = nonempty_array(object.get("choices"))?;
    let reuse_choices = object
        .get("reuseChoices")
        .and_then(Value::as_bool)
        .context("Live Demo matching presentation is invalid")?;
    let prompt_ids = item_ids(prompts, "matching presentation")?;
    let choice_ids = item_ids(choices, "matching presentation")?;
    ensure!(
        ids_are_unique(&prompt_ids)
            && (reuse_choices
                || (ids_are_unique(&choice_ids) && choice_ids.len() >= prompt_ids.len())),
        "Live Demo matching presentation is invalid"
    );
    let matches = prompt_ids
        .iter()
        .enumerate()
        .map(|(index, prompt)| {
            let choice = if reuse_choices {
                choice_ids[index % choice_ids.len()]
            } else {
                choice_ids[index]
            };
            json!({"prompt": prompt, "choice": choice})
        })
        .collect::<Vec<_>>();
    Ok(json!({"kind": "matching", "matches": matches}))
}

fn nonempty_array(value: Option<&Value>) -> Result<&Vec<Value>> {
    value
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .context("Live Demo matching presentation is invalid")
}

fn item_ids<'a>(values: &'a [Value], label: &'static str) -> Result<Vec<&'a str>> {
    values
        .iter()
        .map(|value| {
            closed_object(value, &["id", "body"], label)?
                .get("id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .context("Live Demo matching presentation is invalid")
        })
        .collect()
}

fn ids_are_unique(ids: &[&str]) -> bool {
    ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_derivation_uses_only_presented_response_item_ids() {
        let single_choice = json!({
            "kind": "singleChoice",
            "choices": [{"id": "choice-1", "body": []}]
        });
        assert_eq!(
            response_from_presentation(&single_choice).unwrap(),
            json!({"kind": "multipleChoice", "selected": ["choice-1"]})
        );
        let matching = json!({
            "kind": "matching",
            "prompts": [{"id": "prompt-1", "body": []}],
            "choices": [{"id": "choice-1", "body": []}],
            "reuseChoices": false
        });
        assert_eq!(
            response_from_presentation(&matching).unwrap(),
            json!({
                "kind": "matching",
                "matches": [{"prompt": "prompt-1", "choice": "choice-1"}]
            })
        );
    }

    #[test]
    fn response_derivation_rejects_non_reusable_duplicate_matches() {
        let matching = json!({
            "kind": "matching",
            "prompts": [
                {"id": "prompt-1", "body": []},
                {"id": "prompt-2", "body": []}
            ],
            "choices": [{"id": "choice-1", "body": []}],
            "reuseChoices": false
        });
        assert!(response_from_presentation(&matching).is_err());
    }
}
