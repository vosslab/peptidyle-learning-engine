//! Identity types (WP-C1, WP-C2, MOD-ID).
//!
//! Published Question identity is the stable human-facing Question ID plus a
//! positive Question Version Number. A draft carries neither value;
//! publication establishes the first version and each accepted same-lineage
//! change advances the version number.
//!
//! Fresh server-minted identifiers are UUIDv7: random enough that a Question Library
//! number leaks no volume information, time-ordered enough to index well, and
//! never sequential. Storage and wire decoding still accept every canonical
//! UUID value so deterministic local fixtures and previously persisted IDs can
//! round-trip without the browser imposing a stricter policy than the server.
//! Generation sits behind the `generate` feature, which the server enables and
//! the WebAssembly bridge leaves off, because identifiers are minted server-side.
//!
//! Each struct is written out rather than produced by a macro, because the
//! TypeScript generator in `crates/project-tools` reads this source: a type declared
//! inside a macro body is invisible to it, and to a reader skimming for the
//! contract. The shared behavior is generated once by `impl_identifier!`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An instructor workspace: the place drafts live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(Uuid);

/// One workspace-owned staged import.
///
/// This is deliberately neither a Question Library number nor an object-store key. It
/// identifies an import while it remains private to an instructor workspace;
/// publication resolves the import into fresh immutable published identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkspaceImportId(Uuid);

/// Positive, monotonic version number within one Question lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct QuestionVersionNumber(u32);

impl QuestionVersionNumber {
    /// Creates a positive version number.
    pub fn new(value: u32) -> Result<Self, &'static str> {
        if value == 0 {
            return Err("question version number must be positive");
        }
        Ok(Self(value))
    }

    /// Returns the stored positive integer.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for QuestionVersionNumber {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<QuestionVersionNumber> for u32 {
    fn from(value: QuestionVersionNumber) -> Self {
        value.0
    }
}

impl std::fmt::Display for QuestionVersionNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A stored asset: an image, a figure, or an imported source package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetId(Uuid);

/// One immutable object-store record.
///
/// An asset may point at an object, but the two identities stay distinct so a
/// later physical deduplication scheme can change object placement without
/// changing the logical asset referenced by content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(Uuid);

/// Gives an identifier newtype its shared behavior.
///
/// Written once so the four identifiers stay identical in how they wrap,
/// unwrap, mint, and display their value.
macro_rules! impl_identifier {
    ($name:ident) => {
        impl $name {
            /// Wraps an existing UUID, for values read back from storage.
            pub fn from_uuid(value: Uuid) -> Self {
                $name(value)
            }

            /// The underlying UUID, for storage and logging.
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Mints a fresh identifier.
            ///
            /// Server-side only: the `generate` feature stays off in the
            /// browser bundle, so this is unavailable there.
            #[cfg(feature = "generate")]
            pub fn generate() -> Self {
                $name(Uuid::now_v7())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }
    };
}

impl_identifier!(WorkspaceId);
impl_identifier!(WorkspaceImportId);
impl_identifier!(AssetId);
impl_identifier!(ObjectId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_version_number_is_positive() {
        let version = QuestionVersionNumber::new(1).expect("positive version");
        assert_eq!(version.get(), 1);
    }

    #[test]
    fn question_version_numbers_display_as_integers() {
        assert_eq!(
            QuestionVersionNumber::new(1)
                .expect("positive version")
                .to_string(),
            "1"
        );
    }

    #[test]
    fn an_identifier_serializes_as_a_plain_string() {
        let id = WorkspaceId::from_uuid(Uuid::from_u128(7));
        let json = serde_json::to_string(&id).expect("serialization should succeed");
        assert_eq!(json, r#""00000000-0000-0000-0000-000000000007""#);
    }
}
