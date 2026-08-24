//! Canonical conversion from an ordinary issued presentation to rehearsal.
//!
//! This pure projection accepts the already-issued answer-free representation,
//! preserving its server-minted rendered identifiers.  Persistence, routes,
//! and execution therefore validate exactly the same committed rehearsal
//! screen without depending on a server-local adapter.

use crate::envelope::ContentBlock;
use crate::{
    PresentationEnvelopeV1, PresentedBlankV1, PresentedChoiceV1, RehearsalActiveScreenV1,
    RehearsalContentBlockV1, RehearsalPresentedBlankV1, RehearsalPresentedChoiceV1,
    RehearsalQuestionPresentationV1, RehearsalResponseSchemaV1, ResponseSchemaV1,
};

/// Closed failure set for the version-one issued-presentation projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalIssuedPresentationErrorV1 {
    /// The current rehearsal asset-token path has not yet been authorized.
    AssetBearingContent,
    /// The issued values violate the bounded version-one rehearsal protocol.
    InvalidPresentation,
}

impl std::fmt::Display for RehearsalIssuedPresentationErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssetBearingContent => formatter
                .write_str("rehearsal delivery does not yet support asset-bearing presentations"),
            Self::InvalidPresentation => {
                formatter.write_str("issued presentation is invalid for rehearsal delivery")
            }
        }
    }
}

impl std::error::Error for RehearsalIssuedPresentationErrorV1 {}

/// Creates one complete committed rehearsal screen from an ordinary issued
/// answer-free presentation.
///
/// The input is intentionally `PresentationEnvelopeV1`, rather than a raw
/// `QuestionEnvelope`: ordinary issuance has already minted presentation-
/// scoped rendered item IDs.  The resulting screen omits the source version,
/// seed, and issuance nonce while retaining only browser-safe visible data.
pub fn rehearsal_active_screen_from_issued_presentation_v1(
    issued: &PresentationEnvelopeV1,
) -> Result<RehearsalActiveScreenV1, RehearsalIssuedPresentationErrorV1> {
    let presentation = RehearsalQuestionPresentationV1 {
        title: issued.title.clone(),
        prompt: content_blocks(&issued.prompt)?,
        response: response_schema(&issued.response)?,
    };
    RehearsalActiveScreenV1::new(presentation)
        .map_err(|_| RehearsalIssuedPresentationErrorV1::InvalidPresentation)
}

fn content_blocks(
    blocks: &[ContentBlock],
) -> Result<Vec<RehearsalContentBlockV1>, RehearsalIssuedPresentationErrorV1> {
    blocks.iter().map(content_block).collect()
}

fn content_block(
    block: &ContentBlock,
) -> Result<RehearsalContentBlockV1, RehearsalIssuedPresentationErrorV1> {
    match block {
        ContentBlock::Text { markdown } => Ok(RehearsalContentBlockV1::Text {
            markdown: markdown.clone(),
        }),
        ContentBlock::Math { latex, description } => Ok(RehearsalContentBlockV1::Math {
            latex: latex.clone(),
            description: description.clone(),
        }),
        ContentBlock::Code { language, source } => Ok(RehearsalContentBlockV1::Code {
            language: language.clone(),
            source: source.clone(),
        }),
        ContentBlock::Table {
            headers,
            rows,
            description,
        } => Ok(RehearsalContentBlockV1::Table {
            headers: headers.clone(),
            rows: rows.clone(),
            description: description.clone(),
        }),
        ContentBlock::Image { .. } => Err(RehearsalIssuedPresentationErrorV1::AssetBearingContent),
    }
}

