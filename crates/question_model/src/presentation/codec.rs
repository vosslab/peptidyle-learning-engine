//! Normative binary descriptor and rendered-item codecs.

use sha2::{Digest, Sha256};

use crate::envelope::{AssetRef, ContentBlock};

use super::builder::{
    IssuedQuestionPresentation, ItemBasisV1, PresentationBuildError, ResponseItemRole,
};
use super::model::{
    IssuedQuestionResponseFormatV1, PresentationResponseItemReference, PresentedChoiceV1,
    PresentedQuestionAsset, QuestionPresentationToken,
};

/// Closed descriptor version stored with every v1 attempt.
pub const DESCRIPTOR_VERSION_V1: u8 = 1;
const PRESENTATION_DOMAIN: &[u8] = b"ple:presentation:v1\0";
const HOTSPOT_COORDINATE_MAXIMUM: u32 = 10_000;

/// Full SHA-256 binding retained by server persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuestionPresentationDigest([u8; 32]);

impl QuestionPresentationDigest {
    pub(crate) fn zero() -> Self {
        Self([0; 32])
    }

    /// Computes the full descriptor binding.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Restores a validated persisted digest.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the full persistence bytes.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Returns the exact lowercase persistence spelling.
    pub fn to_hex(self) -> String {
        let mut value = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(&mut value, "{byte:02x}");
        }
        value
    }

    /// Parses the exact lowercase persistence spelling.
    pub fn parse_hex(value: &str) -> Result<Self, &'static str> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("presentation digest must be 64 lowercase hexadecimal characters");
        }
        let mut bytes = [0_u8; 32];
        for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let text = std::str::from_utf8(chunk).map_err(|_| "invalid presentation digest")?;
            bytes[index] =
                u8::from_str_radix(text, 16).map_err(|_| "invalid presentation digest")?;
        }
        Ok(Self(bytes))
    }

    /// Produces the browser-facing 128-bit token.
    pub fn public_token(self) -> QuestionPresentationToken {
        QuestionPresentationToken::from_digest(&self.0)
    }
}

/// Encodes the complete answer-free presentation deterministically.
pub fn descriptor_bytes_v1(
    presentation: &IssuedQuestionPresentation,
) -> Result<Vec<u8>, PresentationBuildError> {
    let mut encoder = Encoder::new();
    encoder.raw(PRESENTATION_DOMAIN);
    encoder.u8(DESCRIPTOR_VERSION_V1);
    encoder.string(
        &presentation
            .envelope
            .question_version
            .question_id
            .to_string(),
    )?;
    encoder.u32(presentation.envelope.question_version.version_number.get());
    encoder.u64(presentation.envelope.seed.value());
    encoder.raw(&presentation.envelope.presentation_nonce.as_bytes());
    encoder.string(&presentation.envelope.title)?;
    encoder.content_blocks(&presentation.envelope.prompt)?;
    encoder.question_response_format(&presentation.envelope.response, presentation)?;
    encoder.u32_len(presentation.item_bindings.len())?;
    for item in &presentation.item_bindings {
        encoder.u16(item.rendered.as_u16());
        let basis = item_basis_bytes(&item.basis)?;
        encoder.bytes(&basis)?;
    }
    encoder.u32_len(presentation.asset_bindings.len())?;
    for binding in &presentation.asset_bindings {
        encoder.asset_binding(binding)?;
    }
    Ok(encoder.finish())
}

/// Verifies both the full persisted digest and the public prefix.
pub fn verify_presentation_v1(
    presentation: &IssuedQuestionPresentation,
    expected: QuestionPresentationDigest,
    public: &QuestionPresentationToken,
) -> Result<(), PresentationBuildError> {
    let actual = QuestionPresentationDigest::compute(&descriptor_bytes_v1(presentation)?);
    if actual == expected && actual.public_token() == *public {
        Ok(())
    } else {
        Err(PresentationBuildError::InvalidPublicContent(
            "presentation digest does not match the descriptor",
        ))
    }
}

