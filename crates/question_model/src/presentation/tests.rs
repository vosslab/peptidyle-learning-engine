use std::collections::VecDeque;

use crate::answer::{SelectionCardinality, TextMatchMode};
use crate::envelope::{ContentBlock, QuestionEnvelope};
use crate::generation::Seed;
use crate::response::{
    ChoiceId, ChoiceOption, MatchPair, QuestionResponseFormat, StudentResponse, TextEntryAnswer,
    TextEntrySlot,
};
use crate::{QuestionVersionNumber, QuestionVersionReference};

use super::builder::{
    NonceSourceV1, PresentationBuildError, build_presentation_v1_with_hasher,
    build_presentation_v1_with_nonce_source,
};
use super::codec::{crc16_ccitt_false, descriptor_bytes_v1};
use super::{
    InspectedExternalToolStateV1, InspectedStudentArtifactStateV1, InspectedStudentResponseV1,
    PresentationBindingV1, PresentationNonceV1, RenderedItemRoleV1,
    RenderedResponseTranslationErrorV1, IssuedQuestionResponseFormatV1, project_durable_response_to_rendered_v1,
    project_rendered_response_for_inspection_v1, rebuild_public_presentation_v1,
    reproduce_presentation_v1, translate_rendered_response_v1, verify_presentation_v1,
};

fn choice(id: &str, text: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: text.to_owned(),
        }],
    }
}

fn fixture() -> QuestionEnvelope {
    QuestionEnvelope {
        question_version: QuestionVersionReference {
            question_id: "123-4567".parse().expect("valid Question ID"),
            version_number: QuestionVersionNumber::new(1).expect("positive version"),
        },
        seed: Seed::new(42),
        title: "Peptide bond".to_owned(),
        prompt: vec![ContentBlock::Text {
            markdown: "Which group forms the peptide bond?".to_owned(),
        }],
        response: QuestionResponseFormat::MultipleChoice {
            choices: vec![
                choice("amine", "Amino group"),
                choice("carboxyl", "Carboxyl group"),
            ],
            selection: SelectionCardinality::ExactlyOne,
        },
    }
}

struct Nonces {
    values: VecDeque<[u8; 16]>,
    calls: usize,
}

impl Nonces {
    fn new(values: impl IntoIterator<Item = [u8; 16]>) -> Self {
        Self {
            values: values.into_iter().collect(),
            calls: 0,
        }
    }
}

impl NonceSourceV1 for Nonces {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        self.calls += 1;
        self.values
            .pop_front()
            .ok_or(PresentationBuildError::RandomnessUnavailable)
    }
}

#[test]
fn crc_contract_uses_the_required_ccitt_false_vector() {
    assert_eq!(crc16_ccitt_false(b"123456789"), 0x29b1);
}

#[test]
fn descriptor_is_stable_answer_free_and_bound_to_every_visible_field() {
    let mut source = Nonces::new([[0x11; 16]]);
    let presentation = build_presentation_v1_with_nonce_source(&fixture(), &[], &mut source)
        .expect("valid presentation");
    let bytes = descriptor_bytes_v1(&presentation).expect("descriptor");
    let public = presentation.digest.public_token();

    assert!(bytes.starts_with(b"ple:presentation:v1\0\x01"));
    assert_eq!(
        presentation.envelope.presentation_nonce,
        PresentationNonceV1::from_bytes([0x11; 16])
    );
    assert_eq!(presentation.item_bindings.len(), 2);
    assert_ne!(
        presentation.item_bindings[0].rendered,
        presentation.item_bindings[1].rendered
    );
    assert_eq!(presentation.item_bindings[0].rendered.as_str(), "fe11");
    assert_eq!(
        presentation.digest.as_bytes(),
        [
            0x28, 0x33, 0x29, 0xc9, 0x8f, 0x30, 0xab, 0x41, 0xdd, 0x46, 0x65, 0x10, 0x3c, 0xed,
            0x2d, 0xf3, 0xca, 0xb5, 0xec, 0x4c, 0x07, 0xb1, 0xbd, 0xf1, 0x61, 0x3b, 0x8d, 0x30,
            0x39, 0x3f, 0x57, 0x35,
        ]
    );
    assert_eq!(public.as_str(), "pd1_KDMpyY8wq0HdRmUQPO0t8w");
    assert!(
        !serde_json::to_string(&presentation.envelope)
            .expect("public JSON")
            .contains("amine")
    );
    verify_presentation_v1(&presentation, presentation.digest, &public)
        .expect("matching descriptor");
    let public_rebuild = rebuild_public_presentation_v1(&presentation.envelope, &[])
        .expect("public presentation should reproduce the server descriptor");
    assert_eq!(public_rebuild.digest, presentation.digest);

    let mut changed = fixture();
    changed.title.push('!');
    let mut changed_source = Nonces::new([[0x11; 16]]);
    let changed = build_presentation_v1_with_nonce_source(&changed, &[], &mut changed_source)
        .expect("changed presentation");
    assert_ne!(presentation.digest, changed.digest);
}

