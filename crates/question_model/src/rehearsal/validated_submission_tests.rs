use super::*;
use crate::{StudentResponse, response::ChoiceId};

fn identifier(value: &str) -> crate::RenderedItemIdV1 {
    crate::RenderedItemIdV1::parse(value).expect("valid rendered identifier")
}

fn choice(value: &str) -> RehearsalPresentedChoiceV1 {
    RehearsalPresentedChoiceV1 {
        id: identifier(value),
        body: vec![RehearsalContentBlockV1::Text {
            markdown: value.into(),
        }],
    }
}

fn multiple_answer_screen() -> RehearsalActiveScreenV1 {
    RehearsalActiveScreenV1::new(RehearsalQuestionPresentationV1 {
        title: "Rendered identifiers remain server-bound".into(),
        prompt: vec![RehearsalContentBlockV1::Text {
            markdown: "Choose the issued answers.".into(),
        }],
        response: RehearsalResponseSchemaV1::MultipleAnswer {
            choices: vec![choice("0001"), choice("0002")],
            minimum: 1,
            maximum: 2,
        },
    })
    .expect("valid active screen")
}

fn request(
    screen: &RehearsalActiveScreenV1,
    selected: Vec<ChoiceId>,
) -> RehearsalSubmissionRequestV1 {
    RehearsalSubmissionRequestV1 {
        presentation_digest: screen.presentation_digest.clone(),
        response: StudentResponse::MultipleChoice { selected },
    }
}

#[test]
fn validated_submission_retains_rendered_response_and_full_screen_commitment() {
    let screen = multiple_answer_screen();
    let expected_commitment = screen.commitment().expect("screen commitment");
    let response = StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new("0002")],
    };
    let submission = ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
        RehearsalSubmissionRequestV1 {
            presentation_digest: screen.presentation_digest.clone(),
            response: response.clone(),
        },
        &screen,
    )
    .expect("issued rendered identifier is valid");

    assert_eq!(submission.response(), &response);
    assert_eq!(submission.presentation_commitment(), expected_commitment);
    assert_eq!(
        submission.presentation_commitment().public_token(),
        screen.presentation_digest
    );

    let debug = format!("{submission:?}");
    assert!(!debug.contains("0002"));
    assert!(!debug.contains(&format!("{expected_commitment:?}")));
}

#[test]
fn validated_submission_rejects_stale_public_digest_unknown_identifiers_and_cardinality() {
    let screen = multiple_answer_screen();

    let mut stale_digest = request(&screen, vec![ChoiceId::new("0001")]);
    stale_digest.presentation_digest =
        RehearsalPresentationDigestV1::from_bytes([7; 32]).public_token();
    assert!(matches!(
        ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(stale_digest, &screen),
        Err(RehearsalWireValidationError::InvalidDigest)
    ));

    for selected in [
        vec![ChoiceId::new("ffff")],
        vec![ChoiceId::new("0001"), ChoiceId::new("0001")],
        Vec::new(),
    ] {
        assert!(matches!(
            ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                request(&screen, selected),
                &screen,
            ),
            Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
        ));
    }
}

#[test]
fn private_validated_submission_does_not_change_the_public_request_wire_shape() {
    let screen = multiple_answer_screen();
    let encoded = serde_json::to_value(request(&screen, vec![ChoiceId::new("0001")]))
        .expect("public request serializes");

    assert_eq!(
        encoded,
        serde_json::json!({
            "presentationDigest": screen.presentation_digest,
            "response": {"kind": "multipleChoice", "selected": ["0001"]},
        })
    );
    assert!(
        serde_json::from_value::<RehearsalSubmissionRequestV1>(encoded).is_ok(),
        "the browser contract remains independently deserializable"
    );
}
