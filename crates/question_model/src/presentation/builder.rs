//! Construction of globally collision-free presentation-scoped item IDs.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::answer::ResponseSelectionRule;
use crate::envelope::{QuestionContentBlock, QuestionVariationPresentation};
use crate::response::{
    MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice, QuestionResponseFormat,
    ResponseItemReference,
};

use super::assets::{
    content_assets, question_asset_rendition, validate_assets, validate_public_assets,
};
use super::binding::QuestionPresentationBinding;
use super::codec::{
    QuestionPresentationChecksum, crc16_ccitt_false, descriptor_bytes, item_basis_bytes,
};
use super::model::{
    PresentationResponseItemReference, PresentedHotspotRegion, PresentedHotspotSurface,
    PresentedMatchingChoice, PresentedMatchingPrompt, PresentedOrderingItem,
    PresentedQuestionChoice, PresentedResponseItemContent, PresentedTextEntrySlot,
    QuestionAssetRendition, QuestionPresentation, QuestionPresentationNonce,
    QuestionPresentationResponseFormat,
};
const MAX_PRESENTED_ITEMS: usize = 32;
const MAX_NONCE_ATTEMPTS: usize = 8;
const NUMERIC_MAX_CHARACTERS: u32 = 128;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseItemBinding {
    pub rendered: PresentationResponseItemReference,
    pub role: ResponseItemRole,
    pub ordinal: u32,
    /// Exact authored Response Item for issued bindings; absent during public verification.
    pub response_item_reference: Option<ResponseItemReference>,
    pub(super) basis: ResponseItemBasis,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResponseItemBasis {
    pub role: ResponseItemRole,
    pub ordinal: u32,
    pub label: Option<String>,
    pub content: Vec<QuestionContentBlock>,
    pub assets: Vec<QuestionAssetRendition>,
    pub hotspot_width: Option<u32>,
    pub hotspot_height: Option<u32>,
    pub hotspot_regions: Vec<PendingHotspotRegionGeometry>,
}

/// Geometry and label bound to a hotspot before its presentation-scoped ID exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PendingHotspotRegionGeometry {
    pub label: Vec<QuestionContentBlock>,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingResponseItem {
    role: ResponseItemRole,
    ordinal: u32,
    response_item_reference: ResponseItemReference,
    basis: ResponseItemBasis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedQuestionPresentation {
    pub presentation: QuestionPresentation,
    pub question_asset_renditions: Vec<QuestionAssetRendition>,
    pub item_bindings: Vec<ResponseItemBinding>,
    pub checksum: QuestionPresentationChecksum,
}

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
pub trait QuestionPresentationNonceSource {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError>;
}

/// Operating-system nonce source used by server issuance.
#[derive(Debug, Default)]
pub struct OperatingSystemQuestionPresentationNonceSource;

impl QuestionPresentationNonceSource for OperatingSystemQuestionPresentationNonceSource {
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
pub fn build_question_presentation(
    envelope: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    build_question_presentation_with_nonce_source(
        envelope,
        question_asset_renditions,
        &mut OperatingSystemQuestionPresentationNonceSource,
    )
}

struct PersistedNonceSource(Option<[u8; 16]>);

impl QuestionPresentationNonceSource for PersistedNonceSource {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        self.0
            .take()
            .ok_or(PresentationBuildError::RenderedIdCollision)
    }
}

/// Rebuilds one server-issued presentation from its durable nonce and checksum.
///
/// This is the canonical server-side reproduction path. It recomputes every
/// rendered-item ID and the full descriptor rather than trusting stored or
/// browser-supplied public fields.
pub fn reproduce_question_presentation(
    envelope: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    binding: QuestionPresentationBinding,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let mut nonce = PersistedNonceSource(Some(binding.nonce().as_bytes()));
    let presentation = build_question_presentation_with_nonce_source(
        envelope,
        question_asset_renditions,
        &mut nonce,
    )?;
    if presentation.checksum != binding.checksum() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation checksum does not reproduce",
        ));
    }
    Ok(presentation)
}
/// Builds one presentation using an injected nonce source.
pub fn build_question_presentation_with_nonce_source<N: QuestionPresentationNonceSource>(
    envelope: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    nonce_source: &mut N,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    build_with_hasher(envelope, question_asset_renditions, nonce_source, |bytes| {
        crc16_ccitt_false(bytes)
    })
}

