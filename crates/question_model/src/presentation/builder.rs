//! Construction of globally collision-free presentation-scoped item IDs.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::answer::ResponseSelectionRule;
use crate::envelope::{ContentBlock, QuestionPresentation};
use crate::response::{
    MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice, QuestionResponseFormat,
    ResponseItemReference,
};

use super::assets::{asset_binding, content_assets, validate_assets, validate_public_assets};
use super::binding::QuestionPresentationBinding;
use super::codec::{
    QuestionPresentationDigest, crc16_ccitt_false, descriptor_bytes_v1, item_basis_bytes,
};
use super::model::{
    IssuedQuestionResponseFormatV1, PresentationEnvelopeV1, PresentationResponseItemReference,
    PresentedBlankV1, PresentedChoiceV1, PresentedHotspotRegionV1, PresentedHotspotSurfaceV1,
    PresentedQuestionAsset, QuestionPresentationNonce,
};

const MAX_PRESENTED_ITEMS: usize = 32;
const MAX_NONCE_ATTEMPTS: usize = 8;
const NUMERIC_MAX_CHARACTERS: u32 = 128;

/// Item role included in the CRC domain and descriptor codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseItemRole {
    QuestionChoice,
    TextEntrySlot,
    MatchingPrompt,
    MatchingChoice,
    OrderingItem,
    HotspotSurface,
    HotspotRegion,
}

impl ResponseItemRole {
    pub(super) fn tag(self) -> u8 {
        match self {
            Self::QuestionChoice => 0,
            Self::TextEntrySlot => 1,
            Self::MatchingPrompt => 2,
            Self::MatchingChoice => 3,
            Self::OrderingItem => 4,
            Self::HotspotSurface => 5,
            Self::HotspotRegion => 6,
        }
    }
}

/// Server-only mapping from one rendered object to its durable identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseItemBinding {
    pub rendered: PresentationResponseItemReference,
    pub role: ResponseItemRole,
    pub ordinal: u32,
    pub durable_id: String,
    pub(super) basis: ItemBasisV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItemBasisV1 {
    pub role: ResponseItemRole,
    pub ordinal: u32,
    pub label: Option<String>,
    pub content: Vec<ContentBlock>,
    pub assets: Vec<PresentedQuestionAsset>,
    pub hotspot_width: Option<u32>,
    pub hotspot_height: Option<u32>,
    pub hotspot_regions: Vec<HotspotRegionGeometryV1>,
}

/// Geometry and label bound to a hotspot surface before a presentation-scoped
/// region identifier is created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HotspotRegionGeometryV1 {
    pub label: Vec<ContentBlock>,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingItemV1 {
    role: ResponseItemRole,
    ordinal: u32,
    durable_id: String,
    basis: ItemBasisV1,
}

/// Fully constructed public presentation plus its server-only bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedQuestionPresentation {
    pub envelope: PresentationEnvelopeV1,
    pub asset_bindings: Vec<PresentedQuestionAsset>,
    pub item_bindings: Vec<ResponseItemBinding>,
    pub digest: QuestionPresentationDigest,
}

/// Bounded construction failures. None contains answer material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationBuildError {
    RandomnessUnavailable,
    UnsupportedResponse,
    InvalidPublicContent(&'static str),
    TooManyItems,
    RenderedIdCollision,
    DescriptorEncoding(&'static str),
}

impl std::fmt::Display for PresentationBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RandomnessUnavailable => {
                formatter.write_str("presentation nonce randomness is unavailable")
            }
            Self::UnsupportedResponse => {
                formatter.write_str("Question Response Control is outside presentation contract v1")
            }
            Self::InvalidPublicContent(message) => formatter.write_str(message),
            Self::TooManyItems => formatter.write_str("presentation contains more than 32 items"),
            Self::RenderedIdCollision => {
                formatter.write_str("could not mint globally unique rendered item IDs")
            }
            Self::DescriptorEncoding(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PresentationBuildError {}

/// Source of exact 16-byte nonces. Tests inject deterministic values.
pub trait NonceSourceV1 {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError>;
}

/// Operating-system nonce source used by server issuance.
#[derive(Debug, Default)]
pub struct OsNonceSourceV1;

impl NonceSourceV1 for OsNonceSourceV1 {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        #[cfg(feature = "generate")]
        {
            let mut bytes = [0_u8; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| PresentationBuildError::RandomnessUnavailable)?;
            Ok(bytes)
        }
        #[cfg(not(feature = "generate"))]
        {
            Err(PresentationBuildError::RandomnessUnavailable)
        }
    }
}

/// Builds one presentation using operating-system randomness.
pub fn build_presentation_v1(
    envelope: &QuestionPresentation,
    asset_bindings: &[PresentedQuestionAsset],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    build_presentation_v1_with_nonce_source(envelope, asset_bindings, &mut OsNonceSourceV1)
}

struct PersistedNonceSource(Option<[u8; 16]>);

impl NonceSourceV1 for PersistedNonceSource {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        self.0
            .take()
            .ok_or(PresentationBuildError::RenderedIdCollision)
    }
}