fn response_schema(
    response: &ResponseSchemaV1,
) -> Result<RehearsalResponseSchemaV1, RehearsalIssuedPresentationErrorV1> {
    match response {
        ResponseSchemaV1::SingleChoice { choices } => Ok(RehearsalResponseSchemaV1::SingleChoice {
            choices: presented_choices(choices)?,
        }),
        ResponseSchemaV1::MultipleAnswer {
            choices: values,
            minimum,
            maximum,
        } => Ok(RehearsalResponseSchemaV1::MultipleAnswer {
            choices: presented_choices(values)?,
            minimum: *minimum,
            maximum: *maximum,
        }),
        ResponseSchemaV1::FillIn { max_characters } => Ok(RehearsalResponseSchemaV1::FillIn {
            max_characters: *max_characters,
        }),
        ResponseSchemaV1::MultiFillIn { blanks } => Ok(RehearsalResponseSchemaV1::MultiFillIn {
            blanks: presented_blanks(blanks)?,
        }),
        ResponseSchemaV1::Numerical {
            max_characters,
            displayed_unit,
        } => Ok(RehearsalResponseSchemaV1::Numerical {
            max_characters: *max_characters,
            displayed_unit: displayed_unit.clone(),
        }),
        ResponseSchemaV1::Matching {
            prompts,
            choices: values,
            reuse_choices,
        } => Ok(RehearsalResponseSchemaV1::Matching {
            prompts: presented_choices(prompts)?,
            choices: presented_choices(values)?,
            reuse_choices: *reuse_choices,
        }),
        ResponseSchemaV1::Ordering { items } => Ok(RehearsalResponseSchemaV1::Ordering {
            items: presented_choices(items)?,
        }),
        // A hotspot necessarily contains an ordinary AssetRef.  Admit it only
        // with an authorized asset-token issuer and a dedicated projection.
        ResponseSchemaV1::Hotspot { .. } => {
            Err(RehearsalIssuedPresentationErrorV1::AssetBearingContent)
        }
    }
}

fn presented_choices(
    values: &[PresentedChoiceV1],
) -> Result<Vec<RehearsalPresentedChoiceV1>, RehearsalIssuedPresentationErrorV1> {
    values
        .iter()
        .map(|choice| {
            Ok(RehearsalPresentedChoiceV1 {
                id: choice.id.clone(),
                body: content_blocks(&choice.body)?,
            })
        })
        .collect()
}

