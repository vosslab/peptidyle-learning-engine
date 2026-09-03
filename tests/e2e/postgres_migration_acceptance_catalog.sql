-- PostgreSQL Migration Acceptance Runtime catalog and authorization acceptance oracle, executed only by e2e_postgres_migration_acceptance.sh against its disposable PostgreSQL 17 database.
DO $$
DECLARE
	role_name text;
	owner_name text;
	role_count integer;
	membership_count integer;
	unexpected_membership_detail text;
	column_count integer;
BEGIN
	IF current_database() <> 'ple_e2e_baseline' THEN
		RAISE EXCEPTION 'oracle connected to an unexpected database';
	END IF;
	IF (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_question_backend_session' AND column_name IN ('question_attempt_id', 'imathas_deployment_reference', 'imathas_item_reference', 'imathas_profile', 'question_seed', 'imathas_launch_binding_checksum', 'imathas_question_backend_state_key_id', 'imathas_question_backend_state_nonce', 'imathas_question_backend_state_ciphertext') AND is_nullable = 'NO') <> 9
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_question_backend_session' AND column_name = 'attempt_id')
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_question_backend_session' AND column_name = 'imathas_result_token_sha256')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_result_exchange' AND column_name = 'imathas_result_token_sha256' AND data_type = 'bytea' AND is_nullable = 'YES') OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_private.imathas_result_exchange'::regclass AND pg_get_constraintdef(oid) LIKE '%octet_length(imathas_result_token_sha256) = 32%')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_private.imathas_question_backend_session'::regclass AND conname = 'imathas_question_backend_session_state_key_nonce_is_unique')
		OR (SELECT count(*) FROM pg_proc proc JOIN pg_namespace namespace ON namespace.oid = proc.pronamespace JOIN pg_roles owner_role ON owner_role.oid = proc.proowner WHERE namespace.nspname = 'ple_api' AND proc.proname IN ('create_imathas_question_backend_session', 'load_imathas_question_backend_session', 'lease_imathas_question_backend_session', 'stage_verified_imathas_result') AND owner_role.rolname = 'ple_api_owner' AND proc.prosecdef AND array_to_string(proc.proconfig, ',') LIKE 'search_path=pg_catalog,%' AND has_function_privilege('ple_app', proc.oid, 'EXECUTE') AND NOT has_function_privilege('public', proc.oid, 'EXECUTE')) <> 4 THEN
		RAISE EXCEPTION 'iMathAS Question Backend Session Store schema/API boundary is incomplete';
	END IF;
	IF (SELECT count(*) FROM information_schema.columns
		WHERE table_schema = 'ple_private' AND table_name = 'question_source_registration'
		AND column_name IN ('source_object_id', 'source_object_checksum')
		AND data_type IN ('uuid', 'text') AND is_nullable = 'NO') <> 2
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private' AND table_name = 'question_source_registration'
			AND column_name IN ('question_type', 'public_content_checksum', 'source_data', 'source_checksum')
		) THEN
		RAISE EXCEPTION 'Question Source Registration does not have its exact object-backed authority';
	END IF;
	IF (SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname = current_database()) <> 'ple_database_owner' THEN
		RAISE EXCEPTION 'database owner is not ple_database_owner';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_roles WHERE rolname = 'ple_migrator' AND rolcanlogin
		AND NOT rolsuper AND NOT rolcreatedb AND rolcreaterole
		AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
		AND rolconnlimit = 2
	) THEN
		RAISE EXCEPTION 'ple_migrator attributes are not exact';
	END IF;
	FOREACH role_name IN ARRAY ARRAY['ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student', 'ple_imathas_question_backend_grading_worker'] LOOP
		IF NOT EXISTS (
			SELECT 1 FROM pg_roles WHERE rolname = role_name AND NOT rolcanlogin
			AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole
			AND NOT rolreplication AND NOT rolbypassrls AND NOT rolinherit
			AND rolconnlimit = -1
		) THEN
			RAISE EXCEPTION 'reserved role % has incorrect attributes', role_name;
		END IF;
	END LOOP;
	SELECT count(*)::integer INTO role_count
	FROM pg_roles
	WHERE rolname = ANY (ARRAY['ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student', 'ple_imathas_question_backend_grading_worker']);
	IF role_count <> 9 THEN
		RAISE EXCEPTION 'reserved role set is incomplete or duplicated';
	END IF;
	IF NOT has_schema_privilege('ple_imathas_question_backend_grading_worker', 'ple_api', 'USAGE')
		OR has_schema_privilege('ple_imathas_question_backend_grading_worker', 'ple_private', 'USAGE')
		OR has_schema_privilege('ple_imathas_question_backend_grading_worker', 'ple_data', 'USAGE')
		OR has_schema_privilege('ple_imathas_question_backend_grading_worker', 'ple_audit', 'USAGE')
		OR NOT has_function_privilege('ple_imathas_question_backend_grading_worker', 'ple_api.claim_imathas_result_grading_job(uuid,uuid,timestamp with time zone)', 'EXECUTE')
		OR NOT has_function_privilege('ple_imathas_question_backend_grading_worker', 'ple_api.commit_imathas_result_grading(uuid,uuid,timestamp with time zone)', 'EXECUTE') THEN
		RAISE EXCEPTION 'iMathAS Question Backend grading worker capability is not execute-only';
	END IF;
	SELECT count(*)::integer INTO membership_count
	FROM pg_auth_members membership
	JOIN pg_roles member ON member.oid = membership.member
	JOIN pg_roles parent ON parent.oid = membership.roleid
	WHERE member.rolname = 'ple_migrator'
	AND parent.rolname = ANY (ARRAY['ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student', 'ple_imathas_question_backend_grading_worker']);
	IF membership_count <> 13 THEN
		RAISE EXCEPTION 'unexpected membership count: %', membership_count;
	END IF;
	SELECT string_agg(
		format(
			'%s -> %s (grantor=%s, inherit=%s, set=%s, admin=%s)',
			member.rolname,
			parent.rolname,
			grantor.rolname,
			membership.inherit_option,
			membership.set_option,
			membership.admin_option
		),
		'; ' ORDER BY parent.rolname, member.rolname
	) INTO unexpected_membership_detail
	FROM pg_auth_members membership
	JOIN pg_roles member ON member.oid = membership.member
	JOIN pg_roles parent ON parent.oid = membership.roleid
	JOIN pg_roles grantor ON grantor.oid = membership.grantor
	WHERE (
		member.rolname = ANY (ARRAY[
			'ple_migrator', 'ple_database_owner', 'ple_data_owner',
			'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app',
			'ple_auth', 'ple_student'
		])
		OR parent.rolname = ANY (ARRAY[
			'ple_migrator', 'ple_database_owner', 'ple_data_owner',
			'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app',
			'ple_auth', 'ple_student'
		])
	)
	AND NOT (
		member.rolname = 'ple_migrator'
		AND parent.rolname = ANY (ARRAY['ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner', 'ple_app', 'ple_auth', 'ple_student', 'ple_imathas_question_backend_grading_worker'])
		AND grantor.rolname = 'ple_e2e_migrator'
		AND NOT membership.inherit_option
		AND NOT membership.set_option AND membership.admin_option
	)
	AND NOT (
		member.rolname = 'ple_migrator'
		AND parent.rolname = 'ple_database_owner'
		AND grantor.rolname = 'ple_e2e_migrator'
		AND NOT membership.inherit_option AND membership.set_option AND NOT membership.admin_option
	)
	AND NOT (
		member.rolname = 'ple_migrator'
		AND parent.rolname = ANY (ARRAY[
			'ple_data_owner', 'ple_private_owner', 'ple_audit_owner',
			'ple_api_owner'
		])
		AND grantor.rolname = 'ple_migrator'
		AND NOT membership.inherit_option AND membership.set_option AND NOT membership.admin_option
	);
	IF unexpected_membership_detail IS NOT NULL THEN
		RAISE EXCEPTION 'membership graph has an unexpected edge, grantor, or option: %', unexpected_membership_detail;
	END IF;
	FOREACH owner_name IN ARRAY ARRAY[
		'ple_data', 'ple_private', 'ple_audit', 'ple_api'
	] LOOP
		IF NOT EXISTS (
			SELECT 1 FROM pg_namespace namespace
			JOIN pg_roles owner_role ON owner_role.oid = namespace.nspowner
			WHERE namespace.nspname = owner_name
			AND owner_role.rolname = owner_name || '_owner'
		) THEN
			RAISE EXCEPTION 'schema % has an unexpected owner', owner_name;
		END IF;
	END LOOP;
	IF NOT EXISTS (
		SELECT 1 FROM pg_class relation
		JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
		JOIN pg_roles owner_role ON owner_role.oid = relation.relowner
		WHERE namespace.nspname = 'public' AND relation.relname = '_sqlx_migrations'
		AND owner_role.rolname = 'ple_migrator'
	) THEN
		RAISE EXCEPTION 'SQLx migration ledger has an unexpected owner';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_class relation
		JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
		JOIN pg_roles owner_role ON owner_role.oid = relation.relowner
		WHERE namespace.nspname = 'ple_api' AND relation.relname = 'ple_migration_state'
		AND owner_role.rolname = 'ple_api_owner'
	) THEN
		RAISE EXCEPTION 'migration-state projection has an unexpected owner';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_namespace namespace
		CROSS JOIN LATERAL aclexplode(
			COALESCE(namespace.nspacl, acldefault('n', namespace.nspowner))
		) privilege
		WHERE namespace.nspname = ANY (ARRAY['public', 'ple_data', 'ple_private', 'ple_audit', 'ple_api'])
		AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'PUBLIC retains privilege on a PLE schema';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_database database_record
		CROSS JOIN LATERAL aclexplode(
			COALESCE(database_record.datacl, acldefault('c', database_record.datdba))
		) privilege
		WHERE database_record.datname = current_database() AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'PUBLIC retains target database privilege';
	END IF;
	IF NOT has_schema_privilege('ple_migrator', 'public', 'USAGE')
		OR NOT has_schema_privilege('ple_api_owner', 'public', 'USAGE')
		OR has_schema_privilege('ple_migrator', 'public', 'CREATE')
		OR has_schema_privilege('ple_api_owner', 'public', 'CREATE') THEN
		RAISE EXCEPTION 'migration or API owner public-schema privileges are not exact';
	END IF;
	IF EXISTS (
		SELECT 1
		FROM pg_roles owner_role
		CROSS JOIN (VALUES ('n'::"char"), ('r'::"char"), ('S'::"char"), ('f'::"char"), ('T'::"char")) object_kind(kind)
		LEFT JOIN pg_default_acl defaults
			ON defaults.defaclrole = owner_role.oid
			AND defaults.defaclobjtype = object_kind.kind
		CROSS JOIN LATERAL aclexplode(
			COALESCE(defaults.defaclacl, acldefault(object_kind.kind, owner_role.oid))
		) privilege
		WHERE owner_role.rolname = ANY (ARRAY[
			'ple_database_owner', 'ple_data_owner', 'ple_private_owner', 'ple_audit_owner', 'ple_api_owner'
		])
		AND privilege.grantee = 0
	) THEN
		RAISE EXCEPTION 'one or more owner default ACLs grant a privilege to PUBLIC';
	END IF;
	SELECT count(*)::integer INTO column_count
	FROM information_schema.columns
	WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state';
	IF column_count <> 3
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'version')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'success')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_api' AND table_name = 'ple_migration_state' AND column_name = 'checksum') THEN
		RAISE EXCEPTION 'migration-state projection does not have the exact safe columns';
	END IF;
	IF has_table_privilege('ple_app', 'public._sqlx_migrations', 'SELECT')
		OR NOT has_schema_privilege('ple_app', 'ple_api', 'USAGE')
		OR NOT has_table_privilege('ple_app', 'ple_api.ple_migration_state', 'SELECT') THEN
		RAISE EXCEPTION 'ple_app direct/projection privileges are not exact';
	END IF;
	SELECT count(*)::integer INTO column_count
	FROM information_schema.columns
	WHERE table_schema = 'ple_data' AND table_name = 'student_record';
	IF column_count <> 4
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'student_record_id')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'course_id')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'student_account_id')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'created_at')
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'student_record' AND column_name = 'membership_id')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_data.student_record'::regclass AND conname = 'student_record_account_course_is_unique')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_data.student_record'::regclass AND conname = 'student_record_course_reference_is_unique')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_data.course_membership'::regclass AND conname = 'course_membership_student_record_presence')
		OR NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ple_data.course_membership'::regclass AND tgname = 'course_membership_binds_exact_student_record' AND NOT tgisinternal) THEN
		RAISE EXCEPTION 'Student Record ownership is not exact and stable across membership episodes';
	END IF;
	IF NOT has_table_privilege('ple_api_owner', 'public._sqlx_migrations', 'SELECT') THEN
		RAISE EXCEPTION 'ple_api_owner cannot read the migration ledger for its projection';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_private.assignment_attempt'::regclass
		AND conname = 'assignment_attempt_revision_belongs_to_assignment'
	) OR to_regclass('ple_private.question_pool_selection') IS NULL
		OR to_regclass('ple_private.question_pool_selected_item') IS NULL
		OR (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'assignment_attempt' AND column_name IN ('attempt_number', 'question_pool_reuse_rule', 'question_variation_rule')) <> 3 OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'question_pool_selection' AND column_name = 'selected_question_count' AND is_nullable = 'NO')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_private.assignment_attempt'::regclass AND conname = 'assignment_attempt_student_assignment_number_is_unique') OR NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgrelid = 'ple_private.question_pool_selection'::regclass AND tgname IN ('question_pool_selection_reuse_has_exact_student_and_assignment_history', 'question_pool_selection_has_exact_selected_question_pool_item_count') AND NOT tgisinternal GROUP BY tgrelid HAVING count(*) = 2)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_private.issued_question'::regclass
			AND conname = 'issued_question_selection_entry_matches_version'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger WHERE tgrelid = 'ple_private.question_pool_selection'::regclass
			AND tgname = 'question_pool_selection_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.assignment_attempt'::regclass
		AND tgname = 'assignment_attempt_requires_released_revision' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_private' AND table_name = 'issued_question'
		AND column_name = 'statistics_eligible' AND is_nullable = 'NO'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_data.assignment_revision'::regclass
		AND tgname = 'assignment_revision_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.question_submission'::regclass
		AND tgname = 'question_submission_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.assignment_submission'::regclass
		AND tgname = 'assignment_submission_is_immutable' AND NOT tgisinternal
	) THEN
		RAISE EXCEPTION 'issued work lacks exact immutable Question Pool Selection evidence, exact pins, Question Statistics Eligibility, or immutable submissions';
	END IF;
	IF (SELECT count(*) FROM information_schema.columns
		WHERE table_schema = 'ple_private' AND table_name = 'question_attempt'
		AND column_name IN (
			'question_seed', 'generated_parameter_sha256', 'question_attempt_state', 'reproduction_details'
		) AND is_nullable = 'NO') <> 4
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.question_attempt'::regclass
			AND conname = 'question_attempt_seed_is_u64'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.question_attempt'::regclass
			AND conname = 'question_attempt_generated_parameter_sha256_is_lowercase_hex'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.question_attempt'::regclass
			AND conname = 'question_attempt_state_is_closed'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.question_attempt'::regclass
			AND tgname = 'question_attempt_state_transition_is_forward_only' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.question_attempt'::regclass
			AND tgname = 'question_attempt_submission_state_is_exact' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.question_submission'::regclass
			AND tgname = 'question_submission_requires_accepted_attempt_state' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Question Attempt persistence does not enforce the closed state and submission contract';
	END IF;
	IF to_regclass('ple_data.course_schedule_revision') IS NULL
		OR (SELECT count(*) FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'course_schedule_revision') <> 7
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'course_instance'
			AND column_name = 'delivery_time_zone'
	) OR NOT EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
			AND column_name = 'course_schedule_revision_id' AND is_nullable = 'NO'
	) OR NOT EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment'
		AND column_name = 'assignment_status' AND is_nullable = 'NO'
	) OR NOT EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment'
		AND column_name = 'released_assignment_revision_id'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_data.assignment'::regclass
		AND conname = 'assignment_released_revision_matches_assignment'
	) OR NOT EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment'
		AND column_name = 'assignment_edit_number' AND is_nullable = 'NO'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_data.assignment'::regclass
		AND tgname = 'assignment_edit_is_exact' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.assignment_revision'::regclass
			AND conname = 'assignment_revision_course_matches_assignment'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.assignment_revision'::regclass
			AND conname = 'assignment_revision_schedule_matches_course'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.assignment_revision'::regclass
			AND conname = 'assignment_revision_schedule_is_ordered'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_schedule_revision'::regclass
			AND tgname = 'course_schedule_revision_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_class
			JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
			WHERE pg_namespace.nspname = 'ple_data'
			AND pg_class.relname = 'assignment_revision_availability_idx'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_class
			JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
			WHERE pg_namespace.nspname = 'ple_data'
			AND pg_class.relname = 'assignment_released_revision_lookup_idx'
		) THEN
		RAISE EXCEPTION 'Course Schedule Revision and Assignment Revision do not own the exact durable delivery schedule';
	END IF;
	IF EXISTS (
		SELECT 1 FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
		AND column_name = 'course_delivery_settings'
	) OR NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_data.assignment_revision'::regclass
		AND conname = 'assignment_revision_title_is_valid'
	) OR (SELECT count(*) FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
		AND column_name IN (
			'assignment_title', 'assignment_instructions', 'late_work_rule',
			'assignment_deadline_rule', 'assignment_completion_rule',
			'assignment_attempt_grade_rule', 'assignment_attempt_continuation_rule',
			'question_pool_reuse_rule', 'question_variation_rule', 'assignment_attempt_resume_rule',
			'assignment_question_display_rule', 'assignment_navigation_rule',
			'assignment_question_order_rule'
		) AND is_nullable = 'NO') <> 13 OR (SELECT count(*) FROM information_schema.columns
		WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision'
		AND column_name IN (
			'assignment_attempt_time_limit_seconds', 'attempt_limit',
			'assignment_completion_score_threshold', 'max_additional_assignment_attempts'
		)) <> 4 THEN
		RAISE EXCEPTION 'Assignment Revision is not stored as explicit immutable released teaching fields';
	END IF;
	IF NOT EXISTS (
		SELECT 1 FROM pg_trigger
		WHERE tgrelid = 'ple_private.assignment_grade_calculation'::regclass
		AND tgname = 'assignment_grade_calculation_is_immutable' AND NOT tgisinternal
	) OR NOT EXISTS (
		SELECT 1 FROM pg_constraint
		WHERE conrelid = 'ple_private.assignment_grade'::regclass
		AND conname = 'assignment_grade_selected_calculation_matches'
	) THEN
		RAISE EXCEPTION 'Gradebook does not preserve immutable calculations and an exact selected grade';
	END IF;
	IF to_regclass('ple_private.automated_grading_operation') IS NOT NULL
		OR to_regclass('ple_private.question_submission_grading') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.question_submission_grading'::regclass
			AND conname = 'question_submission_grading_job_matches_submission'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.question_submission_grading'::regclass
			AND conname = 'question_submission_grading_state_is_closed'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.job'::regclass
			AND conname = 'job_kind_matches_target'
		)
		OR to_regclass('ple_private.grading_result') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.grading_result'::regclass
			AND conname = 'grading_result_submission_matches_attempt'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.grading_result'::regclass
			AND conname = 'grading_result_matches_question_submission_grading'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_audit.automated_grading_receipt'::regclass
			AND conname = 'automated_grading_receipt_matches_grading_result'
		) THEN
		RAISE EXCEPTION 'Grading Result does not bind one Question Submission grading lifecycle, Job, and receipt';
	END IF;
	IF to_regclass('ple_private.course_object_metadata') IS NOT NULL
		OR to_regclass('ple_private.course_object_reference') IS NULL
		OR to_regclass('ple_private.imathas_render_cache_entry') IS NULL
		OR to_regclass('ple_private.course_retention_plan') IS NOT NULL
		OR to_regclass('ple_audit.retention_lifecycle_event') IS NOT NULL
		OR to_regclass('ple_private.course_retention_plan_revision') IS NULL
		OR to_regclass('ple_audit.course_retention_event') IS NULL
		OR to_regclass('ple_private.webauthn_ceremony') IS NOT NULL
		OR to_regclass('ple_private.passkey_ceremony') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private'
				AND table_name = 'imathas_question_backend_session'
				AND column_name = 'imathas_question_backend_session_authentication'
				AND is_nullable = 'NO'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.imathas_question_backend_session'::regclass
				AND conname = 'imathas_question_backend_session_authentication_has_expected_shape'
		)
		OR NOT EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private'
				AND table_name = 'imathas_question_backend_session'
				AND column_name = 'imathas_question_backend_session_challenge'
				AND is_nullable = 'NO'
		)
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private'
				AND table_name = 'imathas_result_exchange'
				AND column_name IN (
					'attempt_id', 'course_id', 'assignment_id', 'account_id',
					'imathas_deployment_reference', 'question_id', 'revision_number',
					'source_object_id', 'source_object_checksum', 'imathas_profile',
					'imathas_response_sha256', 'correlation'
				)
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.imathas_result_exchange'::regclass
				AND conname = 'imathas_result_exchange_session_matches'
		)
		OR to_regclass('ple_private.assignment_export_request') IS NOT NULL
		OR to_regclass('ple_private.assignment_export_artifact') IS NOT NULL
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private'
				AND table_name = 'job'
				AND column_name IN ('assignment_export_id', 'expected_object_id')
		)
		OR EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.job'::regclass
				AND pg_get_constraintdef(oid) LIKE '%''export''%'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.imathas_result_exchange'::regclass
				AND conname = 'imathas_result_exchange_state_matches'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.job'::regclass
				AND conname = 'job_course_retention_plan_revision_matches'
		)
		OR EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.job'::regclass
				AND conname = 'job_assignment_export_matches'
		)
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private'
				AND table_name = 'course_object_reference'
				AND column_name IN ('scope', 'owner_student_record_id', 'sha256')
		)
		OR NOT EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_private'
				AND table_name = 'course_object_reference'
				AND column_name = 'object_checksum'
				AND is_nullable = 'NO'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_audit.object_delivery_access_event'::regclass AND conname = 'object_delivery_access_event_decision_is_closed'
		) THEN
		RAISE EXCEPTION 'Course Object Reference, iMathAS Render Cache Entry, or Object Delivery Access Event lacks its exact boundary';
	END IF;
	IF (SELECT count(*) FROM information_schema.columns
	    WHERE table_schema = 'ple_private' AND table_name = 'imathas_render_cache_entry'
	      AND column_name IN (
	          'imathas_deployment_reference', 'question_id', 'revision_number',
	          'imathas_normalized_question_seed', 'imathas_profile',
	          'source_payload_digest', 'encrypted_render_data', 'fetched_at', 'expires_at'
	      ) AND is_nullable = 'NO') <> 9
	    OR NOT EXISTS (
	        SELECT 1 FROM pg_constraint
	        WHERE conrelid = 'ple_private.imathas_render_cache_entry'::regclass
	          AND conname = 'imathas_render_cache_entry_question_revision_matches'
	    ) THEN
		RAISE EXCEPTION 'iMathAS Render Cache Entry does not retain its exact iMathAS render identity';
	END IF;
	IF to_regclass('ple_private.account_state_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.account'::regclass
			AND tgname = 'account_creation_records_active_state' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.account_state_event'::regclass
			AND tgname = 'account_restriction_revokes_sessions' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Account State does not govern authenticated-session access';
	END IF;
	IF to_regclass('ple_data.course_membership_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_membership'::regclass
			AND tgname = 'course_membership_creation_records_started_event' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_membership_event'::regclass
			AND tgname = 'course_membership_event_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_membership_event'::regclass
			AND tgname = 'course_membership_event_has_valid_transition' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Membership does not preserve immutable state history';
	END IF;
	IF to_regclass('ple_private.authoring_workspace_collaborator') IS NOT NULL
		OR to_regclass('ple_private.authoring_workspace_collaborator_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.authoring_workspace_collaborator_event'::regclass
			AND tgname = 'authoring_workspace_collaborator_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.authoring_workspace_collaborator_event'::regclass
			AND tgname = 'authoring_workspace_collaborator_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Workspace Collaborator does not preserve immutable start and end evidence';
	END IF;
	IF to_regclass('ple_private.course_observer_relationship') IS NOT NULL
		OR to_regclass('ple_private.course_observer_relationship_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_observer_relationship_event'::regclass
			AND tgname = 'course_observer_relationship_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_observer_relationship_event'::regclass
			AND tgname = 'course_observer_relationship_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Observer does not preserve immutable relationship evidence';
	END IF;
	IF to_regclass('ple_data.question_stewardship_event') IS NOT NULL
		OR to_regclass('ple_data.question_change_proposal_revision') IS NULL
		OR to_regclass('ple_data.question_change_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.question_change_proposal_revision'::regclass
			AND conname = 'question_change_proposal_revision_base_revision_matches'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_change_proposal_revision'::regclass
			AND tgname = 'question_change_proposal_revision_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_change_event'::regclass
			AND tgname = 'question_change_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.question_change_event'::regclass
			AND tgname = 'question_change_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Question Change Proposals do not preserve immutable revisions and lifecycle evidence';
	END IF;
	IF to_regclass('ple_private.course_invitation_event') IS NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_private.course_invitation_event'::regclass
			AND conname = 'course_invitation_event_invitation_id_key'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_invitation_event'::regclass
			AND tgname = 'course_invitation_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_private.course_invitation_event'::regclass
			AND tgname = 'course_invitation_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Invitation does not retain one valid immutable terminal Event';
	END IF;
	IF to_regclass('ple_data.blueprint_publication_event') IS NULL
		OR to_regclass('ple_data.blueprint_collaborator_event') IS NULL
		OR to_regclass('ple_data.blueprint_revision_availability_event') IS NULL
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'blueprint_course'
			AND column_name = 'archived_at'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.blueprint_publication_event'::regclass
			AND conname = 'blueprint_publication_event_revision_is_unique'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_collaborator_event'::regclass
			AND tgname = 'blueprint_collaborator_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_collaborator_event'::regclass
			AND tgname = 'blueprint_collaborator_event_is_immutable' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_revision_availability_event'::regclass
			AND tgname = 'blueprint_revision_availability_event_has_valid_transition' AND NOT tgisinternal
		) OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.blueprint_revision_availability_event'::regclass
			AND tgname = 'blueprint_revision_availability_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Blueprint publication, availability, and Draft Blueprint Revision collaboration evidence is incomplete';
	END IF;
	IF to_regclass('ple_data.course_origin') IS NULL
		OR to_regclass('ple_data.course_instance_blueprint_adoption') IS NOT NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_data.course_origin'::regclass
			AND tgname = 'course_origin_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Course Origin is not retained as immutable exact source evidence';
	END IF;
	IF to_regclass('ple_audit.forced_question_correction_assignment_target') IS NULL
		OR to_regclass('ple_audit.forced_question_correction_attempt_target') IS NULL
		OR to_regclass('ple_audit.forced_question_correction_issued_question_target') IS NULL
		OR to_regclass('ple_audit.forced_question_correction_grade_target') IS NULL
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'forced_question_correction'
			AND column_name IN ('flawed_problem_id', 'replacement_problem_id')
		)
		OR (
			SELECT count(*) FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'forced_question_correction'
			AND column_name = ANY (ARRAY[
				'flawed_question_id', 'flawed_revision_number',
				'replacement_question_id', 'replacement_revision_number'
			])
		) <> 4
		OR EXISTS (
			SELECT 1 FROM information_schema.columns
			WHERE table_schema = 'ple_data' AND table_name = 'forced_question_correction'
			AND column_name IN ('remediation', 'generation')
		)
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_data' AND table_name = 'forced_question_correction' AND column_name = 'correction_generation')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_audit' AND table_name = 'correction_recalculation_evidence' AND column_name = 'correction_generation')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_audit' AND table_name = 'correction_recalculation_evidence' AND column_name = 'outcome_checksum' AND data_type = 'bytea' AND is_nullable = 'NO')
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_audit' AND table_name = 'correction_recalculation_evidence' AND column_name = 'digest')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_audit.correction_recalculation_evidence'::regclass AND pg_get_constraintdef(oid) LIKE '%octet_length(outcome_checksum) = 32%')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_audit' AND table_name = 'object_storage_check_event' AND column_name = 'object_storage_check_event_checksum' AND data_type = 'bytea' AND is_nullable = 'NO')
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_audit' AND table_name = 'object_storage_check_event' AND column_name = 'digest')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_audit.object_storage_check_event'::regclass AND pg_get_constraintdef(oid) LIKE '%octet_length(object_storage_check_event_checksum) = 32%')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_audit.object_storage_check_event'::regclass AND contype = 'u' AND pg_get_constraintdef(oid) = 'UNIQUE (object_storage_check_id, check_result, object_storage_check_event_checksum)')
		OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.forced_question_correction'::regclass
			AND conname = 'forced_question_correction_flawed_revision_matches'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_constraint
			WHERE conrelid = 'ple_data.forced_question_correction'::regclass
			AND conname = 'forced_question_correction_replacement_revision_matches'
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_assignment_target'::regclass
			AND tgname = 'forced_question_correction_assignment_target_is_immutable' AND NOT tgisinternal
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_attempt_target'::regclass
			AND tgname = 'forced_question_correction_attempt_target_is_immutable' AND NOT tgisinternal
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_issued_question_target'::regclass
			AND tgname = 'forced_question_correction_issued_question_target_is_immutable' AND NOT tgisinternal
		)
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.forced_question_correction_grade_target'::regclass
			AND tgname = 'forced_question_correction_grade_target_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Forced Question Correction Manifest lacks immutable exact Question Revision references or teaching targets';
	END IF;
	IF to_regclass('ple_audit.assignment_grade_event') IS NULL
		OR to_regclass('ple_audit.grade_control_event') IS NOT NULL
		OR NOT EXISTS (
			SELECT 1 FROM pg_trigger
			WHERE tgrelid = 'ple_audit.assignment_grade_event'::regclass
			AND tgname = 'assignment_grade_event_is_immutable' AND NOT tgisinternal
		) THEN
		RAISE EXCEPTION 'Assignment Grade Event is not immutable exact calculation evidence';
	END IF;
	IF EXISTS (
		SELECT 1 FROM pg_roles capability
		WHERE capability.rolname = ANY (ARRAY['ple_app', 'ple_auth', 'ple_student'])
		AND (has_schema_privilege(capability.rolname, 'ple_data', 'USAGE')
			OR has_schema_privilege(capability.rolname, 'ple_private', 'USAGE')
			OR has_schema_privilege(capability.rolname, 'ple_audit', 'USAGE'))
	) THEN
		RAISE EXCEPTION 'capability role has ambient data-schema usage';
	END IF;
