//! Exact PostgreSQL catalog oracle for the sealed course-invitation broker.

use super::*;

const FUNCTIONS: &[&str] = &[
    "public.ple_create_course_invitation_v1(uuid,character,uuid,uuid,bytea,text,text,text,text,bigint)",
    "public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text)",
    "public.ple_revoke_course_invitation_v1(uuid,character,uuid,uuid,bigint)",
];

pub(super) async fn catalog(pool: &PgPool) {
    role_and_functions(pool).await;
    policies_and_relations(pool).await;
}

async fn role_and_functions(pool: &PgPool) {
    let flags: (bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolinherit,rolreplication,rolbypassrls \
         FROM pg_roles WHERE rolname='ple_course_invitation_broker'",
    )
    .fetch_one(pool)
    .await
    .expect("closed invitation broker");
    assert_eq!(flags, (false, false, false, false, false, false, false));
    let edges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members \
         WHERE member='ple_course_invitation_broker'::regrole \
            OR roleid='ple_course_invitation_broker'::regrole",
    )
    .fetch_one(pool)
    .await
    .expect("invitation broker has no membership edges");
    assert_eq!(edges, 0);

    for function in FUNCTIONS {
        let row = sqlx::query(
            "SELECT owner.rolname,p.prosecdef,p.provolatile::text,p.proconfig \
             FROM pg_proc p JOIN pg_roles owner ON owner.oid=p.proowner \
             WHERE p.oid=to_regprocedure($1)",
        )
        .bind(function)
        .fetch_one(pool)
        .await
        .expect("one invitation capability");
        assert_eq!(
            row.try_get::<String, _>("rolname").expect("owner"),
            "ple_course_invitation_broker"
        );
        assert!(row.try_get::<bool, _>("prosecdef").expect("definer"));
        assert_eq!(
            row.try_get::<String, _>("provolatile").expect("volatile"),
            "v"
        );
        assert_eq!(
            row.try_get::<Vec<String>, _>("proconfig")
                .expect("fixed search path"),
            vec![SEARCH_PATH.to_owned()]
        );
        let public_execute: bool =
            sqlx::query_scalar("SELECT has_function_privilege('public',$1,'EXECUTE')")
                .bind(function)
                .fetch_one(pool)
                .await
                .expect("PUBLIC invitation execute matrix");
        assert!(!public_execute, "PUBLIC cannot execute {function}");
        for role in [
            "ple_app",
            "ple_course_invitation_broker",
            "ple_course_creation_broker",
        ] {
            let actual: bool = sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')")
                .bind(role)
                .bind(function)
                .fetch_one(pool)
                .await
                .expect("invitation execute matrix");
            assert_eq!(
                actual,
                matches!(role, "ple_app" | "ple_course_invitation_broker"),
                "exact execute audience for {function}"
            );
        }
    }

    let claim_arguments: Vec<String> = sqlx::query_scalar(
        "SELECT proargnames FROM pg_proc \
         WHERE oid=to_regprocedure('public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text)')",
    )
    .fetch_one(pool)
    .await
    .expect("claim witness signature");
    assert_eq!(
        claim_arguments,
        [
            "p_token",
            "p_user",
            "p_normalized",
            "p_delivery",
            "p_display",
            "tenant_id",
            "course_id",
            "invitation_id",
            "claimed_user_id",
            "student_id",
            "record_id",
            "user_id",
            "member_role",
            "status",
            "roster_id",
            "created_at_millis",
            "revoked_at_millis",
            "display_name",
            "normalized_email",
            "delivery_email",
            "invitation_status",
            "invitation_claimed_user_id",
            "replayed",
            "delivery_cancelled",
            "roster_revision",
        ]
        .map(str::to_owned)
    );
}

async fn policies_and_relations(pool: &PgPool) {
    let actual_policies: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT p.polname,c.relname,p.polcmd::text FROM pg_policy p \
         JOIN pg_class c ON c.oid=p.polrelid \
         WHERE 'ple_course_invitation_broker'::regrole::oid=ANY(p.polroles) \
         ORDER BY c.relname,p.polname",
    )
    .fetch_all(pool)
    .await
    .expect("invitation broker RLS policies");
    assert_eq!(
        actual_policies,
        [
            ("course_invitation_broker_course", "course", "r"),
            (
                "course_invitation_broker_domain",
                "course_allowed_email_domain",
                "r"
            ),
            (
                "course_invitation_broker_invitation",
                "course_invitation",
                "*"
            ),
            (
                "course_invitation_broker_delivery",
                "course_invitation_delivery",
                "*"
            ),
            ("course_invitation_broker_member", "course_member", "*"),
            (
                "course_invitation_broker_profile",
                "course_roster_profile",
                "*"
            ),
            ("course_invitation_broker_state", "course_roster_state", "*"),
            (
                "course_invitation_broker_identity",
                "tenant_learner_identity",
                "*"
            ),
        ]
        .map(|(name, relation, command)| (
            name.to_owned(),
            relation.to_owned(),
            command.to_owned()
        ))
    );
    let tables: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname,acl.privilege_type FROM pg_class c \
         JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl \
         WHERE n.nspname='public' AND c.relkind IN ('r','p') \
           AND acl.grantee='ple_course_invitation_broker'::regrole \
           AND acl.grantee<>c.relowner ORDER BY c.relname,acl.privilege_type",
    )
    .fetch_all(pool)
    .await
    .expect("exact invitation table ACLs");
    assert_eq!(
        tables,
        [
            ("course", "SELECT"),
            ("course_allowed_email_domain", "SELECT"),
            ("course_invitation", "INSERT"),
            ("course_invitation", "SELECT"),
            ("course_invitation_delivery", "INSERT"),
            ("course_invitation_delivery", "SELECT"),
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
    let columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname,a.attname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace \
         JOIN pg_attribute a ON a.attrelid=c.oid CROSS JOIN LATERAL aclexplode(a.attacl) acl \
         WHERE n.nspname='public' AND a.attnum>0 AND NOT a.attisdropped \
           AND acl.grantee='ple_course_invitation_broker'::regrole \
           AND acl.privilege_type='UPDATE' ORDER BY c.relname,a.attname",
    )
    .fetch_all(pool)
    .await
    .expect("exact invitation column ACLs");
    assert_eq!(
        columns,
        [
            ("course", "course_id"),
            ("course_invitation", "claimed_at"),
            ("course_invitation", "claimed_user_id"),
            ("course_invitation", "status"),
            ("course_invitation_delivery", "state"),
            ("course_invitation_delivery", "updated_at"),
            ("course_roster_state", "revision"),
            ("course_roster_state", "updated_at"),
        ]
        .map(|(relation, column)| (relation.to_owned(), column.to_owned()))
    );
    let direct_dml: bool = sqlx::query_scalar(
        "SELECT bool_or(has_table_privilege('ple_app',c.oid,p)) FROM pg_class c \
         JOIN pg_namespace n ON n.oid=c.relnamespace \
         CROSS JOIN unnest(ARRAY['INSERT','UPDATE','DELETE']) p \
         WHERE n.nspname='public' AND c.relname IN \
           ('course_invitation','course_invitation_delivery','course_member', \
            'tenant_learner_identity','course_roster_profile','course_roster_state')",
    )
    .fetch_one(pool)
    .await
    .expect("application invitation direct DML matrix");
    assert!(!direct_dml);
}