/// Rebuilds one server-issued presentation from its durable nonce and digest.
///
/// This is the canonical server-side reproduction path. It recomputes every
/// rendered-item ID and the full descriptor rather than trusting stored or
/// browser-supplied public fields.
pub fn reproduce_presentation_v1(
    envelope: &QuestionPresentation,
    asset_bindings: &[PresentedQuestionAsset],
    binding: QuestionPresentationBinding,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let mut nonce = PersistedNonceSource(Some(binding.nonce().as_bytes()));
    let presentation =
        build_presentation_v1_with_nonce_source(envelope, asset_bindings, &mut nonce)?;
    if presentation.digest != binding.digest() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation digest does not reproduce",
        ));
    }
    Ok(presentation)
}

/// Builds one presentation using an injected nonce source.
pub fn build_presentation_v1_with_nonce_source<N: NonceSourceV1>(
    envelope: &QuestionPresentation,
    asset_bindings: &[PresentedQuestionAsset],
    nonce_source: &mut N,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    build_with_hasher(envelope, asset_bindings, nonce_source, |bytes| {
        crc16_ccitt_false(bytes)
    })
}

/// Reconstructs the exact descriptor inputs from a browser-safe envelope.
///
/// This performs no grading and does not require durable IDs. It is the only
/// operation the Wasm bridge needs in order to verify that the browser holds
/// one coherent server-issued presentation.
pub fn rebuild_public_presentation_v1(
    envelope: &PresentationEnvelopeV1,
    asset_bindings: &[PresentedQuestionAsset],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let assets = validate_public_assets(envelope, asset_bindings)?;
    let item_bindings = public_item_bindings(&envelope.response, &assets)?;
    if item_bindings.len() > MAX_PRESENTED_ITEMS {
        return Err(PresentationBuildError::TooManyItems);
    }
    let unique: BTreeSet<_> = item_bindings
        .iter()
        .map(|item| item.rendered.clone())
        .collect();
    if unique.len() != item_bindings.len() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation repeats a rendered item ID",
        ));
    }
    let mut presentation = IssuedQuestionPresentation {
        envelope: envelope.clone(),
        asset_bindings: assets,
        item_bindings,
        digest: QuestionPresentationDigest::zero(),
    };
    presentation.digest = QuestionPresentationDigest::compute(&descriptor_bytes_v1(&presentation)?);
    Ok(presentation)
}

#[cfg(test)]
pub(super) fn build_presentation_v1_with_hasher<N, H>(
    envelope: &QuestionPresentation,
    asset_bindings: &[PresentedQuestionAsset],
    nonce_source: &mut N,
    hasher: H,
) -> Result<IssuedQuestionPresentation, PresentationBuildError>
where
    N: NonceSourceV1,
    H: FnMut(&[u8]) -> u16,
{
    build_with_hasher(envelope, asset_bindings, nonce_source, hasher)
}