#[test]
fn collision_retries_the_whole_presentation_with_a_fresh_nonce() {
    let mut source = Nonces::new([[1; 16], [2; 16]]);
    let mut calls = 0_usize;
    let presentation = build_presentation_v1_with_hasher(&fixture(), &[], &mut source, |bytes| {
        calls += 1;
        if calls <= 2 {
            7
        } else {
            crc16_ccitt_false(bytes)
        }
    })
    .expect("second nonce should resolve the injected collision");

    assert_eq!(source.calls, 2);
    assert_eq!(presentation.envelope.presentation_nonce.as_bytes(), [2; 16]);
}

#[test]
fn eight_colliding_presentations_fail_closed() {
    let mut source = Nonces::new([[3; 16]; 8]);
    let error = build_presentation_v1_with_hasher(&fixture(), &[], &mut source, |_| 0)
        .expect_err("a colliding presentation must not be issued");

    assert_eq!(error, PresentationBuildError::RenderedIdCollision);
    assert_eq!(source.calls, 8);
}

#[test]
fn public_json_uses_rendered_ids_and_schema_kind_only() {
    let mut source = Nonces::new([[4; 16]]);
    let presentation = build_presentation_v1_with_nonce_source(&fixture(), &[], &mut source)
        .expect("valid presentation");
    let IssuedQuestionResponseFormatV1::SingleChoice { choices } = &presentation.envelope.response else {
        panic!("single choice schema")
    };
    assert_eq!(choices.len(), 2);
    assert!(choices.iter().all(|choice| choice.id.as_str().len() == 4));
    let json = serde_json::to_value(&presentation.envelope).expect("public JSON");
    assert_eq!(json["response"]["kind"], "singleChoice");
    assert!(json.get("grading").is_none());
}

