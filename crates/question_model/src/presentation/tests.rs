use std::collections::VecDeque;

use crate::answer::{ResponseSelectionRule, TextResponseMatchRule};
use crate::envelope::{QuestionContentBlock, QuestionVariationPresentation};
use crate::generation::QuestionSeed;
use crate::response::{
    HotspotRegion, MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice,
    QuestionResponseFormat, ResponseItemReference, StudentHotspotSelection, StudentMatch,
    StudentResponse, StudentTextEntry, TextEntrySlot,
};
use crate::{QuestionAttemptId, QuestionRevisionNumber, QuestionRevisionReference};

use super::builder::{
    PresentationBuildError, QuestionPresentationNonceSource,
    build_question_presentation_with_hasher, build_question_presentation_with_nonce_source,
};
use super::codec::{crc16_ccitt_false, descriptor_bytes};
use super::{
    InspectedImathasQuestionBackendState, QuestionPresentationBinding, QuestionPresentationNonce,
    QuestionPresentationResponseFormat, QuestionPresentationToken,
    RenderedResponseTranslationError, ResponseItemRole, StudentAttemptDescriptor,
    StudentResponseInspection, project_durable_response_to_rendered,
    project_rendered_response_for_inspection, rebuild_public_question_presentation,
    reproduce_question_presentation, translate_rendered_response, verify_question_presentation,
};

fn question_choice(id: &str, text: &str) -> QuestionChoice {
    QuestionChoice {
        id: ResponseItemReference::new(id),
        body: response_item_body(text),
    }
}

fn matching_prompt(id: &str, text: &str) -> MatchingPrompt {
    MatchingPrompt {
        id: ResponseItemReference::new(id),
        body: response_item_body(text),
    }
}

fn matching_choice(id: &str, text: &str) -> MatchingChoice {
    MatchingChoice {
        id: ResponseItemReference::new(id),
        body: response_item_body(text),
    }
}

fn ordering_item(id: &str, text: &str) -> OrderingItem {
    OrderingItem {
        id: ResponseItemReference::new(id),
        body: response_item_body(text),
    }
}

fn response_item_body(text: &str) -> Vec<QuestionContentBlock> {
    vec![QuestionContentBlock::Text {
        markdown: text.to_owned(),
    }]
}

