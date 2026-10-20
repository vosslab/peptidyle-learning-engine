//! Published Question evidence from the existing Native and WeBWorK source boundaries.

use adapter_ple::question_json::{PLE_QUESTION_JSON_MEDIA_TYPE, PleQuestionJsonDocument};
use adapter_webwork::{
    HttpWebworkRenderer, ResolvedWebworkQuestionSource, WebworkAdapter,
    WebworkQuestionSourceBinding,
};
use learning_data_access::postgres::{BloomCandidate, BloomSource};
use objects::{ObjectStore, ResolvedQuestionSource};
use question_model::generation::QuestionSeed;
use scraper::{Html, Node};
use serde_json::{Value, json};

use super::client::Failure;

// UTF-8 bytes conservatively bound tokenizer input; leave room for the two rubric questions.
// JEV supports 32k tokens per question / 64k total with the standard context.
const EVIDENCE_BYTES: usize = 24_000;

pub(super) async fn prepare<O: ObjectStore>(
    candidate: &BloomCandidate,
    objects: &O,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
) -> Result<Value, Failure> {
    let source: BloomSource = candidate
        .sources
        .first()
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(|| Failure::persistent("source_binding_invalid"))?;
    let content = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        question_evidence(&source, objects, webwork),
    )
    .await
    .map_err(|_| Failure::temporary("evidence_timeout"))??;
    let evidence = json!({"title":candidate.title, "description":candidate.description,
        "content":content,
        "evidenceLimitations":"Text and answer information only; diagrams and browser-generated visuals may be incomplete. WeBWorK uses seed 0."
    });
    if serde_json::to_vec(&evidence)
        .map_err(|_| Failure::persistent("evidence_json"))?
        .len()
        > EVIDENCE_BYTES
    {
        return Err(Failure::persistent("question_exceeds_context"));
    }
    Ok(evidence)
}

async fn question_evidence<O: ObjectStore>(
    source: &BloomSource,
    objects: &O,
    webwork: &WebworkAdapter<HttpWebworkRenderer>,
) -> Result<Value, Failure> {
    let invalid = || Failure::persistent("source_evidence_invalid");
    let tuple = source.revision_tuple().map_err(|_| invalid())?;
    match source.backend.as_str() {
        "ple" => {
            let resolved = ResolvedQuestionSource::resolve(
                objects,
                tuple,
                source.object_id,
                source.checksum.clone(),
            )
            .await
            .map_err(|_| Failure::temporary("source_unavailable"))?;
            if resolved.media_type() != PLE_QUESTION_JSON_MEDIA_TYPE {
                return Err(invalid());
            }
            let document =
                PleQuestionJsonDocument::parse(resolved.bytes()).map_err(|_| invalid())?;
            let mut evidence = document.bloom_evidence().map_err(|_| invalid())?;
            readable_values(&mut evidence);
            Ok(evidence)
        }
        "webwork" => {
            let binding = WebworkQuestionSourceBinding::new(
                tuple,
                source.webwork_pg_path.clone().ok_or_else(invalid)?,
            )
            .map_err(|_| invalid())?;
            let resolved = ResolvedWebworkQuestionSource::resolve(
                objects,
                binding,
                source.object_id,
                source.checksum.clone(),
            )
            .await
            .map_err(|_| Failure::temporary("source_unavailable"))?;
            let html = webwork
                .answer_review_document(QuestionSeed::new(0), &resolved)
                .await
                .map_err(|_| Failure::temporary("webwork_unavailable"))?;
            let html = String::from_utf8(html).map_err(|_| invalid())?;
            Ok(json!({"renderedQuestionAndCorrectAnswers":readable_html(&html)}))
        }
        _ => Err(Failure::persistent("unsupported_question_backend")),
    }
}

fn readable_values(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(readable_values),
        Value::Object(values) => {
            for (key, value) in values {
                if matches!(key.as_str(), "prompt" | "text" | "label") {
                    if let Value::String(text) = value {
                        *text = readable_html(text);
                    }
                } else {
                    readable_values(value);
                }
            }
        }
        _ => {}
    }
}

/// BPW-style readable HTML: preserve table boundaries, image labels and answer controls.
/// MathJax TeX scripts are content; executable scripts and CSS are excluded.
fn readable_html(source: &str) -> String {
    let html = Html::parse_fragment(source);
    let mut output = String::new();
    for node in html.tree.nodes() {
        let hidden = node.ancestors().any(|ancestor| match ancestor.value() {
            Node::Element(e) => {
                e.name() == "style"
                    || (e.name() == "script"
                        && !e.attr("type").is_some_and(|v| v.starts_with("math/tex")))
            }
            _ => false,
        });
        if hidden {
            continue;
        }
        match node.value() {
            Node::Text(text) => {
                output.push_str(text);
                output.push(' ');
            }
            Node::Element(e) => match e.name() {
                "img" => {
                    output.push_str(e.attr("alt").unwrap_or("[image]"));
                    output.push(' ');
                }
                "input" if e.attr("type") != Some("hidden") => {
                    if let Some(value) = e.attr("value") {
                        output.push_str(value);
                        output.push(' ');
                    }
                }
                "tr" | "p" | "br" | "div" => output.push('\n'),
                "td" | "th" => output.push_str(" | "),
                _ => {}
            },
            _ => {}
        }
    }
    output
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_math_choices_tables_and_correct_answers_without_executable_scripts() {
        let text = readable_html(
            r#"<p>Calculate <script type="math/tex">x^2</script></p>
            <script>secret_code()</script><style>css</style><table><tr><td>A</td><td>4</td></tr></table>
            <input value="answer-control-4"><p>Correct answer: 4</p><img alt="two chromosomes">"#,
        );
        for expected in [
            "x^2",
            "A | 4",
            "answer-control-4",
            "Correct answer: 4",
            "two chromosomes",
        ] {
            assert!(text.contains(expected), "{text}");
        }
        assert!(!text.contains("secret_code"));
        assert!(!text.contains("css"));
    }
}
