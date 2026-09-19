//! Promotion authorization and metadata-CAS checks within the connected lifecycle oracle.

use super::*;
use learning_data_access::BlueprintPromotionStore;

pub(super) fn discovery(
    include_archived: bool,
) -> learning_data_access::BlueprintCourseListRequest {
    learning_data_access::BlueprintCourseListRequest {
        page: learning_data_access::PageRequest::first(
            learning_data_access::PageSize::new(100).expect("bounded fixture discovery"),
        ),
        query: String::new(),
        include_archived,
        public_only: false,
        promoted_only: false,
        discipline_uuid: None,
        subject_uuid: None,
        topic_uuid: None,
        subtopic_uuid: None,
        cross_discipline: false,
    }
}

// Promotion authorization and CAS earn permanent protection; exercise them inside
// the connected lifecycle oracle rather than constructing another full fixture.
pub(super) async fn promotion_boundary(
    store: &PostgresBlueprintCourseStore,
    blueprint_course_id: BlueprintCourseId,
) {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    let sysadmin = SessionTokenHash::compute(&[0xb5; 32]);
    let student = SessionTokenHash::compute(&[0xb6; 32]);
    let mut transaction = admin.begin().await.expect("promotion fixture transaction");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *transaction)
        .await
        .expect("private owner");
    let sysadmin_id: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, product_role, created_at) \
         VALUES ('U00000009', 'sysadmin', clock_timestamp()) RETURNING account_id",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("Sysadmin fixture");
    sqlx::query(
        "INSERT INTO ple_private.authenticated_session \
         (session_id, account_id, product_role, token_hash, created_at, expires_at) \
         VALUES ($1, $2, 'sysadmin', decode($3, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour'), \
                ($4, $5, 'student', decode($6, 'hex'), clock_timestamp(), \
                 clock_timestamp() + interval '1 hour')",
    )
    .bind(id(0xb106))
    .bind(&sysadmin_id)
    .bind(sysadmin.to_string())
    .bind(id(0xb107))
    .bind(student_account_id())
    .bind(student.to_string())
    .execute(&mut *transaction)
    .await
    .expect("promotion role session");
    transaction.commit().await.expect("fixture commit");
    admin.close().await;
    let initial = store
        .load_blueprint_promotion(sysadmin, blueprint_course_id.clone())
        .await
        .expect("Sysadmin reads promotion");
    assert!(!initial.promoted, "new lineage defaults unpromoted");
    let head = store
        .load_blueprint_course(token(), blueprint_course_id.clone())
        .await
        .expect("owner head")
        .current_revision;
    for actor in [token(), student] {
        assert!(matches!(
            store
                .load_blueprint_promotion(actor, blueprint_course_id.clone())
                .await,
            Err(StoreError::NotFound | StoreError::Forbidden)
        ));
        assert!(matches!(
            store
                .set_blueprint_promotion(
                    actor,
                    blueprint_course_id.clone(),
                    initial.blueprint_edit_number,
                    true
                )
                .await,
            Err(StoreError::Forbidden)
        ));
    }
    let mut filter = discovery(false);
    filter.promoted_only = true;
    assert!(
        store
            .list_blueprint_courses(token(), filter.clone())
            .await
            .expect("unpromoted filtering")
            .items
            .iter()
            .all(|item| item.id != blueprint_course_id)
    );
    let promoted = store
        .set_blueprint_promotion(
            sysadmin,
            blueprint_course_id.clone(),
            initial.blueprint_edit_number,
            true,
        )
        .await
        .expect("Sysadmin promotes");
    assert!(promoted.promoted);
    assert_ne!(
        promoted.blueprint_edit_number,
        initial.blueprint_edit_number
    );
    assert!(matches!(
        store
            .set_blueprint_promotion(
                sysadmin,
                blueprint_course_id.clone(),
                initial.blueprint_edit_number,
                false
            )
            .await,
        Err(StoreError::RetryableTransaction | StoreError::Conflict)
    ));
    assert_eq!(
        store
            .set_blueprint_promotion(
                sysadmin,
                blueprint_course_id.clone(),
                promoted.blueprint_edit_number,
                true
            )
            .await
            .expect("no-op promotion"),
        promoted
    );
    assert!(
        store
            .list_blueprint_courses(token(), filter.clone())
            .await
            .expect("owner promoted filtering")
            .items
            .iter()
            .any(|item| item.id == blueprint_course_id)
    );
    assert!(
        store
            .list_blueprint_courses(reader_token(), filter)
            .await
            .expect("promotion does not expose Private lineage")
            .items
            .iter()
            .all(|item| item.id != blueprint_course_id)
    );
    assert_eq!(
        store
            .load_blueprint_course(token(), blueprint_course_id.clone())
            .await
            .expect("unchanged Revision")
            .current_revision,
        head
    );
    store
        .set_blueprint_promotion(
            sysadmin,
            blueprint_course_id,
            promoted.blueprint_edit_number,
            false,
        )
        .await
        .expect("restore fixture flag");
}