END $$;
-- Assignment Question Analysis retains exact source Assignment Entry and Question Revision without Student data.
DO $$
BEGIN
    IF to_regclass('ple_data.assignment_question_analysis') IS NULL
       OR to_regclass('ple_data.assignment_item_analysis') IS NOT NULL THEN
        RAISE EXCEPTION 'Assignment Question Analysis table cutover is incomplete';
    END IF;
    IF (SELECT count(*) FROM information_schema.columns
        WHERE table_schema = 'ple_data' AND table_name = 'assignment_question_analysis'
        AND column_name IN (
            'assignment_question_analysis_id', 'assignment_analysis_id', 'course_id',
            'assignment_id', 'assignment_entry_id', 'question_id', 'revision_number',
            'graded_attempt_count', 'aggregate'
        ) AND is_nullable = 'NO') <> 9 THEN
        RAISE EXCEPTION 'Assignment Question Analysis does not retain exact source facts';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_analysis'::regclass
          AND conname = 'assignment_analysis_course_assignment_matches'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_private.job'::regclass
          AND conname = 'job_kind_matches_target'
          AND pg_get_constraintdef(oid) LIKE '%recalculate_assignment_question_analysis%'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_question_analysis'::regclass
          AND conname = 'assignment_question_analysis_parent_matches'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_question_analysis'::regclass
          AND conname = 'assignment_question_analysis_revision_matches'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_question_analysis'::regclass
          AND conname = 'assignment_question_analysis_source_is_unique'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'ple_data'
          AND relation.relname = 'assignment_question_analysis_parent_idx'
    ) THEN
        RAISE EXCEPTION 'Assignment Question Analysis relationships or read index are incomplete';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        CROSS JOIN LATERAL aclexplode(
            COALESCE(relation.relacl, acldefault('r', relation.relowner))
        ) AS privilege
        WHERE namespace.nspname = 'ple_data'
          AND relation.relname IN ('assignment_analysis', 'assignment_question_analysis')
          AND privilege.grantee = 0
    ) OR has_table_privilege('ple_app', 'ple_data.assignment_analysis', 'SELECT')
       OR has_table_privilege('ple_app', 'ple_data.assignment_question_analysis', 'SELECT') THEN
        RAISE EXCEPTION 'Assignment Analysis must remain protected course-local aggregate evidence';
    END IF;
    IF (SELECT count(*)
        FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'ple_data'
          AND relation.relname IN ('assignment_analysis', 'assignment_question_analysis')
          AND relation.relrowsecurity
          AND relation.relforcerowsecurity) <> 2 THEN
        RAISE EXCEPTION 'Assignment Analysis tables must enforce row-level security';
    END IF;