fn build_with_hasher<N, H>(
    envelope: &QuestionPresentation,
    asset_bindings: &[PresentedQuestionAsset],
    nonce_source: &mut N,
    mut hasher: H,
) -> Result<IssuedQuestionPresentation, PresentationBuildError>
where
    N: NonceSourceV1,
    H: FnMut(&[u8]) -> u16,
{
    let assets = validate_assets(envelope, asset_bindings)?;
    let pending = pending_items(envelope, &assets)?;
    if pending.len() > MAX_PRESENTED_ITEMS {
        return Err(PresentationBuildError::TooManyItems);
    }

    for _ in 0..MAX_NONCE_ATTEMPTS {
        let nonce = QuestionPresentationNonce::from_bytes(nonce_source.next_nonce()?);
        let mut used = BTreeSet::new();
        let mut bindings = Vec::with_capacity(pending.len());
        let mut collision = false;
        for item in &pending {
            let basis_bytes = item_basis_bytes(&item.basis)?;
            let input = rendered_id_input(envelope, nonce, item, &basis_bytes)?;
            let rendered = PresentationResponseItemReference::from_crc(hasher(&input));
            if !used.insert(rendered.clone()) {
                collision = true;
                break;
            }
            bindings.push(ResponseItemBinding {
                rendered,
                role: item.role,
                ordinal: item.ordinal,
                durable_id: item.durable_id.clone(),
                basis: item.basis.clone(),
            });
        }
        if collision {
            continue;
        }
        let public = public_envelope(envelope, nonce, &bindings)?;
        let mut presentation = IssuedQuestionPresentation {
            envelope: public,
            asset_bindings: assets.clone(),
            item_bindings: bindings,
            digest: QuestionPresentationDigest::zero(),
        };
        let bytes = descriptor_bytes_v1(&presentation)?;
        presentation.digest = QuestionPresentationDigest::compute(&bytes);
        return Ok(presentation);
    }
    Err(PresentationBuildError::RenderedIdCollision)
}

fn rendered_id_input(
    envelope: &QuestionPresentation,
    nonce: QuestionPresentationNonce,
    item: &PendingItemV1,
    basis_bytes: &[u8],
) -> Result<Vec<u8>, PresentationBuildError> {
    let mut bytes = b"ple:rendered-item:v1\0".to_vec();
    bytes.extend_from_slice(&nonce.as_bytes());
    push_bytes(
        &mut bytes,
        envelope
            .variation
            .question_version
            .question_id
            .to_string()
            .as_bytes(),
    )?;
    bytes.extend_from_slice(
        &envelope
            .variation
            .question_version
            .version_number
            .get()
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&envelope.variation.seed.value().to_be_bytes());
    bytes.push(item.role.tag());
    bytes.extend_from_slice(&item.ordinal.to_be_bytes());
    push_bytes(&mut bytes, item.durable_id.as_bytes())?;
    bytes.extend_from_slice(&Sha256::digest(basis_bytes));
    Ok(bytes)
}

