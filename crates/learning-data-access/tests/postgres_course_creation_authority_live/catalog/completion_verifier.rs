//! Exact catalog oracle for the sealed completion verification capability.

use super::*;

const ROLE: &str = "ple_base_course_completion_verification_broker";
const RELATIONS: &[&str] = &[
    "ple_account",
    "instructor_approval",
    "tenant_student_identity",
    "course",
    "course_roster_state",
    "course_appearance",
    "course_allowed_email_domain",
    "course_group_membership_policy",
    "course_grade_scheme",
    "course_grade_category",
    "course_grade_category_assignment",
    "course_grade_letter_band",
    "course_total_export_audit",
    "course_member",
    "course_roster_profile",
    "course_group",
    "course_group_member",
    "problem",
    "problem_version",
    "problem_version_payload",
    "catalog_tenant_grant",
    "catalog_search_document",
    "published_source_artifact",
    "published_flat_import_origin",
    "published_flat_import_choice_map",
    "published_qti_grading",
    "answer_key",
    "workspace_draft",
    "workspace_draft_access",
    "workspace_flat_question_source",
    "workspace_flat_question_grading",
    "assignment",
    "assignment_item",
    "assignment_selection_group",
    "assignment_selection_candidate",
    "assignment_audience_group",
    "assignment_effective_policy_base",
    "assignment_group_schedule_offset",
    "assignment_group_accommodation",
    "assignment_individual_policy_exception",
    "enrollment",
    "enrollment_entitlement_basis_receipt",
    "enrollment_applicable_policy_scope_receipt",
    "student_assignment_summary",
    "assignment_run",
    "assignment_run_item",
    "question_attempt",
    "attempt_effective_policy_receipt",
    "attempt_effective_policy_receipt_field_source",
    "attempt_effective_policy_current",
    "submission",
    "submission_idempotency",
    "submission_evaluation",
    "attempt_feedback",
    "attempt_score_current",
    "submission_receipt_snapshot",
    "submission_next_attempt",
    "feedback_release",
    "question_prefetch",
    "question_statistics_contribution_receipt",
    "question_statistics_aggregate",
];

pub(super) async fn catalog(pool: &PgPool) {
    relation_authority(pool).await;
    policies(pool).await;
}

async fn relation_authority(pool: &PgPool) {
    let mut expected = RELATIONS
        .iter()
        .map(|relation| ((*relation).to_owned(), "SELECT".to_owned()))
        .collect::<Vec<_>>();
    expected.sort_unstable();
    let actual: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname,acl.privilege_type FROM pg_class c \
         JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') AND acl.grantee=$1::regrole \
         AND acl.grantee<>c.relowner ORDER BY c.relname,acl.privilege_type",
    )
    .bind(ROLE)
    .fetch_all(pool)
    .await
    .expect("completion verifier direct relation ACLs");
    assert_eq!(actual, expected, "exact SELECT-only completion graph");

    let forbidden: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN unnest(ARRAY['INSERT','UPDATE','DELETE','TRUNCATE','REFERENCES','TRIGGER']) privilege \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
         AND has_table_privilege($1,c.oid,privilege)",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("completion verifier forbidden table privileges");
    assert_eq!(forbidden, 0);
    let column_acls: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_attribute a CROSS JOIN LATERAL aclexplode(a.attacl) acl \
         WHERE acl.grantee=$1::regrole",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("completion verifier column ACLs");
    assert_eq!(column_acls, 0);
    let sequence_privileges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind='S' AND (has_sequence_privilege($1,c.oid,'USAGE') \
         OR has_sequence_privilege($1,c.oid,'SELECT') OR has_sequence_privilege($1,c.oid,'UPDATE'))",
    )
    .bind(ROLE)
    .fetch_one(pool)
    .await
    .expect("completion verifier sequence privileges");
    assert_eq!(sequence_privileges, 0);
}

async fn policies(pool: &PgPool) {
    let expected: Vec<PolicyCatalogRow> = sqlx::query_as(
        "SELECT 'ple_base_course_completion_select',c.relname,'r',true,ARRAY[$1::text], \
         'true'::text,NULL::text FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') AND c.relname=ANY($2) \
         AND c.relrowsecurity ORDER BY c.relname",
    )
    .bind(ROLE)
    .bind(RELATIONS)
    .fetch_all(pool)
    .await
    .expect("expected completion verifier policies");
    let actual: Vec<PolicyCatalogRow> = sqlx::query_as(
        "SELECT p.polname,c.relname,p.polcmd::text,p.polpermissive, \
         array(SELECT r.rolname FROM unnest(p.polroles) role_oid JOIN pg_roles r ON r.oid=role_oid ORDER BY r.rolname), \
         pg_get_expr(p.polqual,p.polrelid),pg_get_expr(p.polwithcheck,p.polrelid) \
         FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid WHERE $1::regrole::oid=ANY(p.polroles) \
         ORDER BY c.relname,p.polname",
    )
    .bind(ROLE)
    .fetch_all(pool)
    .await
    .expect("actual completion verifier policies");
    assert_eq!(actual, expected, "exact completion verifier RLS policy set");
    let unforced: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') AND c.relname=ANY($1) \
         AND c.relrowsecurity AND NOT c.relforcerowsecurity ORDER BY c.relname",
    )
    .bind(RELATIONS)
    .fetch_all(pool)
    .await
    .expect("completion verifier force-RLS catalog");
    assert!(
        unforced.is_empty(),
        "unforced completion relations: {unforced:?}"
    );
}
