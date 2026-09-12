-- Stable PostgreSQL authorization oracle for the canonical database baseline.
-- The connected baseline owner runs this as the actual ple_migrator login after
-- application-role verification. It asserts durable catalog boundaries rather
-- than a full catalog or file layout.
DO $$
BEGIN
	IF NOT EXISTS (
		SELECT 1
		FROM pg_class relation
		JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
		WHERE namespace.nspname = 'ple_migration'
			AND relation.relname = '_sqlx_migrations'
	)
		OR EXISTS (
			SELECT 1
			FROM pg_class relation
			JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
			WHERE namespace.nspname = 'public'
				AND relation.relname = '_sqlx_migrations'
		) THEN
		RAISE EXCEPTION 'SQLx ledger is not restricted to ple_migration';
	END IF;
	IF to_regclass('ple_api.ple_schema_state') IS NULL
		OR NOT has_table_privilege('ple_app', 'ple_api.ple_schema_state', 'SELECT')
		OR has_table_privilege('ple_app', 'ple_migration._sqlx_migrations', 'SELECT') THEN
		RAISE EXCEPTION 'application schema-state access is not restricted';
	END IF;
	IF current_user <> 'ple_migrator'
		OR NOT has_schema_privilege(current_user, 'ple_api', 'USAGE')
		OR NOT has_table_privilege(current_user, 'ple_api.ple_schema_state', 'SELECT')
		OR has_table_privilege(current_user, 'ple_api.ple_schema_state', 'INSERT')
		OR has_table_privilege(current_user, 'ple_api.ple_schema_state', 'UPDATE')
		OR has_schema_privilege(current_user, 'ple_api', 'CREATE')
		OR pg_has_role(current_user, 'ple_app', 'MEMBER')
		OR has_function_privilege(current_user, 'ple_api.current_session_account_id()', 'EXECUTE') THEN
		RAISE EXCEPTION 'migrator schema-state inspection is not read-only and isolated';
	END IF;
	IF has_database_privilege('ple_app', current_database(), 'CREATE')
		OR EXISTS (
			SELECT 1
			FROM pg_namespace
			WHERE nspname LIKE 'ple\_%' ESCAPE '\'
				AND has_schema_privilege('ple_app', oid, 'CREATE')
		) THEN
		RAISE EXCEPTION 'application capability retains DDL authority';
	END IF;
	IF NOT COALESCE((
		SELECT has_table_privilege('ple_private_owner', relation.oid, 'SELECT')
		  FROM pg_class AS relation
		  JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
		 WHERE namespace.nspname = 'ple_data'
		   AND relation.relname = 'question_current_owner'
	), false)
		OR COALESCE((
			SELECT has_table_privilege('ple_app', relation.oid, 'SELECT')
			  FROM pg_class AS relation
			  JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
			 WHERE namespace.nspname = 'ple_data'
			   AND relation.relname = 'question_current_owner'
		), false) THEN
		RAISE EXCEPTION 'Question publication owner view is not restricted to its publication capability';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_namespace namespace
		CROSS JOIN LATERAL aclexplode(
			COALESCE(namespace.nspacl, acldefault('n', namespace.nspowner))
		) privilege
		WHERE namespace.nspname LIKE 'ple\_%' ESCAPE '\' AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'PUBLIC retains privilege on a PLE schema';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_class relation
		JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
		WHERE namespace.nspname IN ('ple_data', 'ple_private', 'ple_audit')
			AND relation.relkind IN ('r', 'p')
			AND (NOT relation.relrowsecurity OR NOT relation.relforcerowsecurity)
	) THEN
		RAISE EXCEPTION 'PLE protected tables do not force row-level security';
	END IF;
END
$$;

-- Bootstrap creates the role graph as the platform administrator. The
-- restricted migrator receives exactly the four schema owners plus its
-- database-owner and Unrelease capabilities, each as SET-only membership.
DO $$
BEGIN
	IF (
		SELECT count(*)
		  FROM pg_catalog.pg_auth_members AS membership
		 WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = current_user)
	) <> 6 OR (
		SELECT count(*)
		  FROM pg_catalog.pg_auth_members AS membership
		  JOIN pg_catalog.pg_roles AS granted_role ON granted_role.oid = membership.roleid
		 WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = current_user)
		   AND granted_role.rolname IN (
			   'ple_database_owner',
			   'ple_unrelease_executor',
			   'ple_data_owner',
			   'ple_private_owner',
			   'ple_audit_owner',
			   'ple_api_owner'
		   )
		   AND NOT membership.admin_option
		   AND NOT membership.inherit_option
		   AND membership.set_option
	) <> 6
	OR pg_has_role(current_user, 'ple_app', 'MEMBER')
	OR pg_has_role(current_user, 'ple_auth', 'MEMBER')
	OR pg_has_role(current_user, 'ple_student', 'MEMBER') THEN
		RAISE EXCEPTION 'migrator role memberships exceed the SET-only bootstrap boundary';
	END IF;
END
$$;

SET ROLE ple_data_owner;
DO $$
BEGIN
	IF current_user <> 'ple_data_owner' THEN
		RAISE EXCEPTION 'migrator could not assume its explicit data-owner role';
	END IF;
END
$$;
RESET ROLE;

DO $$
BEGIN
	BEGIN
		EXECUTE 'SET LOCAL ROLE ple_app';
		RAISE EXCEPTION 'migrator unexpectedly assumed the application capability';
	EXCEPTION WHEN insufficient_privilege THEN
		NULL;
	END;
END
$$;

-- This catalog runs as the actual migrator login.  Verify its coordinator
-- projection works while a DDL attempt is rejected at the schema boundary.
SELECT base_release FROM ple_api.ple_schema_state LIMIT 1;
DO $$
BEGIN
	BEGIN
		EXECUTE 'CREATE TABLE ple_api.migrator_privilege_probe (id integer)';
		RAISE EXCEPTION 'migrator unexpectedly created an API-schema object';
	EXCEPTION WHEN insufficient_privilege THEN
		NULL;
	END;
	BEGIN
		EXECUTE 'INSERT INTO ple_api.ple_schema_state (base_release) VALUES (''invalid'')';
		RAISE EXCEPTION 'migrator unexpectedly wrote schema state';
	EXCEPTION WHEN insufficient_privilege OR object_not_in_prerequisite_state THEN
		NULL;
	END;
	BEGIN
		EXECUTE 'UPDATE ple_api.ple_schema_state SET base_release = ''invalid''';
		RAISE EXCEPTION 'migrator unexpectedly updated schema state';
	EXCEPTION WHEN insufficient_privilege OR object_not_in_prerequisite_state THEN
		NULL;
	END;
	BEGIN
		PERFORM ple_api.current_session_account_id();
		RAISE EXCEPTION 'migrator unexpectedly executed an application procedure';
	EXCEPTION WHEN insufficient_privilege THEN
		NULL;
	END;
END
$$;
