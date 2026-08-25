//! Course-scoped instructor or sysadmin authorization for retention routes.

use axum::http::StatusCode;
use learning_data_access::{Store, StoreError};
use question_model::{CourseId, CourseMembershipRole, UserRole};

use crate::auth::AuthenticatedSession;
use crate::http_refusal::HttpResult;

use super::projection::{error_response, route_store_error};

#[derive(Debug, Clone, Copy)]
pub(super) struct CourseRetentionAuthority {
    pub(super) is_sysadmin: bool,
}

pub(super) async fn require_course_retention_authority<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> HttpResult<CourseRetentionAuthority>
where
    S: Store,
{
    let user = authenticated.record.subject.user();
    let roles = authenticated.record.subject.roles();
    let is_sysadmin = roles.contains(&UserRole::Sysadmin);
    let membership = match store
        .get_current_course_membership(authenticated.tenant_context, course, user)
        .await
    {
        Ok(membership) => membership,
        Err(StoreError::Forbidden | StoreError::TenantMismatch | StoreError::NotFound) => None,
        Err(error) => return Err(route_store_error(error).into()),
    };
    if !(is_sysadmin
        || matches!(
            membership.as_ref().map(|membership| membership.role),
            Some(CourseMembershipRole::Instructor)
        ))
    {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "course does not authorize this action",
        )
        .into());
    }
    Ok(CourseRetentionAuthority { is_sysadmin })
}
