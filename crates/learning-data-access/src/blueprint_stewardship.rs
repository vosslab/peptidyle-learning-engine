//! Authenticated Instructor stewardship state for one Blueprint Course lineage.
//!
//! A Blueprint Course Reference, rather than a Revision reference, is the
//! durable identity. The closed store surface exposes only the caller's own
//! Star and private Watch state, the approved Star aggregate, and the
//! caller's own immutable Watch-event records.

use async_trait::async_trait;
use question_model::{BlueprintCourseReference, Timestamp};

use crate::{SessionTokenHash, StoreError};

/// Self-only stewardship state for one Public or Archived Blueprint Course.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintStewardshipState {
    /// Whether the authenticated Instructor Stars this Blueprint lineage.
    pub starred: bool,
    /// Whether the authenticated Instructor privately Watches this lineage.
    pub watching: bool,
}

/// Browser-safe Star facts for one Blueprint Course lineage.
///
/// This deliberately has no endorser identity. C856 adds the separately
/// authorized Verified Instructor Display Name projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintCourseStarProjection {
    pub viewer_has_starred: bool,
    pub star_count: u64,
}

/// One approved identity in the separately authorized Blueprint Star list.
///
/// This intentionally carries only the immutable vetted display name. It is
/// neither an Account/Profile projection nor a directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintCourseStarredInstructor {
    pub display_name: String,
}

/// Browser-safe private Watch state for the authenticated Instructor only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintCourseWatchProjection {
    pub watching: bool,
}

/// The four immutable source changes that can notify a Blueprint watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseWatchEventKind {
    Revision,
    Published,
    Archived,
    Restored,
}

/// One recipient-owned, immutable Blueprint Course Watch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintCourseWatchEvent {
    pub kind: BlueprintCourseWatchEventKind,
    pub occurred_at: Timestamp,
}

/// Authenticated self-only Blueprint Course stewardship boundary.
#[async_trait]
pub trait BlueprintStewardshipStore: Send + Sync {
    /// Reads the caller's Star state and the active-Instructor Star count.
    ///
    /// It reveals neither a Starred-by identity nor any Watch fact.
    async fn blueprint_course_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
    ) -> Result<BlueprintCourseStarProjection, StoreError>;

    /// Sets only the caller's Star state, then reads the closed Star projection.
    async fn set_current_blueprint_course_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
        starred: bool,
    ) -> Result<BlueprintCourseStarProjection, StoreError>;

    /// Lists only active vetted Instructors' immutable display names for one
    /// Public or Archived Blueprint Course. It returns no Account, email,
    /// avatar, Course, substitute identifier, Profile, or Watch fact.
    async fn blueprint_course_starred_instructors(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
    ) -> Result<Vec<BlueprintCourseStarredInstructor>, StoreError>;

    /// Reads only the caller's private Watch state.
    async fn blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
    ) -> Result<BlueprintCourseWatchProjection, StoreError>;

    /// Sets only the caller's private Watch state, then reads it.
    async fn set_current_blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
        watching: bool,
    ) -> Result<BlueprintCourseWatchProjection, StoreError>;

    /// Lists at most 100 immutable Watch events for this caller and lineage.
    ///
    /// No other watcher, recipient, count, or identity is exposed.
    async fn blueprint_course_watch_events(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
        limit: u16,
    ) -> Result<Vec<BlueprintCourseWatchEvent>, StoreError>;

    /// Reads only the caller's state for a Public or Archived Blueprint Course.
    ///
    /// The database rejects anonymous, inactive, and non-Instructor sessions,
    /// and no returned field permits discovery of another Instructor's Watch.
    async fn blueprint_stewardship_state(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
    ) -> Result<BlueprintStewardshipState, StoreError>;

    /// Sets only the authenticated Instructor's Star state, idempotently.
    async fn set_current_blueprint_course_star(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
        starred: bool,
    ) -> Result<BlueprintStewardshipState, StoreError>;

    /// Sets only the authenticated Instructor's private Watch state, idempotently.
    async fn set_current_blueprint_course_watch(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_reference: BlueprintCourseReference,
        watching: bool,
    ) -> Result<BlueprintStewardshipState, StoreError>;
}
