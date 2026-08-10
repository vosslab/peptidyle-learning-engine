use std::collections::VecDeque;

use uuid::Uuid;

use crate::answer::SelectionCardinality;
use crate::envelope::{ContentBlock, QuestionEnvelope};
use crate::generation::Seed;
use crate::identity::VersionId;
use crate::response::{ChoiceId, ChoiceOption, ResponseDefinition};

use super::builder::{
    NonceSourceV1, PresentationBuildError, build_presentation_v1_with_hasher,
    build_presentation_v1_with_nonce_source,
};
use super::codec::{crc16_ccitt_false, descriptor_bytes_v1};
use super::{
    PresentationBindingV1, PresentationNonceV1, ResponseSchemaV1, rebuild_public_presentation_v1,
    reproduce_presentation_v1, verify_presentation_v1,
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
        version: VersionId::from_uuid(Uuid::from_u128(0x0192_3f4b_5c6d_7e8f_9012_3456_789a_bcde)),
        seed: Seed::new(42),
        title: "Peptide bond".to_owned(),
        prompt: vec![ContentBlock::Text {
            markdown: "Which group forms the peptide bond?".to_owned(),
        }],
        response: ResponseDefinition::MultipleChoice {
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
    assert_eq!(presentation.item_bindings[0].rendered.as_str(), "cfdf");
    assert_eq!(presentation.item_bindings[1].rendered.as_str(), "6603");
    assert_eq!(
        presentation.digest.as_bytes(),
        [
            0x84, 0xbd, 0x81, 0x11, 0xe1, 0x88, 0x7f, 0x35, 0x07, 0xc9, 0xa0, 0xcc, 0x5b, 0x9b,
            0x28, 0xe8, 0x86, 0xb7, 0x2e, 0x70, 0x4a, 0xc5, 0x90, 0x8c, 0xa9, 0x51, 0x3c, 0x5e,
            0xec, 0x97, 0xe7, 0x7d,
        ]
    );
    assert_eq!(public.as_str(), "pd1_hL2BEeGIfzUHyaDMW5so6A");
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
    let ResponseSchemaV1::SingleChoice { choices } = &presentation.envelope.response else {
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
