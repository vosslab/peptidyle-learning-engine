//! Connected Save oracle: new reusable Assessments append without replacing daughter work.

use super::*;

struct AppendPoolIds(AtomicUsize);

impl CourseInstancePoolIdIssuer for AppendPoolIds {
    fn issue_question_pool_id(&self) -> Result<QuestionId, StoreError> {
        let index = self.0.fetch_add(1, Ordering::SeqCst);
        format!("{}K3M-X8P1", index + 1)
            .parse()
            .map_err(|_| StoreError::InvalidRecord("append fixture Pool ID".into()))
    }
}

pub(super) async fn assert_new_assessment_save_preserves_daughter_work() {
    // Regression: adding a reusable Assessment must not overwrite customized
    // live Assessments or move a daughter's original adoption pin. Repair the
    // atomic Save/adoption boundary on failure, never relax those invariants.
    let mut inspection = adoption_inspection_connection().await;
    let url = std::env::var("DATABASE_URL").expect("application database URL");
    let application = lazy_pool(&url).expect("append application pool");
    let ids = Arc::new(AppendPoolIds(AtomicUsize::new(0)));
    let courses = PostgresCourseInstanceStore::new(application.clone())
        .with_question_pool_id_issuer(ids.clone());
    let store =
        PostgresBlueprintCourseStore::new(application.clone()).with_question_pool_id_issuer(ids);
    let created = store
        .create_blueprint_course(
            token(),
            question_model::RequestChecksum::from_bytes([0x61; 32]),
            content_input("Revision one Assessment"),
            Default::default(),
        )
        .await
        .expect("owner creates append fixture Blueprint through the application Store");
    let blueprint = created.blueprint_revision.reference;
    let blueprint_number = blueprint_reference_number(&blueprint).await;
    let private = store
        .load_blueprint_course(token(), blueprint.clone())
        .await
        .expect("owner loads Private Blueprint metadata");
    let initial = private.content.clone();
    transition_blueprint_availability(
        &url,
        &blueprint_number,
        private.blueprint_edit_number.as_i64(),
        "public",
        None,
    )
    .await
    .expect("publish fixture before adoption");
    let term = near_now_term(&url).await;
    let mut daughter_numbers = Vec::new();
    let mut empty_number = String::new();
    for index in 0..3 {
        let result = courses
            .create_course_instance(
                token(),
                CreateCourseInstanceInput {
                    classification: question_model::CourseClassification {
                        discipline_uuid: uuid::Uuid::from_u128(0xcc01),
                        subject_uuid: None,
                        topic_uuid: None,
                        subtopic_uuid: None,
                        tags: Vec::new(),
                    },
                    source: if index == 2 {
                        CourseInstanceCreationSource::Empty
                    } else {
                        CourseInstanceCreationSource::Adopted {
                            blueprint_course: blueprint.clone(),
                            blueprint_revision: BlueprintRevision::INITIAL,
                        }
                    },
                    short_name: format!("APPEND-{index}"),
                    long_name: format!("Append Save Course {index}"),
                    term: term.clone(),
                    assigned_instructor: None,
                },
                Default::default(),
            )
            .await
            .expect("create append fixture Course");
        let number = result.course.reference.as_string();
        if index == 2 {
            empty_number = number;
        } else {
            daughter_numbers.push(number);
        }
    }
    // A daughter owns its live title and release state independently of its source.
    sqlx::query(
        "UPDATE ple_data.assessment SET assessment_status = 'released', \
         assessment_policy_snapshot_id = ple_private.ensure_assessment_policy_snapshot( \
             'Daughter customization', snapshot.assessment_instructions, \
             snapshot.available_at, snapshot.due_at, snapshot.closes_at, \
             snapshot.assessment_attempt_time_limit_seconds, snapshot.assessment_attempt_limit, \
             snapshot.late_work_rule, snapshot.question_variation_rule, \
             snapshot.assessment_question_order_rule, snapshot.feedback_score, \
             snapshot.feedback_per_item_correctness, snapshot.feedback_submitted_response, \
             snapshot.feedback_question_answer, snapshot.feedback_question_answer_explanation, \
             snapshot.feedback_class_statistics, assessment.assessment_type) \
         FROM ple_data.assessment_policy_snapshot AS snapshot \
        WHERE snapshot.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id \
          AND assessment.course_instance_id = ANY($1)",
    )
    .bind(&daughter_numbers)
    .execute(&mut inspection)
    .await
    .expect("customize daughter fixture");
    sqlx::query(
        "UPDATE ple_data.course_instance SET course_lifecycle_state = 'inactive', \
        course_became_inactive_at = clock_timestamp() WHERE course_instance_id = $1",
    )
    .bind(&daughter_numbers[1])
    .execute(&mut inspection)
    .await
    .expect("Inactive daughter fixture");
    let mut student_fixture = inspection.begin().await.expect("daughter Student fixture");
    // ASVS 8.2.1: Student Records admit INSERT only through the explicit API-owner role.
    sqlx::query("SET LOCAL ROLE ple_api_owner")
        .execute(&mut *student_fixture)
        .await
        .expect("daughter Student fixture role");
    sqlx::query("INSERT INTO ple_data.student_record (student_record_id, course_instance_id, student_account_id, created_at) \
        SELECT $1, course_instance_id, $2, clock_timestamp() FROM ple_data.course_instance WHERE course_instance_id = $3")
        .bind(id(0xb220)).bind(student_account_id()).bind(&daughter_numbers[0])
        .execute(&mut *student_fixture).await.expect("daughter Student record");
    student_fixture
        .commit()
        .await
        .expect("daughter Student fixture commit");
    let mut work_fixture = inspection.begin().await.expect("daughter Work fixture");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *work_fixture)
        .await
        .expect("daughter Work fixture role");
    sqlx::query(
        "INSERT INTO ple_private.assessment_attempt ( \
        course_instance_id, assessment_attempt_id, student_record_id, assessment_id, \
        assessment_attempt_number, started_at, assessment_policy_snapshot_id) \
        SELECT a.course_instance_id, $1, $2, a.assessment_id, 1, clock_timestamp(), \
               a.assessment_policy_snapshot_id \
          FROM ple_data.assessment a \
         WHERE a.course_instance_id = $3 \
         LIMIT 1",
    )
    .bind(id(0xb221))
    .bind(id(0xb220))
    .bind(&daughter_numbers[0])
    .execute(&mut *work_fixture)
    .await
    .expect("existing daughter StudentWork attempt");
    work_fixture
        .commit()
        .await
        .expect("daughter Work fixture commit");
    let before = old_daughter_rows(&mut inspection, &daughter_numbers).await;
    let input = ReplaceBlueprintCourseContentInput {
        modules: vec![BlueprintModuleReplacementInput {
            choice: BlueprintModuleEditChoice::Retained {
                blueprint_module_reference: initial.modules[0].blueprint_module_reference,
            },
            label: "Module alpha".into(),
            assessments: vec![
                BlueprintAssessmentReplacementInput {
                    choice: BlueprintAssessmentEditChoice::Retained {
                        blueprint_assessment_reference: initial.modules[0].assessments[0]
                            .blueprint_assessment_reference,
                    },
                    content: retained_assessment_input(
                        &initial.modules[0].assessments[0].content,
                        "Changed existing source only",
                    ),
                },
                BlueprintAssessmentReplacementInput {
                    choice: BlueprintAssessmentEditChoice::New,
                    content: assessment_input("New mixed Fixed and Pool Assessment"),
                },
            ],
        }],
    };
    let checksum = question_model::RequestChecksum::from_bytes([0x62; 32]);
    let receipt = store
        .save_blueprint_course(
            token(),
            blueprint.clone(),
            BlueprintRevision::INITIAL,
            checksum,
            input.clone(),
            Default::default(),
        )
        .await
        .expect("normal Store Save appends new Assessment");
    assert!(receipt.changed);
    assert_eq!(
        receipt.blueprint_revision.revision,
        BlueprintRevision::new(2).expect("Revision two")
    );
    for (course, independent) in [
        (&daughter_numbers[0], &daughter_numbers[1]),
        (&daughter_numbers[1], &daughter_numbers[0]),
    ] {
        blueprint_course_postgres_adoption::assert_append_projection(
            &mut inspection,
            course,
            &blueprint_number,
            2,
            independent,
            1,
        )
        .await;
    }
    assert_eq!(
        old_daughter_rows(&mut inspection, &daughter_numbers).await,
        before,
        "retained live Assessment, release state, authored entries and Pool identity remain unchanged"
    );
    let replay = store
        .save_blueprint_course(
            token(),
            blueprint.clone(),
            BlueprintRevision::INITIAL,
            checksum,
            input.clone(),
            Default::default(),
        )
        .await
        .expect("changed Save replay");
    assert_eq!(replay, receipt);
    assert!(
        store
            .save_blueprint_course(
                token(),
                blueprint.clone(),
                BlueprintRevision::INITIAL,
                question_model::RequestChecksum::from_bytes([0x63; 32]),
                input,
                Default::default(),
            )
            .await
            .is_err(),
        "stale Save rejected"
    );
    let head = store
        .load_blueprint_revision(token(), receipt.blueprint_revision.clone())
        .await
        .expect("sealed new Revision");
    let no_op_input = ReplaceBlueprintCourseContentInput {
        modules: vec![BlueprintModuleReplacementInput {
            choice: BlueprintModuleEditChoice::Retained {
                blueprint_module_reference: head.content.modules[0].blueprint_module_reference,
            },
            label: "Module alpha".into(),
            assessments: head.content.modules[0]
                .assessments
                .iter()
                .enumerate()
                .map(|(index, assessment)| BlueprintAssessmentReplacementInput {
                    choice: BlueprintAssessmentEditChoice::Retained {
                        blueprint_assessment_reference: assessment.blueprint_assessment_reference,
                    },
                    content: retained_assessment_input(
                        &assessment.content,
                        if index == 0 {
                            "Changed existing source only"
                        } else {
                            "New mixed Fixed and Pool Assessment"
                        },
                    ),
                })
                .collect(),
        }],
    };
    let no_op = store
        .save_blueprint_course(
            token(),
            blueprint,
            receipt.blueprint_revision.revision,
            question_model::RequestChecksum::from_bytes([0x64; 32]),
            no_op_input,
            Default::default(),
        )
        .await
        .expect("no-op Save");
    assert!(!no_op.changed);
    let counts: Vec<(String, i64)> = sqlx::query_as("SELECT c.course_instance_id, count(a.assessment_id) \
        FROM ple_data.course_instance c LEFT JOIN ple_data.assessment a ON a.course_instance_id = c.course_instance_id \
        WHERE c.course_instance_id = ANY($1) GROUP BY c.course_instance_id ORDER BY c.course_instance_id")
        .bind(vec![daughter_numbers[0].clone(), daughter_numbers[1].clone(), empty_number.clone()])
        .fetch_all(&mut inspection).await.expect("append counts after replay/no-op/stale");
    assert_eq!(
        counts,
        vec![
            (daughter_numbers[0].clone(), 2),
            (daughter_numbers[1].clone(), 2),
            (empty_number.clone(), 0)
        ]
    );
    assert_eq!(
        old_daughter_rows(&mut inspection, &daughter_numbers).await,
        before
    );
    application.close().await;
    inspection.close().await.expect("append inspection close");
}