fn presented_blanks(
    values: &[PresentedBlankV1],
) -> Result<Vec<RehearsalPresentedBlankV1>, RehearsalIssuedPresentationErrorV1> {
    values
        .iter()
        .map(|blank| {
            Ok(RehearsalPresentedBlankV1 {
                id: blank.id.clone(),
                label: content_blocks(&blank.label)?,
                max_characters: blank.max_characters,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::envelope::{AssetRef, ContentBlock};
    use crate::generation::Seed;
    use crate::{
        AssetId, PresentationEnvelopeV1, PresentationNonceV1, PresentedBlankV1, PresentedChoiceV1,
        RehearsalContentBlockV1, RehearsalResponseSchemaV1, RenderedItemIdV1, ResponseSchemaV1,
        VersionId,
    };

    use super::{
        RehearsalIssuedPresentationErrorV1, rehearsal_active_screen_from_issued_presentation_v1,
    };

    fn id(value: &str) -> RenderedItemIdV1 {
        RenderedItemIdV1::parse(value).expect("fixture rendered ID")
    }

    fn choice(value: &str) -> PresentedChoiceV1 {
        PresentedChoiceV1 {
            id: id(value),
            body: vec![ContentBlock::Text {
                markdown: format!("choice {value}"),
            }],
        }
    }

    fn issued(response: ResponseSchemaV1) -> PresentationEnvelopeV1 {
        PresentationEnvelopeV1 {
            version: VersionId::from_uuid(uuid::Uuid::from_u128(1)),
            seed: Seed::new(7),
            presentation_nonce: PresentationNonceV1::from_bytes([9; 16]),
            title: "Live enzyme question".to_string(),
            prompt: vec![
                ContentBlock::Math {
                    latex: "k_{cat}".to_string(),
                    description: "k cat".to_string(),
                },
                ContentBlock::Code {
                    language: "python".to_string(),
                    source: "rate = 4".to_string(),
                },
                ContentBlock::Table {
                    headers: vec!["trial".to_string()],
                    rows: vec![vec!["1".to_string()]],
                    description: "one trial".to_string(),
                },
            ],
            response,
        }
    }

    #[test]
    fn converts_every_current_no_asset_native_response_family() {
        let choice_a = choice("000a");
        let choice_b = choice("000b");
        let blank = PresentedBlankV1 {
            id: id("000c"),
            label: vec![ContentBlock::Text {
                markdown: "substrate".to_string(),
            }],
            max_characters: 40,
        };
        let schemas = vec![
            ResponseSchemaV1::SingleChoice {
                choices: vec![choice_a.clone(), choice_b.clone()],
            },
            ResponseSchemaV1::MultipleAnswer {
                choices: vec![choice_a.clone(), choice_b.clone()],
                minimum: 1,
                maximum: 2,
            },
            ResponseSchemaV1::FillIn { max_characters: 40 },
            ResponseSchemaV1::MultiFillIn {
                blanks: vec![blank],
            },
            ResponseSchemaV1::Numerical {
                max_characters: 12,
                displayed_unit: Some("mM".to_string()),
            },
            ResponseSchemaV1::Matching {
                prompts: vec![choice_a.clone()],
                choices: vec![choice_b.clone()],
                reuse_choices: false,
            },
            ResponseSchemaV1::Ordering {
                items: vec![choice_a, choice_b],
            },
        ];

        for schema in schemas {
            let screen = rehearsal_active_screen_from_issued_presentation_v1(&issued(schema))
                .expect("current native family converts");
            screen
                .validate()
                .expect("adapter builds a committed screen");
            assert!(matches!(
                screen.presentation.prompt.as_slice(),
                [
                    RehearsalContentBlockV1::Math { .. },
                    RehearsalContentBlockV1::Code { .. },
                    RehearsalContentBlockV1::Table { .. }
                ]
            ));
        }
    }

    #[test]
    fn preserves_issued_rendered_ids_without_retaining_issuance_metadata() {
        let screen = rehearsal_active_screen_from_issued_presentation_v1(&issued(
            ResponseSchemaV1::SingleChoice {
                choices: vec![choice("000a"), choice("000b")],
            },
        ))
        .expect("screen");

        let RehearsalResponseSchemaV1::SingleChoice { ref choices } = screen.presentation.response
        else {
            panic!("expected single-choice schema");
        };
        assert_eq!(choices[0].id.as_str(), "000a");
        assert_eq!(choices[1].id.as_str(), "000b");
        let wire = serde_json::to_value(&screen).expect("screen wire value");
        assert!(wire.get("version").is_none());
        assert!(wire.get("seed").is_none());
        assert!(wire.get("presentationNonce").is_none());
    }

    #[test]
    fn rejects_asset_bearing_prompt_choice_blank_and_hotspot_presentations() {
        let image = ContentBlock::Image {
            asset: AssetRef {
                asset: AssetId::from_uuid(uuid::Uuid::from_u128(2)),
                checksum: "a".repeat(64),
            },
            description: "enzyme structure".to_string(),
        };
        let mut prompt_image = issued(ResponseSchemaV1::FillIn { max_characters: 10 });
        prompt_image.prompt = vec![image.clone()];
        let choice_image = issued(ResponseSchemaV1::SingleChoice {
            choices: vec![
                PresentedChoiceV1 {
                    id: id("000a"),
                    body: vec![image.clone()],
                },
                choice("000b"),
            ],
        });
        let blank_image = issued(ResponseSchemaV1::MultiFillIn {
            blanks: vec![PresentedBlankV1 {
                id: id("000c"),
                label: vec![image],
                max_characters: 10,
            }],
        });
        let hotspot = issued(ResponseSchemaV1::Hotspot {
            surface: crate::PresentedHotspotSurfaceV1 {
                id: id("000d"),
                asset: AssetRef {
                    asset: AssetId::from_uuid(uuid::Uuid::from_u128(3)),
                    checksum: "b".repeat(64),
                },
                description: "enzyme surface".to_string(),
                regions: Vec::new(),
            },
            minimum: 1,
            maximum: 1,
        });

        for presentation in [prompt_image, choice_image, blank_image, hotspot] {
            assert_eq!(
                rehearsal_active_screen_from_issued_presentation_v1(&presentation),
                Err(RehearsalIssuedPresentationErrorV1::AssetBearingContent)
            );
        }
    }

    #[test]
    fn rejects_presentations_outside_rehearsal_bounds() {
        let invalid = issued(ResponseSchemaV1::FillIn { max_characters: 0 });

        assert_eq!(
            rehearsal_active_screen_from_issued_presentation_v1(&invalid),
            Err(RehearsalIssuedPresentationErrorV1::InvalidPresentation)
        );
    }
}