END
$$;
INSERT INTO ple_data.assignment_analysis (
    assignment_analysis_id, course_id, assignment_id, scoring_generation, completed_at,
    completed_assignment_attempt_count, in_progress_assignment_attempt_count,
    minimum_cohort_size, aggregate
) VALUES (
    '00000000-0000-0000-0000-000000000820',
    '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000110',
    1, '2026-01-01 00:00:00+00', 5, 0, 5, '{}'::jsonb
);
INSERT INTO ple_data.assignment_question_analysis (
    assignment_question_analysis_id, assignment_analysis_id, course_id, assignment_id,
    assignment_entry_id, question_id, revision_number, graded_attempt_count, aggregate
) VALUES (
    '00000000-0000-0000-0000-000000000821',
    '00000000-0000-0000-0000-000000000820',
    '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000110',
    '00000000-0000-0000-0000-000000000112', 'ABC-DEF0', 1, 5, '{}'::jsonb
);
-- This is a real Course Instance, but the known Assignment belongs to a
-- different Course Instance. The composite header boundary must reject it.
BEGIN;
INSERT INTO ple_data.course_instance (
    course_id, blueprint_id, blueprint_revision_id, assigned_instructor_account_id,
    assigned_instructor_role, created_at
) VALUES (
    '00000000-0000-0000-0000-000000002000',
    '00000000-0000-0000-0000-000000000103',
    '00000000-0000-0000-0000-000000000104',
    '00000000-0000-0000-0000-000000000102', 'instructor', '2026-01-01 00:00:00+00'
);
INSERT INTO ple_data.course_membership (
    membership_id, course_id, account_id, role, joined_at, student_record_id
) VALUES (
    '00000000-0000-0000-0000-000000002010',
    '00000000-0000-0000-0000-000000002000',
    '00000000-0000-0000-0000-000000000102',
    'instructor', '2026-01-01 00:00:00+00', NULL
);
COMMIT;
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.assignment_analysis (
            assignment_analysis_id, course_id, assignment_id, scoring_generation, completed_at,
            completed_assignment_attempt_count, in_progress_assignment_attempt_count,
            minimum_cohort_size, aggregate
        ) VALUES (
            '00000000-0000-0000-0000-000000000822',
            '00000000-0000-0000-0000-000000000105',
            '00000000-0000-0000-0000-000000000110',
            1, '2026-01-01 00:00:00+00', 5, 0, 5, '{}'::jsonb
        );
        RAISE EXCEPTION 'Assignment Analysis accepted a duplicate scoring generation';
    EXCEPTION WHEN unique_violation THEN NULL;
    END;
    BEGIN
        INSERT INTO ple_data.assignment_analysis (
            assignment_analysis_id, course_id, assignment_id, scoring_generation, completed_at,
            completed_assignment_attempt_count, in_progress_assignment_attempt_count,
            minimum_cohort_size, aggregate
        ) VALUES (
            '00000000-0000-0000-0000-000000002001',
            '00000000-0000-0000-0000-000000002000',
            '00000000-0000-0000-0000-000000000110',
            1, '2026-01-01 00:00:00+00', 5, 0, 5, '{}'::jsonb
        );
        RAISE EXCEPTION 'Assignment Analysis accepted a Course Instance and Assignment mismatch';
    EXCEPTION WHEN foreign_key_violation THEN NULL;
    END;
    BEGIN
        INSERT INTO ple_data.assignment_question_analysis (
            assignment_question_analysis_id, assignment_analysis_id, course_id, assignment_id,
            assignment_entry_id, question_id, revision_number, graded_attempt_count, aggregate
        ) VALUES (
            '00000000-0000-0000-0000-000000000823',
            '00000000-0000-0000-0000-000000000820',
            '00000000-0000-0000-0000-000000002000',
            '00000000-0000-0000-0000-000000000110',
            '00000000-0000-0000-0000-000000000112', 'SRC-0001', 1, 5, '{}'::jsonb
        );
        RAISE EXCEPTION 'Assignment Question Analysis accepted a mismatched parent Course Instance';
    EXCEPTION WHEN foreign_key_violation THEN NULL;
    END;
    BEGIN
        INSERT INTO ple_data.assignment_question_analysis (
            assignment_question_analysis_id, assignment_analysis_id, course_id, assignment_id,
            assignment_entry_id, question_id, revision_number, graded_attempt_count, aggregate
        ) VALUES (
            '00000000-0000-0000-0000-000000002002',
            '00000000-0000-0000-0000-000000000820',
            '00000000-0000-0000-0000-000000000105',
            '00000000-0000-0000-0000-000000000110',
            '00000000-0000-0000-0000-000000000112', 'ABC-DEF0', 2, 5, '{}'::jsonb
        );
        RAISE EXCEPTION 'Assignment Question Analysis accepted an unknown exact Question Revision';
    EXCEPTION WHEN foreign_key_violation THEN NULL;
    END;
    INSERT INTO ple_data.assignment_question_analysis (
        assignment_question_analysis_id, assignment_analysis_id, course_id, assignment_id,
        assignment_entry_id, question_id, revision_number, graded_attempt_count, aggregate
    ) VALUES (
        '00000000-0000-0000-0000-000000002003',
        '00000000-0000-0000-0000-000000000820',
        '00000000-0000-0000-0000-000000000105',
        '00000000-0000-0000-0000-000000000110',
        '00000000-0000-0000-0000-000000000112', 'SRC-0001', 1, 5, '{}'::jsonb
    );
    BEGIN
        INSERT INTO ple_data.assignment_question_analysis (
            assignment_question_analysis_id, assignment_analysis_id, course_id, assignment_id,
            assignment_entry_id, question_id, revision_number, graded_attempt_count, aggregate
        ) VALUES (
            '00000000-0000-0000-0000-000000000824',
            '00000000-0000-0000-0000-000000000820',
            '00000000-0000-0000-0000-000000000105',
            '00000000-0000-0000-0000-000000000110',
            '00000000-0000-0000-0000-000000000112', 'ABC-DEF0', 1, 5, '{}'::jsonb
        );
        RAISE EXCEPTION 'Assignment Question Analysis accepted a duplicate source Question Revision';
    EXCEPTION WHEN unique_violation THEN NULL;
    END;