#[test]
fn persisted_binding_is_strict_and_round_trips_full_digest() {
    let mut source = Nonces::new([[5; 16]]);
    let presentation = build_presentation_v1_with_nonce_source(&fixture(), &[], &mut source)
        .expect("valid presentation");
    let binding = PresentationBindingV1::new(
        presentation.envelope.presentation_nonce,
        presentation.digest,
    );
    let json = serde_json::to_value(binding).expect("binding JSON");

    assert_eq!(json["descriptorVersion"], 1);
    assert_eq!(json["nonce"].as_str().expect("nonce").len(), 32);
    assert_eq!(json["digest"].as_str().expect("digest").len(), 64);
    assert_eq!(
        serde_json::from_value::<PresentationBindingV1>(json.clone()).expect("binding"),
        binding
    );

    let mut wrong_version = json.clone();
    wrong_version["descriptorVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<PresentationBindingV1>(wrong_version).is_err());

    let mut unknown = json;
    unknown["grading"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PresentationBindingV1>(unknown).is_err());

    let reproduced = reproduce_presentation_v1(&fixture(), &[], binding)
        .expect("persisted binding reproduces the exact presentation");
    assert_eq!(reproduced, presentation);

    let mut changed = fixture();
    changed.title.push('!');
    assert!(reproduce_presentation_v1(&changed, &[], binding).is_err());
}

fn presentation_for(response: QuestionResponseFormat) -> super::PresentationV1 {
    let mut envelope = fixture();
    envelope.response = response;
    let mut source = Nonces::new([[0x91; 16]]);
    build_presentation_v1_with_nonce_source(&envelope, &[], &mut source)
        .expect("valid presentation")
}

fn rendered(presentation: &super::PresentationV1, role: RenderedItemRoleV1) -> ChoiceId {
    ChoiceId::new(
        presentation
            .item_bindings
            .iter()
            .find(|binding| binding.role == role)
            .expect("role binding")
            .rendered
            .as_str(),
    )
}

#[test]
fn rendered_response_translation_rewrites_every_identifier_family() {
    let multiple = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![choice("a", "A"), choice("b", "B")],
        selection: SelectionCardinality::ExactlyOne,
    });
    let multiple_response = StudentResponse::MultipleChoice {
        selected: vec![rendered(&multiple, RenderedItemRoleV1::Choice)],
    };
    assert_eq!(
        translate_rendered_response_v1(&multiple_response, &multiple).expect("choice response"),
        StudentResponse::MultipleChoice {
            selected: vec![ChoiceId::new("a")],
        }
    );

    let blanks = presentation_for(QuestionResponseFormat::MultiBlank {
        blanks: vec![TextEntrySlot {
            id: ChoiceId::new("slot-a"),
            label: vec![ContentBlock::Text {
                markdown: "A".to_owned(),
            }],
            match_mode: TextMatchMode::Exact,
            max_length: 10,
        }],
    });
    let blanks_response = StudentResponse::MultiBlank {
        answers: vec![TextEntryAnswer {
            slot: rendered(&blanks, RenderedItemRoleV1::Blank),
            text: "value".to_owned(),
        }],
    };
    assert_eq!(
        translate_rendered_response_v1(&blanks_response, &blanks).expect("blank response"),
        StudentResponse::MultiBlank {
            answers: vec![TextEntryAnswer {
                slot: ChoiceId::new("slot-a"),
                text: "value".to_owned(),
            }],
        }
    );

    let matching = presentation_for(QuestionResponseFormat::Matching {
        prompts: vec![choice("prompt-a", "Prompt")],
        choices: vec![choice("choice-a", "Choice")],
    });
    let matching_response = StudentResponse::Matching {
        matches: vec![MatchPair {
            prompt: rendered(&matching, RenderedItemRoleV1::MatchPrompt),
            choice: rendered(&matching, RenderedItemRoleV1::MatchChoice),
        }],
    };
    assert_eq!(
        translate_rendered_response_v1(&matching_response, &matching).expect("matching response"),
        StudentResponse::Matching {
            matches: vec![MatchPair {
                prompt: ChoiceId::new("prompt-a"),
                choice: ChoiceId::new("choice-a"),
            }],
        }
    );

    let ordering = presentation_for(QuestionResponseFormat::Ordering {
        items: vec![choice("first", "First"), choice("second", "Second")],
    });
    let ordering_response = StudentResponse::Ordering {
        order: vec![rendered(&ordering, RenderedItemRoleV1::OrderItem)],
    };
    assert_eq!(
        translate_rendered_response_v1(&ordering_response, &ordering).expect("ordering response"),
        StudentResponse::Ordering {
            order: vec![ChoiceId::new("first")],
        }
    );
}

#[test]
fn rendered_response_translation_preserves_scalar_question_types() {
    let presentation = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![choice("a", "A")],
        selection: SelectionCardinality::ExactlyOne,
    });
    for response in [
        StudentResponse::Numeric { value: 1.25 },
        StudentResponse::ShortText {
            text: "alpha".to_owned(),
        },
        StudentResponse::Hotspot { points: vec![] },
        StudentResponse::FileUpload {
            object_key: "record.pdf".to_owned(),
        },
        StudentResponse::ExternalTool {},
    ] {
        assert_eq!(
            translate_rendered_response_v1(&response, &presentation).expect("scalar response"),
            response
        );
    }
}

