use axum::http::StatusCode;
use learning_data_access::{CourseCreationAuthority, CourseRecordsAccessStore, Store, StoreError};
use question_model::{CourseId, CourseMembershipRole, UserRole};

use crate::auth::AuthenticatedSession;
use crate::http_refusal::HttpResult;

use super::projection::{error_response, store_error_response};

pub(super) async fn require_course_access<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
    manage: bool,
) -> HttpResult<()>
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
        Ok(None) => return Err(error_response(StatusCode::NOT_FOUND, "course not found").into()),
        Err(error) => return Err(store_error_response(error).into()),
    };
    match membership.role {
        CourseMembershipRole::Instructor => Ok(()),
        CourseMembershipRole::Student => {
            match course_records_are_visible(store, authenticated, course).await {
                Ok(true) => {}
                Ok(false) => {
                    return Err(error_response(StatusCode::NOT_FOUND, "course not found").into());
                }
                Err(response) => return Err(response),
            }
            if manage {
                Err(
                    error_response(StatusCode::FORBIDDEN, "assignment change is not authorized")
                        .into(),
                )
            } else {
                Ok(())
            }
        }
    }
}

/// Requires current direct Instructor authority while concealing course records.
pub(super) async fn require_direct_instructor_course<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> HttpResult<()>
where
    S: Store + CourseRecordsAccessStore,
{
    if authenticated.record.subject.role() != UserRole::Instructor {
        return Err(error_response(StatusCode::NOT_FOUND, "course record not found").into());
    }
    match store
        .get_current_course_membership(
            authenticated.tenant_context,
            course,
            authenticated.record.subject.user(),
        )
        .await
    {
        Ok(Some(membership)) if membership.role == CourseMembershipRole::Instructor => {
            match store.course_records_accessible(course).await {
                Ok(true) => Ok(()),
                Ok(false) | Err(StoreError::NotFound | StoreError::Forbidden) => {
                    Err(error_response(StatusCode::NOT_FOUND, "course record not found").into())
                }
                Err(error) => Err(store_error_response(error).into()),
            }
        }
        Ok(Some(_)) | Ok(None) | Err(StoreError::NotFound | StoreError::Forbidden) => {
            Err(error_response(StatusCode::NOT_FOUND, "course record not found").into())
        }
        Err(error) => Err(store_error_response(error).into()),
    }
}

pub(super) async fn course_records_are_visible<S>(
    store: &S,
    authenticated: &AuthenticatedSession,
    course: CourseId,
) -> HttpResult<bool>
where
    S: CourseRecordsAccessStore,
{
    match store.course_records_accessible(course).await {
        Ok(accessible) => Ok(accessible),
        Err(error) => Err(store_error_response(error).into()),
    }
}

/// Derives the closed provisioning authority from the resolved session.
///
/// ASVS 2.2.2: the service derives every authorization input from the
/// authenticated session rather than accepting actor or role claims from the
/// course-create request body.
pub(super) fn course_creation_authority(
    authenticated: &AuthenticatedSession,
) -> Option<CourseCreationAuthority> {
    let actor = authenticated.record.subject.user();
    let session = authenticated.session_hash;
    match authenticated.record.subject.role() {
        UserRole::Sysadmin => Some(CourseCreationAuthority::Sysadmin { actor, session }),
        UserRole::Instructor => {
            Some(CourseCreationAuthority::ApprovedInstructor { actor, session })
        }
        UserRole::Student => None,
    }
}
