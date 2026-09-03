//! Normative binary descriptor and Presentation Response Item Reference codecs.

use sha2::{Digest, Sha256};

use crate::question_content::{QuestionAssetReference, QuestionContentBlock};

use super::builder::{
    IssuedQuestionPresentation, PresentationBuildError, ResponseItemBasis, ResponseItemRole,
};
use super::model::{
    PresentationResponseItemReference, PresentedResponseItemContent, QuestionAssetRendition,
    QuestionPresentationResponseFormat, QuestionPresentationToken,
};

/// Closed descriptor version stored with every v1 attempt.
pub const CURRENT_DESCRIPTOR_VERSION: u8 = 1;
const PRESENTATION_DOMAIN: &[u8] = b"ple:presentation:v1\0";
const HOTSPOT_COORDINATE_MAXIMUM: u32 = 10_000;

/// Full SHA-256 binding retained by server persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuestionPresentationChecksum([u8; 32]);

impl QuestionPresentationChecksum {
    pub(crate) fn zero() -> Self {
        Self([0; 32])
    }

    /// Computes the full descriptor binding.
    pub fn compute(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Restores a validated persisted checksum.
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
            return Err("presentation checksum must be 64 lowercase hexadecimal characters");
        }
        let mut bytes = [0_u8; 32];
        for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let text = std::str::from_utf8(chunk).map_err(|_| "invalid presentation checksum")?;
            bytes[index] =
                u8::from_str_radix(text, 16).map_err(|_| "invalid presentation checksum")?;
        }
        Ok(Self(bytes))
    }

    /// Produces the browser-facing 128-bit token.
    pub fn public_token(self) -> QuestionPresentationToken {
        QuestionPresentationToken::from_checksum(&self.0)
    }
}

/// Encodes the complete answer-free presentation deterministically.
pub fn descriptor_bytes(
    presentation: &IssuedQuestionPresentation,
) -> Result<Vec<u8>, PresentationBuildError> {
    let mut encoder = Encoder::new();
    encoder.raw(PRESENTATION_DOMAIN);
    encoder.u8(CURRENT_DESCRIPTOR_VERSION);
    encoder.string(
        &presentation
            .presentation
            .question_revision
            .question_id
            .to_string(),
    )?;
    encoder.u32(
        presentation
            .presentation
            .question_revision
            .revision_number
            .get(),
    );
    encoder.u64(presentation.presentation.question_seed.value());
    encoder.raw(&presentation.presentation.presentation_nonce.as_bytes());
    encoder.string(&presentation.presentation.title)?;
    encoder.content_blocks(&presentation.presentation.prompt)?;
    encoder.question_response_format(&presentation.presentation.response, presentation)?;
    encoder.u32_len(presentation.item_bindings.len())?;
    for item in &presentation.item_bindings {
        encoder.u16(item.presentation_response_item_reference.as_u16());
        let basis = item_basis_bytes(&item.basis)?;
        encoder.bytes(&basis)?;
    }
    encoder.u32_len(presentation.question_asset_renditions.len())?;
    for binding in &presentation.question_asset_renditions {
        encoder.question_asset_rendition(binding)?;
    }
    Ok(encoder.finish())
}

/// Verifies both the full persisted checksum and the public prefix.
pub fn verify_question_presentation(
    presentation: &IssuedQuestionPresentation,
    expected: QuestionPresentationChecksum,
    public: &QuestionPresentationToken,
) -> Result<(), PresentationBuildError> {
    let actual = QuestionPresentationChecksum::compute(&descriptor_bytes(presentation)?);
    if actual == expected && actual.public_token() == *public {
        Ok(())
    } else {
        Err(PresentationBuildError::InvalidPublicContent(
            "presentation checksum does not match the descriptor",
        ))
    }
}

