//! Owner-only Private Blueprint discovery for the connected lifecycle oracle.

use super::*;

pub(super) async fn assert_private_blueprint_is_owner_only(
    owner_store: &PostgresBlueprintCourseStore,
    reader_store: &PostgresBlueprintCourseStore,
    blueprint_reference: &BlueprintCourseId,
) {
    assert!(
        owner_store
            .list_blueprint_courses(token(), discovery(false))
            .await
            .expect("owner Private Blueprint list")
            .items
            .iter()
            .any(|summary| summary.id == *blueprint_reference),
        "owner's Private Blueprint remains in normal discovery"
    );
    assert!(
        reader_store
            .list_blueprint_courses(reader_token(), discovery(true))
            .await
            .expect("non-owner discovery including Archived")
            .items
            .iter()
            .all(|summary| summary.id != *blueprint_reference),
        "including Archived never discloses another owner's Private Blueprint"
    );
    assert!(
        reader_store
            .list_blueprint_courses(reader_token(), discovery(false))
            .await
            .expect("non-owner Private Blueprint list")
            .items
            .iter()
            .all(|summary| summary.id != *blueprint_reference),
        "Private Blueprint is absent from non-owner discovery"
    );
    assert!(matches!(
        reader_store
            .load_blueprint_course(reader_token(), blueprint_reference.clone())
            .await,
        Err(StoreError::NotFound)
    ));
    assert!(matches!(
        reader_store
            .load_blueprint_revision(
                reader_token(),
                question_model::BlueprintRevisionReference {
                    blueprint_course_id: blueprint_reference.clone(),
                    revision: BlueprintRevision::INITIAL,
                },
            )
            .await,
        Err(StoreError::NotFound)
    ));
}