fn push_bytes(target: &mut Vec<u8>, value: &[u8]) -> Result<(), PresentationBuildError> {
    let length = u32::try_from(value.len()).map_err(|_| {
        PresentationBuildError::DescriptorEncoding("presentation field is too large")
    })?;
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn pending_items(
    envelope: &QuestionPresentation,
    assets: &[PresentedQuestionAsset],
) -> Result<Vec<PendingItemV1>, PresentationBuildError> {
    let mut items = Vec::new();
    match &envelope.response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            push_choices(
                &mut items,
                choices,
                ResponseItemRole::QuestionChoice,
                assets,
            )?;
        }
        QuestionResponseFormat::ShortText { .. } | QuestionResponseFormat::Numeric { .. } => {}
        QuestionResponseFormat::MultiBlank { blanks } => {
            for blank in blanks {
                push_item(
                    &mut items,
                    ResponseItemRole::TextEntrySlot,
                    blank.id.as_str(),
                    blank.label.clone(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        QuestionResponseFormat::Matching { prompts, choices } => {
            push_choices(
                &mut items,
                prompts,
                ResponseItemRole::MatchingPrompt,
                assets,
            )?;
            push_choices(
                &mut items,
                choices,
                ResponseItemRole::MatchingChoice,
                assets,
            )?;
        }
        QuestionResponseFormat::Ordering { items: choices } => {
            push_choices(&mut items, choices, ResponseItemRole::OrderingItem, assets)?;
        }
        QuestionResponseFormat::Hotspot {
            surface,
            description,
            regions,
            ..
        } => {
            let binding = asset_binding(surface, assets)?;
            let hotspot_regions = regions.iter().map(hotspot_region_geometry).collect();
            push_item(
                &mut items,
                ResponseItemRole::HotspotSurface,
                &surface.asset.to_string(),
                vec![ContentBlock::Image {
                    asset: surface.clone(),
                    description: description.clone(),
                }],
                assets,
                Some((
                    binding
                        .intrinsic_width
                        .ok_or(PresentationBuildError::InvalidPublicContent(
                            "hotspot surface lacks intrinsic dimensions",
                        ))?,
                    binding.intrinsic_height.ok_or(
                        PresentationBuildError::InvalidPublicContent(
                            "hotspot surface lacks intrinsic dimensions",
                        ),
                    )?,
                )),
                hotspot_regions,
            )?;
            for region in regions {
                push_item(
                    &mut items,
                    ResponseItemRole::HotspotRegion,
                    region.id.as_str(),
                    Vec::new(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        QuestionResponseFormat::ExternalTool {} => {
            return Err(PresentationBuildError::UnsupportedResponse);
        }
    }
    Ok(items)
}

trait PresentedResponseItem {
    fn id(&self) -> &ResponseItemReference;
    fn body(&self) -> &[ContentBlock];
}

impl PresentedResponseItem for QuestionChoice {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[ContentBlock] {
        &self.body
    }
}

impl PresentedResponseItem for MatchingPrompt {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[ContentBlock] {
        &self.body
    }
}

impl PresentedResponseItem for MatchingChoice {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[ContentBlock] {
        &self.body
    }
}

impl PresentedResponseItem for OrderingItem {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[ContentBlock] {
        &self.body
    }
}

fn push_choices<T: PresentedResponseItem>(
    target: &mut Vec<PendingItemV1>,
    choices: &[T],
    role: ResponseItemRole,
    assets: &[PresentedQuestionAsset],
) -> Result<(), PresentationBuildError> {
    for choice in choices {
        push_item(
            target,
            role,
            choice.id().as_str(),
            choice.body().to_vec(),
            assets,
            None,
            Vec::new(),
        )?;
    }
    Ok(())
}

fn push_item(
    target: &mut Vec<PendingItemV1>,
    role: ResponseItemRole,
    durable_id: &str,
    content: Vec<ContentBlock>,
    assets: &[PresentedQuestionAsset],
    hotspot_dimensions: Option<(u32, u32)>,
    hotspot_regions: Vec<HotspotRegionGeometryV1>,
) -> Result<(), PresentationBuildError> {
    if durable_id.is_empty() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation item has an empty durable identity",
        ));
    }
    let ordinal = u32::try_from(target.len()).map_err(|_| PresentationBuildError::TooManyItems)?;
    let item_assets = content_assets(&content, assets)?;
    target.push(PendingItemV1 {
        role,
        ordinal,
        durable_id: durable_id.to_owned(),
        basis: ItemBasisV1 {
            role,
            ordinal,
            label: None,
            content,
            assets: item_assets,
            hotspot_width: hotspot_dimensions.map(|value| value.0),
            hotspot_height: hotspot_dimensions.map(|value| value.1),
            hotspot_regions,
        },
    });
    Ok(())
}

fn public_envelope(
    source: &QuestionPresentation,
    nonce: QuestionPresentationNonce,
    bindings: &[ResponseItemBinding],
) -> Result<PresentationEnvelopeV1, PresentationBuildError> {
    let by_role = |role| bindings.iter().filter(move |binding| binding.role == role);
    let choices = |role: ResponseItemRole| {
        by_role(role)
            .map(|binding| PresentedChoiceV1 {
                id: binding.rendered.clone(),
                body: binding.basis.content.clone(),
            })
            .collect::<Vec<_>>()
    };
    let response = match &source.response {
        QuestionResponseFormat::MultipleChoice {
            choices: source_choices,
            selection,
        } => match selection {
            ResponseSelectionRule::ExactlyOne => IssuedQuestionResponseFormatV1::SingleChoice {
                choices: choices(ResponseItemRole::QuestionChoice),
            },
            _ => {
                let (minimum, maximum) = selection_bounds(*selection, source_choices.len())?;
                IssuedQuestionResponseFormatV1::MultipleAnswer {
                    choices: choices(ResponseItemRole::QuestionChoice),
                    minimum,
                    maximum,
                }
            }
        },
        QuestionResponseFormat::ShortText { max_length, .. } => {
            IssuedQuestionResponseFormatV1::FillIn {
                max_characters: *max_length,
            }
        }
        QuestionResponseFormat::MultiBlank { blanks } => {
            let rendered: Vec<_> = by_role(ResponseItemRole::TextEntrySlot).collect();
            if rendered.len() != blanks.len() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "blank presentation mapping is incomplete",
                ));
            }
            IssuedQuestionResponseFormatV1::MultiFillIn {
                blanks: blanks
                    .iter()
                    .zip(rendered)
                    .map(|(blank, binding)| PresentedBlankV1 {
                        id: binding.rendered.clone(),
                        label: blank.label.clone(),
                        max_characters: blank.max_length,
                    })
                    .collect(),
            }
        }
        QuestionResponseFormat::Numeric { unit, .. } => IssuedQuestionResponseFormatV1::Numerical {
            max_characters: NUMERIC_MAX_CHARACTERS,
            displayed_unit: unit.clone(),
        },
        QuestionResponseFormat::Matching { .. } => IssuedQuestionResponseFormatV1::Matching {
            prompts: choices(ResponseItemRole::MatchingPrompt),
            choices: choices(ResponseItemRole::MatchingChoice),
            reuse_choices: false,
        },
        QuestionResponseFormat::Ordering { .. } => IssuedQuestionResponseFormatV1::Ordering {
            items: choices(ResponseItemRole::OrderingItem),
        },
        QuestionResponseFormat::Hotspot {
            regions, selection, ..
        } => {
            let surface = bindings
                .iter()
                .find(|binding| binding.role == ResponseItemRole::HotspotSurface)
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "hotspot surface mapping is absent",
                ))?;
            let ContentBlock::Image { asset, description } = &surface.basis.content[0] else {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "hotspot surface is not image-backed",
                ));
            };
            let (minimum, maximum) = selection_bounds(*selection, regions.len())?;
            let rendered_regions: Vec<_> = by_role(ResponseItemRole::HotspotRegion).collect();
            if rendered_regions.len() != regions.len() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "hotspot region presentation mapping is incomplete",
                ));
            }
            IssuedQuestionResponseFormatV1::Hotspot {
                surface: PresentedHotspotSurfaceV1 {
                    id: surface.rendered.clone(),
                    asset: asset.clone(),
                    description: description.clone(),
                    regions: regions
                        .iter()
                        .zip(rendered_regions)
                        .map(|(region, binding)| PresentedHotspotRegionV1 {
                            id: binding.rendered.clone(),
                            label: region.label.clone(),
                            x: region.x,
                            y: region.y,
                            width: region.width,
                            height: region.height,
                        })
                        .collect(),
                },
                minimum,
                maximum,
            }
        }
        QuestionResponseFormat::ExternalTool {} => {
            return Err(PresentationBuildError::UnsupportedResponse);
        }
    };
    Ok(PresentationEnvelopeV1 {
        question_version: source.variation.question_version.clone(),
        seed: source.variation.seed,
        presentation_nonce: nonce,
        title: source.title.clone(),
        prompt: source.prompt.clone(),
        response,
    })
}

