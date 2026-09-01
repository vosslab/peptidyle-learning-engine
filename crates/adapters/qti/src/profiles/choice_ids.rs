//! Deterministic vendor choice identifiers for the PLE Question JSON boundary.
//!
//! Vendor response identifiers are private import-mapping input, not presentation labels. This
//! module keeps their PLE equivalents stable without exposing a parser's raw
//! choice map in any browser-safe type.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use objects::Sha256Checksum;

use super::QtiProfileId;

pub(crate) const MAX_PLE_CHOICE_ID_BYTES: usize = 64;
const DERIVED_PREFIX: &str = "qti_";
const INITIAL_DIGEST_HEX_LENGTH: usize = 16;
const DIGEST_HEX_STEP: usize = 2;
pub(crate) const MAX_VENDOR_ID_BYTES: usize = 16_384;
const MAX_ITEM_IDENTIFIER_BYTES: usize = 1_024;
const CHOICE_ID_DOMAIN: &[u8] = b"ple:qti-choice-id:v1\0";

/// One private vendor-to-PLE choice identity binding.
#[derive(Clone, PartialEq, Eq)]
pub struct QtiChoiceIdMap {
    vendor_choice_id: String,
    ple_choice_id: String,
}

impl QtiChoiceIdMap {
    pub(crate) fn new(vendor_choice_id: String, ple_choice_id: String) -> Self {
        Self {
            vendor_choice_id,
            ple_choice_id,
        }
    }

    /// Raw vendor identifier retained only for server-side import-mapping assembly.
    pub fn server_vendor_choice_id(&self) -> &str {
        &self.vendor_choice_id
    }

    /// Stable PLE semantic identifier assigned to this vendor identifier.
    pub fn ple_choice_id(&self) -> &str {
        &self.ple_choice_id
    }
}

/// Stable refusal reasons for choice identity mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QtiChoiceIdMappingError {
    TooManyChoices,
    VendorChoiceIdTooLong,
    ItemIdentifierTooLong,
    DuplicateVendorChoiceId,
    UnresolvableCollision,
}

