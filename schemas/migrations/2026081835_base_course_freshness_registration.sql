-- Keep the Base Course freshness capability aligned with every repository-owned relation.
-- The migration runner repeats this catalog reconciliation after every embedded epoch run.
DO $$
DECLARE
    target_relation record;
    target_policy record;
    target_column record;
    membership_edge record;
BEGIN
    ALTER ROLE ple_base_course_freshness_broker
        NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

    REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public
        FROM ple_base_course_freshness_broker;
    REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public
        FROM ple_base_course_freshness_broker;

    FOR target_column IN
        SELECT namespace.nspname, table_row.relname, attribute.attname
          FROM pg_catalog.pg_attribute AS attribute
          JOIN pg_catalog.pg_class AS table_row ON table_row.oid = attribute.attrelid
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = table_row.relnamespace
         WHERE namespace.nspname = 'public'
           AND table_row.relkind IN ('r', 'p')
           AND attribute.attnum > 0
           AND NOT attribute.attisdropped
           AND EXISTS (
               SELECT 1
                 FROM aclexplode(attribute.attacl) AS privilege
                WHERE privilege.grantee = 'ple_base_course_freshness_broker'::regrole
           )
    LOOP
        EXECUTE format(
            'REVOKE ALL PRIVILEGES (%I) ON TABLE %I.%I FROM ple_base_course_freshness_broker',
            target_column.attname,
            target_column.nspname,
            target_column.relname
        );
    END LOOP;

    FOR membership_edge IN
        SELECT parent_role.rolname AS parent_name, member_role.rolname AS member_name
          FROM pg_catalog.pg_auth_members AS membership
          JOIN pg_catalog.pg_roles AS parent_role ON parent_role.oid = membership.roleid
          JOIN pg_catalog.pg_roles AS member_role ON member_role.oid = membership.member
         WHERE membership.member = 'ple_base_course_freshness_broker'::regrole
            OR membership.roleid = 'ple_base_course_freshness_broker'::regrole
    LOOP
        EXECUTE format(
            'REVOKE %I FROM %I',
            membership_edge.parent_name,
            membership_edge.member_name
        );
    END LOOP;

    FOR target_policy IN
        SELECT namespace.nspname, table_row.relname, policy_catalog.polname
         FROM pg_catalog.pg_policy AS policy_catalog
          JOIN pg_catalog.pg_class AS table_row ON table_row.oid = policy_catalog.polrelid
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = table_row.relnamespace
         WHERE namespace.nspname = 'public'
           AND table_row.relkind IN ('r', 'p')
           AND (
               policy_catalog.polname = 'ple_base_course_freshness_select'
               OR 'ple_base_course_freshness_broker'::regrole::oid = ANY(policy_catalog.polroles)
           )
    LOOP
        EXECUTE format(
            'DROP POLICY %I ON %I.%I',
            target_policy.polname,
            target_policy.nspname,
            target_policy.relname
        );
    END LOOP;

    FOR target_relation IN
        SELECT namespace.nspname, table_row.relname, table_row.relrowsecurity
          FROM pg_catalog.pg_class AS table_row
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = table_row.relnamespace
         WHERE namespace.nspname = 'public'
           AND table_row.relkind IN ('r', 'p')
           AND table_row.relname <> '_sqlx_migrations'
         ORDER BY namespace.nspname, table_row.relname, table_row.oid
    LOOP
        EXECUTE format(
            'GRANT SELECT, MAINTAIN ON TABLE %I.%I TO ple_base_course_freshness_broker',
            target_relation.nspname,
            target_relation.relname
        );
        IF target_relation.relrowsecurity THEN
            EXECUTE format(
                'CREATE POLICY ple_base_course_freshness_select ON %I.%I FOR SELECT '
                || 'TO ple_base_course_freshness_broker USING (true)',
                target_relation.nspname,
                target_relation.relname
            );
        END IF;
    END LOOP;
END
$$;
