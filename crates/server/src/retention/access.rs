//! Course-scoped manager authorization for retention routes.

use axum::http::StatusCode;
use axum::response::Response;
use learning_data_access::{Store, StoreError};
use question_model::{CourseId, CourseRole, UserRole};

use crate::auth::AuthenticatedSession;

use super::projection::{error_response, route_store_error};

#[derive(Debug, Clone, Copy)]
pub(super) struct CourseManagerAccess {
    pub(super) is_admin: bool,
}

pub(super) async fn require_course_manager<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> Result<CourseManagerAccess, Response>
where
    S: Store,
{
    let user = authenticated.record.subject.user();
    let roles = authenticated.record.subject.roles();
    let is_platform_admin = roles.contains(&UserRole::Administrator);
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
    if !(is_platform_admin
        || matches!(
            course_role,
            Some(CourseRole::Instructor | CourseRole::Administrator)
        ))
    {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "course does not authorize this action",
        ));
    }
    Ok(CourseManagerAccess {
        is_admin: is_platform_admin || matches!(course_role, Some(CourseRole::Administrator)),
    })
}
