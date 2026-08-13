//! Course-scoped instructor or sysadmin authorization for retention routes.

use axum::http::StatusCode;
use axum::response::Response;
use learning_data_access::{Store, StoreError};
use question_model::{CourseId, CourseMembershipRole, UserRole};

use crate::auth::AuthenticatedSession;

use super::projection::{error_response, route_store_error};

#[derive(Debug, Clone, Copy)]
pub(super) struct CourseRetentionAuthority {
    pub(super) is_sysadmin: bool,
}

pub(super) async fn require_course_retention_authority<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> Result<CourseRetentionAuthority, Response>
where
    S: Store,
{
    let user = authenticated.record.subject.user();
    let roles = authenticated.record.subject.roles();
    let is_sysadmin = roles.contains(&UserRole::Sysadmin);
    let course_record = match store.get_course(authenticated.tenant_context, course).await {
        Ok(None)
        | Err(StoreError::Forbidden | StoreError::TenantMismatch | StoreError::NotFound) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "course does not authorize this action",
            ));
        }
        Ok(Some(record)) => record,
        Err(error) => return Err(route_store_error(error)),
    };
    let course_role = course_record.role_for(user);
    if !(is_sysadmin || matches!(course_role, Some(CourseMembershipRole::Instructor))) {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "course does not authorize this action",
        ));
    }
    Ok(CourseRetentionAuthority { is_sysadmin })
}
