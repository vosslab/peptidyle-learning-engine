//! Answer-free response derivation for ordinary native Live Demo Questions.

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use super::closed_object;

/// Returns a response for the PLE-native controls this activity owns.
pub(super) fn response_from_presentation(response: &Value) -> Result<Value> {
    let object = response
        .as_object()
        .context("Live Demo Student presentation response is invalid")?;
    match object.get("kind").and_then(Value::as_str) {
        Some("singleChoice") => single_choice_response(response, object),
        Some("multipleAnswer") => {
            let choices = nonempty_array(object.get("choices"))?;
            let minimum = object["minimum"]
                .as_u64()
                .context("selection minimum is invalid")? as usize;
            let count = minimum.max(1).min(choices.len());
            let selected = item_ids(choices, "multiple-answer presentation")?
                .into_iter()
                .take(count)
                .collect::<Vec<_>>();
            Ok(json!({"kind":"multipleChoice", "selected":selected}))
        }
        Some("fillIn") => Ok(json!({"kind":"shortText", "text":"example"})),
        Some("numerical") => Ok(json!({"kind":"numeric", "value":0})),
        Some("multiFillIn") => {
            let blanks = nonempty_array(object.get("blanks"))?;
            let answers = blanks.iter().map(|blank| {
                Ok(json!({"slot":blank["id"].as_str().context("blank ID is invalid")?, "text":"example"}))
            }).collect::<Result<Vec<_>>>()?;
            Ok(json!({"kind":"multiBlank", "answers":answers}))
        }
        Some("ordering") => {
            let items = nonempty_array(object.get("items"))?;
            Ok(json!({"kind":"ordering", "order":item_ids(items, "ordering presentation")?}))
        }
        Some("matching") => matching_response(response, object),
        _ => bail!("Live Demo Question has an unsupported presentation kind"),
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