#[test]
fn durable_response_projection_uses_only_issued_rendered_identifiers_and_safe_states() {
    let multiple = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![choice("a", "A")],
        selection: SelectionCardinality::ExactlyOne,
    });
    assert!(matches!(
        project_durable_response_to_rendered_v1(&StudentResponse::MultipleChoice { selected: vec![ChoiceId::new("a")] }, &multiple),
        Ok(InspectedStudentResponseV1::MultipleChoice { selected }) if selected == vec![multiple.item_bindings[0].rendered.clone()]
    ));

    let blank = presentation_for(QuestionResponseFormat::MultiBlank {
        blanks: vec![TextEntrySlot {
            id: ChoiceId::new("slot"),
            label: vec![],
            match_mode: TextMatchMode::Exact,
            max_length: 10,
        }],
    });
    assert!(matches!(
        project_durable_response_to_rendered_v1(&StudentResponse::MultiBlank { answers: vec![TextEntryAnswer { slot: ChoiceId::new("slot"), text: "entered".into() }] }, &blank),
        Ok(InspectedStudentResponseV1::MultiBlank { answers }) if answers[0].text == "entered"
    ));

    let matching = presentation_for(QuestionResponseFormat::Matching {
        prompts: vec![choice("p", "P")],
        choices: vec![choice("c", "C")],
    });
    assert!(matches!(
        project_durable_response_to_rendered_v1(
            &StudentResponse::Matching {
                matches: vec![MatchPair {
                    prompt: ChoiceId::new("p"),
                    choice: ChoiceId::new("c")
                }]
            },
            &matching
        ),
        Ok(InspectedStudentResponseV1::Matching { .. })
    ));
    let ordering = presentation_for(QuestionResponseFormat::Ordering {
        items: vec![choice("first", "First")],
    });
    assert!(matches!(
        project_durable_response_to_rendered_v1(
            &StudentResponse::Ordering {
                order: vec![ChoiceId::new("first")]
            },
            &ordering
        ),
        Ok(InspectedStudentResponseV1::Ordering { .. })
    ));

    assert_eq!(
        project_durable_response_to_rendered_v1(
            &StudentResponse::Numeric { value: 1.5 },
            &multiple,
        ),
        Ok(InspectedStudentResponseV1::Numeric { value: 1.5 })
    );
    assert_eq!(
        project_durable_response_to_rendered_v1(
            &StudentResponse::ShortText {
                text: "written".into(),
            },
            &multiple,
        ),
        Ok(InspectedStudentResponseV1::ShortText {
            text: "written".into(),
        })
    );
    assert_eq!(
        project_durable_response_to_rendered_v1(
            &StudentResponse::Hotspot { points: vec![] },
            &multiple,
        ),
        Ok(InspectedStudentResponseV1::Hotspot { points: vec![] })
    );
    assert_eq!(
        project_durable_response_to_rendered_v1(
            &StudentResponse::FileUpload {
                object_key: "private/object".into()
            },
            &multiple
        ),
        Ok(InspectedStudentResponseV1::FileUpload {
            artifact: InspectedStudentArtifactStateV1::Submitted
        })
    );
    assert_eq!(
        project_durable_response_to_rendered_v1(&StudentResponse::ExternalTool {}, &multiple),
        Ok(InspectedStudentResponseV1::ExternalTool {
            completion: InspectedExternalToolStateV1::SubmissionRecorded
        })
    );
}

#[test]
fn browser_submitted_response_round_trips_through_safe_inspection() {
    let presentation = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![choice("a", "A"), choice("b", "B")],
        selection: SelectionCardinality::ExactlyOne,
    });
    let submitted = StudentResponse::MultipleChoice {
        selected: vec![rendered(&presentation, RenderedItemRoleV1::Choice)],
    };
    let rebuilt = rebuild_public_presentation_v1(&presentation.envelope, &[])
        .expect("browser-safe presentation rebuild");

    assert!(matches!(
        project_rendered_response_for_inspection_v1(&submitted, &rebuilt),
        Ok(InspectedStudentResponseV1::MultipleChoice { selected })
            if selected == vec![presentation.item_bindings[0].rendered.clone()]
    ));
}

#[test]
fn rendered_response_translation_rejects_malformed_unknown_duplicate_and_wrong_role_ids() {
    let presentation = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![choice("a", "A")],
        selection: SelectionCardinality::ExactlyOne,
    });
    let response_for = |id| StudentResponse::MultipleChoice {
        selected: vec![ChoiceId::new(id)],
    };
    assert_eq!(
        translate_rendered_response_v1(&response_for("not-an-id"), &presentation),
        Err(RenderedResponseTranslationErrorV1::MalformedRenderedId)
    );

    let unknown = (0_u16..=u16::MAX)
        .map(|value| format!("{value:04x}"))
        .find(|id| {
            !presentation
                .item_bindings
                .iter()
                .any(|binding| binding.rendered.as_str() == id)
        })
        .expect("unused rendered identifier");
    assert_eq!(
        translate_rendered_response_v1(&response_for(&unknown), &presentation),
        Err(RenderedResponseTranslationErrorV1::UnknownRenderedId)
    );

    let mut duplicate = presentation.clone();
    duplicate
        .item_bindings
        .push(duplicate.item_bindings[0].clone());
    assert_eq!(
        translate_rendered_response_v1(
            &response_for(presentation.item_bindings[0].rendered.as_str()),
            &duplicate,
        ),
        Err(RenderedResponseTranslationErrorV1::DuplicateRenderedIdBinding)
    );

    let matching = presentation_for(QuestionResponseFormat::Matching {
        prompts: vec![choice("prompt-a", "Prompt")],
        choices: vec![choice("choice-a", "Choice")],
    });
    assert_eq!(
        translate_rendered_response_v1(
            &response_for(rendered(&matching, RenderedItemRoleV1::MatchPrompt).as_str()),
            &matching,
        ),
        Err(RenderedResponseTranslationErrorV1::WrongRenderedItemRole)
    );
}
