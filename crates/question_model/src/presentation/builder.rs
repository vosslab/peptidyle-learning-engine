//! Construction of globally collision-free Presentation Response Item IDs.

#[path = "builder_items.rs"]
mod builder_items;

pub(super) use builder_items::pending_hotspot_region_geometry;
use builder_items::{pending_items, public_presentation};

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::response::ResponseItemId;
use crate::{
    AuthorContentPresentation, QuestionContentBlock, QuestionReproduction,
    QuestionVariationPresentation,
};

use super::assets::{validate_assets, validate_public_assets};
use super::binding::QuestionPresentationBinding;
use super::codec::{
    QuestionPresentationChecksum, crc16_ccitt_false, descriptor_bytes, item_basis_bytes,
};
use super::model::{
    PresentationResponseItemId, QuestionAssetRendition, QuestionPresentation,
    QuestionPresentationNonce, QuestionPresentationResponseFormat,
};
use super::public_response_items::public_item_bindings;
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
    pub presentation_response_item_id: PresentationResponseItemId,
    pub role: ResponseItemRole,
    pub ordinal: u32,
    /// Exact authored Response Item for issued bindings; absent during public verification.
    pub response_item_id: Option<ResponseItemId>,
    pub(super) basis: ResponseItemBasis,
}

/// One retained link from a browser-scoped response item ID to the
/// durable authored response item it denotes.
///
/// The public descriptor can deterministically rebuild the left side.  The
/// right side is retained with Student Work so a saved response remains
/// meaningful after source content is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableResponseItemBinding {
    pub presentation_response_item_id: PresentationResponseItemId,
    pub response_item_id: ResponseItemId,
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
    response_item_id: ResponseItemId,
    basis: ResponseItemBasis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedQuestionPresentation {
    pub presentation: QuestionPresentation,
    /// Server-only tagged reproduction evidence. It never crosses the public
    /// Question Presentation boundary.
    pub reproduction: QuestionReproduction,
    pub question_asset_renditions: Vec<QuestionAssetRendition>,
    pub item_bindings: Vec<ResponseItemBinding>,
    /// Server-only isolated author-content evidence. It is not part of the
    /// generic browser `QuestionPresentation` DTO.
    pub author_content: Option<AuthorContentPresentation>,
    pub checksum: QuestionPresentationChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationBuildError {
    RandomnessUnavailable,
    InvalidPublicContent(&'static str),
    TooManyItems,
    PresentationResponseItemIdCollision,
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
            Self::PresentationResponseItemIdCollision => {
                formatter.write_str("could not mint globally unique Presentation Response Item IDs")
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
            .ok_or(PresentationBuildError::PresentationResponseItemIdCollision)
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
/// Rebuilds a browser-visible native static presentation.
///
/// This intentionally rejects renderer-owned response formats. They have
/// private tagged reproduction evidence and must be rebuilt by the server
/// through [`rebuild_question_presentation_with_reproduction`].
pub fn rebuild_native_static_question_presentation(
    presentation: &QuestionPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    if matches!(
        presentation.response,
        QuestionPresentationResponseFormat::BackendOwned {}
            | QuestionPresentationResponseFormat::ImathasQuestionBackend {}
    ) {
        return Err(PresentationBuildError::InvalidPublicContent(
            "backend-owned presentation cannot rebuild as native static evidence",
        ));
    }
    rebuild_question_presentation_with_reproduction(
        presentation,
        question_asset_renditions,
        QuestionReproduction::Static,
    )
}

/// Reconstructs one descriptor using server-retained reproduction evidence.
///
/// Browser-safe presentation JSON never carries a source or generator seed.
/// Server resume and history reads must therefore provide the exact tagged
/// reproduction evidence they retained alongside the presentation.
pub fn rebuild_question_presentation_with_reproduction(
    presentation: &QuestionPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    reproduction: QuestionReproduction,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    rebuild_question_presentation_with_reproduction_and_author_content(
        presentation,
        question_asset_renditions,
        reproduction,
        None,
    )
}

/// Reconstructs one descriptor with separately retained server-only author
/// content. The generic browser presentation deliberately cannot carry it.
pub fn rebuild_question_presentation_with_reproduction_and_author_content(
    presentation: &QuestionPresentation,
    question_asset_renditions: &[QuestionAssetRendition],
    reproduction: QuestionReproduction,
    author_content: Option<AuthorContentPresentation>,
) -> Result<IssuedQuestionPresentation, PresentationBuildError> {
    if let Some(author_content) = author_content.as_ref()
        && presentation.author_content_digest.as_deref() != Some(author_content.digest().as_str())
    {
        return Err(PresentationBuildError::InvalidPublicContent(
            "author content digest does not match retained descriptor",
        ));
    }
    let assets = validate_public_assets(presentation, question_asset_renditions)?;
    let item_bindings = public_item_bindings(&presentation.response, &assets)?;
    if item_bindings.len() > MAX_PRESENTED_ITEMS {
        return Err(PresentationBuildError::TooManyItems);
    }
    let unique: BTreeSet<_> = item_bindings
        .iter()
        .map(|item| item.presentation_response_item_id.clone())
        .collect();
    if unique.len() != item_bindings.len() {
        return Err(PresentationBuildError::InvalidPublicContent(
            "presentation repeats a Presentation Response Item ID",
        ));
    }
    let mut presentation = IssuedQuestionPresentation {
        presentation: presentation.clone(),
        reproduction,
        question_asset_renditions: assets,
        item_bindings,
        author_content,
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
                .response_item_id
                .clone()
                .map(|response_item_id| DurableResponseItemBinding {
                    presentation_response_item_id: binding.presentation_response_item_id.clone(),
                    response_item_id,
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
    let public_ids: BTreeSet<_> = presentation
        .item_bindings
        .iter()
        .map(|binding| binding.presentation_response_item_id.clone())
        .collect();
    if public_ids.len() != presentation.item_bindings.len()
        || bindings.len() != presentation.item_bindings.len()
    {
        return Err(PresentationBuildError::InvalidPublicContent(
            "retained response-item bindings do not exactly cover the presentation",
        ));
    }
    let retained_ids: BTreeSet<_> = bindings
        .iter()
        .map(|binding| binding.presentation_response_item_id.clone())
        .collect();
    let durable_ids: BTreeSet<_> = bindings
        .iter()
        .map(|binding| binding.response_item_id.clone())
        .collect();
    if retained_ids.len() != bindings.len()
        || durable_ids.len() != bindings.len()
        || retained_ids != public_ids
        || bindings
            .iter()
            .any(|binding| binding.response_item_id.as_str().is_empty())
    {
        return Err(PresentationBuildError::InvalidPublicContent(
            "retained response-item bindings are invalid",
        ));
    }
    for item in &mut presentation.item_bindings {
        let binding = bindings
            .iter()
            .find(|binding| {
                binding.presentation_response_item_id == item.presentation_response_item_id
            })
            .ok_or(PresentationBuildError::InvalidPublicContent(
                "retained response-item bindings do not exactly cover the presentation",
            ))?;
        item.response_item_id = Some(binding.response_item_id.clone());
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
            let input =
                presentation_response_item_id_input(presentation, nonce, item, &basis_bytes)?;
            let presentation_response_item_id =
                PresentationResponseItemId::from_crc(hasher(&input));
            if !used.insert(presentation_response_item_id.clone()) {
                collision = true;
                break;
            }
            bindings.push(ResponseItemBinding {
                presentation_response_item_id,
                role: item.role,
                ordinal: item.ordinal,
                response_item_id: Some(item.response_item_id.clone()),
                basis: item.basis.clone(),
            });
        }
        if collision {
            continue;
        }
        let public = public_presentation(presentation, nonce, &bindings)?;
        let mut presentation = IssuedQuestionPresentation {
            presentation: public,
            reproduction: presentation.variation.reproduction.clone(),
            question_asset_renditions: assets.clone(),
            item_bindings: bindings,
            author_content: presentation.author_content.clone(),
            checksum: QuestionPresentationChecksum::zero(),
        };
        let bytes = descriptor_bytes(&presentation)?;
        presentation.checksum = QuestionPresentationChecksum::compute(&bytes);
        return Ok(presentation);
    }
    Err(PresentationBuildError::PresentationResponseItemIdCollision)
}
fn presentation_response_item_id_input(
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
    match &presentation.variation.reproduction {
        QuestionReproduction::Static => bytes.push(0),
        QuestionReproduction::Seeded {
            question_seed,
            generated_parameter_sha256,
        } => {
            bytes.push(1);
            bytes.extend_from_slice(&question_seed.value().to_be_bytes());
            push_bytes(&mut bytes, generated_parameter_sha256.as_bytes())?;
        }
    }
    bytes.push(item.role.tag());
    bytes.extend_from_slice(&item.ordinal.to_be_bytes());
    push_bytes(&mut bytes, item.response_item_id.as_str().as_bytes())?;
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
