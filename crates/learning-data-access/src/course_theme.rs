//! Session-authorized current Course Theme persistence.

use async_trait::async_trait;
use question_model::{CourseId, CourseTheme};

use crate::{SessionTokenHash, StoreError};

/// Persistence boundary for the scalar Course Theme portion of Course Appearance.
#[async_trait]
pub trait CourseThemeStore: Send + Sync {
    /// Reads the current theme only when the current Account is an active Course Member.
    async fn read_course_theme(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
    ) -> Result<CourseTheme, StoreError>;

    /// Replaces the current theme only when the current Account is an active Instructor Course Member.
    async fn update_course_theme(
        &self,
        session_token_hash: SessionTokenHash,
        course: CourseId,
        theme: CourseTheme,
    ) -> Result<CourseTheme, StoreError>;
}
