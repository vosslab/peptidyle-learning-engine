//! T5 pre-issuance structural-replacement oracle cases.

use super::*;
use std::time::Duration;

fn payload_with_references(
    source: &Source,
    item: Uuid,
    candidate: Uuid,
    selection: Uuid,
    title: &str,
    fixed: (Uuid, Uuid),
    selected: (Uuid, Uuid),
) -> Value {
    let mut value = valid_payload(source, item, candidate, selection, title, "4.0");
    value["entries"][0]["problemId"] = json!(fixed.0);
    value["entries"][0]["versionId"] = json!(fixed.1);
    value["entries"][1]["candidates"][0]["problemId"] = json!(selected.0);
    value["entries"][1]["candidates"][0]["versionId"] = json!(selected.1);
    value
}

async fn create_unissued_definition(
    tx: &mut Transaction<'_, Postgres>,
    source: &Source,
    assignment: Uuid,
    definition: Value,
) {
    sqlx::query("SELECT * FROM public.ple_create_assignment_definition_v1($1,$2,$3,$4,$5,$6,$7)")
        .bind(source.tenant)
        .bind(source.actor)
        .bind(source.course)
        .bind(assignment)
        .bind(definition)
        .bind(Option::<Uuid>::None)
        .bind(Option::<i32>::None)
        .execute(&mut **tx)
        .await
        .expect("create unissued definition");
}

