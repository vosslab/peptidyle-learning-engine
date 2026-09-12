//! Construction of globally collision-free Presentation Response Item References.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::answer::ResponseSelectionRule;
use crate::response::{
    MatchingChoice, MatchingPrompt, OrderingItem, QuestionChoice, QuestionResponseFormat,
    ResponseItemReference,
};
use crate::{QuestionContentBlock, QuestionVariationPresentation};

use super::assets::{
    content_assets, question_asset_rendition, validate_assets, validate_public_assets,
};
use super::binding::QuestionPresentationBinding;
use super::choice_order::nonce_randomized_choices;
use super::codec::{
    QuestionPresentationChecksum, crc16_ccitt_false, descriptor_bytes, item_basis_bytes,
};
use super::model::{
    PresentationResponseItemReference, PresentedHotspotRegion, PresentedHotspotSurface,
    PresentedMatchingChoice, PresentedMatchingPrompt, PresentedOrderingItem,
    PresentedQuestionChoice, PresentedTextEntrySlot, QuestionAssetRendition, QuestionPresentation,
    QuestionPresentationNonce, QuestionPresentationResponseFormat,
};
use super::public_response_items::public_item_bindings;
use super::response_validation::selection_bounds;
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
    pub presentation_response_item_reference: PresentationResponseItemReference,
    pub role: ResponseItemRole,
    pub ordinal: u32,
    /// Exact authored Response Item for issued bindings; absent during public verification.
    pub response_item_reference: Option<ResponseItemReference>,
    pub(super) basis: ResponseItemBasis,
}

/// One retained link from a browser-scoped response item reference to the
/// durable authored response item it denotes.
///
/// The public descriptor can deterministically rebuild the left side.  The
/// right side is retained with Student Work so a saved response remains
/// meaningful after source content is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableResponseItemBinding {
    pub presentation_response_item_reference: PresentationResponseItemReference,
    pub response_item_reference: ResponseItemReference,
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
    InvalidPublicContent(&'static str),
    TooManyItems,
    PresentationResponseItemReferenceCollision,
    DescriptorEncoding(&'static str),
}

impl std::fmt::Display for PresentationBuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RandomnessUnavailable => {
                formatter.write_str("presentation nonce randomness is unavailable")
            }
            Self::InvalidPublicContent(message) => formatter.write_str(message),
            Self::TooManyItems => formatter.write_str("presentation contains more than 32 items"),
            Self::PresentationResponseItemReferenceCollision => formatter
                .write_str("could not mint globally unique Presentation Response Item References"),
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
    presentation: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    build_question_presentation_with_nonce_source(
        presentation,
        question_asset_renditions,
        &mut OperatingSystemQuestionPresentationNonceSource,
    )
}

struct PersistedNonceSource(Option<[u8; 16]>);

impl QuestionPresentationNonceSource for PersistedNonceSource {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        self.0
            .take()
            .ok_or(PresentationBuildError::PresentationResponseItemReferenceCollision)
    }
}

/// Rebuilds the server-issued Question Presentation from its durable nonce and checksum.
pub fn reproduce_question_presentation(
    presentation: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    binding: QuestionPresentationBinding,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let mut nonce = PersistedNonceSource(Some(binding.nonce().as_bytes()));
    let presentation = build_question_presentation_with_nonce_source(
        presentation,
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
    presentation: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    nonce_source: &mut N,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    build_with_hasher(
        presentation,
        question_asset_renditions,
        nonce_source,
        crc16_ccitt_false,
    )
}

/// Reconstructs descriptor inputs from one browser-safe Question Presentation for Wasm verification.
pub fn rebuild_public_question_presentation(
    presentation: &QuestionPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let assets = validate_public_assets(presentation, question_asset_renditions)?;
    let item_bindings = public_item_bindings(&presentation.response, &assets)?;
    if item_bindings.len() > MAX_PRESENTED_ITEMS {
        return Err(PresentationBuildError::TooManyItems);
    }
    let unique: BTreeSet<_> = item_bindings
        .iter()
        .map(|item| item.presentation_response_item_reference.clone())
        .collect();
    if unique.len() != item_bindings.len() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation repeats a Presentation Response Item Reference",
        ));
    }
    let mut presentation = IssuedQuestionPresentation {
        presentation: presentation.clone(),
        question_asset_renditions: assets,
        item_bindings,
        checksum: QuestionPresentationChecksum::zero(),
    };
    presentation.checksum =
        QuestionPresentationChecksum::compute(&descriptor_bytes(&presentation)?);
    Ok(presentation)
}

