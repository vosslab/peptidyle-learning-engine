//! Connected proof that the Blueprint-scoped Pool reader returns Pool-owned eligibility.

use super::*;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn blueprint_pool_members_keep_pool_eligibility_scoped_and_stable() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let admin = lazy_pool(runtime.migration_url().expose()).expect("migration pool");
    seed_if_needed(&admin).await;
    let application = lazy_pool(&std::env::var("DATABASE_URL").expect("application URL"))
        .expect("application pool");
    let store = PostgresBlueprintCourseStore::new(application.clone())
        .with_question_pool_id_issuer(Arc::new(FixturePoolIdIssuer(AtomicUsize::new(0))));
    let created = store
        .create_blueprint_course(
            token(),
            RequestChecksum::from_bytes([0x71; 32]),
            content_input("Scoped Pool eligibility"),
            Default::default(),
        )
        .await
        .expect("create Blueprint with imported Pool");
    let blueprint_id = created.blueprint_revision_tuple.blueprint_course_id;
    let loaded = store
        .load_blueprint_course(token(), blueprint_id.clone())
        .await
        .expect("load Blueprint content");
    let assessment = &loaded.content.modules[0].assessments[0];
    let (pool_id, _) = assessment
        .content
        .entries
        .iter()
        .find_map(|entry| match entry {
            learning_data_access::StoredBlueprintAssessmentEntry::Pool {
                question_pool_id,
                question_pool_edit_number,
                ..
            } => Some((question_pool_id.clone(), *question_pool_edit_number)),
            learning_data_access::StoredBlueprintAssessmentEntry::Fixed { .. } => None,
        })
        .expect("imported Blueprint Pool");
    let first = store
        .load_blueprint_pool_members(
            token(),
            blueprint_id.clone(),
            assessment.blueprint_assessment_id,
            pool_id.clone(),
        )
        .await
        .expect("scoped Pool members");
    assert_eq!(first.question_pool_id, pool_id);
    assert_eq!(first.discipline_uuid, id(0xcc01));
    assert_eq!(first.subject_uuid, id(0xcc02));
    assert_eq!(
        first.question_type,
        question_model::QuestionType::MultipleChoice
    );
    assert_eq!(first.backend, question_model::QuestionBackend::Ple);
    assert_eq!(first.members.len(), 1);

    let mut change = admin
        .begin()
        .await
        .expect("Question reclassification transaction");
    sqlx::query("SET LOCAL ROLE ple_data_owner")
        .execute(&mut *change)
        .await
        .expect("Question reclassification owner");
    sqlx::query(
        "INSERT INTO ple_data.content_subject (content_subject_id, name) \
         VALUES ($1, 'Reclassified Question Subject')",
    )
    .bind(id(0xcc03))
    .execute(&mut *change)
    .await
    .expect("reclassified Question Subject");
    sqlx::query(
        "INSERT INTO ple_data.content_subject_discipline \
         (content_subject_id, content_discipline_id) VALUES ($1, $2)",
    )
    .bind(id(0xcc03))
    .bind(id(0xcc01))
    .execute(&mut *change)
    .await
    .expect("reclassified Question Subject Discipline");
    sqlx::query(
        "UPDATE ple_data.published_question_metadata \
         SET content_subject_id = $1 WHERE published_question_id = $2",
    )
    .bind(id(0xcc03))
    .bind(QUESTION)
    .execute(&mut *change)
    .await
    .expect("reclassify Pool member Question");
    change
        .commit()
        .await
        .expect("Question reclassification commit");

    let after_reclassification = store
        .load_blueprint_pool_members(
            token(),
            blueprint_id.clone(),
            assessment.blueprint_assessment_id,
            pool_id.clone(),
        )
        .await
        .expect("scoped Pool members after Question reclassification");
    assert_eq!(
        after_reclassification.discipline_uuid,
        first.discipline_uuid
    );
    assert_eq!(after_reclassification.subject_uuid, first.subject_uuid);
    assert_eq!(after_reclassification.question_type, first.question_type);
    assert_eq!(after_reclassification.backend, first.backend);

    let unrelated_pool = question_model::QuestionPoolId::from_random_identifier("P300001")
        .expect("unrelated Pool ID");
    assert!(matches!(
        store
            .load_blueprint_pool_members(
                token(),
                blueprint_id,
                assessment.blueprint_assessment_id,
                unrelated_pool,
            )
            .await,
        Err(StoreError::Forbidden | StoreError::NotFound)
    ));
    application.close().await;
    admin.close().await;
}