fn fixture() -> QuestionVariationPresentation {
    QuestionVariationPresentation {
        variation: crate::QuestionVariation::static_variation(
            QuestionRevisionReference {
                question_id: "123-4567".parse().expect("valid Question ID"),
                revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
            },
            QuestionSeed::new(42),
        ),
        title: "Peptide bond".to_owned(),
        prompt: vec![QuestionContentBlock::Text {
            markdown: "Which group forms the peptide bond?".to_owned(),
        }],
        response: QuestionResponseFormat::MultipleChoice {
            choices: vec![
                question_choice("amine", "Amino group"),
                question_choice("carboxyl", "Carboxyl group"),
            ],
            selection: ResponseSelectionRule::ExactlyOne,
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

impl QuestionPresentationNonceSource for Nonces {
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
    let presentation = build_question_presentation_with_nonce_source(&fixture(), &[], &mut source)
        .expect("valid presentation");
    let bytes = descriptor_bytes(&presentation).expect("descriptor");
    let public = presentation.checksum.public_token();

    assert!(bytes.starts_with(b"ple:presentation:v1\0\x01"));
    assert_eq!(
        presentation.presentation.presentation_nonce,
        QuestionPresentationNonce::from_bytes([0x11; 16])
    );
    assert_eq!(presentation.item_bindings.len(), 2);
    assert!(
        presentation
            .item_bindings
            .iter()
            .all(|binding| binding.response_item_reference.is_some())
    );
    assert_ne!(
        presentation.item_bindings[0].rendered,
        presentation.item_bindings[1].rendered
    );
    assert_eq!(presentation.item_bindings[0].rendered.as_str(), "fe11");
    assert_eq!(
        presentation.checksum.as_bytes(),
        [
            0x28, 0x33, 0x29, 0xc9, 0x8f, 0x30, 0xab, 0x41, 0xdd, 0x46, 0x65, 0x10, 0x3c, 0xed,
            0x2d, 0xf3, 0xca, 0xb5, 0xec, 0x4c, 0x07, 0xb1, 0xbd, 0xf1, 0x61, 0x3b, 0x8d, 0x30,
            0x39, 0x3f, 0x57, 0x35,
        ]
    );
    assert_eq!(public.as_str(), "pd1_KDMpyY8wq0HdRmUQPO0t8w");
    assert!(
        !serde_json::to_string(&presentation.presentation)
            .expect("public JSON")
            .contains("amine")
    );
    verify_question_presentation(&presentation, presentation.checksum, &public)
        .expect("matching descriptor");
    let public_rebuild = rebuild_public_question_presentation(&presentation.presentation, &[])
        .expect("public presentation should reproduce the server descriptor");
    assert_eq!(public_rebuild.checksum, presentation.checksum);
    assert!(
        public_rebuild
            .item_bindings
            .iter()
            .all(|binding| binding.response_item_reference.is_none())
    );

    let mut changed = fixture();
    changed.title.push('!');
    let mut changed_source = Nonces::new([[0x11; 16]]);
    let changed = build_question_presentation_with_nonce_source(&changed, &[], &mut changed_source)
        .expect("changed presentation");
    assert_ne!(presentation.checksum, changed.checksum);
}

#[test]
fn student_attempt_descriptor_serializes_the_browser_safe_presentation_token() {
    let descriptor = StudentAttemptDescriptor {
        id: QuestionAttemptId::from_uuid(uuid::Uuid::from_u128(1)),
        deadline: None,
        presentation_token: QuestionPresentationToken::parse("pd1_q2fE1ezXCkT6_yd7zeqkCQ")
            .expect("canonical presentation token"),
    };

    let value = serde_json::to_value(&descriptor).expect("student screen JSON");
    assert_eq!(
        value,
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "deadline": null,
            "presentationToken": "pd1_q2fE1ezXCkT6_yd7zeqkCQ",
        })
    );
    assert!(
        serde_json::from_value::<StudentAttemptDescriptor>(serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "deadline": null,
            "presentationDigest": "pd1_q2fE1ezXCkT6_yd7zeqkCQ",
        }))
        .is_err(),
        "the retired public field is not a compatibility alias"
    );
}

#[test]
fn collision_retries_the_whole_presentation_with_a_fresh_nonce() {
    let mut source = Nonces::new([[1; 16], [2; 16]]);
    let mut calls = 0_usize;
    let presentation =
        build_question_presentation_with_hasher(&fixture(), &[], &mut source, |bytes| {
            calls += 1;
            if calls <= 2 {
                7
            } else {
                crc16_ccitt_false(bytes)
            }
        })
        .expect("second nonce should resolve the injected collision");

    assert_eq!(source.calls, 2);
    assert_eq!(
        presentation.presentation.presentation_nonce.as_bytes(),
        [2; 16]
    );
}

#[test]
fn eight_colliding_presentations_fail_closed() {
    let mut source = Nonces::new([[3; 16]; 8]);
    let error = build_question_presentation_with_hasher(&fixture(), &[], &mut source, |_| 0)
        .expect_err("a colliding presentation must not be issued");

    assert_eq!(error, PresentationBuildError::RenderedIdCollision);
    assert_eq!(source.calls, 8);
}

#[test]
fn public_json_uses_rendered_ids_and_schema_kind_only() {
    let mut source = Nonces::new([[4; 16]]);
    let presentation = build_question_presentation_with_nonce_source(&fixture(), &[], &mut source)
        .expect("valid presentation");
    let QuestionPresentationResponseFormat::SingleChoice { choices } =
        &presentation.presentation.response
    else {
        panic!("single choice schema")
    };
    assert_eq!(choices.len(), 2);
    assert!(choices.iter().all(|choice| choice.id.as_str().len() == 4));
    let json = serde_json::to_value(&presentation.presentation).expect("public JSON");
    assert_eq!(json["response"]["kind"], "singleChoice");
    assert!(json.get("grading").is_none());
}

