//! Exact catalog oracle for the direct-Instructor roster mutation capability.

use super::*;

const FUNCTION: &str = "public.ple_upsert_course_student_as_instructor_v1(uuid,uuid,uuid,uuid,uuid,uuid,text,text,text,text)";
type PolicyCatalogRow = (String, String, String, Option<String>, Option<String>);

pub(super) async fn catalog(pool: &PgPool) {
    role_and_function(pool).await;
    policies(pool).await;
    relation_authority(pool).await;
}

async fn role_and_function(pool: &PgPool) {
    let flags: (bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolinherit,rolreplication,rolbypassrls \
         FROM pg_roles WHERE rolname='ple_course_roster_mutator_broker'",
    )
    .fetch_one(pool)
    .await
    .expect("closed roster mutator broker");
    assert_eq!(flags, (false, false, false, false, false, false, false));
    let membership_edges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members \
         WHERE member='ple_course_roster_mutator_broker'::regrole \
            OR roleid='ple_course_roster_mutator_broker'::regrole",
    )
    .fetch_one(pool)
    .await
    .expect("roster mutator membership graph");
    assert_eq!(membership_edges, 0);

    let row = sqlx::query(
        "SELECT owner.rolname,p.prosecdef,p.provolatile::text,p.proconfig,p.proargnames,\
                array(SELECT mode::text FROM unnest(p.proargmodes) mode) AS argument_modes \
           FROM pg_proc p JOIN pg_roles owner ON owner.oid=p.proowner \
          WHERE p.oid=to_regprocedure($1)",
    )
    .bind(FUNCTION)
    .fetch_one(pool)
    .await
    .expect("one roster mutator capability");
    assert_eq!(
        row.try_get::<String, _>("rolname").expect("function owner"),
        "ple_course_roster_mutator_broker"
    );
    assert!(
        row.try_get::<bool, _>("prosecdef")
            .expect("security definer")
    );
    assert_eq!(
        row.try_get::<String, _>("provolatile")
            .expect("function volatility"),
        "v"
    );
    assert_eq!(
        row.try_get::<Vec<String>, _>("proconfig")
            .expect("fixed search path"),
        vec![SEARCH_PATH.to_owned()]
    );
    assert_eq!(
        row.try_get::<Vec<String>, _>("proargnames")
            .expect("input and witness names"),
        [
            "p_tenant",
            "p_actor",
            "p_course",
            "p_target_user",
            "p_candidate_student",
            "p_candidate_membership",
            "p_display_name",
            "p_email_normalized",
            "p_email_delivery",
            "p_roster_id",
            "tenant_id",
            "actor_id",
            "direct_instructor_membership_id",
            "course_id",
            "target_user_id",
            "student_id",
            "course_membership_id",
            "created",
            "roster_revision",
        ]
        .map(str::to_owned)
    );
    assert_eq!(
        row.try_get::<Vec<String>, _>("argument_modes")
            .expect("input and witness modes"),
        (0..10)
            .map(|_| "i".to_owned())
            .chain((0..9).map(|_| "t".to_owned()))
            .collect::<Vec<_>>()
    );

    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT rolname FROM pg_roles WHERE rolname LIKE 'ple\\_%' ESCAPE '\\' ORDER BY rolname",
    )
    .fetch_all(pool)
    .await
    .expect("PLE role catalog");
    for role in roles {
        let can_execute: bool =
            sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')")
                .bind(&role)
                .bind(FUNCTION)
                .fetch_one(pool)
                .await
                .expect("roster capability execution matrix");
        assert_eq!(
            can_execute,
            matches!(
                role.as_str(),
                "ple_app" | "ple_course_roster_mutator_broker"
            ),
            "{role} effective roster capability execution"
        );
    }
    let public_execute: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_proc p \
         CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) acl \
         WHERE p.oid=to_regprocedure($1) AND acl.grantee=0 AND acl.privilege_type='EXECUTE')",
    )
    .bind(FUNCTION)
    .fetch_one(pool)
    .await
    .expect("PUBLIC roster capability ACL");
    assert!(!public_execute);
}