fn public_item_bindings(
    response: &IssuedQuestionResponseFormatV1,
    assets: &[PresentedQuestionAsset],
) -> Result<Vec<ResponseItemBinding>, PresentationBuildError> {
    let mut target = Vec::new();
    match response {
        IssuedQuestionResponseFormatV1::SingleChoice { choices }
        | IssuedQuestionResponseFormatV1::MultipleAnswer { choices, .. } => {
            push_public_choices(
                &mut target,
                choices,
                ResponseItemRole::QuestionChoice,
                assets,
            )?;
        }
        IssuedQuestionResponseFormatV1::FillIn { max_characters } => {
            require_positive(*max_characters, "fill-in maximum must be positive")?;
        }
        IssuedQuestionResponseFormatV1::MultiFillIn { blanks } => {
            if blanks.is_empty() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "multi-fill presentation has no blanks",
                ));
            }
            for blank in blanks {
                require_positive(blank.max_characters, "blank maximum must be positive")?;
                push_public_item(
                    &mut target,
                    blank.id.clone(),
                    ResponseItemRole::TextEntrySlot,
                    blank.label.clone(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
        IssuedQuestionResponseFormatV1::Numerical { max_characters, .. } => {
            require_positive(*max_characters, "numeric maximum must be positive")?;
        }
        IssuedQuestionResponseFormatV1::Matching {
            prompts,
            choices,
            reuse_choices,
        } => {
            if prompts.is_empty()
                || choices.is_empty()
                || (!reuse_choices && prompts.len() > choices.len())
            {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "matching presentation has incompatible sides",
                ));
            }
            push_public_choices(
                &mut target,
                prompts,
                ResponseItemRole::MatchingPrompt,
                assets,
            )?;
            push_public_choices(
                &mut target,
                choices,
                ResponseItemRole::MatchingChoice,
                assets,
            )?;
        }
        IssuedQuestionResponseFormatV1::Ordering { items } => {
            if items.len() < 2 {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "ordering presentation requires at least two items",
                ));
            }
            push_public_choices(&mut target, items, ResponseItemRole::OrderingItem, assets)?;
        }
        IssuedQuestionResponseFormatV1::Hotspot {
            surface,
            minimum,
            maximum,
        } => {
            validate_public_bounds(*minimum, *maximum, u32::MAX)?;
            let binding = asset_binding(&surface.asset, assets)?;
            let dimensions = Some((
                binding
                    .intrinsic_width
                    .ok_or(PresentationBuildError::InvalidPublicContent(
                        "hotspot surface lacks intrinsic dimensions",
                    ))?,
                binding
                    .intrinsic_height
                    .ok_or(PresentationBuildError::InvalidPublicContent(
                        "hotspot surface lacks intrinsic dimensions",
                    ))?,
            ));
            validate_regions(&surface.regions)?;
            push_public_item(
                &mut target,
                surface.id.clone(),
                ResponseItemRole::HotspotSurface,
                vec![ContentBlock::Image {
                    asset: surface.asset.clone(),
                    description: surface.description.clone(),
                }],
                assets,
                dimensions,
                surface
                    .regions
                    .iter()
                    .map(hotspot_region_geometry)
                    .collect(),
            )?;
            for region in &surface.regions {
                push_public_item(
                    &mut target,
                    region.id.clone(),
                    ResponseItemRole::HotspotRegion,
                    Vec::new(),
                    assets,
                    None,
                    Vec::new(),
                )?;
            }
        }
    }
    match response {
        IssuedQuestionResponseFormatV1::SingleChoice { choices } if choices.len() < 2 => {
            return Err(PresentationBuildError::InvalidPublicContent(
                "single-choice presentation requires at least two choices",
            ));
        }
        IssuedQuestionResponseFormatV1::MultipleAnswer {
            choices,
            minimum,
            maximum,
        } => validate_public_bounds(
            *minimum,
            *maximum,
            u32::try_from(choices.len()).map_err(|_| PresentationBuildError::TooManyItems)?,
        )?,
        _ => {}
    }
    Ok(target)
}

