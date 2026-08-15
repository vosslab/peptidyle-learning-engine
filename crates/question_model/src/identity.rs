//! Identity types (WP-C1, WP-C2, MOD-ID).
//!
//! The published-content identifiers are distinct types, not aliases for one another. A
//! function taking a [`VersionId`] cannot be handed a [`ProblemId`], so the
//! class of bug where a draft identifier reaches a published-content lookup
//! cannot compile.
//!
//! The lifecycle rule in one sentence a maintainer can apply: a draft lives in
//! an instructor workspace and has no [`ProblemId`]; publishing is the only
//! transition that constructs a fresh `(ProblemId, VersionId)` pair; one
//! immutable published question owns that pair thereafter.
//!
//! Fresh server-minted identifiers are UUIDv7: random enough that a catalog
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

/// A tenant-owned instructor workspace: the place drafts live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(Uuid);

/// One tenant-workspace-owned staged import.
///
/// This is deliberately neither a catalog number nor an object-store key. It
/// identifies an import while it remains private to an instructor workspace;
/// publication resolves the import into fresh immutable published identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkspaceImportId(Uuid);

/// One opaque identity in an immutable published-question pair.
///
/// Publishing constructs this together with a fresh [`VersionId`]. A content
/// change receives a new Question ID and another fresh pair. A draft has no
/// `ProblemId`, which makes "drafts carry no catalog number" a property of
/// the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProblemId(Uuid);

/// The second opaque identity in an immutable published-question pair.
///
/// Trusted storage keeps `(ProblemId, VersionId)` as exact delivery, grading,
/// replay, and audit evidence. Browser summaries use the assigned Question ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VersionId(Uuid);

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
impl_identifier!(ProblemId);
impl_identifier!(VersionId);
impl_identifier!(AssetId);
impl_identifier!(ObjectId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identifier_round_trips_through_its_uuid() {
        let raw = Uuid::from_u128(0x0192_3f4b_5c6d_7e8f_9012_3456_789a_bcde);
        let id = VersionId::from_uuid(raw);
        assert_eq!(id.as_uuid(), raw);
    }

    #[test]
    fn identifiers_display_as_their_uuid() {
        let raw = Uuid::from_u128(1);
        assert_eq!(
            ProblemId::from_uuid(raw).to_string(),
            "00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn an_identifier_serializes_as_a_plain_string() {
        let id = WorkspaceId::from_uuid(Uuid::from_u128(7));
        let json = serde_json::to_string(&id).expect("serialization should succeed");
        assert_eq!(json, r#""00000000-0000-0000-0000-000000000007""#);
    }
}