async fn replace_unissued_definition(
    pool: &PgPool,
    source: &Source,
    assignment: Uuid,
    expected_revision: i64,
    definition: Value,
) -> (String, Option<i64>) {
    let mut tx = app(pool, source.tenant).await;
    let outcome: (String, Option<i64>) = sqlx::query_as(
        "SELECT outcome, revision FROM public.ple_replace_unissued_assignment_definition_v1($1,$2,$3,$4,$5,$6)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .bind(expected_revision)
    .bind(definition)
    .fetch_one(&mut *tx)
    .await
    .expect("replace unissued definition");
    tx.commit().await.expect("commit replacement");
    outcome
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn unissued_definition_replacement_is_complete_revision_checked_and_execute_only() {
    let pool = pool().await;
    let source = source(&pool).await;
    let assignment = id();
    let old_item = id();
    let old_candidate = id();
    let old_group = id();
    let mut create = app(&pool, source.tenant).await;
    create_unissued_definition(
        &mut create,
        &source,
        assignment,
        valid_payload(
            &source,
            old_item,
            old_candidate,
            old_group,
            "Original unissued definition",
            "4.0",
        ),
    )
    .await;
    create.commit().await.expect("commit initial definition");

    let replacement_item = id();
    let replacement_candidate = id();
    let replacement_group = id();
    let mut replace = app(&pool, source.tenant).await;
    let returned: (String, i64, i64, String) = sqlx::query_as(
        "SELECT outcome, revision, scoring_generation, scoring_status FROM public.ple_replace_unissued_assignment_definition_v1($1,$2,$3,$4,$5,$6)",
    )
    .bind(source.tenant).bind(source.actor).bind(source.course).bind(assignment).bind(1_i64)
    .bind(valid_payload(&source, replacement_item, replacement_candidate, replacement_group, "Replacement before learner work", "7.0"))
    .fetch_one(&mut *replace).await.expect("complete replacement is accepted before a run");
    assert_eq!(
        returned.1, 2,
        "the capability advances exactly one revision"
    );
    assert_eq!(returned.0, "replaced");
    let active_source_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT assignment_item_id FROM assignment_item WHERE tenant_id=$1 AND assignment_id=$2 UNION ALL SELECT selection_group_id FROM assignment_selection_group WHERE tenant_id=$1 AND assignment_id=$2",
    )
    .bind(source.tenant).bind(assignment).fetch_all(&mut *replace).await.expect("replacement graph is readable through the capability transaction");
    assert_eq!(active_source_ids.len(), 2);
    assert!(active_source_ids.contains(&replacement_item));
    assert!(active_source_ids.contains(&replacement_group));
    assert!(!active_source_ids.contains(&old_item));
    assert!(!active_source_ids.contains(&old_group));
    replace
        .commit()
        .await
        .expect("commit structural replacement");

    let mut stale = app(&pool, source.tenant).await;
    let stale_result = sqlx::query(
        "SELECT * FROM public.ple_replace_unissued_assignment_definition_v1($1,$2,$3,$4,$5,$6)",
    )
    .bind(source.tenant)
    .bind(source.actor)
    .bind(source.course)
    .bind(assignment)
    .bind(1_i64)
    .bind(valid_payload(
        &source,
        id(),
        id(),
        id(),
        "Stale definition",
        "1.0",
    ))
    .execute(&mut *stale)
    .await;
    assert!(stale_result.is_err(), "stale revision is refused");
    stale
        .rollback()
        .await
        .expect("rollback stale structural replacement");
    let direct_mutation = sqlx::query_scalar::<_, bool>("SELECT has_table_privilege('ple_app','public.assignment_selection_group','INSERT,UPDATE,DELETE')")
        .fetch_one(&pool).await.expect("read direct-mutation privilege");
    assert!(
        !direct_mutation,
        "ple_app receives only the execute capability"
    );
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn unissued_definition_replacement_reports_issued_without_mutating_committed_source() {
    let pool = pool().await;
    let source = source(&pool).await;
    let assignment = id();
    let item = id();
    let candidate = id();
    let selection = id();
    let mut create = app(&pool, source.tenant).await;
    create_unissued_definition(
        &mut create,
        &source,
        assignment,
        valid_payload(
            &source,
            item,
            candidate,
            selection,
            "Source frozen by learner work",
            "4.0",
        ),
    )
    .await;
    create.commit().await.expect("commit original definition");

    let enrollment = id();
    let mut owner = pool
        .begin()
        .await
        .expect("begin committed learner-run fixture");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(source.tenant.to_string())
        .execute(&mut *owner)
        .await
        .expect("owner tenant");
    let student_membership: Uuid = sqlx::query_scalar("SELECT course_membership_id FROM course_member WHERE tenant_id=$1 AND course_id=$2 AND user_id=$3 AND role='student'")
        .bind(source.tenant).bind(source.course).bind(source.student).fetch_one(&mut *owner).await.expect("student membership");
    sqlx::query("INSERT INTO course_group_member (tenant_id,course_id,course_group_id,course_membership_id) VALUES ($1,$2,$3,$4)")
        .bind(source.tenant).bind(source.course).bind(source.group).bind(student_membership)
        .execute(&mut *owner).await.expect("materialize group-audience membership");
    sqlx::query("INSERT INTO enrollment (tenant_id,enrollment_id,assignment_id,student_id,user_id,course_id,course_membership_id,materialized_at,materialization_purpose,materialized_by_user_id,evaluator_version) VALUES ($1,$2,$3,$4,$4,$5,$6,transaction_timestamp(),'instructor_issue',$7,1)")
        .bind(source.tenant).bind(enrollment).bind(assignment).bind(source.student).bind(source.course).bind(student_membership).bind(source.actor).execute(&mut *owner).await.expect("materialize learner enrollment");
    sqlx::query("INSERT INTO enrollment_entitlement_basis_receipt (tenant_id,enrollment_id,scope_receipt_id,scope_kind,course_id,course_group_id,course_group_purpose) VALUES ($1,$2,$3,'group_audience',$4,$5,'section')")
        .bind(source.tenant).bind(enrollment).bind(id()).bind(source.course).bind(source.group)
        .execute(&mut *owner).await.expect("persist entitlement basis receipt");
    sqlx::query("INSERT INTO enrollment_applicable_policy_scope_receipt (tenant_id,enrollment_id,course_id,course_group_id,course_group_purpose) VALUES ($1,$2,$3,$4,'section')")
        .bind(source.tenant).bind(enrollment).bind(source.course).bind(source.group)
        .execute(&mut *owner).await.expect("persist applicable policy scope receipt");
    sqlx::query("UPDATE enrollment SET entitlement_receipts_sealed_at=transaction_timestamp() WHERE tenant_id=$1 AND enrollment_id=$2")
        .bind(source.tenant).bind(enrollment).execute(&mut *owner).await.expect("seal entitlement receipt set");
    sqlx::query("INSERT INTO assignment_run (tenant_id,run_id,enrollment_id,run_number,started_at,payload,payload_sha256) VALUES ($1,$2,$3,1,transaction_timestamp(),'{}'::jsonb,$4)")
        .bind(source.tenant).bind(id()).bind(enrollment).bind("0".repeat(64)).execute(&mut *owner).await.expect("write committed learner run evidence");
    owner.commit().await.expect("commit learner run");

    let before: (i64, String, Vec<Uuid>, Vec<Uuid>, Vec<Uuid>) =
        source_graph(&pool, &source, assignment).await;
    let outcome = replace_unissued_definition(
        &pool,
        &source,
        assignment,
        before.0,
        valid_payload(&source, id(), id(), id(), "Forbidden issued edit", "7.0"),
    )
    .await;
    assert_eq!(outcome, ("issued".to_string(), None));
    assert_eq!(
        source_graph(&pool, &source, assignment).await,
        before,
        "an issued refusal preserves the exact source graph and revision"
    );
}

async fn source_graph(
    pool: &PgPool,
    source: &Source,
    assignment: Uuid,
) -> (i64, String, Vec<Uuid>, Vec<Uuid>, Vec<Uuid>) {
    sqlx::query_as("SELECT a.revision,a.title,array_agg(DISTINCT item.assignment_item_id ORDER BY item.assignment_item_id),array_agg(DISTINCT grp.selection_group_id ORDER BY grp.selection_group_id),array_agg(DISTINCT candidate.candidate_id ORDER BY candidate.candidate_id) FROM assignment a JOIN assignment_item item USING (tenant_id,assignment_id) JOIN assignment_selection_group grp USING (tenant_id,assignment_id) JOIN assignment_selection_candidate candidate ON (candidate.tenant_id,candidate.assignment_id,candidate.selection_group_id)=(grp.tenant_id,grp.assignment_id,grp.selection_group_id) WHERE a.tenant_id=$1 AND a.assignment_id=$2 GROUP BY a.revision,a.title")
        .bind(source.tenant).bind(assignment).fetch_one(pool).await.expect("read source graph")
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn unissued_definition_replacement_holds_final_audience_group_lock() {
    let pool = pool().await;
    let source = source(&pool).await;
    let assignment = id();
    let mut create = app(&pool, source.tenant).await;
    create_unissued_definition(
        &mut create,
        &source,
        assignment,
        valid_payload(&source, id(), id(), id(), "Audience lock original", "4.0"),
    )
    .await;
    create.commit().await.expect("commit audience-lock source");

    let mut replacement = app(&pool, source.tenant).await;
    let replaced: (String, Option<i64>) = sqlx::query_as("SELECT outcome, revision FROM public.ple_replace_unissued_assignment_definition_v1($1,$2,$3,$4,$5,$6)")
        .bind(source.tenant).bind(source.actor).bind(source.course).bind(assignment).bind(1_i64)
        .bind(valid_payload(&source, id(), id(), id(), "Audience lock replacement", "5.0"))
        .fetch_one(&mut *replacement).await.expect("replacement obtains final audience lock");
    assert_eq!(replaced, ("replaced".to_string(), Some(2)));

    let mut concurrent_change = pool.begin().await.expect("begin concurrent group change");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(source.tenant.to_string())
        .execute(&mut *concurrent_change)
        .await
        .expect("concurrent tenant");
    let locked = sqlx::query("SELECT course_group_id FROM course_group WHERE tenant_id=$1 AND course_id=$2 AND course_group_id=$3 FOR UPDATE NOWAIT")
        .bind(source.tenant).bind(source.course).bind(source.group).fetch_one(&mut *concurrent_change).await;
    assert!(
        locked.is_err(),
        "the final group lock serializes a purpose change or delete with replacement"
    );
    concurrent_change
        .rollback()
        .await
        .expect("rollback serialized group change");
    replacement.commit().await.expect("commit replacement");
    let valid_audience: (String, i64) = sqlx::query_as("SELECT group_row.purpose,count(audience.course_group_id) FROM course_group group_row JOIN assignment_audience_group audience ON audience.tenant_id=group_row.tenant_id AND audience.course_id=group_row.course_id AND audience.course_group_id=group_row.course_group_id WHERE group_row.tenant_id=$1 AND group_row.course_id=$2 AND group_row.course_group_id=$3 AND audience.assignment_id=$4 GROUP BY group_row.purpose")
        .bind(source.tenant).bind(source.course).bind(source.group).bind(assignment).fetch_one(&pool).await.expect("replacement leaves valid audience binding");
    assert_eq!(valid_audience, ("section".to_string(), 1));
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn unissued_definition_replacement_orders_shared_publications_across_assignments() {
    let pool = pool().await;
    let source = source(&pool).await;
    let second_problem = id();
    let second_version = id();
    let mut owner = pool
        .begin()
        .await
        .expect("begin second publication fixture");
    sqlx::query("SELECT set_config('ple.tenant_id',$1,true)")
        .bind(source.tenant.to_string())
        .execute(&mut *owner)
        .await
        .expect("owner tenant");
    sqlx::query("INSERT INTO problem (problem_id,owner_tenant_id,owner_user_id,visibility,license,lifecycle,question_id) VALUES ($1,$2,$3,'institution','CC-BY','published',$4)")
        .bind(second_problem).bind(source.tenant).bind(source.actor).bind(id().simple().to_string()[..7].to_ascii_uppercase()).execute(&mut *owner).await.expect("second problem");
    sqlx::query("INSERT INTO problem_version (problem_id,version_id,content_sha256,workspace_id,title,lifecycle,backend,publication_scope,author_ids,public_byline,response_family) VALUES ($1,$2,$3,$4,'Second authority problem','published','native','institution',jsonb_build_array($5::text),ARRAY['Oracle author'],'shortText')")
        .bind(second_problem).bind(second_version).bind("b".repeat(64)).bind(id()).bind(source.actor).execute(&mut *owner).await.expect("second published version");
    sqlx::query(
        "INSERT INTO catalog_tenant_grant (tenant_id,problem_id,version_id) VALUES ($1,$2,$3)",
    )
    .bind(source.tenant)
    .bind(second_problem)
    .bind(second_version)
    .execute(&mut *owner)
    .await
    .expect("second catalog grant");
    owner.commit().await.expect("commit second publication");

    let first_assignment = id();
    let second_assignment = id();
    let first_refs = (
        (source.problem, source.version),
        (second_problem, second_version),
    );
    let second_refs = (first_refs.1, first_refs.0);
    let mut create = app(&pool, source.tenant).await;
    create_unissued_definition(
        &mut create,
        &source,
        first_assignment,
        payload_with_references(
            &source,
            id(),
            id(),
            id(),
            "First reverse-reference definition",
            first_refs.0,
            first_refs.1,
        ),
    )
    .await;
    create_unissued_definition(
        &mut create,
        &source,
        second_assignment,
        payload_with_references(
            &source,
            id(),
            id(),
            id(),
            "Second reverse-reference definition",
            second_refs.0,
            second_refs.1,
        ),
    )
    .await;
    create
        .commit()
        .await
        .expect("commit reverse-reference sources");

    let first_payload = payload_with_references(
        &source,
        id(),
        id(),
        id(),
        "First reverse-reference replacement",
        first_refs.0,
        first_refs.1,
    );
    let second_payload = payload_with_references(
        &source,
        id(),
        id(),
        id(),
        "Second reverse-reference replacement",
        second_refs.0,
        second_refs.1,
    );
    let outcomes = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            replace_unissued_definition(&pool, &source, first_assignment, 1, first_payload),
            replace_unissued_definition(&pool, &source, second_assignment, 1, second_payload),
        )
    })
    .await
    .expect("canonical publication ordering completes without deadlock");
    assert_eq!(outcomes.0, ("replaced".to_string(), Some(2)));
    assert_eq!(outcomes.1, ("replaced".to_string(), Some(2)));
}
