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
	IF to_regprocedure('ple_api.sweep_expired_student_assignment_attempts(integer)') IS NOT NULL
		OR NOT has_function_privilege(
			'ple_assessment_attempt_expiry_worker',
			'ple_api.prepare_expired_student_assessment_attempt_finalizations(integer)',
			'EXECUTE'
		)
		OR NOT has_function_privilege(
			'ple_assessment_attempt_expiry_worker',
			'ple_api.commit_expired_student_assessment_attempt_finalization(uuid,jsonb)',
			'EXECUTE'
		)
		OR has_function_privilege(
			'ple_app',
			'ple_api.prepare_expired_student_assessment_attempt_finalizations(integer)',
			'EXECUTE'
		)
		OR has_function_privilege(
			'ple_app',
			'ple_api.commit_expired_student_assessment_attempt_finalization(uuid,jsonb)',
			'EXECUTE'
		) THEN
		RAISE EXCEPTION 'Assignment Attempt expiry finalization capability is not isolated';
	END IF;
	IF to_regrole('ple_imathas_question_backend_grading_worker') IS NOT NULL
		OR to_regrole('ple_native_ple_grading_worker') IS NOT NULL
		OR to_regrole('ple_webwork_grading_worker') IS NOT NULL
		OR to_regprocedure('ple_api.stage_verified_imathas_result(uuid,uuid,uuid,uuid,text,text,text,integer,uuid,bytea,text,numeric,text,bytea,bytea,double precision,bytea,uuid,uuid,uuid,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.claim_imathas_result_grading_job(uuid,uuid,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.commit_imathas_result_grading(uuid,uuid,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.fail_imathas_question_backend_grading_retryable(uuid,uuid,timestamptz,text)') IS NOT NULL
		OR to_regprocedure('ple_api.fail_imathas_question_backend_grading_final(uuid,uuid,text)') IS NOT NULL
		OR to_regprocedure('ple_api.claim_native_ple_grading_job(uuid,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.commit_native_ple_grading(uuid,uuid,boolean,double precision,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.fail_native_ple_grading_retryable(uuid,uuid,timestamptz,text)') IS NOT NULL
		OR to_regprocedure('ple_api.fail_native_ple_grading_final(uuid,uuid,text)') IS NOT NULL
		OR to_regprocedure('ple_api.claim_webwork_grading_job(uuid,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.commit_webwork_grading(uuid,uuid,boolean,double precision,timestamptz)') IS NOT NULL
		OR to_regprocedure('ple_api.fail_webwork_grading_retryable(uuid,uuid,timestamptz,text)') IS NOT NULL
		OR to_regprocedure('ple_api.fail_webwork_grading_final(uuid,uuid,text)') IS NOT NULL THEN
		RAISE EXCEPTION 'retired grading Job capability remains';
	END IF;
	IF to_regprocedure('ple_api.read_attempt_grading_status(bigint)') IS NOT NULL
		OR to_regprocedure('ple_api.read_attempt_grading_detail(bigint,bigint)') IS NOT NULL
		OR to_regprocedure('ple_api.finalize_student_assignment_attempt(bigint)') IS NOT NULL
		OR to_regprocedure(
				'ple_api.retry_grading_for_question_attempt(bigint,bigint,bigint,integer,bigint)'
			) IS NOT NULL
			OR EXISTS (
				SELECT 1
				  FROM pg_proc AS routine
				  JOIN pg_namespace AS namespace ON namespace.oid = routine.pronamespace
				 WHERE namespace.nspname = 'ple_private'
				   AND routine.proname IN (
					   'retry_grade_accepted_submission',
					   'resolve_instructor_question_attempt_for_grading'
				   )
			)
		OR EXISTS (
			SELECT 1
			  FROM pg_proc AS routine
			  JOIN pg_namespace AS namespace ON namespace.oid = routine.pronamespace
			 WHERE namespace.nspname = 'ple_private'
			   AND routine.proname IN (
				   'read_student_attempt_grading_status',
				   'read_instructor_attempt_grading_detail',
				   'public_grading_operation_state',
				   'finalize_student_assignment_attempt',
				   'finalize_assignment_attempt_saved_responses'
			   )
		) THEN
			RAISE EXCEPTION 'retired public grading readers or mutation capability remains';
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
			-- The append-only public-ID registry is a private allocation
			-- authority, not a product table; its table ACL and immutable
			-- trigger provide its boundary instead of row visibility.
			AND NOT (namespace.nspname = 'ple_private'
				AND relation.relname = 'public_id_reservation')
			AND (NOT relation.relrowsecurity OR NOT relation.relforcerowsecurity)
	) THEN
		RAISE EXCEPTION 'PLE protected tables do not force row-level security';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_class AS relation
		JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
		CROSS JOIN LATERAL aclexplode(
			COALESCE(relation.relacl, acldefault('r', relation.relowner))
		) AS privilege
		WHERE namespace.nspname = 'ple_private'
		  AND relation.relname = 'public_id_reservation'
		  AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'PUBLIC retains privilege on public-ID reservations';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_class AS relation
		JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
		CROSS JOIN (VALUES ('ple_app'::name), ('ple_auth'::name), ('ple_student'::name)) AS role_name(rolname)
		CROSS JOIN (VALUES
			('SELECT'::text), ('INSERT'::text), ('UPDATE'::text),
			('DELETE'::text), ('TRUNCATE'::text), ('REFERENCES'::text), ('TRIGGER'::text)
		) AS requested(privilege_type)
		WHERE namespace.nspname = 'ple_private'
		  AND relation.relname = 'public_id_reservation'
		  AND has_table_privilege(role_name.rolname, relation.oid, requested.privilege_type)
	) THEN
		RAISE EXCEPTION 'an application capability can access public-ID reservations directly';
	END IF;
	IF NOT EXISTS (
		SELECT 1
		FROM pg_trigger AS trigger_row
		JOIN pg_class AS relation ON relation.oid = trigger_row.tgrelid
		JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
		WHERE namespace.nspname = 'ple_private'
		  AND relation.relname = 'public_id_reservation'
		  AND trigger_row.tgname = 'public_id_reservation_is_permanent'
		  AND NOT trigger_row.tgisinternal
		  AND trigger_row.tgenabled = 'O'
		  AND trigger_row.tgfoid = (
			SELECT routine.oid
			  FROM pg_proc AS routine
			  JOIN pg_namespace AS routine_namespace ON routine_namespace.oid = routine.pronamespace
			 WHERE routine_namespace.nspname = 'ple_private'
			   AND routine.proname = 'reject_public_id_reservation_change'
			   AND routine.pronargs = 0
		  )
	) THEN
		RAISE EXCEPTION 'public-ID reservations lack their enabled immutable trigger';
	END IF;
