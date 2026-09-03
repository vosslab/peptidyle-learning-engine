-- iMathAS Question Backend Session catalog and lock-authority acceptance oracle.
-- Executed by the existing PostgreSQL Migration Acceptance Runtime lane.

DO $$
BEGIN
	IF (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_question_backend_session' AND column_name IN ('question_attempt_id', 'imathas_deployment_reference', 'imathas_item_reference', 'imathas_profile', 'question_seed', 'imathas_launch_binding_checksum', 'imathas_question_backend_state_key_id', 'imathas_question_backend_state_nonce', 'imathas_question_backend_state_ciphertext') AND is_nullable = 'NO') <> 9
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_question_backend_session' AND column_name = 'attempt_id')
		OR EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_question_backend_session' AND column_name = 'imathas_result_token_sha256')
		OR NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'ple_private' AND table_name = 'imathas_result_exchange' AND column_name = 'imathas_result_token_sha256' AND data_type = 'bytea' AND is_nullable = 'YES') OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_private.imathas_result_exchange'::regclass AND pg_get_constraintdef(oid) LIKE '%octet_length(imathas_result_token_sha256) = 32%')
		OR NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'ple_private.imathas_question_backend_session'::regclass AND conname = 'imathas_question_backend_session_state_key_nonce_is_unique')
		OR (SELECT count(*) FROM pg_proc proc JOIN pg_namespace namespace ON namespace.oid = proc.pronamespace JOIN pg_roles owner_role ON owner_role.oid = proc.proowner WHERE namespace.nspname = 'ple_api' AND proc.proname IN ('create_imathas_question_backend_session', 'load_imathas_question_backend_session', 'lease_imathas_question_backend_session', 'stage_verified_imathas_result') AND owner_role.rolname = 'ple_api_owner' AND proc.prosecdef AND array_to_string(proc.proconfig, ',') LIKE 'search_path=pg_catalog,%' AND has_function_privilege('ple_app', proc.oid, 'EXECUTE') AND NOT has_function_privilege('public', proc.oid, 'EXECUTE')) <> 4 THEN
		RAISE EXCEPTION 'iMathAS Question Backend Session Store schema/API boundary is incomplete';
	END IF;
	IF NOT has_column_privilege('ple_api_owner', 'ple_private.issued_question', 'issued_question_id', 'UPDATE')
		OR NOT has_column_privilege('ple_api_owner', 'ple_private.assignment_attempt', 'assignment_attempt_id', 'UPDATE')
		OR NOT EXISTS (
			SELECT 1 FROM pg_policy
			WHERE polrelid = 'ple_private.issued_question'::regclass
			  AND polname = 'issued_question_api_owner_lock'
			  AND pg_get_expr(polqual, polrelid) = 'true'
			  AND pg_get_expr(polwithcheck, polrelid) = 'false'
		) OR NOT EXISTS (
			SELECT 1 FROM pg_policy
			WHERE polrelid = 'ple_private.assignment_attempt'::regclass
			  AND polname = 'assignment_attempt_api_owner_lock'
			  AND pg_get_expr(polqual, polrelid) = 'true'
			  AND pg_get_expr(polwithcheck, polrelid) = 'false'
		) THEN
		RAISE EXCEPTION 'iMathAS issued scoring lineage locks are not column-scoped and update-refusing';
	END IF;
END
$$;
