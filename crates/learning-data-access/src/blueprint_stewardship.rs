//! Authenticated Instructor stewardship state for one Blueprint Course lineage.
//!
//! A Blueprint Course ID, rather than a Revision Tuple, is the
//! durable identity. The closed store surface exposes only the caller's own
//! Star and private Watch state, the public Star aggregate, and the
//! caller's own immutable Watch-event records.

use async_trait::async_trait;
use question_model::{BlueprintCourseId, Timestamp};
use uuid::Uuid;

use crate::{NumberedPage, NumberedPageRequest, PageSize, SessionTokenHash, StoreError};

/// Browser-safe Star facts for one Blueprint Course lineage.
///
/// This deliberately has no endorser identity. C856 adds the separately
/// authorized active-Instructor display-name projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintCourseStarProjection {
    pub viewer_has_starred: bool,
    pub star_count: u64,
}

/// One public identity in the separately authorized Blueprint Star list.
///
/// This intentionally carries only the active Instructor's display name. It is
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

/// The three immutable source changes that can notify a Blueprint watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseWatchEventKind {
    Revision,
    Archived,
    Restored,
}

/// One recipient-owned, immutable Blueprint Course Watch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintCourseWatchEvent {
    pub event_id: Uuid,
    pub kind: BlueprintCourseWatchEventKind,
    pub occurred_at: Timestamp,
}

/// Chronological ordering for the caller's private Watch history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintCourseWatchDirection {
    Ascending,
    Descending,
}

/// Complete query for one recipient's activity on one Blueprint lineage.
#[derive(Debug, Clone)]
pub struct BlueprintCourseWatchEventsQuery {
    pub search: String,
    pub event_kinds: Vec<BlueprintCourseWatchEventKind>,
    pub direction: BlueprintCourseWatchDirection,
    pub page: NumberedPageRequest,
}

/// One activity-kind count before the query's selected-kind filter is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintCourseWatchEventFacet {
    pub value: BlueprintCourseWatchEventKind,
    pub count: u64,
}

/// A numbered private page and its full-query, cleared-kind facet counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintCourseWatchEventsPage {
    pub page: NumberedPage<BlueprintCourseWatchEvent, PageSize>,
    pub event_kind_facets: Vec<BlueprintCourseWatchEventFacet>,
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
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseStarProjection, StoreError>;

    /// Sets only the caller's Star state, then reads the closed Star projection.
    async fn set_current_blueprint_course_star_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        starred: bool,
    ) -> Result<BlueprintCourseStarProjection, StoreError>;

    /// Lists only active Instructors' display names for one
    /// Active or Archived Blueprint Course. It returns no Account, email,
    /// avatar, Course, substitute identifier, Profile, or Watch fact.
    async fn blueprint_course_starred_instructors(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<Vec<BlueprintCourseStarredInstructor>, StoreError>;

    /// Reads only the caller's private Watch state.
    async fn blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<BlueprintCourseWatchProjection, StoreError>;

    /// Sets only the caller's private Watch state, then reads it.
    async fn set_current_blueprint_course_watch_projection(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        watching: bool,
    ) -> Result<BlueprintCourseWatchProjection, StoreError>;

    /// Lists numbered immutable Watch events for this caller and lineage.
    ///
    /// Counts and facets cover only this caller and lineage, including history
    /// retained after Unwatch. No other watcher, recipient, or identity is exposed.
    async fn blueprint_course_watch_events(
        &self,
        session_token_hash: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        query: BlueprintCourseWatchEventsQuery,
    ) -> Result<BlueprintCourseWatchEventsPage, StoreError>;
}