pub(super) fn item_basis_bytes(basis: &ItemBasisV1) -> Result<Vec<u8>, PresentationBuildError> {
    let mut encoder = Encoder::new();
    encoder.u8(basis.role.tag());
    encoder.u32(basis.ordinal);
    encoder.optional_string(basis.label.as_deref())?;
    encoder.content_blocks(&basis.content)?;
    encoder.u32_len(basis.assets.len())?;
    for asset in &basis.assets {
        encoder.asset_binding(asset)?;
    }
    encoder.optional_u32(basis.hotspot_width);
    encoder.optional_u32(basis.hotspot_height);
    if basis.role == ResponseItemRole::HotspotSurface {
        encoder.u32(HOTSPOT_COORDINATE_MAXIMUM);
        encoder.u32_len(basis.hotspot_regions.len())?;
        for region in &basis.hotspot_regions {
            encoder.content_blocks(&region.label)?;
            encoder.u32(region.x.into());
            encoder.u32(region.y.into());
            encoder.u32(region.width.into());
            encoder.u32(region.height.into());
        }
    } else if !basis.hotspot_regions.is_empty() {
        return Err(PresentationBuildError::DescriptorEncoding(
            "non-hotspot item contains hotspot geometry",
        ));
    }
    Ok(encoder.finish())
}