fn push_public_choices(
    target: &mut Vec<ResponseItemBinding>,
    choices: &[PresentedChoiceV1],
    role: ResponseItemRole,
    assets: &[PresentedQuestionAsset],
) -> Result<(), PresentationBuildError> {
    for choice in choices {
        push_public_item(
            target,
            choice.id.clone(),
            role,
            choice.body.clone(),
            assets,
            None,
            Vec::new(),
        )?;
    }
    Ok(())
}

fn push_public_item(
    target: &mut Vec<ResponseItemBinding>,
    rendered: PresentationResponseItemReference,
    role: ResponseItemRole,
    content: Vec<ContentBlock>,
    assets: &[PresentedQuestionAsset],
    hotspot_dimensions: Option<(u32, u32)>,
    hotspot_regions: Vec<HotspotRegionGeometryV1>,
) -> Result<(), PresentationBuildError> {
    let ordinal = u32::try_from(target.len()).map_err(|_| PresentationBuildError::TooManyItems)?;
    let item_assets = content_assets(&content, assets)?;
    target.push(ResponseItemBinding {
        rendered,
        role,
        ordinal,
        durable_id: String::new(),
        basis: ItemBasisV1 {
            role,
            ordinal,
            label: None,
            content,
            assets: item_assets,
            hotspot_width: hotspot_dimensions.map(|value| value.0),
            hotspot_height: hotspot_dimensions.map(|value| value.1),
            hotspot_regions,
        },
    });
    Ok(())
}