#[test]
fn persisted_binding_is_strict_and_round_trips_full_checksum() {
    let mut source = Nonces::new([[5; 16]]);
    let presentation = build_question_presentation_with_nonce_source(&fixture(), &[], &mut source)
        .expect("valid presentation");
    let binding = QuestionPresentationBinding::new(
        presentation.presentation.presentation_nonce,
        presentation.checksum,
    );
    let json = serde_json::to_value(binding).expect("binding JSON");

    assert_eq!(json["descriptorVersion"], 1);
    assert_eq!(json["nonce"].as_str().expect("nonce").len(), 32);
    assert_eq!(json["checksum"].as_str().expect("checksum").len(), 64);
    assert_eq!(
        serde_json::from_value::<QuestionPresentationBinding>(json.clone()).expect("binding"),
        binding
    );

    let mut wrong_version = json.clone();
    wrong_version["descriptorVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<QuestionPresentationBinding>(wrong_version).is_err());

    let mut unknown = json;
    unknown["grading"] = serde_json::json!(true);
    assert!(serde_json::from_value::<QuestionPresentationBinding>(unknown).is_err());

    let reproduced = reproduce_question_presentation(&fixture(), &[], binding)
        .expect("persisted binding reproduces the exact presentation");
    assert_eq!(reproduced, presentation);

    let mut changed = fixture();
    changed.title.push('!');
    assert!(reproduce_question_presentation(&changed, &[], binding).is_err());
}

fn presentation_for(response: QuestionResponseFormat) -> super::IssuedQuestionPresentation {
    let mut envelope = fixture();
    envelope.response = response;
    let mut source = Nonces::new([[0x91; 16]]);
    build_question_presentation_with_nonce_source(&envelope, &[], &mut source)
        .expect("valid presentation")
}

fn hotspot_presentation() -> super::IssuedQuestionPresentation {
    let asset = crate::envelope::QuestionAssetReference {
        asset: crate::QuestionAssetId::from_uuid(uuid::Uuid::from_u128(1)),
        checksum: "a".repeat(64),
    };
    let mut envelope = fixture();
    envelope.response = QuestionResponseFormat::Hotspot {
        surface: asset.clone(),
        description: "Cell diagram".to_owned(),
        regions: vec![HotspotRegion {
            id: ResponseItemReference::new("nucleus"),
            label: vec![QuestionContentBlock::Text {
                markdown: "Nucleus".to_owned(),
            }],
            x: 1_000,
            y: 1_000,
            width: 2_000,
            height: 2_000,
        }],
        selection: ResponseSelectionRule::ExactlyOne,
    };
    let bindings = [super::QuestionAssetRendition {
        question_asset: asset.clone(),
        rendition_checksum: asset.checksum,
        intrinsic_width: Some(800),
        intrinsic_height: Some(600),
    }];
    let mut source = Nonces::new([[0x92; 16]]);
    build_question_presentation_with_nonce_source(&envelope, &bindings, &mut source)
        .expect("valid hotspot presentation")
}

fn rendered(
    presentation: &super::IssuedQuestionPresentation,
    role: ResponseItemRole,
) -> ResponseItemReference {
    ResponseItemReference::new(
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
        choices: vec![question_choice("a", "A"), question_choice("b", "B")],
        selection: ResponseSelectionRule::ExactlyOne,
    });
    let multiple_response = StudentResponse::MultipleChoice {
        selected: vec![rendered(&multiple, ResponseItemRole::QuestionChoice)],
    };
    assert_eq!(
        translate_rendered_response(&multiple_response, &multiple).expect("choice response"),
        StudentResponse::MultipleChoice {
            selected: vec![ResponseItemReference::new("a")],
        }
    );

    let blanks = presentation_for(QuestionResponseFormat::MultiBlank {
        blanks: vec![TextEntrySlot {
            id: ResponseItemReference::new("slot-a"),
            label: vec![QuestionContentBlock::Text {
                markdown: "A".to_owned(),
            }],
            match_mode: TextResponseMatchRule::Exact,
            max_length: 10,
        }],
    });
    let blanks_response = StudentResponse::MultiBlank {
        answers: vec![StudentTextEntry {
            slot: rendered(&blanks, ResponseItemRole::TextEntrySlot),
            text: "value".to_owned(),
        }],
    };
    assert_eq!(
        translate_rendered_response(&blanks_response, &blanks).expect("blank response"),
        StudentResponse::MultiBlank {
            answers: vec![StudentTextEntry {
                slot: ResponseItemReference::new("slot-a"),
                text: "value".to_owned(),
            }],
        }
    );

    let matching = presentation_for(QuestionResponseFormat::Matching {
        prompts: vec![matching_prompt("prompt-a", "Prompt")],
        choices: vec![matching_choice("choice-a", "Choice")],
    });
    let matching_response = StudentResponse::Matching {
        matches: vec![StudentMatch {
            prompt: rendered(&matching, ResponseItemRole::MatchingPrompt),
            choice: rendered(&matching, ResponseItemRole::MatchingChoice),
        }],
    };
    assert_eq!(
        translate_rendered_response(&matching_response, &matching).expect("matching response"),
        StudentResponse::Matching {
            matches: vec![StudentMatch {
                prompt: ResponseItemReference::new("prompt-a"),
                choice: ResponseItemReference::new("choice-a"),
            }],
        }
    );

    let ordering = presentation_for(QuestionResponseFormat::Ordering {
        items: vec![
            ordering_item("first", "First"),
            ordering_item("second", "Second"),
        ],
    });
    let ordering_response = StudentResponse::Ordering {
        order: vec![rendered(&ordering, ResponseItemRole::OrderingItem)],
    };
    assert_eq!(
        translate_rendered_response(&ordering_response, &ordering).expect("ordering response"),
        StudentResponse::Ordering {
            order: vec![ResponseItemReference::new("first")],
        }
    );

    let hotspot = hotspot_presentation();
    let hotspot_response = StudentResponse::Hotspot {
        selections: vec![StudentHotspotSelection {
            region: rendered(&hotspot, ResponseItemRole::HotspotRegion),
        }],
    };
    assert_eq!(
        translate_rendered_response(&hotspot_response, &hotspot).expect("hotspot response"),
        StudentResponse::Hotspot {
            selections: vec![StudentHotspotSelection {
                region: ResponseItemReference::new("nucleus"),
            }],
        }
    );
}