END
$$;

-- C24 authorization contract: an active Sysadmin receives the explicit
-- platform-administration predicate but no Course membership or Course-record
-- read authority merely by holding that Product Role. The transaction rolls
-- back its synthetic Account, leaving the canonical baseline unchanged.
BEGIN;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, product_role, created_at)
VALUES ('U00000009', 'sysadmin', pg_catalog.transaction_timestamp())
RETURNING account_id AS sysadmin_account_id \gset
SET LOCAL ROLE ple_api_owner;
SELECT pg_catalog.set_config(
    'ple.test_sysadmin_account_id', :'sysadmin_account_id', true
);
-- C803: a primary-authentication caller cannot issue a Sysadmin session
-- through the ordinary session boundary. If this fails, restore mandatory
-- TOTP enforcement rather than relaxing the denial or adding a bypass.
DO $$
BEGIN
    BEGIN
        PERFORM ple_api.create_authenticated_session(
            '00000000-0000-0000-0000-00000000c242'::uuid,
            pg_catalog.current_setting('ple.test_sysadmin_account_id'),
            decode(repeat('24', 32), 'hex'), 900
        );
        RAISE EXCEPTION 'ordinary issuance bypassed mandatory Sysadmin TOTP';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
END
$$;
SET LOCAL ROLE ple_private_owner;
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_private.authenticated_session
        WHERE account_id = pg_catalog.current_setting('ple.test_sysadmin_account_id')
    ) THEN
        RAISE EXCEPTION 'denied Sysadmin issuance left an authenticated session';
    END IF;