/// Reconstructs the exact descriptor inputs from a browser-safe envelope.
///
/// This performs no grading and does not require durable IDs. It is the only
/// operation the Wasm bridge needs in order to verify that the browser holds
/// one coherent server-issued presentation.
pub fn rebuild_public_question_presentation(
    envelope: &QuestionPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let assets = validate_public_assets(envelope, question_asset_renditions)?;
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
        presentation: envelope.clone(),
        question_asset_renditions: assets,
        item_bindings,
        checksum: QuestionPresentationChecksum::zero(),
    };
    presentation.checksum =
        QuestionPresentationChecksum::compute(&descriptor_bytes(&presentation)?);
    Ok(presentation)
}

#[cfg(test)]
pub(super) fn build_question_presentation_with_hasher<N, H>(
    envelope: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    nonce_source: &mut N,
    hasher: H,
) -> Result<IssuedQuestionPresentation, PresentationBuildError>
where
    N: QuestionPresentationNonceSource,
    H: FnMut(&[u8]) -> u16,
{
    build_with_hasher(envelope, question_asset_renditions, nonce_source, hasher)
}

fn build_with_hasher<N, H>(
    envelope: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    nonce_source: &mut N,
    mut hasher: H,
) -> Result<IssuedQuestionPresentation, PresentationBuildError>
where
    N: QuestionPresentationNonceSource,
    H: FnMut(&[u8]) -> u16,
{
    let assets = validate_assets(envelope, question_asset_renditions)?;
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
                response_item_reference: Some(item.response_item_reference.clone()),
                basis: item.basis.clone(),
            });
        }
        if collision {
            continue;
        }
        let public = public_envelope(envelope, nonce, &bindings)?;
        let mut presentation = IssuedQuestionPresentation {
            presentation: public,
            question_asset_renditions: assets.clone(),
            item_bindings: bindings,
            checksum: QuestionPresentationChecksum::zero(),
        };
        let bytes = descriptor_bytes(&presentation)?;
        presentation.checksum = QuestionPresentationChecksum::compute(&bytes);
        return Ok(presentation);
    }
    Err(PresentationBuildError::RenderedIdCollision)
}

