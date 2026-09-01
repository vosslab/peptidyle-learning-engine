//! Strict internal persistence form of one issued presentation binding.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::codec::{CURRENT_DESCRIPTOR_VERSION, QuestionPresentationChecksum};
use super::model::QuestionPresentationNonce;

/// Physical descriptor columns stored with an attempt or prefetch row.
///
/// This type is serialized only inside trusted persistence payloads. The
/// student receives the nonce in the presentation envelope and only the
/// truncated `pd1_` checksum token in its minimal attempt descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionPresentationBinding {
    nonce: QuestionPresentationNonce,
    checksum: QuestionPresentationChecksum,
}

impl QuestionPresentationBinding {
    /// Binds the exact nonce and full checksum computed by the v1 codec.
    pub fn new(nonce: QuestionPresentationNonce, checksum: QuestionPresentationChecksum) -> Self {
        Self { nonce, checksum }
    }

    /// Closed physical descriptor version.
    pub fn descriptor_version(self) -> u8 {
        CURRENT_DESCRIPTOR_VERSION
    }

    /// Exact 16 bytes persisted in the nonce column.
    pub fn nonce(self) -> QuestionPresentationNonce {
        self.nonce
    }

    /// Exact 32 bytes persisted in the checksum column.
    pub fn checksum(self) -> QuestionPresentationChecksum {
        self.checksum
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuestionPresentationBindingWire {
    descriptor_version: u8,
    nonce: String,
    checksum: String,
}

impl Serialize for QuestionPresentationBinding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        QuestionPresentationBindingWire {
            descriptor_version: CURRENT_DESCRIPTOR_VERSION,
            nonce: self.nonce.to_hex(),
            checksum: self.checksum.to_hex(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QuestionPresentationBinding {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = QuestionPresentationBindingWire::deserialize(deserializer)?;
        if wire.descriptor_version != CURRENT_DESCRIPTOR_VERSION {
            return Err(serde::de::Error::custom(
                "unsupported presentation descriptor version",
            ));
        }
        let nonce =
            QuestionPresentationNonce::parse(&wire.nonce).map_err(serde::de::Error::custom)?;
        let checksum = QuestionPresentationChecksum::parse_hex(&wire.checksum)
            .map_err(serde::de::Error::custom)?;
        Ok(Self::new(nonce, checksum))
    }
}