async fn policies(pool: &PgPool) {
    let rows: Vec<PolicyCatalogRow> = sqlx::query_as(
        "SELECT p.polname,c.relname,p.polcmd::text,\
                pg_get_expr(p.polqual,p.polrelid),pg_get_expr(p.polwithcheck,p.polrelid) \
           FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid \
          WHERE 'ple_course_roster_mutator_broker'::regrole::oid=ANY(p.polroles) \
          ORDER BY c.relname",
    )
    .fetch_all(pool)
    .await
    .expect("roster mutator RLS catalog");
    assert_eq!(
        rows,
        vec![
            (
                "course_roster_mutator_course_tenant",
                "course",
                "*",
                true,
                false
            ),
            (
                "course_roster_mutator_member_tenant",
                "course_member",
                "*",
                true,
                true
            ),
            (
                "course_roster_mutator_profile_tenant",
                "course_roster_profile",
                "*",
                true,
                true
            ),
            (
                "course_roster_mutator_state_tenant",
                "course_roster_state",
                "*",
                true,
                true
            ),
            (
                "course_roster_mutator_identity_tenant",
                "tenant_learner_identity",
                "*",
                true,
                true
            ),
        ]
        .into_iter()
        .map(|(name, relation, command, using, check)| {
            (
                name.to_owned(),
                relation.to_owned(),
                command.to_owned(),
                using.then(|| "(tenant_id = ple_current_tenant())".to_owned()),
                check.then(|| "(tenant_id = ple_current_tenant())".to_owned()),
            )
        })
        .collect::<Vec<_>>()
    );
}

async fn relation_authority(pool: &PgPool) {
    let actual_tables: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname,acl.privilege_type FROM pg_class c \
         JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
           AND acl.grantee='ple_course_roster_mutator_broker'::regrole \
           AND acl.grantee<>c.relowner ORDER BY c.relname,acl.privilege_type",
    )
    .fetch_all(pool)
    .await
    .expect("exact roster mutator table ACLs");
    assert_eq!(
        actual_tables,
        [
            ("course", "SELECT"),
            ("course_member", "INSERT"),
            ("course_member", "SELECT"),
            ("course_roster_profile", "INSERT"),
            ("course_roster_profile", "SELECT"),
            ("course_roster_state", "SELECT"),
            ("tenant_learner_identity", "INSERT"),
            ("tenant_learner_identity", "SELECT"),
        ]
        .map(|(relation, privilege)| (relation.to_owned(), privilege.to_owned()))
    );
    let actual_columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname,a.attname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         JOIN pg_attribute a ON a.attrelid=c.oid CROSS JOIN LATERAL aclexplode(a.attacl) acl \
         WHERE n.nspname='public' AND a.attnum>0 AND NOT a.attisdropped \
           AND acl.grantee='ple_course_roster_mutator_broker'::regrole \
           AND acl.privilege_type='UPDATE' ORDER BY c.relname,a.attname",
    )
    .fetch_all(pool)
    .await
    .expect("exact roster mutator column ACLs");
    assert_eq!(
        actual_columns,
        [
            ("course", "course_id"),
            ("course_member", "course_membership_id"),
            ("course_member", "revoked_at"),
            ("course_member", "status"),
            ("course_roster_state", "revision"),
            ("course_roster_state", "updated_at"),
        ]
        .map(|(relation, column)| (relation.to_owned(), column.to_owned()))
    );
    let actual_sequences: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname,acl.privilege_type FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('S',c.relowner))) acl \
         WHERE n.nspname='public' AND c.relkind='S' \
           AND acl.grantee='ple_course_roster_mutator_broker'::regrole \
           AND acl.grantee<>c.relowner ORDER BY c.relname,acl.privilege_type",
    )
    .fetch_all(pool)
    .await
    .expect("exact roster mutator sequence ACLs");
    assert_eq!(
        actual_sequences,
        [
            ("course_member_public_id_seq", "SELECT"),
            ("course_member_public_id_seq", "USAGE"),
        ]
        .map(|(sequence, privilege)| (sequence.to_owned(), privilege.to_owned()))
    );
}