END
$$;
SET LOCAL ROLE ple_api_owner;
SELECT pg_catalog.set_config(
    'ple.session_account_id', :'sysadmin_account_id', true
);
DO $$
BEGIN
    IF NOT ple_api.current_session_account_has_platform_administration()
       OR NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'active Sysadmin lacks platform-administration authority';
    END IF;
    IF ple_api.current_session_account_is_course_member(
        'CI0000000Y'
    ) OR ple_api.current_session_account_is_course_instructor(
        'CI0000000Y'
    ) THEN
        RAISE EXCEPTION 'Sysadmin Product Role unexpectedly grants Course-record authority';
    END IF;
END
$$;
ROLLBACK;

-- Bootstrap creates the role graph as the platform administrator. The
-- restricted migrator receives exactly the four schema owners plus its
-- database-owner, Unrelease, and Course-retention capabilities, each as
-- SET-only membership. Neither retention capability is a process login.
DO $$
BEGIN
	IF (
		SELECT count(*)
		  FROM pg_catalog.pg_auth_members AS membership
		 WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = current_user)
	) <> 9 OR (
		SELECT count(*)
		  FROM pg_catalog.pg_auth_members AS membership
		  JOIN pg_catalog.pg_roles AS granted_role ON granted_role.oid = membership.roleid
		 WHERE membership.member = (SELECT oid FROM pg_catalog.pg_roles WHERE rolname = current_user)
		   AND granted_role.rolname IN (
			   'ple_database_owner',
			   'ple_unrelease_executor',
			   'ple_course_retention_executor',
			   'ple_course_retention_notifier',
			   'ple_course_retention_notification_owner',
			   'ple_data_owner',
			   'ple_private_owner',
			   'ple_audit_owner',
			   'ple_api_owner'
		   )
		   AND NOT membership.admin_option
		   AND NOT membership.inherit_option
		   AND membership.set_option
	) <> 9
	OR pg_has_role(current_user, 'ple_app', 'MEMBER')
	OR pg_has_role(current_user, 'ple_auth', 'MEMBER')
	OR pg_has_role(current_user, 'ple_student', 'MEMBER') THEN
		RAISE EXCEPTION 'migrator role memberships exceed the SET-only bootstrap boundary';
	END IF;
END
$$;

DO $$
BEGIN
	IF NOT EXISTS (
		SELECT 1
		  FROM pg_catalog.pg_roles AS role
		 WHERE role.rolname = 'ple_course_retention_notification_owner'
		   AND NOT role.rolcanlogin
		   AND NOT role.rolinherit
		   AND NOT role.rolsuper
		   AND NOT role.rolcreatedb
		   AND NOT role.rolcreaterole
		   AND NOT role.rolreplication
		   AND NOT role.rolbypassrls
		   AND role.rolconnlimit = -1
	) THEN
		RAISE EXCEPTION 'Course-retention notification owner is not an ordinary no-login role';
	END IF;
	BEGIN
		EXECUTE 'SET LOCAL ROLE ple_course_retention_notification_owner';
		RESET ROLE;
	EXCEPTION WHEN insufficient_privilege THEN
		RAISE EXCEPTION 'migrator could not assume the Course-retention notification owner';
	END;
END
$$;