/// CRC-16/CCITT-FALSE (`poly=0x1021`, `init=0xffff`, `xorout=0`).
pub(super) fn crc16_ccitt_false(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff_u16;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.raw(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.raw(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.raw(&value.to_be_bytes());
    }

    fn u32_len(&mut self, value: usize) -> Result<(), PresentationBuildError> {
        self.u32(u32::try_from(value).map_err(|_| {
            PresentationBuildError::DescriptorEncoding("presentation vector is too large")
        })?);
        Ok(())
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), PresentationBuildError> {
        self.u32_len(value.len())?;
        self.raw(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), PresentationBuildError> {
        self.bytes(value.as_bytes())
    }

    fn optional_string(&mut self, value: Option<&str>) -> Result<(), PresentationBuildError> {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.string(value)?;
            }
        }
        Ok(())
    }

    fn optional_u32(&mut self, value: Option<u32>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.u32(value);
            }
        }
    }

    fn checksum(&mut self, value: &str) -> Result<(), PresentationBuildError> {
        if value.len() != 64 {
            return Err(PresentationBuildError::DescriptorEncoding(
                "asset checksum is not SHA-256",
            ));
        }
        let mut bytes = [0_u8; 32];
        for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let text = std::str::from_utf8(chunk).map_err(|_| {
                PresentationBuildError::DescriptorEncoding("asset checksum is not hexadecimal")
            })?;
            bytes[index] = u8::from_str_radix(text, 16).map_err(|_| {
                PresentationBuildError::DescriptorEncoding("asset checksum is not hexadecimal")
            })?;
        }
        self.raw(&bytes);
        Ok(())
    }

    fn asset_ref(&mut self, asset: &AssetRef) -> Result<(), PresentationBuildError> {
        self.raw(asset.asset.as_uuid().as_bytes());
        self.checksum(&asset.checksum)
    }

    fn asset_binding(
        &mut self,
        asset: &PresentedQuestionAsset,
    ) -> Result<(), PresentationBuildError> {
        self.raw(asset.asset.as_uuid().as_bytes());
        self.checksum(&asset.authored_checksum)?;
        self.checksum(&asset.rendition_checksum)?;
        self.optional_u32(asset.intrinsic_width);
        self.optional_u32(asset.intrinsic_height);
        Ok(())
    }

    fn content_blocks(&mut self, blocks: &[ContentBlock]) -> Result<(), PresentationBuildError> {
        self.u32_len(blocks.len())?;
        for block in blocks {
            match block {
                ContentBlock::Text { markdown } => {
                    self.u8(0);
                    self.string(markdown)?;
                }
                ContentBlock::Math { latex, description } => {
                    self.u8(1);
                    self.string(latex)?;
                    self.string(description)?;
                }
                ContentBlock::Image { asset, description } => {
                    self.u8(2);
                    self.asset_ref(asset)?;
                    self.string(description)?;
                }
                ContentBlock::Code { language, source } => {
                    self.u8(3);
                    self.string(language)?;
                    self.string(source)?;
                }
                ContentBlock::Table {
                    headers,
                    rows,
                    description,
                } => {
                    self.u8(4);
                    self.string_vector(headers)?;
                    self.u32_len(rows.len())?;
                    for row in rows {
                        self.string_vector(row)?;
                    }
                    self.string(description)?;
                }
            }
        }
        Ok(())
    }

    fn string_vector(&mut self, values: &[String]) -> Result<(), PresentationBuildError> {
        self.u32_len(values.len())?;
        for value in values {
            self.string(value)?;
        }
        Ok(())
    }

    fn question_response_format(
        &mut self,
        response: &IssuedQuestionResponseFormatV1,
        presentation: &IssuedQuestionPresentation,
    ) -> Result<(), PresentationBuildError> {
        match response {
            IssuedQuestionResponseFormatV1::SingleChoice { choices } => {
                self.u8(0);
                self.choice_ordinals(choices, presentation, ResponseItemRole::QuestionChoice)?;
            }
            IssuedQuestionResponseFormatV1::MultipleAnswer {
                choices,
                minimum,
                maximum,
            } => {
                self.u8(1);
                self.u32(*minimum);
                self.u32(*maximum);
                self.choice_ordinals(choices, presentation, ResponseItemRole::QuestionChoice)?;
            }
            IssuedQuestionResponseFormatV1::FillIn { max_characters } => {
                self.u8(2);
                self.u32(*max_characters);
            }
            IssuedQuestionResponseFormatV1::MultiFillIn { blanks } => {
                self.u8(3);
                self.u32_len(blanks.len())?;
                for blank in blanks {
                    self.u32(ordinal_for(
                        &blank.id,
                        presentation,
                        ResponseItemRole::TextEntrySlot,
                    )?);
                    self.u32(blank.max_characters);
                }
            }
            IssuedQuestionResponseFormatV1::Numerical {
                max_characters,
                displayed_unit,
            } => {
                self.u8(4);
                self.u32(*max_characters);
                self.optional_string(displayed_unit.as_deref())?;
            }
            IssuedQuestionResponseFormatV1::Matching {
                prompts,
                choices,
                reuse_choices,
            } => {
                self.u8(5);
                self.choice_ordinals(prompts, presentation, ResponseItemRole::MatchingPrompt)?;
                self.choice_ordinals(choices, presentation, ResponseItemRole::MatchingChoice)?;
                self.u8(u8::from(*reuse_choices));
            }
            IssuedQuestionResponseFormatV1::Ordering { items } => {
                self.u8(6);
                self.choice_ordinals(items, presentation, ResponseItemRole::OrderingItem)?;
            }
            IssuedQuestionResponseFormatV1::Hotspot {
                surface,
                minimum,
                maximum,
            } => {
                self.u8(7);
                self.u32(ordinal_for(
                    &surface.id,
                    presentation,
                    ResponseItemRole::HotspotSurface,
                )?);
                self.u32(*minimum);
                self.u32(*maximum);
            }
        }
        Ok(())
    }

    fn choice_ordinals(
        &mut self,
        choices: &[PresentedChoiceV1],
        presentation: &IssuedQuestionPresentation,
        role: ResponseItemRole,
    ) -> Result<(), PresentationBuildError> {
        self.u32_len(choices.len())?;
        for choice in choices {
            self.u32(ordinal_for(&choice.id, presentation, role)?);
        }
        Ok(())
    }
}

fn ordinal_for(
    id: &PresentationResponseItemReference,
    presentation: &IssuedQuestionPresentation,
    role: ResponseItemRole,
) -> Result<u32, PresentationBuildError> {
    presentation
        .item_bindings
        .iter()
        .find(|item| item.rendered == *id && item.role == role)
        .map(|item| item.ordinal)
        .ok_or(PresentationBuildError::DescriptorEncoding(
            "Question Response Format refers to an unknown rendered item",
        ))
}