#[test]
fn rendered_response_translation_preserves_scalar_question_types() {
    let presentation = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![question_choice("a", "A")],
        selection: ResponseSelectionRule::ExactlyOne,
    });
    for response in [
        StudentResponse::Numeric { value: 1.25 },
        StudentResponse::ShortText {
            text: "alpha".to_owned(),
        },
        StudentResponse::Hotspot { selections: vec![] },
        StudentResponse::ImathasQuestionBackend {},
    ] {
        assert_eq!(
            translate_rendered_response(&response, &presentation).expect("scalar response"),
            response
        );
    }
}

#[test]
fn durable_response_projection_uses_only_issued_rendered_identifiers_and_safe_states() {
    let multiple = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![question_choice("a", "A")],
        selection: ResponseSelectionRule::ExactlyOne,
    });
    assert!(matches!(
        project_durable_response_to_rendered(&StudentResponse::MultipleChoice { selected: vec![ResponseItemReference::new("a")] }, &multiple),
        Ok(StudentResponseInspection::MultipleChoice { selected }) if selected == vec![multiple.item_bindings[0].rendered.clone()]
    ));

    let blank = presentation_for(QuestionResponseFormat::MultiBlank {
        blanks: vec![TextEntrySlot {
            id: ResponseItemReference::new("slot"),
            label: vec![],
            match_mode: TextResponseMatchRule::Exact,
            max_length: 10,
        }],
    });
    assert!(matches!(
        project_durable_response_to_rendered(&StudentResponse::MultiBlank { answers: vec![StudentTextEntry { slot: ResponseItemReference::new("slot"), text: "entered".into() }] }, &blank),
        Ok(StudentResponseInspection::MultiBlank { answers }) if answers[0].text == "entered"
    ));

    let matching = presentation_for(QuestionResponseFormat::Matching {
        prompts: vec![matching_prompt("p", "P")],
        choices: vec![matching_choice("c", "C")],
    });
    assert!(matches!(
        project_durable_response_to_rendered(
            &StudentResponse::Matching {
                matches: vec![StudentMatch {
                    prompt: ResponseItemReference::new("p"),
                    choice: ResponseItemReference::new("c")
                }]
            },
            &matching
        ),
        Ok(StudentResponseInspection::Matching { .. })
    ));
    let ordering = presentation_for(QuestionResponseFormat::Ordering {
        items: vec![ordering_item("first", "First")],
    });
    assert!(matches!(
        project_durable_response_to_rendered(
            &StudentResponse::Ordering {
                order: vec![ResponseItemReference::new("first")]
            },
            &ordering
        ),
        Ok(StudentResponseInspection::Ordering { .. })
    ));

    assert_eq!(
        project_durable_response_to_rendered(&StudentResponse::Numeric { value: 1.5 }, &multiple,),
        Ok(StudentResponseInspection::Numeric { value: 1.5 })
    );
    assert_eq!(
        project_durable_response_to_rendered(
            &StudentResponse::ShortText {
                text: "written".into(),
            },
            &multiple,
        ),
        Ok(StudentResponseInspection::ShortText {
            text: "written".into(),
        })
    );
    assert_eq!(
        project_durable_response_to_rendered(
            &StudentResponse::Hotspot { selections: vec![] },
            &multiple,
        ),
        Ok(StudentResponseInspection::Hotspot {
            selected_regions: vec![],
        })
    );
    assert_eq!(
        project_durable_response_to_rendered(
            &StudentResponse::ImathasQuestionBackend {},
            &multiple
        ),
        Ok(StudentResponseInspection::ImathasQuestionBackend {
            completion: InspectedImathasQuestionBackendState::SubmissionRecorded
        })
    );
}