pub(super) fn item_basis_bytes(
    basis: &ResponseItemBasis,
) -> Result<Vec<u8>, PresentationBuildError> {
    let mut encoder = Encoder::new();
    encoder.u8(basis.role.tag());
    encoder.u32(basis.ordinal);
    encoder.optional_string(basis.label.as_deref())?;
    encoder.content_blocks(&basis.content)?;
    encoder.u32_len(basis.assets.len())?;
    for asset in &basis.assets {
        encoder.question_asset_rendition(asset)?;
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

    fn asset_ref(&mut self, asset: &QuestionAssetReference) -> Result<(), PresentationBuildError> {
        self.raw(asset.asset.as_uuid().as_bytes());
        self.checksum(&asset.checksum)
    }

    fn question_asset_rendition(
        &mut self,
        asset: &QuestionAssetRendition,
    ) -> Result<(), PresentationBuildError> {
        self.asset_ref(&asset.question_asset)?;
        self.checksum(&asset.rendition_checksum)?;
        self.optional_u32(asset.intrinsic_width);
        self.optional_u32(asset.intrinsic_height);
        Ok(())
    }

    fn content_blocks(
        &mut self,
        blocks: &[QuestionContentBlock],
    ) -> Result<(), PresentationBuildError> {
        self.u32_len(blocks.len())?;
        for block in blocks {
            match block {
                QuestionContentBlock::Text { markdown } => {
                    self.u8(0);
                    self.string(markdown)?;
                }
                QuestionContentBlock::Math { latex, description } => {
                    self.u8(1);
                    self.string(latex)?;
                    self.string(description)?;
                }
                QuestionContentBlock::Image { asset, description } => {
                    self.u8(2);
                    self.asset_ref(asset)?;
                    self.string(description)?;
                }
                QuestionContentBlock::Code { language, source } => {
                    self.u8(3);
                    self.string(language)?;
                    self.string(source)?;
                }
                QuestionContentBlock::Table {
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
        response: &QuestionPresentationResponseFormat,
        presentation: &IssuedQuestionPresentation,
    ) -> Result<(), PresentationBuildError> {
        match response {
            QuestionPresentationResponseFormat::SingleChoice { choices } => {
                self.u8(0);
                self.response_item_ordinals(
                    choices,
                    presentation,
                    ResponseItemRole::QuestionChoice,
                )?;
            }
            QuestionPresentationResponseFormat::MultipleAnswer {
                choices,
                minimum,
                maximum,
            } => {
                self.u8(1);
                self.u32(*minimum);
                self.u32(*maximum);
                self.response_item_ordinals(
                    choices,
                    presentation,
                    ResponseItemRole::QuestionChoice,
                )?;
            }
            QuestionPresentationResponseFormat::FillIn { max_characters } => {
                self.u8(2);
                self.u32(*max_characters);
            }
            QuestionPresentationResponseFormat::MultiFillIn { blanks } => {
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
            QuestionPresentationResponseFormat::Numerical {
                max_characters,
                displayed_unit,
            } => {
                self.u8(4);
                self.u32(*max_characters);
                self.optional_string(displayed_unit.as_deref())?;
            }
            QuestionPresentationResponseFormat::Matching {
                prompts,
                choices,
                reuse_choices,
            } => {
                self.u8(5);
                self.response_item_ordinals(
                    prompts,
                    presentation,
                    ResponseItemRole::MatchingPrompt,
                )?;
                self.response_item_ordinals(
                    choices,
                    presentation,
                    ResponseItemRole::MatchingChoice,
                )?;
                self.u8(u8::from(*reuse_choices));
            }
            QuestionPresentationResponseFormat::Ordering { items } => {
                self.u8(6);
                self.response_item_ordinals(items, presentation, ResponseItemRole::OrderingItem)?;
            }
            QuestionPresentationResponseFormat::Hotspot {
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
            QuestionPresentationResponseFormat::ImathasQuestionBackend {} => {
                self.u8(8);
            }
        }
        Ok(())
    }

    fn response_item_ordinals<T: PresentedResponseItemContent>(
        &mut self,
        items: &[T],
        presentation: &IssuedQuestionPresentation,
        role: ResponseItemRole,
    ) -> Result<(), PresentationBuildError> {
        self.u32_len(items.len())?;
        for item in items {
            self.u32(ordinal_for(
                item.presentation_item_id(),
                presentation,
                role,
            )?);
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
        .find(|item| item.presentation_response_item_reference == *id && item.role == role)
        .map(|item| item.ordinal)
        .ok_or(PresentationBuildError::DescriptorEncoding(
            "Question Response Format refers to an unknown Presentation Response Item Reference",
        ))
}
