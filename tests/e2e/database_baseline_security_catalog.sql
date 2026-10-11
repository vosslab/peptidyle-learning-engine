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
	IF NOT has_function_privilege(
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

END
$$;

-- C24 authorization contract: an active Sysadmin receives the explicit
-- platform-administration predicate but no Course membership or Course-record
-- read authority merely by holding that User Role. The transaction rolls
-- back its synthetic Account, leaving the canonical baseline unchanged.
BEGIN;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, user_role, created_at)
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
        RAISE EXCEPTION 'Sysadmin User Role unexpectedly grants Course-record authority';
    END IF;
END
$$;
ROLLBACK;

-- Confirmed Sysadmin access to one Student roster record is direct, audited,
-- and derived from the authenticated session. Confirmation and audit are in
-- the same transaction as the protected projection.
-- BEGIN sysadmin_student_data_access
BEGIN;
SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('00000000-0000-0000-0000-00000000c291', 'Sysadmin access fixture');
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES ('U00000009', 'instructor', pg_catalog.transaction_timestamp())
RETURNING account_id AS access_instructor_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES ('U00000009', 'sysadmin', pg_catalog.transaction_timestamp())
RETURNING account_id AS access_sysadmin_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES ('U00000009', 'student', pg_catalog.transaction_timestamp())
RETURNING account_id AS access_student_id \gset
SET LOCAL ROLE ple_api_owner;
SELECT pg_catalog.set_config('ple.session_account_id', :'access_instructor_id', true);
SELECT course_instance_id AS access_course_id
  FROM ple_api.create_course_instance(
    'CI0000000Y',
    '00000000-0000-0000-0000-00000000c292'::uuid,
    '00000000-0000-0000-0000-00000000c293'::uuid,
    '00000000-0000-0000-0000-00000000c294'::uuid,
    'empty', NULL, NULL,
    'ACCESS', 'Confirmed Student access fixture',
    CURRENT_DATE, CURRENT_DATE,
    NULL, '[]'::jsonb,
    '00000000-0000-0000-0000-00000000c291'::uuid,
    NULL, NULL, NULL, ARRAY[]::text[]
  ) \gset
SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_private.course_roster_profile(
    course_roster_profile_id, course_instance_id, student_account_id,
    roster_id, roster_name, created_at
) VALUES (
    '00000000-0000-0000-0000-00000000c295'::uuid, :'access_course_id',
    :'access_student_id', 'access-student', 'Synthetic Student',
    pg_catalog.transaction_timestamp()
);
SET LOCAL ROLE ple_api_owner;
SELECT pg_catalog.set_config('ple.session_account_id', :'access_instructor_id', true);
SELECT pg_catalog.set_config('ple.access_course_id', :'access_course_id', true);
SELECT pg_catalog.set_config('ple.access_student_id', :'access_student_id', true);
SELECT pg_catalog.set_config('ple.access_sysadmin_id', :'access_sysadmin_id', true);
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.read_sysadmin_student_roster_record(
            current_setting('ple.access_course_id'), 'access-student', true
        )
    ) THEN
        RAISE EXCEPTION 'Instructor session read protected Sysadmin Student data';
    END IF;
END
$$;
SELECT pg_catalog.set_config('ple.session_account_id', :'access_sysadmin_id', true);
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ple_api.read_sysadmin_student_roster_record(
            current_setting('ple.access_course_id'), 'access-student', false
        )
    ) THEN
        RAISE EXCEPTION 'unconfirmed Student-data access returned a record';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.read_sysadmin_student_roster_record(
            current_setting('ple.access_course_id'), 'access-student', true
        ) AS record
        WHERE record.course_instance_id = current_setting('ple.access_course_id')
          AND record.student_account_id = current_setting('ple.access_student_id')
          AND record.roster_id = 'access-student'
          AND record.roster_name = 'Synthetic Student'
          AND record.state = 'removed'
          AND record.event_id IS NOT NULL
          AND record.occurred_at_millis > 0
    ) THEN
        RAISE EXCEPTION 'confirmed Sysadmin Student-data projection is incomplete';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.course_membership
         WHERE course_instance_id = current_setting('ple.access_course_id')
           AND account_id = current_setting('ple.access_sysadmin_id')
    ) THEN
        RAISE EXCEPTION 'Sysadmin Student-data access added Course membership';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_proc AS routine
          CROSS JOIN LATERAL pg_catalog.aclexplode(
              COALESCE(routine.proacl, pg_catalog.acldefault('f', routine.proowner))
          ) AS privilege
         WHERE routine.oid = pg_catalog.to_regprocedure(
                   'ple_api.read_sysadmin_student_roster_record(text, text, boolean)'
               )
           AND privilege.grantee = 0
           AND privilege.privilege_type = 'EXECUTE'
    ) OR NOT has_function_privilege(
        'ple_app', 'ple_api.read_sysadmin_student_roster_record(text, text, boolean)', 'EXECUTE'
    ) THEN
        RAISE EXCEPTION 'Sysadmin Student-data reader execute privilege is wrong';
    END IF;
END
$$;
SET LOCAL ROLE ple_audit_owner;
ALTER TABLE ple_audit.course_roster_event NO FORCE ROW LEVEL SECURITY;
DO $$
BEGIN
    IF (
        SELECT count(*) FROM ple_audit.course_roster_event AS event
         WHERE event.course_instance_id = current_setting('ple.access_course_id')
           AND event.student_account_id = current_setting('ple.access_student_id')
           AND event.acting_account_id = current_setting('ple.access_sysadmin_id')
           AND event.event_kind = 'sysadmin_student_data_accessed'
    ) IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'confirmed Sysadmin Student-data read was not audited exactly once';
    END IF;
END
$$;
ALTER TABLE ple_audit.course_roster_event FORCE ROW LEVEL SECURITY;
SELECT 'confirmed Sysadmin Student-data access proof passed' AS sysadmin_student_data_access_proof;
ROLLBACK;
-- END sysadmin_student_data_access


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
