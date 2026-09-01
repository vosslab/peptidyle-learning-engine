//! Versioned private bytes for a mapped vendor-to-PLE choice map.
//!
//! The profile parser owns the map and this module owns its transport
//! encoding.  The encoding is deliberately a small binary contract rather
//! than a serializable Rust value: the server may persist the bytes and their
//! checksum, while browser-safe projections cannot accidentally include them.

use std::collections::BTreeSet;

use objects::Sha256Checksum;

use super::choice_ids::{MAX_PLE_CHOICE_ID_BYTES, MAX_VENDOR_ID_BYTES, QtiChoiceIdMap};

const CHOICE_MAP_DOMAIN: &[u8] = b"ple:qti-choice-map:v1\0";
const MAX_CHOICE_COUNT: usize = 100;

/// Canonical private choice-map bytes and the checksum over those exact bytes.
///
/// This type has no `Debug`, `Serialize`, or `Deserialize` implementation.
/// Its server-prefixed accessors are the only public route to the retained
/// bytes.  The payload is created only from an adapter-validated ordered map.
///
/// ```compile_fail
/// use adapter_qti::profiles::QtiChoiceMapPayload;
/// fn needs_debug<T: std::fmt::Debug>() {}
/// needs_debug::<QtiChoiceMapPayload>();
/// ```
///
/// ```compile_fail
/// use adapter_qti::profiles::QtiChoiceMapPayload;
/// fn needs_serialize<T: serde::Serialize>() {}
/// needs_serialize::<QtiChoiceMapPayload>();
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct QtiChoiceMapPayload {
    bytes: Vec<u8>,
    sha256: Sha256Checksum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QtiChoiceMapPayloadError {
    TooManyChoices,
    VendorChoiceIdTooLong,
    PleChoiceIdTooLong,
    DuplicateVendorChoiceId,
    DuplicatePleChoiceId,
    LengthOverflow,
}

impl QtiChoiceMapPayload {
    /// Encodes an already mapped choice list while retaining its exact order.
    pub(crate) fn from_ordered_map(
        entries: &[QtiChoiceIdMap],
    ) -> Result<Self, QtiChoiceMapPayloadError> {
        validate_bounds(entries)?;

        let mut bytes = Vec::with_capacity(CHOICE_MAP_DOMAIN.len() + 4);
        bytes.extend_from_slice(CHOICE_MAP_DOMAIN);
        append_u32(&mut bytes, entries.len())?;
        for entry in entries {
            append_string(&mut bytes, entry.server_vendor_choice_id())?;
            append_string(&mut bytes, entry.ple_choice_id())?;
        }
        let sha256 = Sha256Checksum::compute(&bytes);
        Ok(Self { bytes, sha256 })
    }

    /// Returns the exact private bytes for server-side persistence.
    pub fn server_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the checksum for [`Self::server_bytes`].
    pub fn server_sha256(&self) -> Sha256Checksum {
        self.sha256
    }
}

fn validate_bounds(entries: &[QtiChoiceIdMap]) -> Result<(), QtiChoiceMapPayloadError> {
    if entries.len() > MAX_CHOICE_COUNT {
        return Err(QtiChoiceMapPayloadError::TooManyChoices);
    }
    let mut vendor_ids = BTreeSet::new();
    let mut ple_ids = BTreeSet::new();
    for entry in entries {
        if entry.server_vendor_choice_id().len() > MAX_VENDOR_ID_BYTES {
            return Err(QtiChoiceMapPayloadError::VendorChoiceIdTooLong);
        }
        if entry.ple_choice_id().len() > MAX_PLE_CHOICE_ID_BYTES {
            return Err(QtiChoiceMapPayloadError::PleChoiceIdTooLong);
        }
        if !vendor_ids.insert(entry.server_vendor_choice_id()) {
            return Err(QtiChoiceMapPayloadError::DuplicateVendorChoiceId);
        }
        if !ple_ids.insert(entry.ple_choice_id()) {
            return Err(QtiChoiceMapPayloadError::DuplicatePleChoiceId);
        }
    }
    Ok(())
}

fn append_u32(bytes: &mut Vec<u8>, value: usize) -> Result<(), QtiChoiceMapPayloadError> {
    let value = u32::try_from(value).map_err(|_| QtiChoiceMapPayloadError::LengthOverflow)?;
    bytes.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn append_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), QtiChoiceMapPayloadError> {
    append_u32(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::QtiChoiceIdMap;

    fn entries(values: &[(&str, &str)]) -> Vec<QtiChoiceIdMap> {
        values
            .iter()
            .map(|(vendor, ple)| QtiChoiceIdMap::new((*vendor).to_string(), (*ple).to_string()))
            .collect()
    }

    #[test]
    fn encoding_has_a_versioned_domain_and_preserves_order() {
        let payload =
            QtiChoiceMapPayload::from_ordered_map(&entries(&[("blue", "blue"), ("red", "red")]))
                .expect("bounded map");
        let expected = [
            CHOICE_MAP_DOMAIN,
            &[0, 0, 0, 2],
            &[0, 0, 0, 4],
            b"blue",
            &[0, 0, 0, 4],
            b"blue",
            &[0, 0, 0, 3],
            b"red",
            &[0, 0, 0, 3],
            b"red",
        ]
        .concat();
        assert_eq!(payload.server_bytes(), expected);
    }

    #[test]
    fn golden_digest_is_stable() {
        let payload =
            QtiChoiceMapPayload::from_ordered_map(&entries(&[("blue", "blue"), ("red", "red")]))
                .expect("bounded map");
        assert_eq!(
            payload.server_sha256().to_string(),
            "304b5c4bd3bda1952f96be4f3e3bbc1da68636d70aaf47b764ab5e3a9cd2cdb9",
        );
    }

    #[test]
    fn repeated_encoding_is_deterministic() {
        let values = entries(&[("vendor-blue", "blue"), ("vendor-red", "red")]);
        let first = QtiChoiceMapPayload::from_ordered_map(&values).expect("bounded map");
        let second = QtiChoiceMapPayload::from_ordered_map(&values).expect("bounded map");
        assert!(first == second);
        assert_eq!(first.server_sha256(), second.server_sha256());
    }

    #[test]
    fn changing_either_side_of_a_pair_changes_payload_and_digest() {
        let baseline = QtiChoiceMapPayload::from_ordered_map(&entries(&[("vendor", "blue")]))
            .expect("bounded map");
        let vendor = QtiChoiceMapPayload::from_ordered_map(&entries(&[("other", "blue")]))
            .expect("bounded map");
        let ple = QtiChoiceMapPayload::from_ordered_map(&entries(&[("vendor", "red")]))
            .expect("bounded map");
        assert_ne!(baseline.server_bytes(), vendor.server_bytes());
        assert_ne!(baseline.server_bytes(), ple.server_bytes());
        assert_ne!(baseline.server_sha256(), vendor.server_sha256());
        assert_ne!(baseline.server_sha256(), ple.server_sha256());
    }

    #[test]
    fn reordering_pairs_changes_payload_and_digest() {
        let first = QtiChoiceMapPayload::from_ordered_map(&entries(&[
            ("vendor-blue", "blue"),
            ("vendor-red", "red"),
        ]))
        .expect("bounded map");
        let second = QtiChoiceMapPayload::from_ordered_map(&entries(&[
            ("vendor-red", "red"),
            ("vendor-blue", "blue"),
        ]))
        .expect("bounded map");
        assert_ne!(first.server_bytes(), second.server_bytes());
        assert_ne!(first.server_sha256(), second.server_sha256());
    }

    #[test]
    fn inherited_limits_are_rechecked_at_payload_boundary() {
        let too_many = (0..=MAX_CHOICE_COUNT)
            .map(|index| QtiChoiceIdMap::new(format!("vendor-{index}"), format!("choice-{index}")))
            .collect::<Vec<_>>();
        assert!(matches!(
            QtiChoiceMapPayload::from_ordered_map(&too_many),
            Err(QtiChoiceMapPayloadError::TooManyChoices)
        ));

        let too_long_vendor = vec![QtiChoiceIdMap::new(
            "v".repeat(MAX_VENDOR_ID_BYTES + 1),
            "choice".to_string(),
        )];
        assert!(matches!(
            QtiChoiceMapPayload::from_ordered_map(&too_long_vendor),
            Err(QtiChoiceMapPayloadError::VendorChoiceIdTooLong)
        ));
    }
}
