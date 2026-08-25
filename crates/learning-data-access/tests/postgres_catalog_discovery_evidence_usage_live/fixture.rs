use sqlx::PgPool;
use uuid::Uuid;

use super::id;

pub(super) struct Fixture {
    pub(super) tenant: Uuid,
    pub(super) actor: Uuid,
    pub(super) sysadmin: Uuid,
    pub(super) question_id: &'static str,
    pub(super) problem: Uuid,
    pub(super) version: Uuid,
    pub(super) replacement_problem: Uuid,
    pub(super) replacement_version: Uuid,
    pub(super) actor_course: Uuid,
    pub(super) foreign_course: Uuid,
    pub(super) actor_session: String,
    pub(super) sysadmin_session: String,
    pub(super) unapproved_instructor_session: String,
    pub(super) foreign_session: String,
    pub(super) observations: Vec<Observation>,
    pub(super) ineligible_attempt: Uuid,
    pub(super) later_attempt: Uuid,
}

#[derive(Clone, Copy)]
pub(super) struct Observation {
    pub(super) tenant: Uuid,
    pub(super) enrollment: Uuid,
    pub(super) run: Uuid,
    pub(super) attempt: Uuid,
}

pub(super) async fn seed(pool: &PgPool) -> Fixture {
    let tenant = id();
    let tenant_two = id();
    let actor = id();
    let sysadmin = id();
    let unapproved_instructor = id();
    let foreign_actor = id();
    let problem = id();
    let version = id();
    let replacement_problem = id();
    let replacement_version = id();
    let actor_course = id();
    let foreign_course = id();
    let actor_assignment = id();
    let foreign_assignment = id();
    let tenant_two_course = id();
    let tenant_two_assignment = id();
    let actor_session = "a".repeat(64);
    let sysadmin_session = "c".repeat(64);
    let unapproved_instructor_session = "d".repeat(64);
    let foreign_session = "b".repeat(64);

    let mut catalog = pool.begin().await.expect("begin catalog fixture");
    for (user, name) in [(actor, "D1 Actor"), (foreign_actor, "D1 Foreign")] {
        let email = format!("d1-{user}@example.test");
        sqlx::query(
            "INSERT INTO ple_account (user_id,normalized_email,delivery_email,display_name) \
             VALUES ($1,$2,$2,$3)",
        )
        .bind(user)
        .bind(&email)
        .bind(name)
        .execute(&mut *catalog)
        .await
        .expect("insert instructor account");
        sqlx::query(
            "INSERT INTO instructor_approval \
             (user_id,approved_by,approved_at,revision) \
             VALUES ($1,$1,transaction_timestamp(),1)",
        )
        .bind(user)
        .execute(&mut *catalog)
        .await
        .expect("approve instructor");
    }
    let sysadmin_email = format!("d1-{sysadmin}@example.test");
    sqlx::query(
        "INSERT INTO ple_account (user_id,normalized_email,delivery_email,display_name) \
         VALUES ($1,$2,$2,'Morgan Sysadmin')",
    )
    .bind(sysadmin)
    .bind(&sysadmin_email)
    .execute(&mut *catalog)
    .await
    .expect("insert Morgan Sysadmin account");
    let unapproved_email = format!("d1-{unapproved_instructor}@example.test");
    sqlx::query(
        "INSERT INTO ple_account (user_id,normalized_email,delivery_email,display_name) \
         VALUES ($1,$2,$2,'D1 Unapproved Instructor')",
    )
    .bind(unapproved_instructor)
    .bind(&unapproved_email)
    .execute(&mut *catalog)
    .await
    .expect("insert unapproved Instructor account");
    sqlx::query(
        "INSERT INTO problem \
         (problem_id,owner_tenant_id,owner_user_id,visibility,license,lifecycle,question_id) \
         VALUES ($1,$2,$3,'public','CC-BY-4.0','published','D1A0001')",
    )
    .bind(problem)
    .bind(tenant)
    .bind(actor)
    .execute(&mut *catalog)
    .await
    .expect("insert discovery problem");
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id,version_id,content_sha256,workspace_id,title,lifecycle,backend, \
          publication_scope,author_ids,public_byline,response_family) \
         VALUES ($1,$2,$3,$4,'D1 evidence oracle','published','native','public', \
                 jsonb_build_array($5::text),ARRAY['D1 Oracle'],'multipleChoice')",
    )
    .bind(problem)
    .bind(version)
    .bind("d".repeat(64))
    .bind(id())
    .bind(actor)
    .execute(&mut *catalog)
    .await
    .expect("publish discovery problem");
    sqlx::query(
        "INSERT INTO problem_version_payload(problem_id,version_id,payload,payload_sha256) \
         VALUES ($1,$2, \
           '{\"question\":{\"response\":{\"kind\":\"multipleChoice\"}}}'::jsonb,$3)",
    )
    .bind(problem)
    .bind(version)
    .bind("d".repeat(64))
    .execute(&mut *catalog)
    .await
    .expect("insert immutable response-family source");
    sqlx::query(
        "INSERT INTO problem \
         (problem_id,owner_tenant_id,owner_user_id,visibility,license,lifecycle,question_id) \
         VALUES ($1,$2,$3,'public','CC-BY-4.0','published','D1A0002')",
    )
    .bind(replacement_problem)
    .bind(tenant)
    .bind(actor)
    .execute(&mut *catalog)
    .await
    .expect("insert replacement identity");
    sqlx::query(
        "INSERT INTO problem_version \
         (problem_id,version_id,content_sha256,workspace_id,title,lifecycle,backend, \
          publication_scope,author_ids,public_byline,response_family, \
          derived_from_problem_id,derived_from_version_id) \
         VALUES ($1,$2,$3,$4,'D1 explicit replacement','published','native','public', \
                 jsonb_build_array($5::text),ARRAY['D1 Oracle'],'multipleChoice',$6,$7)",
    )
    .bind(replacement_problem)
    .bind(replacement_version)
    .bind("e".repeat(64))
    .bind(id())
    .bind(actor)
    .bind(problem)
    .bind(version)
    .execute(&mut *catalog)
    .await
    .expect("publish explicitly linked replacement version");
    sqlx::query(
        "INSERT INTO problem_version_payload(problem_id,version_id,payload,payload_sha256) \
         VALUES ($1,$2,'{\"question\":{\"response\":{\"kind\":\"multipleChoice\"}}}'::jsonb,$3)",
    )
    .bind(replacement_problem)
    .bind(replacement_version)
    .bind("e".repeat(64))
    .execute(&mut *catalog)
    .await
    .expect("insert replacement immutable payload");
    catalog.commit().await.expect("commit catalog fixture");

    let mut fixture = pool.begin().await.expect("begin activity fixture");
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *fixture)
        .await
        .expect("disable unrelated fixture triggers");
    for (course_tenant, course, title) in [
        (tenant, actor_course, "Actor-owned D1 course"),
        (tenant, foreign_course, "Foreign instructor secret course"),
        (
            tenant_two,
            tenant_two_course,
            "Second tenant evidence course",
        ),
    ] {
        sqlx::query(
            "INSERT INTO course \
             (tenant_id,course_id,title,term_start_date,term_end_date,time_zone) \
             VALUES ($1,$2,$3,DATE '2026-08-01',DATE '2026-12-31','America/Chicago')",
        )
        .bind(course_tenant)
        .bind(course)
        .bind(title)
        .execute(&mut *fixture)
        .await
        .expect("insert course");
    }
    for (course, user) in [
        (actor_course, actor),
        (actor_course, foreign_actor),
        (foreign_course, foreign_actor),
    ] {
        sqlx::query(
            "INSERT INTO course_member \
             (tenant_id,course_id,course_membership_id,user_id,role,status,joined_at) \
             VALUES ($1,$2,$3,$4,'instructor','active',transaction_timestamp())",
        )
        .bind(tenant)
        .bind(course)
        .bind(id())
        .bind(user)
        .execute(&mut *fixture)
        .await
        .expect("insert instructor membership");
    }
    for (session, user, name, roles) in [
        (&actor_session, actor, "D1 Actor", r#"["instructor"]"#),
        (
            &sysadmin_session,
            sysadmin,
            "Morgan Sysadmin",
            r#"["sysadmin"]"#,
        ),
        (
            &unapproved_instructor_session,
            unapproved_instructor,
            "D1 Unapproved Instructor",
            r#"["instructor"]"#,
        ),
        (
            &foreign_session,
            foreign_actor,
            "D1 Foreign",
            r#"["instructor"]"#,
        ),
    ] {
        sqlx::query(
            "INSERT INTO auth_session \
             (session_hash,tenant_id,user_id,display_name,roles,expires_at) \
             VALUES ($1,$2,$3,$4,$5::jsonb, \
                     transaction_timestamp() + interval '1 hour')",
        )
        .bind(session)
        .bind(tenant)
        .bind(user)
        .bind(name)
        .bind(roles)
        .execute(&mut *fixture)
        .await
        .expect("insert catalog actor session");
    }
    for (assignment_tenant, assignment, course, title) in [
        (tenant, actor_assignment, actor_course, "Actor assignment"),
        (
            tenant,
            foreign_assignment,
            foreign_course,
            "Foreign assignment",
        ),
        (
            tenant_two,
            tenant_two_assignment,
            tenant_two_course,
            "Second tenant assignment",
        ),
    ] {
        sqlx::query(
            "INSERT INTO assignment \
             (tenant_id,assignment_id,course_id,title,lifecycle,audience_kind, \
              score_disclosure,per_item_correctness_disclosure,feedback_text_disclosure, \
              solution_disclosure,class_statistics_disclosure) \
             VALUES ($1,$2,$3,$4,'published','course_wide','after_submit', \
                     'after_submit','after_submit','after_submit','never')",
        )
        .bind(assignment_tenant)
        .bind(assignment)
        .bind(course)
        .bind(title)
        .execute(&mut *fixture)
        .await
        .expect("insert assignment");
        sqlx::query(
            "INSERT INTO assignment_item \
             (tenant_id,assignment_id,assignment_item_id,position,problem_id,version_id, \
              points_possible,delivery_state,scoring_mode) \
             VALUES ($1,$2,$3,0,$4,$5,1,'active','normal')",
        )
        .bind(assignment_tenant)
        .bind(assignment)
        .bind(id())
        .bind(problem)
        .bind(version)
        .execute(&mut *fixture)
        .await
        .expect("insert current fixed usage");
    }
    let selection_group = id();
    sqlx::query(
        "INSERT INTO assignment_selection_group \
         (tenant_id,assignment_id,selection_group_id,position,draw_count,points_per_item, \
          ordering_policy,algorithm_version) \
         VALUES ($1,$2,$3,1,1,1,'candidate_order',1)",
    )
    .bind(tenant)
    .bind(actor_assignment)
    .bind(selection_group)
    .execute(&mut *fixture)
    .await
    .expect("insert selection group");
    sqlx::query(
        "INSERT INTO assignment_selection_candidate \
         (tenant_id,assignment_id,selection_group_id,candidate_id,position,problem_id, \
          version_id,delivery_state) VALUES ($1,$2,$3,$4,0,$5,$6,'active')",
    )
    .bind(tenant)
    .bind(actor_assignment)
    .bind(selection_group)
    .bind(id())
    .bind(problem)
    .bind(version)
    .execute(&mut *fixture)
    .await
    .expect("insert current pool usage");
    sqlx::query(
        "INSERT INTO assignment_item \
         (tenant_id,assignment_id,assignment_item_id,position,problem_id,version_id, \
          points_possible,delivery_state,scoring_mode) \
         VALUES ($1,$2,$3,2,$4,$5,1,'active','normal')",
    )
    .bind(tenant)
    .bind(actor_assignment)
    .bind(id())
    .bind(replacement_problem)
    .bind(replacement_version)
    .execute(&mut *fixture)
    .await
    .expect("insert separate replacement usage");
    let mut observations = Vec::new();
    let mut ineligible_attempt = Uuid::nil();
    let mut later_attempt = Uuid::nil();
    let repeated_student = id();
    for index in 0..9 {
        let observation_tenant = if index == 8 { tenant_two } else { tenant };
        let course = if index < 5 {
            actor_course
        } else if index == 8 {
            tenant_two_course
        } else {
            foreign_course
        };
        let assignment = if index < 5 {
            actor_assignment
        } else if index == 8 {
            tenant_two_assignment
        } else {
            foreign_assignment
        };
        let enrollment = id();
        let run = id();
        let first_attempt = id();
        let student = if matches!(index, 0 | 5 | 8) {
            repeated_student
        } else {
            id()
        };
        sqlx::query(
            "INSERT INTO enrollment \
             (tenant_id,enrollment_id,assignment_id,student_id,user_id,course_id, \
              course_membership_id,materialized_at,materialization_purpose, \
              materialized_by_user_id,evaluator_version) \
             VALUES ($1,$2,$3,$4,$4,$5,$6,transaction_timestamp(), \
                     'instructor_issue',$7,1)",
        )
        .bind(observation_tenant)
        .bind(enrollment)
        .bind(assignment)
        .bind(student)
        .bind(course)
        .bind(id())
        .bind(if index < 5 { actor } else { foreign_actor })
        .execute(&mut *fixture)
        .await
        .expect("insert enrollment");
        sqlx::query(
            "INSERT INTO assignment_run \
             (tenant_id,run_id,enrollment_id,run_number,started_at,completed_at,payload,payload_sha256) \
             VALUES ($1,$2,$3,1,transaction_timestamp() - interval '2 minutes', \
                     transaction_timestamp(),'{\"mode\":\"assigned\"}'::jsonb,$4)",
        )
        .bind(observation_tenant)
        .bind(run)
        .bind(enrollment)
        .bind("1".repeat(64))
        .execute(&mut *fixture)
        .await
        .expect("insert completed run");

        let canonical_position = if index == 0 { 1 } else { 0 };
        if index == 0 {
            sqlx::query(
                "INSERT INTO assignment_run_item \
                 (tenant_id,run_id,assignment_item_id,source_position,issued_position, \
                  problem_id,version_id,delivery_status,statistics_eligible) \
                 VALUES ($1,$2,$3,0,0,$4,$5,'submitted',false)",
            )
            .bind(observation_tenant)
            .bind(run)
            .bind(id())
            .bind(problem)
            .bind(version)
            .execute(&mut *fixture)
            .await
            .expect("insert ineligible duplicate position");
        }
        sqlx::query(
            "INSERT INTO assignment_run_item \
             (tenant_id,run_id,assignment_item_id,source_position,issued_position, \
              problem_id,version_id,delivery_status,statistics_eligible) \
             VALUES ($1,$2,$3,0,$4,$5,$6,'submitted',true)",
        )
        .bind(observation_tenant)
        .bind(run)
        .bind(id())
        .bind(canonical_position)
        .bind(problem)
        .bind(version)
        .execute(&mut *fixture)
        .await
        .expect("insert eligible issued position");

        let mut attempts = vec![(first_attempt, canonical_position, 1_i64)];
        if index == 0 {
            ineligible_attempt = id();
            later_attempt = id();
            attempts.push((ineligible_attempt, 0, 1));
            attempts.push((later_attempt, canonical_position, 2));
        }
        for (attempt, position, minute) in attempts {
            sqlx::query(
                "INSERT INTO question_attempt \
                 (tenant_id,attempt_id,run_id,problem_id,version_id,occurred_at,payload, \
                  payload_sha256,attempt_status,submitted_at,assignment_position,course_id, \
                  presentation_capability,issued_question_snapshot_payload, \
                  issued_question_snapshot_payload_sha256,authored_timing_grace_seconds) \
                 VALUES ($1,$2,$3,$4,$5,transaction_timestamp() - interval '3 minutes' \
                         + $6 * interval '1 second','{}'::jsonb,$7,'submitted', \
                         transaction_timestamp() - interval '2 minutes' \
                         + $6 * interval '1 second',$8,$9,'not_applicable', \
                         '{\"schemaVersion\":1,\"question\":{},\"familyWitness\":{\"family\":\"native\",\"physicalAssetBindings\":[]}}'::jsonb, \
                         $10,0)",
            )
            .bind(observation_tenant)
            .bind(attempt)
            .bind(run)
            .bind(problem)
            .bind(version)
            .bind(minute)
            .bind("2".repeat(64))
            .bind(position)
            .bind(course)
            .bind("3".repeat(64))
            .execute(&mut *fixture)
            .await
            .expect("insert scored attempt");
            sqlx::query(
                "INSERT INTO submission_idempotency \
                 (tenant_id,attempt_id,idempotency_key,request_sha256,submitted_at,payload, \
                  payload_sha256,course_id,request_contract_version) \
                 VALUES ($1,$2,$3,$4,transaction_timestamp(),'{}'::jsonb,$5,$6,1)",
            )
            .bind(observation_tenant)
            .bind(attempt)
            .bind(format!("d1-{attempt}"))
            .bind("5".repeat(64))
            .bind("6".repeat(64))
            .bind(course)
            .execute(&mut *fixture)
            .await
            .expect("insert accepted-submission receipt");
            sqlx::query(
                "INSERT INTO submission_evaluation \
                 (tenant_id,attempt_id,submission_id,credit_fraction,correct,grading_status, \
                  payload,payload_sha256,course_id) \
                 VALUES ($1,$2,$2,0.5,false,'graded','{}'::jsonb,$3,$4)",
            )
            .bind(observation_tenant)
            .bind(attempt)
            .bind("4".repeat(64))
            .bind(course)
            .execute(&mut *fixture)
            .await
            .expect("insert scored evaluation");
        }
        observations.push(Observation {
            tenant: observation_tenant,
            enrollment,
            run,
            attempt: first_attempt,
        });
    }
    fixture.commit().await.expect("commit activity fixture");
    Fixture {
        tenant,
        actor,
        sysadmin,
        question_id: "D1A0001",
        problem,
        version,
        replacement_problem,
        replacement_version,
        actor_course,
        foreign_course,
        actor_session,
        sysadmin_session,
        unapproved_instructor_session,
        foreign_session,
        observations,
        ineligible_attempt,
        later_attempt,
    }
}