DO $$
BEGIN
	IF NOT EXISTS (
		SELECT 1
		  FROM pg_catalog.pg_roles AS role
		 WHERE role.rolname = 'ple_course_retention_notifier'
		   AND NOT role.rolcanlogin
		   AND NOT role.rolinherit
		   AND NOT role.rolsuper
		   AND NOT role.rolcreatedb
		   AND NOT role.rolcreaterole
		   AND NOT role.rolreplication
		   AND NOT role.rolbypassrls
		   AND role.rolconnlimit = -1
	) THEN
		RAISE EXCEPTION 'Course-retention notifier is not an ordinary no-login capability';
	END IF;
	IF ARRAY(
		SELECT routine.oid::regprocedure::text
		  FROM pg_catalog.pg_proc AS routine
		  JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = routine.pronamespace
		 -- Public pure helpers in ple_private/ple_data are intentionally
		 -- executable by PUBLIC; the notifier cannot reach those schemas.
		 WHERE namespace.nspname = 'ple_api'
		   AND has_function_privilege(
			   'ple_course_retention_notifier', routine.oid, 'EXECUTE'
		   )
		 ORDER BY routine.oid::regprocedure::text
	) <> ARRAY[
		'ple_api.claim_course_retention_notification(timestamp with time zone,integer)',
		'ple_api.fail_course_retention_notification_before_acceptance(uuid,uuid,timestamp with time zone,text)',
		'ple_api.record_course_retention_notification_provider_acceptance(uuid,uuid,uuid,timestamp with time zone)'
	] THEN
		RAISE EXCEPTION 'Course-retention notifier function capability is not exact';
	END IF;
	BEGIN
		EXECUTE 'SET LOCAL ROLE ple_course_retention_notifier';
		RESET ROLE;
	EXCEPTION WHEN insufficient_privilege THEN
		RAISE EXCEPTION 'migrator could not assume the Course-retention notifier needed for installation';
	END;
END
$$;

DO $$
BEGIN
	IF NOT EXISTS (
		SELECT 1
		  FROM pg_catalog.pg_roles AS role
		 WHERE role.rolname = 'ple_course_retention_executor'
		   AND NOT role.rolcanlogin
		   AND NOT role.rolinherit
		   AND NOT role.rolsuper
		   AND NOT role.rolcreatedb
		   AND NOT role.rolcreaterole
		   AND NOT role.rolreplication
		   AND NOT role.rolbypassrls
		   AND role.rolconnlimit = -1
	) THEN
		RAISE EXCEPTION 'Course-retention executor is not an ordinary no-login capability';
	END IF;
	-- Inactivity is executor-only even though it preserves all Student data.
	IF NOT EXISTS (
		SELECT 1 FROM pg_catalog.pg_proc AS routine
		WHERE routine.oid = to_regprocedure(
			'ple_api.mark_course_instance_inactive(text,timestamp with time zone)'
		)
		AND routine.proowner = to_regrole('ple_course_retention_executor')
		AND routine.prosecdef
		AND routine.proconfig = ARRAY['search_path=pg_catalog, ple_api, ple_data']
	) OR EXISTS (
		SELECT 1 FROM pg_catalog.pg_roles AS role
		WHERE role.rolname IN ('ple_app', 'ple_auth', 'ple_course_retention_notifier')
		AND has_function_privilege(
			role.oid,
				'ple_api.mark_course_instance_inactive(text,timestamp with time zone)',
			'EXECUTE'
		)
	) THEN
		RAISE EXCEPTION 'Course inactivity transition is not executor-only';
	END IF;
	BEGIN
		EXECUTE 'SET LOCAL ROLE ple_course_retention_executor';
		RESET ROLE;
	EXCEPTION WHEN insufficient_privilege THEN
		RAISE EXCEPTION 'migrator could not assume the Course-retention capability needed for installation';
	END;
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
SELECT base_release FROM ple_api.ple_schema_state ORDER BY base_release LIMIT 1;
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
