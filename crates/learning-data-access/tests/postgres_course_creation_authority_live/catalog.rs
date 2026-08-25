//! PostgreSQL catalog assertions for the 1818 closed authority boundary.

use super::*;

#[path = "catalog/completion_verifier.rs"]
mod completion_verifier;
#[path = "catalog/freshness.rs"]
mod freshness;
#[path = "catalog/invitation_broker.rs"]
mod invitation_broker;
#[path = "catalog/roster_mutator.rs"]
mod roster_mutator;

const SEARCH_PATH: &str = "search_path=pg_catalog, public, pg_temp";

#[derive(Clone, Copy)]
struct FunctionSpec {
    name: &'static str,
    arguments: &'static str,
    owner: &'static str,
    executable_by: &'static [&'static str],
    direct_execute_by: &'static [&'static str],
}

type PolicyCatalogRow = (
    String,
    String,
    String,
    bool,
    Vec<String>,
    Option<String>,
    Option<String>,
);

const FUNCTIONS: &[FunctionSpec] = &[
    FunctionSpec {
        name: "ple_course_creation_deny_internal",
        arguments: "",
        owner: "ple_course_creation_broker",
        executable_by: &[
            "ple_course_creation_broker",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_install_broker"],
    },
    FunctionSpec {
        name: "ple_course_creation_validate_inputs",
        arguments: "uuid, uuid, text, date, date, text",
        owner: "ple_course_creation_broker",
        executable_by: &[
            "ple_course_creation_broker",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_install_broker"],
    },
    FunctionSpec {
        name: "ple_create_course_core_internal",
        arguments: "uuid, uuid, text, date, date, text, uuid",
        owner: "ple_course_creation_broker",
        executable_by: &[
            "ple_course_creation_broker",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_install_broker"],
    },
    FunctionSpec {
        name: "ple_verify_course_creation_aggregate_internal",
        arguments: "uuid, uuid, text, date, date, text, uuid",
        owner: "ple_course_creation_broker",
        executable_by: &["ple_course_creation_broker"],
        direct_execute_by: &[],
    },
    FunctionSpec {
        name: "ple_create_course_as_instructor_v1",
        arguments: "uuid, uuid, text, date, date, text, uuid, character",
        owner: "ple_course_creation_broker",
        executable_by: &["ple_app", "ple_course_creation_broker"],
        direct_execute_by: &["ple_app"],
    },
    FunctionSpec {
        name: "ple_create_course_as_sysadmin_v1",
        arguments: "uuid, uuid, text, date, date, text, uuid, character",
        owner: "ple_course_creation_broker",
        executable_by: &["ple_app", "ple_course_creation_broker"],
        direct_execute_by: &["ple_app"],
    },
    FunctionSpec {
        name: "ple_require_base_course_install_lock_internal",
        arguments: "",
        owner: "ple_base_course_install_broker",
        executable_by: &["ple_base_course_install_broker"],
        direct_execute_by: &[],
    },
    FunctionSpec {
        name: "ple_base_course_install_validate_recipe_internal",
        arguments: "jsonb",
        owner: "ple_base_course_install_broker",
        executable_by: &["ple_base_course_install_broker"],
        direct_execute_by: &[],
    },
    FunctionSpec {
        name: "ple_verify_base_course_accounts_internal",
        arguments: "jsonb",
        owner: "ple_base_course_install_broker",
        executable_by: &["ple_base_course_install_broker"],
        direct_execute_by: &[],
    },
    FunctionSpec {
        name: "ple_verify_base_course_course_prefix_internal",
        arguments: "jsonb, text",
        owner: "ple_course_creation_broker",
        executable_by: &[
            "ple_course_creation_broker",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_install_broker"],
    },
    FunctionSpec {
        name: "ple_verify_base_course_completion_internal",
        arguments: "uuid, uuid, text, jsonb",
        owner: "ple_base_course_completion_verification_broker",
        executable_by: &[
            "ple_base_course_install_broker",
            "ple_base_course_completion_verification_broker",
        ],
        direct_execute_by: &["ple_base_course_install_broker"],
    },
    FunctionSpec {
        name: "ple_base_course_install_acquire_lock_v1",
        arguments: "",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
    FunctionSpec {
        name: "ple_base_course_install_read_v2",
        arguments: "",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
    FunctionSpec {
        name: "ple_require_fresh_base_course_install_internal",
        arguments: "",
        owner: "ple_base_course_freshness_broker",
        executable_by: &[
            "ple_base_course_install_broker",
            "ple_base_course_freshness_broker",
        ],
        direct_execute_by: &["ple_base_course_install_broker"],
    },
    FunctionSpec {
        name: "ple_base_course_install_prepare_v2",
        arguments: "uuid, text, jsonb, jsonb",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
    FunctionSpec {
        name: "ple_base_course_install_seed_accounts_v2",
        arguments: "uuid",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
    FunctionSpec {
        name: "ple_base_course_install_seed_course_v2",
        arguments: "uuid, text",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
    FunctionSpec {
        name: "ple_base_course_install_complete_v2",
        arguments: "uuid, uuid, text, jsonb, text",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
    FunctionSpec {
        name: "ple_base_course_install_release_lock_v1",
        arguments: "",
        owner: "ple_base_course_install_broker",
        executable_by: &[
            "ple_base_course_installer",
            "ple_base_course_install_broker",
        ],
        direct_execute_by: &["ple_base_course_installer"],
    },
];

async fn role_catalog(pool: &PgPool) {
    for role in [
        "ple_course_creation_broker",
        "ple_base_course_installer",
        "ple_base_course_install_broker",
        "ple_base_course_freshness_broker",
        "ple_base_course_completion_verification_broker",
    ] {
        let flags: (bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
            "SELECT rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolinherit,rolreplication,rolbypassrls \
             FROM pg_roles WHERE rolname=$1",
        )
        .bind(role)
        .fetch_one(pool)
        .await
        .expect("closed authority role");
        assert_eq!(flags, (false, false, false, false, false, false, false));
    }
    let membership_edges: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_auth_members WHERE member=ANY(ARRAY[\
         'ple_course_creation_broker'::regrole,'ple_base_course_installer'::regrole,\
         'ple_base_course_install_broker'::regrole,'ple_base_course_freshness_broker'::regrole,\
         'ple_base_course_completion_verification_broker'::regrole]) \
         OR roleid IN ('ple_base_course_freshness_broker'::regrole,\
         'ple_base_course_completion_verification_broker'::regrole)",
    )
    .fetch_one(pool)
    .await
    .expect("closed authority membership catalog");
    assert_eq!(
        membership_edges, 0,
        "capability roles belong to no role and the freshness broker has no edge either way"
    );
}

async fn function_catalog(pool: &PgPool) {
    let function_count: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.proname IN ('ple_course_creation_deny_internal','ple_course_creation_validate_inputs','ple_create_course_core_internal','ple_verify_course_creation_aggregate_internal','ple_create_course_as_instructor_v1','ple_create_course_as_sysadmin_v1','ple_require_base_course_install_lock_internal','ple_base_course_install_validate_recipe_internal','ple_verify_base_course_accounts_internal','ple_verify_base_course_course_prefix_internal','ple_verify_base_course_completion_internal','ple_base_course_install_acquire_lock_v1','ple_base_course_install_read_v2','ple_require_fresh_base_course_install_internal','ple_base_course_install_prepare_v2','ple_base_course_install_seed_accounts_v2','ple_base_course_install_seed_course_v2','ple_base_course_install_complete_v2','ple_base_course_install_release_lock_v1')")
        .fetch_one(pool)
        .await
        .expect("closed authority function count");
    assert_eq!(
        function_count,
        FUNCTIONS.len() as i64,
        "exact authority function set"
    );
    for spec in FUNCTIONS {
        let row = sqlx::query("SELECT r.rolname,p.prosecdef,p.proconfig FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_roles r ON r.oid=p.proowner WHERE n.nspname='public' AND p.proname=$1 AND oidvectortypes(p.proargtypes)=$2")
            .bind(spec.name).bind(spec.arguments).fetch_one(pool).await.expect("one closed authority function");
        assert_eq!(
            row.try_get::<String, _>(0).expect("function owner"),
            spec.owner
        );
        assert!(row.try_get::<bool, _>(1).expect("security definer"));
        assert_eq!(
            row.try_get::<Vec<String>, _>(2)
                .expect("function search path"),
            vec![SEARCH_PATH.to_owned()]
        );
        let signature = format!("public.{}({})", spec.name, spec.arguments);
        for role in [
            "ple_app",
            "ple_course_creation_broker",
            "ple_base_course_installer",
            "ple_base_course_install_broker",
            "ple_base_course_freshness_broker",
            "ple_base_course_completion_verification_broker",
        ] {
            let granted: bool =
                sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')")
                    .bind(role)
                    .bind(&signature)
                    .fetch_one(pool)
                    .await
                    .expect("function execute matrix");
            assert_eq!(
                granted,
                spec.executable_by.contains(&role),
                "{role} execute grant for {signature}"
            );
            let directly_granted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) acl JOIN pg_roles r ON r.oid=acl.grantee WHERE n.nspname='public' AND p.proname=$1 AND oidvectortypes(p.proargtypes)=$2 AND r.rolname=$3 AND acl.grantee<>p.proowner AND acl.privilege_type='EXECUTE' AND NOT acl.is_grantable)")
                .bind(spec.name)
                .bind(spec.arguments)
                .bind(role)
                .fetch_one(pool)
                .await
                .expect("direct function execute matrix");
            assert_eq!(
                directly_granted,
                spec.direct_execute_by.contains(&role),
                "{role} direct execute ACL for {signature}"
            );
        }
    }
    for (function, arguments, role) in [
        ("ple_current_tenant", "", "ple_course_creation_broker"),
        ("ple_current_tenant", "", "ple_base_course_install_broker"),
        (
            "ple_course_records_accessible",
            "uuid, uuid",
            "ple_course_creation_broker",
        ),
        (
            "ple_lock_instructor_approval_eligibility",
            "uuid",
            "ple_app",
        ),
        (
            "ple_lock_instructor_approval_eligibility",
            "uuid",
            "ple_course_creation_broker",
        ),
    ] {
        let granted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) acl JOIN pg_roles r ON r.oid=acl.grantee WHERE n.nspname='public' AND p.proname=$1 AND oidvectortypes(p.proargtypes)=$2 AND r.rolname=$3 AND acl.grantee<>p.proowner AND acl.privilege_type='EXECUTE' AND NOT acl.is_grantable)",
        )
        .bind(function)
        .bind(arguments)
        .bind(role)
        .fetch_one(pool)
        .await
        .expect("new helper execute matrix");
        assert!(
            granted,
            "{role} direct execute ACL for {function}({arguments})"
        );
    }
    let mut expected_direct_execute_signatures: Vec<(String, String)> = FUNCTIONS
        .iter()
        .flat_map(|spec| {
            spec.direct_execute_by.iter().map(move |role| {
                (
                    format!("public.{}({})", spec.name, spec.arguments),
                    (*role).to_owned(),
                )
            })
        })
        .collect();
    expected_direct_execute_signatures.push((
        "public.ple_current_tenant()".to_owned(),
        "ple_course_creation_broker".to_owned(),
    ));
    expected_direct_execute_signatures.push((
        "public.ple_current_tenant()".to_owned(),
        "ple_base_course_install_broker".to_owned(),
    ));
    expected_direct_execute_signatures.push((
        "public.ple_course_records_accessible(uuid,uuid)".to_owned(),
        "ple_course_creation_broker".to_owned(),
    ));
    expected_direct_execute_signatures.push((
        "public.ple_lock_instructor_approval_eligibility(uuid)".to_owned(),
        "ple_app".to_owned(),
    ));
    expected_direct_execute_signatures.push((
        "public.ple_lock_instructor_approval_eligibility(uuid)".to_owned(),
        "ple_course_creation_broker".to_owned(),
    ));
    let mut expected_direct_executes = Vec::with_capacity(expected_direct_execute_signatures.len());
    for (signature, role) in expected_direct_execute_signatures {
        let oid: i64 = sqlx::query_scalar("SELECT to_regprocedure($1)::oid::int8")
            .bind(&signature)
            .fetch_one(pool)
            .await
            .expect("expected direct execute function exists");
        expected_direct_executes.push((oid, role));
    }
    expected_direct_executes.sort_unstable();
    let mut actual_direct_executes: Vec<(i64, String)> = sqlx::query_as("SELECT p.oid::int8,r.rolname FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) acl JOIN pg_roles r ON r.oid=acl.grantee WHERE n.nspname='public' AND acl.privilege_type='EXECUTE' AND NOT acl.is_grantable AND acl.grantee<>p.proowner AND ((p.proname IN ('ple_course_creation_deny_internal','ple_course_creation_validate_inputs','ple_create_course_core_internal','ple_verify_course_creation_aggregate_internal','ple_create_course_as_instructor_v1','ple_create_course_as_sysadmin_v1','ple_require_base_course_install_lock_internal','ple_base_course_install_validate_recipe_internal','ple_verify_base_course_accounts_internal','ple_verify_base_course_course_prefix_internal','ple_verify_base_course_completion_internal','ple_base_course_install_acquire_lock_v1','ple_base_course_install_read_v2','ple_require_fresh_base_course_install_internal','ple_base_course_install_prepare_v2','ple_base_course_install_seed_accounts_v2','ple_base_course_install_seed_course_v2','ple_base_course_install_complete_v2','ple_base_course_install_release_lock_v1') AND r.rolname IN ('ple_app','ple_base_course_installer','ple_base_course_install_broker')) OR (p.proname='ple_current_tenant' AND oidvectortypes(p.proargtypes)='' AND r.rolname IN ('ple_course_creation_broker','ple_base_course_install_broker')) OR (p.proname='ple_course_records_accessible' AND oidvectortypes(p.proargtypes)='uuid, uuid' AND r.rolname='ple_course_creation_broker') OR (p.oid=to_regprocedure('public.ple_lock_instructor_approval_eligibility(uuid)') AND r.rolname IN ('ple_app','ple_course_creation_broker')))")
        .fetch_all(pool).await.expect("direct execute ACL catalog");
    actual_direct_executes.sort_unstable();
    assert_eq!(
        actual_direct_executes, expected_direct_executes,
        "exact direct execute ACL matrix"
    );
    let public_execute: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) x WHERE n.nspname='public' AND (p.proname IN ('ple_course_creation_deny_internal','ple_course_creation_validate_inputs','ple_create_course_core_internal','ple_verify_course_creation_aggregate_internal','ple_create_course_as_instructor_v1','ple_create_course_as_sysadmin_v1','ple_require_base_course_install_lock_internal','ple_base_course_install_validate_recipe_internal','ple_verify_base_course_accounts_internal','ple_verify_base_course_course_prefix_internal','ple_verify_base_course_completion_internal','ple_base_course_install_acquire_lock_v1','ple_base_course_install_read_v2','ple_require_fresh_base_course_install_internal','ple_base_course_install_prepare_v2','ple_base_course_install_seed_accounts_v2','ple_base_course_install_seed_course_v2','ple_base_course_install_complete_v2','ple_base_course_install_release_lock_v1') OR p.oid=to_regprocedure('public.ple_lock_instructor_approval_eligibility(uuid)')) AND x.grantee=0 AND x.privilege_type='EXECUTE'")
        .fetch_one(pool).await.expect("public internal execute catalog");
    assert_eq!(
        public_execute, 0,
        "PUBLIC cannot execute closed-authority functions"
    );
}

async fn policy_catalog(pool: &PgPool) {
    let rows: Vec<PolicyCatalogRow> = sqlx::query_as("SELECT pol.polname,c.relname,pol.polcmd::text,pol.polpermissive,array(SELECT r.rolname FROM unnest(pol.polroles) role_oid JOIN pg_roles r ON r.oid=role_oid ORDER BY r.rolname),pg_get_expr(pol.polqual,pol.polrelid),pg_get_expr(pol.polwithcheck,pol.polrelid) FROM pg_policy pol JOIN pg_class c ON c.oid=pol.polrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND pol.polname IN ('course_creation_broker_course_tenant','course_creation_broker_member_tenant','course_creation_broker_roster_tenant','course_creation_broker_group_tenant','course_creation_broker_scheme_tenant','course_creation_broker_appearance_tenant','course_creation_broker_session_tenant','course_creation_broker_profile_tenant','course_creation_broker_identity_tenant','course_creation_broker_assignment_tenant','course_creation_broker_domain_tenant','course_creation_broker_course_group_tenant','course_creation_broker_group_member_tenant','course_creation_broker_grade_category_tenant','course_creation_broker_category_assignment_tenant','course_creation_broker_letter_band_tenant','base_course_install_broker_recipe','base_course_install_broker_account','base_course_install_broker_completion_receipt') ORDER BY pol.polname")
        .fetch_all(pool).await.expect("authority policy catalog");
    assert_eq!(rows.len(), 19, "all closed-authority RLS policies exist");
    for (name, relation, command, permissive, roles, using, check) in rows {
        assert!(permissive, "{name} is permissive");
        if name.starts_with("course_creation_broker_") {
            assert_eq!(command, "*", "{name} covers all commands");
            assert_eq!(roles, vec!["ple_course_creation_broker".to_owned()]);
            assert_eq!(
                relation,
                match name.as_str() {
                    "course_creation_broker_course_tenant" => "course",
                    "course_creation_broker_member_tenant" => "course_member",
                    "course_creation_broker_roster_tenant" => "course_roster_state",
                    "course_creation_broker_group_tenant" => "course_group_membership_policy",
                    "course_creation_broker_scheme_tenant" => "course_grade_scheme",
                    "course_creation_broker_appearance_tenant" => "course_appearance",
                    "course_creation_broker_session_tenant" => "auth_session",
                    "course_creation_broker_profile_tenant" => "course_roster_profile",
                    "course_creation_broker_identity_tenant" => "tenant_learner_identity",
                    "course_creation_broker_assignment_tenant" => "assignment",
                    "course_creation_broker_domain_tenant" => "course_allowed_email_domain",
                    "course_creation_broker_course_group_tenant" => "course_group",
                    "course_creation_broker_group_member_tenant" => "course_group_member",
                    "course_creation_broker_grade_category_tenant" => "course_grade_category",
                    "course_creation_broker_category_assignment_tenant" => {
                        "course_grade_category_assignment"
                    }
                    "course_creation_broker_letter_band_tenant" => "course_grade_letter_band",
                    _ => unreachable!("closed course policy name"),
                },
                "{name} relation"
            );
            assert_eq!(using.as_deref(), Some("(tenant_id = ple_current_tenant())"));
            if !matches!(
                name.as_str(),
                "course_creation_broker_appearance_tenant"
                    | "course_creation_broker_session_tenant"
                    | "course_creation_broker_profile_tenant"
                    | "course_creation_broker_identity_tenant"
                    | "course_creation_broker_assignment_tenant"
                    | "course_creation_broker_domain_tenant"
                    | "course_creation_broker_course_group_tenant"
                    | "course_creation_broker_group_member_tenant"
                    | "course_creation_broker_grade_category_tenant"
                    | "course_creation_broker_category_assignment_tenant"
                    | "course_creation_broker_letter_band_tenant"
            ) {
                assert_eq!(check.as_deref(), Some("(tenant_id = ple_current_tenant())"));
            } else {
                assert!(check.is_none(), "{name} has no WITH CHECK");
            }
        } else {
            assert_eq!(roles, vec!["ple_base_course_install_broker".to_owned()]);
            assert_eq!(
                relation,
                match name.as_str() {
                    "base_course_install_broker_recipe" => "live_demo_install_recipe",
                    "base_course_install_broker_account" => "ple_account",
                    "base_course_install_broker_completion_receipt" => {
                        "live_demo_install_completion_receipt"
                    }
                    _ => unreachable!("closed installer policy name"),
                }
            );
            if name == "base_course_install_broker_completion_receipt" {
                assert_eq!(command, "a");
                assert!(using.is_none());
            } else {
                assert_eq!(command, "*");
                assert_eq!(using.as_deref(), Some("true"));
            }
            assert_eq!(check.as_deref(), Some("true"));
        }
    }
}

async fn relation_catalog(pool: &PgPool) {
    const RELATIONS: &[&str] = &[
        "course",
        "course_member",
        "course_roster_state",
        "course_group_membership_policy",
        "course_grade_scheme",
        "course_appearance",
        "auth_session",
        "course_roster_profile",
        "tenant_learner_identity",
        "assignment",
        "course_allowed_email_domain",
        "course_group",
        "course_group_member",
        "course_grade_category",
        "course_grade_category_assignment",
        "course_grade_letter_band",
        "instructor_approval",
        "live_demo_install_state",
        "live_demo_install_recipe",
        "live_demo_install_completion_receipt",
        "ple_account",
    ];
    const COURSE_BROKER_WRITE: &[&str] = &[
        "course",
        "course_member",
        "course_roster_state",
        "course_group_membership_policy",
        "course_grade_scheme",
        "course_appearance",
    ];
    const BASE_BROKER_RELATIONS: &[&str] = &[
        "live_demo_install_state",
        "live_demo_install_recipe",
        "ple_account",
        "instructor_approval",
    ];

    for role in [
        "ple_course_creation_broker",
        "ple_base_course_install_broker",
        "ple_base_course_installer",
    ] {
        for relation in RELATIONS {
            for privilege in [
                "SELECT",
                "INSERT",
                "UPDATE",
                "DELETE",
                "TRUNCATE",
                "REFERENCES",
                "TRIGGER",
            ] {
                let expected = match role {
                    "ple_course_creation_broker" => match privilege {
                        "SELECT" => {
                            !BASE_BROKER_RELATIONS.contains(relation)
                                && *relation != "live_demo_install_completion_receipt"
                        }
                        "INSERT" => COURSE_BROKER_WRITE.contains(relation),
                        "UPDATE" => false,
                        _ => false,
                    },
                    "ple_base_course_install_broker" => match privilege {
                        "SELECT" | "INSERT" | "UPDATE" => {
                            BASE_BROKER_RELATIONS.contains(relation)
                                || (*relation == "live_demo_install_completion_receipt"
                                    && privilege == "INSERT")
                        }
                        _ => false,
                    },
                    "ple_base_course_installer" => false,
                    _ => unreachable!("closed capability role"),
                };
                let granted: bool = sqlx::query_scalar("SELECT has_table_privilege($1,$2,$3)")
                    .bind(role)
                    .bind(format!("public.{relation}"))
                    .bind(privilege)
                    .fetch_one(pool)
                    .await
                    .expect("relation privilege matrix");
                assert_eq!(granted, expected, "{role} {relation} {privilege}");
            }
        }
    }

    for role in [
        "ple_course_creation_broker",
        "ple_base_course_install_broker",
    ] {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT c.table_name,c.column_name FROM information_schema.columns c WHERE c.table_schema='public' AND c.table_name = ANY($1) ORDER BY c.table_name,c.ordinal_position",
        )
        .bind(RELATIONS)
        .fetch_all(pool)
        .await
        .expect("relation columns");
        for (relation, column) in rows {
            let expected = match role {
                "ple_course_creation_broker" => matches!(
                    (relation.as_str(), column.as_str()),
                    ("course", "course_id")
                        | ("course_member", "course_membership_id")
                        | ("course_roster_state", "course_id")
                        | ("auth_session", "session_hash")
                ),
                "ple_base_course_install_broker" => {
                    BASE_BROKER_RELATIONS.contains(&relation.as_str())
                }
                _ => unreachable!("closed broker role"),
            };
            let granted: bool =
                sqlx::query_scalar("SELECT has_column_privilege($1,$2,$3,'UPDATE')")
                    .bind(role)
                    .bind(format!("public.{relation}"))
                    .bind(&column)
                    .fetch_one(pool)
                    .await
                    .expect("column update matrix");
            assert_eq!(granted, expected, "{role} {relation}.{column} UPDATE");
        }
    }

    for sequence in [
        "course_public_id_seq",
        "course_member_public_id_seq",
        "ple_account_public_id_seq",
    ] {
        for role in [
            "ple_course_creation_broker",
            "ple_base_course_install_broker",
            "ple_base_course_installer",
        ] {
            for privilege in ["USAGE", "SELECT", "UPDATE"] {
                let expected = match role {
                    "ple_course_creation_broker" => {
                        sequence != "ple_account_public_id_seq" && privilege != "UPDATE"
                    }
                    "ple_base_course_install_broker" => {
                        sequence == "ple_account_public_id_seq" && privilege != "UPDATE"
                    }
                    "ple_base_course_installer" => false,
                    _ => unreachable!("closed capability role"),
                };
                let granted: bool = sqlx::query_scalar("SELECT has_sequence_privilege($1,$2,$3)")
                    .bind(role)
                    .bind(format!("public.{sequence}"))
                    .bind(privilege)
                    .fetch_one(pool)
                    .await
                    .expect("sequence privilege matrix");
                assert_eq!(granted, expected, "{role} {sequence} {privilege}");
            }
        }
    }

    let mut expected_table_acls = vec![
        ("public.course", "ple_course_creation_broker", "INSERT"),
        ("public.course", "ple_course_creation_broker", "SELECT"),
        (
            "public.course_member",
            "ple_course_creation_broker",
            "INSERT",
        ),
        (
            "public.course_member",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_roster_state",
            "ple_course_creation_broker",
            "INSERT",
        ),
        (
            "public.course_roster_state",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_group_membership_policy",
            "ple_course_creation_broker",
            "INSERT",
        ),
        (
            "public.course_group_membership_policy",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_grade_scheme",
            "ple_course_creation_broker",
            "INSERT",
        ),
        (
            "public.course_grade_scheme",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_appearance",
            "ple_course_creation_broker",
            "INSERT",
        ),
        (
            "public.course_appearance",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.auth_session",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_roster_profile",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.tenant_learner_identity",
            "ple_course_creation_broker",
            "SELECT",
        ),
        ("public.assignment", "ple_course_creation_broker", "SELECT"),
        (
            "public.course_allowed_email_domain",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_group",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_group_member",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_grade_category",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_grade_category_assignment",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_grade_letter_band",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.live_demo_install_completion_receipt",
            "ple_base_course_install_broker",
            "INSERT",
        ),
    ];
    for relation in BASE_BROKER_RELATIONS {
        for privilege in ["SELECT", "INSERT", "UPDATE"] {
            expected_table_acls.push((
                match *relation {
                    "live_demo_install_state" => "public.live_demo_install_state",
                    "live_demo_install_recipe" => "public.live_demo_install_recipe",
                    "ple_account" => "public.ple_account",
                    "instructor_approval" => "public.instructor_approval",
                    _ => unreachable!("closed installer relation"),
                },
                "ple_base_course_install_broker",
                privilege,
            ));
        }
    }
    expected_table_acls.sort_unstable();
    let expected_table_acls: Vec<(String, String, String)> = expected_table_acls
        .into_iter()
        .map(|(relation, role, privilege)| {
            (relation.to_owned(), role.to_owned(), privilege.to_owned())
        })
        .collect();
    let mut actual_table_acls: Vec<(String, String, String)> = sqlx::query_as("SELECT format('%I.%I',n.nspname,c.relname),r.rolname,acl.privilege_type FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) acl JOIN pg_roles r ON r.oid=acl.grantee WHERE n.nspname='public' AND c.relkind IN ('r','p') AND r.rolname IN ('ple_course_creation_broker','ple_base_course_installer','ple_base_course_install_broker') AND acl.grantee<>c.relowner")
        .fetch_all(pool).await.expect("direct table ACL catalog");
    actual_table_acls.sort_unstable();
    assert_eq!(
        actual_table_acls, expected_table_acls,
        "exact direct table ACL matrix"
    );

    let mut expected_column_acls = vec![
        ("public.course", "course_id", "ple_course_creation_broker"),
        (
            "public.course_member",
            "course_membership_id",
            "ple_course_creation_broker",
        ),
        (
            "public.course_roster_state",
            "course_id",
            "ple_course_creation_broker",
        ),
        (
            "public.auth_session",
            "session_hash",
            "ple_course_creation_broker",
        ),
    ];
    expected_column_acls.sort_unstable();
    let expected_column_acls: Vec<(String, String, String)> = expected_column_acls
        .into_iter()
        .map(|(relation, column, role)| (relation.to_owned(), column.to_owned(), role.to_owned()))
        .collect();
    let mut actual_column_acls: Vec<(String, String, String)> = sqlx::query_as("SELECT format('%I.%I',n.nspname,c.relname),a.attname,r.rolname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_attribute a ON a.attrelid=c.oid CROSS JOIN LATERAL aclexplode(a.attacl) acl JOIN pg_roles r ON r.oid=acl.grantee WHERE n.nspname='public' AND c.relkind IN ('r','p') AND a.attnum>0 AND NOT a.attisdropped AND acl.privilege_type='UPDATE' AND r.rolname IN ('ple_course_creation_broker','ple_base_course_installer','ple_base_course_install_broker') AND acl.grantee<>c.relowner")
        .fetch_all(pool).await.expect("direct column ACL catalog");
    actual_column_acls.sort_unstable();
    assert_eq!(
        actual_column_acls, expected_column_acls,
        "exact direct column UPDATE ACL matrix"
    );

    let mut expected_sequence_acls = vec![
        (
            "public.course_public_id_seq",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_public_id_seq",
            "ple_course_creation_broker",
            "USAGE",
        ),
        (
            "public.course_member_public_id_seq",
            "ple_course_creation_broker",
            "SELECT",
        ),
        (
            "public.course_member_public_id_seq",
            "ple_course_creation_broker",
            "USAGE",
        ),
        (
            "public.ple_account_public_id_seq",
            "ple_base_course_install_broker",
            "SELECT",
        ),
        (
            "public.ple_account_public_id_seq",
            "ple_base_course_install_broker",
            "USAGE",
        ),
    ];
    expected_sequence_acls.sort_unstable();
    let expected_sequence_acls: Vec<(String, String, String)> = expected_sequence_acls
        .into_iter()
        .map(|(sequence, role, privilege)| {
            (sequence.to_owned(), role.to_owned(), privilege.to_owned())
        })
        .collect();
    let mut actual_sequence_acls: Vec<(String, String, String)> = sqlx::query_as("SELECT format('%I.%I',n.nspname,c.relname),r.rolname,acl.privilege_type FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('S',c.relowner))) acl JOIN pg_roles r ON r.oid=acl.grantee WHERE n.nspname='public' AND c.relkind='S' AND r.rolname IN ('ple_course_creation_broker','ple_base_course_installer','ple_base_course_install_broker') AND acl.grantee<>c.relowner")
        .fetch_all(pool).await.expect("direct sequence ACL catalog");
    actual_sequence_acls.sort_unstable();
    assert_eq!(
        actual_sequence_acls, expected_sequence_acls,
        "exact direct sequence ACL matrix"
    );
}

#[tokio::test]
#[ignore = "requires the private acceptance runtime workspace"]
async fn course_creation_authority_catalog_is_closed_and_minimal() {
    let pool = pool().await;
    role_catalog(&pool).await;
    function_catalog(&pool).await;
    policy_catalog(&pool).await;
    relation_catalog(&pool).await;
    completion_verifier::catalog(&pool).await;
    freshness::catalog(&pool).await;
    invitation_broker::catalog(&pool).await;
    roster_mutator::catalog(&pool).await;
}
