use axum::http::StatusCode;
use axum::response::Response;
use learning_data_access::{CourseRecordsAccessStore, Store};
use question_model::{CourseId, CourseMembershipRole, UserRole};

use crate::auth::AuthenticatedSession;

use super::projection::{error_response, store_error_response};

pub(super) async fn require_course_access<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
    manage: bool,
) -> Result<(), Response>
where
    S: Store + CourseRecordsAccessStore,
{
    let membership = match store
        .get_current_course_membership(
            authenticated.tenant_context,
            course,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(Some(membership)) => membership,
        Ok(None) => return Err(error_response(StatusCode::NOT_FOUND, "course not found")),
        Err(error) => return Err(store_error_response(error)),
    };
    match membership.role {
        CourseMembershipRole::Instructor => Ok(()),
        CourseMembershipRole::Student => {
            match course_records_are_visible(store, authenticated, course).await {
                Ok(true) => {}
                Ok(false) => {
                    return Err(error_response(StatusCode::NOT_FOUND, "course not found"));
                }
                Err(response) => return Err(response),
            }
            if manage {
                Err(error_response(
                    StatusCode::FORBIDDEN,
                    "assignment change is not authorized",
                ))
            } else {
                Ok(())
            }
        }
    }
}

pub(super) async fn course_records_are_visible<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> Result<bool, Response>
where
    S: CourseRecordsAccessStore,
{
    match store
        .course_records_accessible(authenticated.tenant_context, course)
        .await
    {
        Ok(accessible) => Ok(accessible),
        Err(error) => Err(store_error_response(error)),
    }
}

pub(super) fn may_create_course(authenticated: &AuthenticatedSession) -> bool {
    authenticated
        .record
        .subject
        .roles()
        .iter()
        .any(|role| matches!(role, UserRole::Instructor | UserRole::Sysadmin))
}
