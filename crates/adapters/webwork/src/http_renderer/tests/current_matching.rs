//! Current standalone-renderer matching-shape fixtures.

use super::*;

const CURRENT_MATCHING_BODY: &str = r#"<div class="PGML">
Match each description with its disorder.
Note: Each choice will be used exactly once.
<div style="margin-top:1em"></div>
<div class="two-column"><div>
<select class="pg-select" name="AnSwEr0001" id="AnSwEr0001" aria-label="answer 1 " size="1"><option selected value="" class="tex2jax_ignore"></option><option value="A" class="tex2jax_ignore">A</option><option value="B" class="tex2jax_ignore">B</option></select> <b>1.</b> a <span style="color: #d40000; font-weight:700;">disorder</span> affecting blood
<div style="margin-top:1em"></div>
<select class="pg-select" name="AnSwEr0002" id="AnSwEr0002" aria-label="answer 2 " size="1"><option selected value="" class="tex2jax_ignore"></option><option value="A" class="tex2jax_ignore">A</option><option value="B" class="tex2jax_ignore">B</option></select> <b>2.</b> affects connective tissue
</div><div class="right-col">
A. Beta-Thalassemia
<div style="margin-top:1em"></div>
B. Marfan <span style="color: #6c6c00; font-weight:700;">syndrome</span>
</div></div>
</div>
<input type=hidden name="MaThQuIlL_AnSwEr0001" id="MaThQuIlL_AnSwEr0001" value=""><input type=hidden name="MaThQuIlL_AnSwEr0002" id="MaThQuIlL_AnSwEr0002" value="">"#;

fn parsed_current_matching() -> ParsedRender {
    let settings = config();
    parse_render_rpc(
        Value::Object(response(&settings, CURRENT_MATCHING_BODY)),
        ExpectedEcho::from_request(&settings, request()),
        request(),
        &settings.base_uri,
    )
    .expect("current renderer matching response is supported")
}

#[test]
fn current_renderer_matching_shape_becomes_answer_free_typed_matching() {
    let parsed = parsed_current_matching();
    let QuestionResponseFormat::Matching { prompts, choices } = &parsed.envelope.response else {
        panic!("typed matching envelope")
    };
    assert_eq!(prompts.len(), 2);
    assert_eq!(choices.len(), 2);
    assert_eq!(
        prompts[0].body,
        vec![QuestionContentBlock::Text {
            markdown: "a disorder affecting blood".into()
        }]
    );
    assert_eq!(
        choices[1].body,
        vec![QuestionContentBlock::Text {
            markdown: "Marfan syndrome".into()
        }]
    );
    let public = serde_json::to_string(&parsed.envelope).expect("public envelope serializes");
    for protected in [
        "AnSwEr0001",
        "MaThQuIlL_AnSwEr0001",
        "header.payload.signature",
    ] {
        assert!(!public.contains(protected));
    }
}

#[test]
fn current_matching_refuses_mismatched_compatibility_fields_and_hostile_styles() {
    let settings = config();
    for body in [
        CURRENT_MATCHING_BODY.replacen(
            "MaThQuIlL_AnSwEr0002\" value=\"\"",
            "MaThQuIlL_AnSwEr9999\" value=\"\"",
            1,
        ),
        CURRENT_MATCHING_BODY.replacen(
            "color: #d40000; font-weight:700;",
            "background-image:url(https://attacker.example/)",
            1,
        ),
    ] {
        assert!(
            parse_render_rpc(
                Value::Object(response(&settings, &body)),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
    }
}