fn require_positive(value: u32, message: &'static str) -> Result<(), PresentationBuildError> {
    if value == 0 {
        Err(PresentationBuildError::InvalidPublicContent(message))
    } else {
        Ok(())
    }
}

fn validate_public_bounds(
    minimum: u32,
    maximum: u32,
    available: u32,
) -> Result<(), PresentationBuildError> {
    if minimum > maximum || maximum > available {
        Err(PresentationBuildError::InvalidPublicContent(
            "presentation selection bounds are invalid",
        ))
    } else {
        Ok(())
    }
}

fn hotspot_region_geometry(region: &impl HotspotRegionGeometry) -> HotspotRegionGeometryV1 {
    HotspotRegionGeometryV1 {
        label: region.label().clone(),
        x: region.x(),
        y: region.y(),
        width: region.width(),
        height: region.height(),
    }
}

trait HotspotRegionGeometry {
    fn label(&self) -> &Vec<ContentBlock>;
    fn x(&self) -> u16;
    fn y(&self) -> u16;
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

impl HotspotRegionGeometry for crate::response::HotspotRegion {
    fn label(&self) -> &Vec<ContentBlock> {
        &self.label
    }
    fn x(&self) -> u16 {
        self.x
    }
    fn y(&self) -> u16 {
        self.y
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
}

impl HotspotRegionGeometry for PresentedHotspotRegionV1 {
    fn label(&self) -> &Vec<ContentBlock> {
        &self.label
    }
    fn x(&self) -> u16 {
        self.x
    }
    fn y(&self) -> u16 {
        self.y
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn height(&self) -> u16 {
        self.height
    }
}

fn validate_regions(regions: &[PresentedHotspotRegionV1]) -> Result<(), PresentationBuildError> {
    const MAX: u32 = 10_000;
    if regions.is_empty() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "hotspot presentation has no accessible regions",
        ));
    }
    for region in regions {
        let right = u32::from(region.x) + u32::from(region.width);
        let bottom = u32::from(region.y) + u32::from(region.height);
        if region.width == 0
            || region.height == 0
            || right > MAX
            || bottom > MAX
            || region.label.is_empty()
        {
            return Err(PresentationBuildError::InvalidPublicContent(
                "hotspot region is outside the normalized surface",
            ));
        }
    }
    Ok(())
}

fn selection_bounds(
    selection: ResponseSelectionRule,
    item_count: usize,
) -> Result<(u32, u32), PresentationBuildError> {
    let maximum = u32::try_from(item_count).map_err(|_| PresentationBuildError::TooManyItems)?;
    let bounds = match selection {
        ResponseSelectionRule::ExactlyOne => (1, 1),
        ResponseSelectionRule::Exactly { count } => (count, count),
        ResponseSelectionRule::AnyNumber => (0, maximum),
        ResponseSelectionRule::AtLeastOne => (1, maximum),
    };
    if bounds.0 > bounds.1 || bounds.1 > maximum {
        return Err(PresentationBuildError::InvalidPublicContent(
            "Response Selection Rule exceeds presented objects",
        ));
    }
    Ok(bounds)
}