fn rendered_id_input(
    envelope: &QuestionVariationPresentation,
    nonce: QuestionPresentationNonce,
    item: &PendingResponseItem,
    basis_bytes: &[u8],
) -> Result<Vec<u8>, PresentationBuildError> {
    let mut bytes = b"ple:rendered-item:v1\0".to_vec();
    bytes.extend_from_slice(&nonce.as_bytes());
    push_bytes(
        &mut bytes,
        envelope
            .variation
            .question_revision
            .question_id
            .to_string()
            .as_bytes(),
    )?;
    bytes.extend_from_slice(
        &envelope
            .variation
            .question_revision
            .revision_number
            .get()
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&envelope.variation.seed.value().to_be_bytes());
    bytes.push(item.role.tag());
    bytes.extend_from_slice(&item.ordinal.to_be_bytes());
    push_bytes(&mut bytes, item.response_item_reference.as_str().as_bytes())?;
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
    envelope: &QuestionVariationPresentation,
    assets: &[QuestionAssetRendition],
) -> Result<Vec<PendingResponseItem>, PresentationBuildError> {
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
            let binding = question_asset_rendition(surface, assets)?;
            let hotspot_regions = regions
                .iter()
                .map(pending_hotspot_region_geometry)
                .collect();
            push_item(
                &mut items,
                ResponseItemRole::HotspotSurface,
                &surface.asset.to_string(),
                vec![QuestionContentBlock::Image {
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
    fn body(&self) -> &[QuestionContentBlock];
}

impl PresentedResponseItem for QuestionChoice {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl PresentedResponseItem for MatchingPrompt {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl PresentedResponseItem for MatchingChoice {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

impl PresentedResponseItem for OrderingItem {
    fn id(&self) -> &ResponseItemReference {
        &self.id
    }
    fn body(&self) -> &[QuestionContentBlock] {
        &self.body
    }
}

fn push_choices<T: PresentedResponseItem>(
    target: &mut Vec<PendingResponseItem>,
    choices: &[T],
    role: ResponseItemRole,
    assets: &[QuestionAssetRendition],
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
    target: &mut Vec<PendingResponseItem>,
    role: ResponseItemRole,
    response_item_reference: &str,
    content: Vec<QuestionContentBlock>,
    assets: &[QuestionAssetRendition],
    hotspot_dimensions: Option<(u32, u32)>,
    hotspot_regions: Vec<PendingHotspotRegionGeometry>,
) -> Result<(), PresentationBuildError> {
    if response_item_reference.is_empty() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation item has an empty durable identity",
        ));
    }
    let ordinal = u32::try_from(target.len()).map_err(|_| PresentationBuildError::TooManyItems)?;
    let item_assets = content_assets(&content, assets)?;
    target.push(PendingResponseItem {
        role,
        ordinal,
        response_item_reference: ResponseItemReference::new(response_item_reference),
        basis: ResponseItemBasis {
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
    source: &QuestionVariationPresentation,
    nonce: QuestionPresentationNonce,
    bindings: &[ResponseItemBinding],
) -> Result<QuestionPresentation, PresentationBuildError> {
    let by_role = |role| bindings.iter().filter(move |binding| binding.role == role);
    let presented_item_parts = |role: ResponseItemRole| {
        by_role(role)
            .map(|binding| (binding.rendered.clone(), binding.basis.content.clone()))
            .collect::<Vec<_>>()
    };
    let response = match &source.response {
        QuestionResponseFormat::MultipleChoice {
            choices: source_choices,
            selection,
        } => match selection {
            ResponseSelectionRule::ExactlyOne => QuestionPresentationResponseFormat::SingleChoice {
                choices: presented_item_parts(ResponseItemRole::QuestionChoice)
                    .into_iter()
                    .map(|(id, body)| PresentedQuestionChoice { id, body })
                    .collect(),
            },
            _ => {
                let (minimum, maximum) = selection_bounds(*selection, source_choices.len())?;
                QuestionPresentationResponseFormat::MultipleAnswer {
                    choices: presented_item_parts(ResponseItemRole::QuestionChoice)
                        .into_iter()
                        .map(|(id, body)| PresentedQuestionChoice { id, body })
                        .collect(),
                    minimum,
                    maximum,
                }
            }
        },
        QuestionResponseFormat::ShortText { max_length, .. } => {
            QuestionPresentationResponseFormat::FillIn {
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
            QuestionPresentationResponseFormat::MultiFillIn {
                blanks: blanks
                    .iter()
                    .zip(rendered)
                    .map(|(blank, binding)| PresentedTextEntrySlot {
                        id: binding.rendered.clone(),
                        label: blank.label.clone(),
                        max_characters: blank.max_length,
                    })
                    .collect(),
            }
        }
        QuestionResponseFormat::Numeric { unit, .. } => {
            QuestionPresentationResponseFormat::Numerical {
                max_characters: NUMERIC_MAX_CHARACTERS,
                displayed_unit: unit.clone(),
            }
        }
        QuestionResponseFormat::Matching { .. } => QuestionPresentationResponseFormat::Matching {
            prompts: presented_item_parts(ResponseItemRole::MatchingPrompt)
                .into_iter()
                .map(|(id, body)| PresentedMatchingPrompt { id, body })
                .collect(),
            choices: presented_item_parts(ResponseItemRole::MatchingChoice)
                .into_iter()
                .map(|(id, body)| PresentedMatchingChoice { id, body })
                .collect(),
            reuse_choices: false,
        },
        QuestionResponseFormat::Ordering { .. } => QuestionPresentationResponseFormat::Ordering {
            items: presented_item_parts(ResponseItemRole::OrderingItem)
                .into_iter()
                .map(|(id, body)| PresentedOrderingItem { id, body })
                .collect(),
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
            let QuestionContentBlock::Image { asset, description } = &surface.basis.content[0]
            else {
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
            QuestionPresentationResponseFormat::Hotspot {
                surface: PresentedHotspotSurface {
                    id: surface.rendered.clone(),
                    asset: asset.clone(),
                    description: description.clone(),
                    regions: regions
                        .iter()
                        .zip(rendered_regions)
                        .map(|(region, binding)| PresentedHotspotRegion {
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
    Ok(QuestionPresentation {
        question_revision: source.variation.question_revision.clone(),
        seed: source.variation.seed,
        presentation_nonce: nonce,
        title: source.title.clone(),
        prompt: source.prompt.clone(),
        response,
    })
}

fn public_item_bindings(
    response: &QuestionPresentationResponseFormat,
    assets: &[QuestionAssetRendition],
) -> Result<Vec<ResponseItemBinding>, PresentationBuildError> {
    let mut target = Vec::new();
    match response {
        QuestionPresentationResponseFormat::SingleChoice { choices }
        | QuestionPresentationResponseFormat::MultipleAnswer { choices, .. } => {
            push_public_response_items(
                &mut target,
                choices,
                ResponseItemRole::QuestionChoice,
                assets,
            )?;
        }
        QuestionPresentationResponseFormat::FillIn { max_characters } => {
            require_positive(*max_characters, "fill-in maximum must be positive")?;
        }
        QuestionPresentationResponseFormat::MultiFillIn { blanks } => {
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
        QuestionPresentationResponseFormat::Numerical { max_characters, .. } => {
            require_positive(*max_characters, "numeric maximum must be positive")?;
        }
        QuestionPresentationResponseFormat::Matching {
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
            push_public_response_items(
                &mut target,
                prompts,
                ResponseItemRole::MatchingPrompt,
                assets,
            )?;
            push_public_response_items(
                &mut target,
                choices,
                ResponseItemRole::MatchingChoice,
                assets,
            )?;
        }
        QuestionPresentationResponseFormat::Ordering { items } => {
            if items.len() < 2 {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "ordering presentation requires at least two items",
                ));
            }
            push_public_response_items(&mut target, items, ResponseItemRole::OrderingItem, assets)?;
        }
        QuestionPresentationResponseFormat::Hotspot {
            surface,
            minimum,
            maximum,
        } => {
            validate_public_bounds(*minimum, *maximum, u32::MAX)?;
            let binding = question_asset_rendition(&surface.asset, assets)?;
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
                vec![QuestionContentBlock::Image {
                    asset: surface.asset.clone(),
                    description: surface.description.clone(),
                }],
                assets,
                dimensions,
                surface
                    .regions
                    .iter()
                    .map(pending_hotspot_region_geometry)
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
        QuestionPresentationResponseFormat::SingleChoice { choices } if choices.len() < 2 => {
            return Err(PresentationBuildError::InvalidPublicContent(
                "single-choice presentation requires at least two choices",
            ));
        }
        QuestionPresentationResponseFormat::MultipleAnswer {
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
fn push_public_response_items<T: PresentedResponseItemContent>(
    target: &mut Vec<ResponseItemBinding>,
    items: &[T],
    role: ResponseItemRole,
    assets: &[QuestionAssetRendition],
) -> Result<(), PresentationBuildError> {
    for item in items {
        push_public_item(
            target,
            item.presentation_item_id().clone(),
            role,
            item.presentation_item_body().to_vec(),
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
    content: Vec<QuestionContentBlock>,
    assets: &[QuestionAssetRendition],
    hotspot_dimensions: Option<(u32, u32)>,
    hotspot_regions: Vec<PendingHotspotRegionGeometry>,
) -> Result<(), PresentationBuildError> {
    let ordinal = u32::try_from(target.len()).map_err(|_| PresentationBuildError::TooManyItems)?;
    let item_assets = content_assets(&content, assets)?;
    target.push(ResponseItemBinding {
        rendered,
        role,
        ordinal,
        response_item_reference: None,
        basis: ResponseItemBasis {
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

fn pending_hotspot_region_geometry(
    region: &impl HotspotRegionGeometry,
) -> PendingHotspotRegionGeometry {
    PendingHotspotRegionGeometry {
        label: region.label().clone(),
        x: region.x(),
        y: region.y(),
        width: region.width(),
        height: region.height(),
    }
}

trait HotspotRegionGeometry {
    fn label(&self) -> &Vec<QuestionContentBlock>;
    fn x(&self) -> u16;
    fn y(&self) -> u16;
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

impl HotspotRegionGeometry for crate::response::HotspotRegion {
    fn label(&self) -> &Vec<QuestionContentBlock> {
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

impl HotspotRegionGeometry for PresentedHotspotRegion {
    fn label(&self) -> &Vec<QuestionContentBlock> {
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

fn validate_regions(regions: &[PresentedHotspotRegion]) -> Result<(), PresentationBuildError> {
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