#[test]
fn browser_submitted_response_round_trips_through_safe_inspection() {
    let presentation = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![question_choice("a", "A"), question_choice("b", "B")],
        selection: ResponseSelectionRule::ExactlyOne,
    });
    let submitted = StudentResponse::MultipleChoice {
        selected: vec![rendered(&presentation, ResponseItemRole::QuestionChoice)],
    };
    let rebuilt = rebuild_public_question_presentation(&presentation.presentation, &[])
        .expect("browser-safe presentation rebuild");

    assert!(matches!(
        project_rendered_response_for_inspection(&submitted, &rebuilt),
        Ok(StudentResponseInspection::MultipleChoice { selected })
            if selected == vec![presentation.item_bindings[0].rendered.clone()]
    ));
}

#[test]
fn rendered_response_translation_rejects_malformed_unknown_duplicate_and_wrong_role_ids() {
    let presentation = presentation_for(QuestionResponseFormat::MultipleChoice {
        choices: vec![question_choice("a", "A")],
        selection: ResponseSelectionRule::ExactlyOne,
    });
    let response_for = |id| StudentResponse::MultipleChoice {
        selected: vec![ResponseItemReference::new(id)],
    };
    assert_eq!(
        translate_rendered_response(&response_for("not-an-id"), &presentation),
        Err(RenderedResponseTranslationError::MalformedRenderedId)
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
        translate_rendered_response(&response_for(&unknown), &presentation),
        Err(RenderedResponseTranslationError::UnknownRenderedId)
    );

    let mut duplicate = presentation.clone();
    duplicate
        .item_bindings
        .push(duplicate.item_bindings[0].clone());
    assert_eq!(
        translate_rendered_response(
            &response_for(presentation.item_bindings[0].rendered.as_str()),
            &duplicate,
        ),
        Err(RenderedResponseTranslationError::DuplicateRenderedIdBinding)
    );

    let matching = presentation_for(QuestionResponseFormat::Matching {
        prompts: vec![matching_prompt("prompt-a", "Prompt")],
        choices: vec![matching_choice("choice-a", "Choice")],
    });
    assert_eq!(
        translate_rendered_response(
            &response_for(rendered(&matching, ResponseItemRole::MatchingPrompt).as_str()),
            &matching,
        ),
        Err(RenderedResponseTranslationError::WrongRenderedItemRole)
    );
}