async fn old_daughter_rows(connection: &mut PgConnection, courses: &[String]) -> serde_json::Value {
    let mut rows: serde_json::Value = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(jsonb_build_object('assessment', to_jsonb(a), \
        'entries', (SELECT jsonb_agg(to_jsonb(e) ORDER BY e.authored_position) FROM ple_data.assessment_entry e \
        WHERE e.assessment_id = a.assessment_id)) ORDER BY a.assessment_id), '[]'::jsonb) \
        FROM ple_data.assessment a JOIN ple_data.course_instance c ON c.course_instance_id = a.course_instance_id \
        WHERE c.course_instance_id = ANY($1) AND a.source_blueprint_revision_number = 1")
        .bind(courses).fetch_one(&mut *connection).await.expect("unchanged daughter snapshot");
    let mut work = connection.begin().await.expect("daughter Work snapshot");
    sqlx::query("SET LOCAL ROLE ple_private_owner")
        .execute(&mut *work)
        .await
        .expect("daughter Work snapshot role");
    for row in rows.as_array_mut().expect("daughter snapshot array") {
        let assessment = row["assessment"]["assessment_id"]
            .as_str()
            .expect("daughter Assessment ID");
        row["attempts"] = sqlx::query_scalar::<_, Option<serde_json::Value>>(
            "SELECT jsonb_agg(to_jsonb(w) ORDER BY w.assessment_attempt_id) \
             FROM ple_private.assessment_attempt w WHERE w.assessment_id = $1",
        )
        .bind(assessment)
        .fetch_one(&mut *work)
        .await
        .expect("daughter Work snapshot rows")
        .unwrap_or(serde_json::Value::Null);
    }
    work.commit().await.expect("daughter Work snapshot commit");
    rows
}