impl fmt::Display for QtiChoiceIdMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooManyChoices => "QTI choice mapping supports at most 100 choices",
            Self::VendorChoiceIdTooLong => "QTI vendor choice identifier exceeds the input limit",
            Self::ItemIdentifierTooLong => "QTI item identifier exceeds the input limit",
            Self::DuplicateVendorChoiceId => "QTI item has a duplicate vendor choice identifier",
            Self::UnresolvableCollision => {
                "QTI choice identifier collision cannot fit the PLE 64-byte limit"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for QtiChoiceIdMappingError {}

/// Maps vendor IDs to PLE IDs while preserving input order in the returned map.
///
/// Existing valid PLE IDs are reserved before any derived ID is allocated.
/// Derived IDs are allocated in sorted raw-ID order, so reversing vendor XML
/// choices cannot change the mapping for a particular source identifier.
pub fn map_qti_choice_ids(
    profile: QtiProfileId,
    item_identifier: &str,
    vendor_choice_ids: &[String],
) -> Result<Vec<QtiChoiceIdMap>, QtiChoiceIdMappingError> {
    map_qti_choice_ids_with_hash(profile, item_identifier, vendor_choice_ids, sha256_hash)
}

fn map_qti_choice_ids_with_hash(
    profile: QtiProfileId,
    item_identifier: &str,
    vendor_choice_ids: &[String],
    hash: fn(&[u8]) -> [u8; 32],
) -> Result<Vec<QtiChoiceIdMap>, QtiChoiceIdMappingError> {
    if vendor_choice_ids.len() > 100 {
        return Err(QtiChoiceIdMappingError::TooManyChoices);
    }
    if item_identifier.len() > MAX_ITEM_IDENTIFIER_BYTES {
        return Err(QtiChoiceIdMappingError::ItemIdentifierTooLong);
    }
    if vendor_choice_ids
        .iter()
        .any(|identifier| identifier.len() > MAX_VENDOR_ID_BYTES)
    {
        return Err(QtiChoiceIdMappingError::VendorChoiceIdTooLong);
    }
    let distinct = vendor_choice_ids.iter().collect::<BTreeSet<_>>();
    if distinct.len() != vendor_choice_ids.len() {
        return Err(QtiChoiceIdMappingError::DuplicateVendorChoiceId);
    }

    let mut reserved = vendor_choice_ids
        .iter()
        .filter(|identifier| is_valid_ple_choice_id(identifier))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut resolved = BTreeMap::new();
    for identifier in &reserved {
        resolved.insert(identifier.clone(), identifier.clone());
    }

    let invalid = vendor_choice_ids
        .iter()
        .filter(|identifier| !is_valid_ple_choice_id(identifier))
        .collect::<BTreeSet<_>>();
    for identifier in invalid {
        let hash_input = choice_id_hash_input(profile, item_identifier, identifier)?;
        let hash_hex = hex_hash(&hash(&hash_input));
        let ple_choice_id = reserve_derived_choice_id(&hash_hex, &mut reserved)?;
        resolved.insert(identifier.clone(), ple_choice_id);
    }

    vendor_choice_ids
        .iter()
        .map(|vendor_choice_id| {
            let ple_choice_id = resolved
                .get(vendor_choice_id)
                .expect("every validated vendor choice identifier resolves")
                .clone();
            Ok(QtiChoiceIdMap::new(vendor_choice_id.clone(), ple_choice_id))
        })
        .collect()
}

fn choice_id_hash_input(
    profile: QtiProfileId,
    item_identifier: &str,
    vendor_choice_id: &str,
) -> Result<Vec<u8>, QtiChoiceIdMappingError> {
    let mut bytes = Vec::with_capacity(
        CHOICE_ID_DOMAIN.len()
            + profile.as_str().len()
            + item_identifier.len()
            + vendor_choice_id.len()
            + 24,
    );
    bytes.extend_from_slice(CHOICE_ID_DOMAIN);
    append_length_prefixed(&mut bytes, profile.as_str())?;
    append_length_prefixed(&mut bytes, item_identifier)?;
    append_length_prefixed(&mut bytes, vendor_choice_id)?;
    Ok(bytes)
}

fn append_length_prefixed(bytes: &mut Vec<u8>, value: &str) -> Result<(), QtiChoiceIdMappingError> {
    let length =
        u64::try_from(value.len()).map_err(|_| QtiChoiceIdMappingError::VendorChoiceIdTooLong)?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn reserve_derived_choice_id(
    digest_hex: &str,
    reserved: &mut BTreeSet<String>,
) -> Result<String, QtiChoiceIdMappingError> {
    let max_digest_hex_length = MAX_PLE_CHOICE_ID_BYTES - DERIVED_PREFIX.len();
    for digest_length in
        (INITIAL_DIGEST_HEX_LENGTH..=max_digest_hex_length).step_by(DIGEST_HEX_STEP)
    {
        let candidate = format!("{DERIVED_PREFIX}{}", &digest_hex[..digest_length]);
        if reserved.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }
    Err(QtiChoiceIdMappingError::UnresolvableCollision)
}

fn is_valid_ple_choice_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_PLE_CHOICE_ID_BYTES
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn sha256_hash(bytes: &[u8]) -> [u8; 32] {
    *Sha256Checksum::compute(bytes).as_bytes()
}

fn hex_hash(bytes: &[u8; 32]) -> String {
    let mut value = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifiers(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn map_by_vendor(entries: &[QtiChoiceIdMap]) -> BTreeMap<&str, &str> {
        entries
            .iter()
            .map(|entry| (entry.server_vendor_choice_id(), entry.ple_choice_id()))
            .collect()
    }

    #[test]
    fn preserves_valid_ple_ids_and_derives_a_golden_vendor_id() {
        let entries = map_qti_choice_ids(
            QtiProfileId::CANVAS,
            "item-1",
            &identifiers(&["blue", "bad id"]),
        )
        .expect("choice identifiers map");
        assert_eq!(entries[0].ple_choice_id(), "blue");
        assert_eq!(entries[1].ple_choice_id(), "qti_bfa06305c7f5eeff");
    }

    #[test]
    fn valid_ids_are_reserved_before_derived_ids_even_when_source_order_changes() {
        let first = map_qti_choice_ids(
            QtiProfileId::CANVAS,
            "item-1",
            &identifiers(&["bad id", "qti_bfa06305c7f5eeff"]),
        )
        .expect("choice identifiers map");
        let second = map_qti_choice_ids(
            QtiProfileId::CANVAS,
            "item-1",
            &identifiers(&["qti_bfa06305c7f5eeff", "bad id"]),
        )
        .expect("choice identifiers map");
        assert_eq!(map_by_vendor(&first), map_by_vendor(&second));
        assert_eq!(map_by_vendor(&first)["bad id"], "qti_bfa06305c7f5eeffde");
    }

    #[test]
    fn derived_collisions_extend_stably_and_do_not_follow_source_order() {
        fn same_digest(_: &[u8]) -> [u8; 32] {
            [0xab; 32]
        }

        let first = map_qti_choice_ids_with_hash(
            QtiProfileId::BLACKBOARD,
            "item-1",
            &identifiers(&["bad two", "bad one"]),
            same_digest,
        )
        .expect("choice identifiers map");
        let second = map_qti_choice_ids_with_hash(
            QtiProfileId::BLACKBOARD,
            "item-1",
            &identifiers(&["bad one", "bad two"]),
            same_digest,
        )
        .expect("choice identifiers map");
        assert_eq!(map_by_vendor(&first), map_by_vendor(&second));
        assert_eq!(map_by_vendor(&first)["bad one"], "qti_abababababababab");
        assert_eq!(map_by_vendor(&first)["bad two"], "qti_ababababababababab");
    }

    #[test]
    fn refuses_when_every_permitted_derived_length_is_reserved() {
        fn same_digest(_: &[u8]) -> [u8; 32] {
            [0xab; 32]
        }

        let mut identifiers = (INITIAL_DIGEST_HEX_LENGTH
            ..=(MAX_PLE_CHOICE_ID_BYTES - DERIVED_PREFIX.len()))
            .step_by(DIGEST_HEX_STEP)
            .map(|length| {
                let digest = "ab".repeat(30);
                format!("{DERIVED_PREFIX}{}", &digest[..length])
            })
            .collect::<Vec<_>>();
        identifiers.push("bad id".to_string());
        assert!(matches!(
            map_qti_choice_ids_with_hash(QtiProfileId::CANVAS, "item-1", &identifiers, same_digest),
            Err(QtiChoiceIdMappingError::UnresolvableCollision)
        ));
    }

    #[test]
    fn enforces_duplicate_and_length_bounds_without_echoing_raw_ids() {
        assert!(matches!(
            map_qti_choice_ids(
                QtiProfileId::CANVAS,
                "item",
                &identifiers(&["blue", "blue"])
            ),
            Err(QtiChoiceIdMappingError::DuplicateVendorChoiceId)
        ));
        assert!(matches!(
            map_qti_choice_ids(QtiProfileId::CANVAS, "x".repeat(1_025).as_str(), &[]),
            Err(QtiChoiceIdMappingError::ItemIdentifierTooLong)
        ));
        assert!(matches!(
            map_qti_choice_ids(
                QtiProfileId::CANVAS,
                "item",
                &["x".repeat(MAX_VENDOR_ID_BYTES + 1)],
            ),
            Err(QtiChoiceIdMappingError::VendorChoiceIdTooLong)
        ));
        assert!(
            !QtiChoiceIdMappingError::DuplicateVendorChoiceId
                .to_string()
                .contains("blue")
        );
    }

    #[test]
    fn accepts_the_64_byte_native_choice_id_boundary_and_derives_for_65_bytes() {
        let boundary = format!("a{}", "b".repeat(63));
        let beyond = format!("a{}", "b".repeat(64));
        let entries = map_qti_choice_ids(
            QtiProfileId::CANVAS,
            "item",
            &[boundary.clone(), beyond.clone()],
        )
        .expect("choice identifiers map");
        assert_eq!(entries[0].ple_choice_id(), boundary);
        assert!(entries[1].ple_choice_id().starts_with(DERIVED_PREFIX));
        assert!(entries[1].ple_choice_id().len() <= MAX_PLE_CHOICE_ID_BYTES);
    }
}