END
$$;
-- No other Job Kind may target a Course Assignment.
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_private.job (
            job_id, job_kind, job_target_kind, course_id, assignment_id, generation,
            payload, state, available_at, max_attempts, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000002007',
            'recalculate_assignment', 'course_assignment',
            '00000000-0000-0000-0000-000000000105',
            '00000000-0000-0000-0000-000000000110', 1,
            '{}'::jsonb, 'ready', '2026-01-01 00:00:00+00', 1, '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'a non-analysis Job Kind accepted a Course Assignment target';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
-- Recalculation is a Course Assignment Job; Question Submission targets must be rejected.
INSERT INTO ple_private.job (
    job_id, job_kind, job_target_kind, course_id, assignment_id, generation,
    payload, state, available_at, max_attempts, created_at
) VALUES (
    '00000000-0000-0000-0000-000000002004',
    'recalculate_assignment_question_analysis', 'course_assignment',
    '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000110', 1,
    '{}'::jsonb, 'ready', '2026-01-01 00:00:00+00', 1, '2026-01-01 00:00:00+00'
);
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_private.job (
            job_id, job_kind, job_target_kind, question_submission_id, generation,
            payload, state, available_at, max_attempts, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000002005',
            'recalculate_assignment_question_analysis', 'question_submission',
            '00000000-0000-0000-0000-000000002006', 1,
            '{}'::jsonb, 'ready', '2026-01-01 00:00:00+00', 1, '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'Assignment Question Analysis recalculation accepted a Question Submission target';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