/// Extracts the durable response-item links before an issued presentation
/// crosses into answer-free persistence.
pub fn extract_durable_response_item_bindings(
    presentation: &IssuedQuestionPresentation,
) -> Result<Vec<DurableResponseItemBinding>, PresentationBuildError> {
    presentation
        .item_bindings
        .iter()
        .map(|binding| {
            binding
                .response_item_reference
                .clone()
                .map(|response_item_reference| DurableResponseItemBinding {
                    presentation_response_item_reference: binding
                        .presentation_response_item_reference
                        .clone(),
                    response_item_reference,
                })
                .ok_or(PresentationBuildError::InvalidPublicContent(
                    "issued presentation lacks a durable response-item binding",
                ))
        })
        .collect()
}

/// Reattaches retained durable response-item links to a checksum-verified
/// public reconstruction.  The two sets must agree exactly: a stored mapping
/// cannot add, omit, duplicate, or redirect a public response item.
pub fn rebind_durable_response_item_bindings(
    mut presentation: IssuedQuestionPresentation,
    bindings: &[DurableResponseItemBinding],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    let public_references: BTreeSet<_> = presentation
        .item_bindings
        .iter()
        .map(|binding| binding.presentation_response_item_reference.clone())
        .collect();
    if public_references.len() != presentation.item_bindings.len()
        || bindings.len() != presentation.item_bindings.len()
    {
        return Err(PresentationBuildError::InvalidPublicContent(
            "retained response-item bindings do not exactly cover the presentation",
        ));
    }
    let retained_references: BTreeSet<_> = bindings
        .iter()
        .map(|binding| binding.presentation_response_item_reference.clone())
        .collect();
    let durable_references: BTreeSet<_> = bindings
        .iter()
        .map(|binding| binding.response_item_reference.clone())
        .collect();
    if retained_references.len() != bindings.len()
        || durable_references.len() != bindings.len()
        || retained_references != public_references
        || bindings
            .iter()
            .any(|binding| binding.response_item_reference.as_str().is_empty())
    {
        return Err(PresentationBuildError::InvalidPublicContent(
            "retained response-item bindings are invalid",
        ));
    }
    for item in &mut presentation.item_bindings {
        let binding = bindings
            .iter()
            .find(|binding| {
                binding.presentation_response_item_reference
                    == item.presentation_response_item_reference
            })
            .ok_or(PresentationBuildError::InvalidPublicContent(
                "retained response-item bindings do not exactly cover the presentation",
            ))?;
        item.response_item_reference = Some(binding.response_item_reference.clone());
    }
    Ok(presentation)
}
#[cfg(test)]
pub(super) fn build_question_presentation_with_hasher<N, H>(
    presentation: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    nonce_source: &mut N,
    hasher: H,
) -> Result<IssuedQuestionPresentation, PresentationBuildError>
where
    N: QuestionPresentationNonceSource,
    H: FnMut(&[u8]) -> u16,
{
    build_with_hasher(
        presentation,
        question_asset_renditions,
        nonce_source,
        hasher,
    )
}
fn build_with_hasher<N, H>(
    presentation: &QuestionVariationPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    nonce_source: &mut N,
    mut hasher: H,
) -> Result<IssuedQuestionPresentation, PresentationBuildError>
where
    N: QuestionPresentationNonceSource,
    H: FnMut(&[u8]) -> u16,
{
    let assets = validate_assets(presentation, question_asset_renditions)?;
    for _ in 0..MAX_NONCE_ATTEMPTS {
        let nonce = QuestionPresentationNonce::from_bytes(nonce_source.next_nonce()?);
        let pending = pending_items(presentation, &assets, nonce)?;
        if pending.len() > MAX_PRESENTED_ITEMS {
            return Err(PresentationBuildError::TooManyItems);
        }
        let mut used = BTreeSet::new();
        let mut bindings = Vec::with_capacity(pending.len());
        let mut collision = false;
        for item in &pending {
            let basis_bytes = item_basis_bytes(&item.basis)?;
            let input = presentation_response_item_reference_input(
                presentation,
                nonce,
                item,
                &basis_bytes,
            )?;
            let presentation_response_item_reference =
                PresentationResponseItemReference::from_crc(hasher(&input));
            if !used.insert(presentation_response_item_reference.clone()) {
                collision = true;
                break;
            }
            bindings.push(ResponseItemBinding {
                presentation_response_item_reference,
                role: item.role,
                ordinal: item.ordinal,
                response_item_reference: Some(item.response_item_reference.clone()),
                basis: item.basis.clone(),
            });
        }
        if collision {
            continue;
        }
        let public = public_presentation(presentation, nonce, &bindings)?;
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
    Err(PresentationBuildError::PresentationResponseItemReferenceCollision)
}
fn presentation_response_item_reference_input(
    presentation: &QuestionVariationPresentation,
    nonce: QuestionPresentationNonce,
    item: &PendingResponseItem,
    basis_bytes: &[u8],
) -> Result<Vec<u8>, PresentationBuildError> {
    let mut bytes = b"ple:presentation-response-item-reference:v1\0".to_vec();
    bytes.extend_from_slice(&nonce.as_bytes());
    push_bytes(
        &mut bytes,
        presentation
            .variation
            .question_revision
            .question_id
            .to_string()
            .as_bytes(),
    )?;
    bytes.extend_from_slice(
        &presentation
            .variation
            .question_revision
            .revision_number
            .get()
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&presentation.variation.question_seed.value().to_be_bytes());
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
    presentation: &QuestionVariationPresentation,
    assets: &[QuestionAssetRendition],
    nonce: QuestionPresentationNonce,
) -> Result<Vec<PendingResponseItem>, PresentationBuildError> {
    let mut items = Vec::new();
    match &presentation.response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            let choices =
                if presentation.native_choice_order == crate::NativeChoiceOrder::NonceRandomized {
                    nonce_randomized_choices(choices, nonce)
                } else {
                    choices.clone()
                };
            push_choices(
                &mut items,
                &choices,
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
                &surface.question_asset.to_string(),
                vec![QuestionContentBlock::Image {
                    question_asset: surface.clone(),
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
        QuestionResponseFormat::ImathasQuestionBackend {} => {}
    }
    Ok(items)
}

/// Derives the issued choice order from the durable presentation nonce before bindings are minted.
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
fn public_presentation(
    source: &QuestionVariationPresentation,
    nonce: QuestionPresentationNonce,
    bindings: &[ResponseItemBinding],
) -> Result<QuestionPresentation, PresentationBuildError> {
    let by_role = |role| bindings.iter().filter(move |binding| binding.role == role);
    let presented_item_parts = |role: ResponseItemRole| {
        by_role(role)
            .map(|binding| {
                (
                    binding.presentation_response_item_reference.clone(),
                    binding.basis.content.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let response = match &source.response {
        QuestionResponseFormat::MultipleChoice {
            choices: source_choices,
            selection,
            ..
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
            let text_entry_bindings: Vec<_> = by_role(ResponseItemRole::TextEntrySlot).collect();
            if text_entry_bindings.len() != blanks.len() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "blank presentation mapping is incomplete",
                ));
            }
            QuestionPresentationResponseFormat::MultiFillIn {
                blanks: blanks
                    .iter()
                    .zip(text_entry_bindings)
                    .map(|(blank, binding)| PresentedTextEntrySlot {
                        id: binding.presentation_response_item_reference.clone(),
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
            let QuestionContentBlock::Image {
                question_asset,
                description,
            } = &surface.basis.content[0]
            else {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "hotspot surface is not image-backed",
                ));
            };
            let (minimum, maximum) = selection_bounds(*selection, regions.len())?;
            let hotspot_region_bindings: Vec<_> =
                by_role(ResponseItemRole::HotspotRegion).collect();
            if hotspot_region_bindings.len() != regions.len() {
                return Err(PresentationBuildError::InvalidPublicContent(
                    "hotspot region presentation mapping is incomplete",
                ));
            }
            QuestionPresentationResponseFormat::Hotspot {
                surface: PresentedHotspotSurface {
                    id: surface.presentation_response_item_reference.clone(),
                    question_asset: question_asset.clone(),
                    description: description.clone(),
                    regions: regions
                        .iter()
                        .zip(hotspot_region_bindings)
                        .map(|(region, binding)| PresentedHotspotRegion {
                            id: binding.presentation_response_item_reference.clone(),
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
        QuestionResponseFormat::ImathasQuestionBackend {} => {
            QuestionPresentationResponseFormat::ImathasQuestionBackend {}
        }
    };
    Ok(QuestionPresentation {
        question_revision: source.variation.question_revision.clone(),
        question_seed: source.variation.question_seed,
        presentation_nonce: nonce,
        question_title: source.question_title.clone(),
        prompt: source.prompt.clone(),
        response,
    })
}

pub(super) fn pending_hotspot_region_geometry(
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

pub(super) trait HotspotRegionGeometry {
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
