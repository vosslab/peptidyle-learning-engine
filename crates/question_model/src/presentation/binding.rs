//! Strict internal persistence form of one issued presentation binding.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::codec::{DESCRIPTOR_VERSION_V1, PresentationDigestV1};
use super::model::PresentationNonceV1;

/// Physical descriptor columns stored with an attempt or prefetch row.
///
/// This type is serialized only inside trusted persistence payloads. The
/// learner receives the nonce in the presentation envelope and only the
/// truncated `pd1_` digest token in its minimal attempt descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationBindingV1 {
    nonce: PresentationNonceV1,
    digest: PresentationDigestV1,
}

impl PresentationBindingV1 {
    /// Binds the exact nonce and full digest computed by the v1 codec.
    pub fn new(nonce: PresentationNonceV1, digest: PresentationDigestV1) -> Self {
        Self { nonce, digest }
    }

    /// Closed physical descriptor version.
    pub fn descriptor_version(self) -> u8 {
        DESCRIPTOR_VERSION_V1
    }

    /// Exact 16 bytes persisted in the nonce column.
    pub fn nonce(self) -> PresentationNonceV1 {
        self.nonce
    }

    /// Exact 32 bytes persisted in the digest column.
    pub fn digest(self) -> PresentationDigestV1 {
        self.digest
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PresentationBindingWireV1 {
    descriptor_version: u8,
    nonce: String,
    digest: String,
}

impl Serialize for PresentationBindingV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        PresentationBindingWireV1 {
            descriptor_version: DESCRIPTOR_VERSION_V1,
            nonce: self.nonce.to_hex(),
            digest: self.digest.to_hex(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PresentationBindingV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = PresentationBindingWireV1::deserialize(deserializer)?;
        if wire.descriptor_version != DESCRIPTOR_VERSION_V1 {
            return Err(serde::de::Error::custom(
                "unsupported presentation descriptor version",
            ));
        }
        let nonce = PresentationNonceV1::parse(&wire.nonce).map_err(serde::de::Error::custom)?;
        let digest =
            PresentationDigestV1::parse_hex(&wire.digest).map_err(serde::de::Error::custom)?;
        Ok(Self::new(nonce, digest))
    }
}
