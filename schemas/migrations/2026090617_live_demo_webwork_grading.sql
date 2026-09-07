-- M14 deterministic WeBWorK grading. The renderer gets a separate worker
-- capability; it neither shares the native-PLE worker nor exposes PG input.

CREATE ROLE ple_webwork_grading_worker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ple_webwork_grading_worker FROM ple_migrator;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_api.resolve_live_demo_webwork_submission(
    p_course_reference_number bigint, p_assignment_reference_number bigint, p_presentation_nonce text
) RETURNS TABLE (
    question_attempt_id uuid, question_id text, revision_number integer, source_object_id uuid,
    source_object_checksum text, webwork_pg_path text, question_seed numeric,
    presentation_nonce text, presentation_checksum text, replay_details jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL OR p_presentation_nonce !~ '^[0-9a-f]{32}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY SELECT attempt.question_attempt_id, issued.question_id, issued.revision_number,
        binding.source_object_id, binding.source_object_checksum, binding.webwork_pg_path,
        attempt.question_seed, presentation.presentation_nonce, presentation.presentation_checksum,
        replay.replay_details
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_id = assignment.assignment_id AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student_record ON student_record.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id AND attempt.question_attempt_state = 'open'
      JOIN ple_private.question_attempt_presentation_binding AS presentation ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS binding ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
      JOIN ple_private.question_attempt_webwork_replay AS replay ON replay.question_attempt_id = attempt.question_attempt_id
     WHERE course.reference_number = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number
       AND student_record.course_id = assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student_record.student_record_id)
       AND presentation.presentation_nonce = p_presentation_nonce
       AND binding.backend = 'webwork' AND binding.question_format = 'webworkPg';
END $$;

CREATE FUNCTION ple_api.accept_live_demo_webwork_submission(
    p_question_attempt_id uuid, p_student_response jsonb, p_submission_id uuid,
    p_question_submission_grading_id uuid, p_job_id uuid
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_attempt ple_private.question_attempt%ROWTYPE; v_now timestamp with time zone;
BEGIN
    IF p_question_attempt_id IS NULL OR p_submission_id IS NULL OR p_question_submission_grading_id IS NULL OR p_job_id IS NULL
       OR pg_catalog.jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Submission acceptance facts are invalid';
    END IF;
    SELECT attempt.* INTO v_attempt FROM ple_private.question_attempt AS attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = assignment_attempt.assignment_id
      JOIN ple_data.student_record AS student_record ON student_record.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.question_revision_source_binding AS binding ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
     WHERE attempt.question_attempt_id = p_question_attempt_id AND assignment_attempt.completed_at IS NULL
       AND student_record.course_id = assignment.course_id AND student_record.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student_record.student_record_id)
       AND binding.backend = 'webwork' AND binding.question_format = 'webworkPg'
     FOR UPDATE OF attempt;
    IF NOT FOUND OR v_attempt.question_attempt_state <> 'open' OR EXISTS (SELECT 1 FROM ple_private.question_submission AS submission WHERE submission.question_attempt_id = p_question_attempt_id) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Submission cannot be accepted';
    END IF;
    v_now := pg_catalog.clock_timestamp();
    INSERT INTO ple_private.question_submission (submission_id, question_attempt_id, submitted_at, student_response)
        VALUES (p_submission_id, p_question_attempt_id, v_now, p_student_response);
    UPDATE ple_private.question_attempt SET question_attempt_state = 'submission_accepted'
      WHERE question_attempt_id = p_question_attempt_id AND question_attempt_state = 'open';
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Submission cannot be accepted'; END IF;
    INSERT INTO ple_private.job (job_id, job_kind, job_target_kind, question_submission_id, generation, payload, state, available_at, max_attempts, created_at)
        VALUES (p_job_id, 'grade_accepted_submission', 'question_submission', p_submission_id, 1, '{}'::jsonb, 'ready', v_now, 3, v_now);
    INSERT INTO ple_private.question_submission_grading (question_submission_grading_id, submission_id, job_id, grading_state, created_at)
        VALUES (p_question_submission_grading_id, p_submission_id, p_job_id, 'pending', v_now);
END $$;

CREATE POLICY question_attempt_webwork_replay_private_owner_grading_select
    ON ple_private.question_attempt_webwork_replay FOR SELECT TO ple_private_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_api.claim_live_demo_webwork_grading_job(
    p_lease_token uuid, p_lease_expires_at timestamp with time zone
) RETURNS TABLE (
    job_id uuid, question_attempt_id uuid, question_id text, revision_number integer,
    source_object_id uuid, source_object_checksum text, webwork_pg_path text,
    question_seed numeric, student_response jsonb, replay_details jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE v_job_id uuid;
BEGIN
    IF p_lease_token IS NULL OR p_lease_expires_at <= pg_catalog.clock_timestamp()
       OR p_lease_expires_at > pg_catalog.clock_timestamp() + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'WeBWorK grading Job lease is invalid';
    END IF;
    SELECT job.job_id INTO v_job_id FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS attempt ON attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS binding ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
      JOIN ple_private.question_attempt_webwork_replay AS replay ON replay.question_attempt_id = attempt.question_attempt_id
     WHERE job.job_kind = 'grade_accepted_submission' AND job.job_target_kind = 'question_submission'
       AND binding.backend = 'webwork' AND binding.question_format = 'webworkPg'
       AND (job.state = 'ready' OR (job.state = 'leased' AND job.lease_expires_at <= pg_catalog.clock_timestamp()))
       AND job.available_at <= pg_catalog.clock_timestamp() AND job.attempt_count < job.max_attempts
     ORDER BY job.available_at, job.job_id FOR UPDATE OF job SKIP LOCKED LIMIT 1;
    IF v_job_id IS NULL THEN RETURN; END IF;
    UPDATE ple_private.job AS job SET state = 'leased', lease_token = p_lease_token,
        lease_expires_at = p_lease_expires_at, attempt_count = attempt_count + 1 WHERE job.job_id = v_job_id;
    RETURN QUERY SELECT job.job_id, attempt.question_attempt_id, issued.question_id, issued.revision_number,
        binding.source_object_id, binding.source_object_checksum, binding.webwork_pg_path, attempt.question_seed,
        submission.student_response, replay.replay_details
      FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS attempt ON attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS binding ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
      JOIN ple_private.question_attempt_webwork_replay AS replay ON replay.question_attempt_id = attempt.question_attempt_id
     WHERE job.job_id = v_job_id AND job.lease_token = p_lease_token;
END $$;

CREATE FUNCTION ple_api.commit_live_demo_webwork_grading(
    p_job_id uuid, p_lease_token uuid, p_correct boolean, p_normalized_credit double precision,
    p_committed_at timestamp with time zone
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_audit AS $$
DECLARE j ple_private.job%ROWTYPE; g ple_private.question_submission_grading%ROWTYPE;
    s ple_private.question_submission%ROWTYPE; a ple_private.question_attempt%ROWTYPE;
    i ple_private.issued_question%ROWTYPE; v_result uuid := pg_catalog.gen_random_uuid();
    v_receipt uuid := pg_catalog.gen_random_uuid(); v_possible double precision; v_earned double precision;
BEGIN
    IF p_job_id IS NULL OR p_lease_token IS NULL OR p_committed_at IS NULL OR p_normalized_credit IS NULL
       OR p_normalized_credit < 0 OR p_normalized_credit > 1
       OR p_normalized_credit IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'WeBWorK grading commit facts are invalid';
    END IF;
    SELECT * INTO j FROM ple_private.job WHERE job_id = p_job_id FOR UPDATE;
    SELECT * INTO g FROM ple_private.question_submission_grading WHERE job_id = p_job_id FOR UPDATE;
    SELECT * INTO s FROM ple_private.question_submission WHERE submission_id = j.question_submission_id;
    SELECT * INTO a FROM ple_private.question_attempt WHERE question_attempt_id = s.question_attempt_id;
    SELECT * INTO i FROM ple_private.issued_question WHERE issued_question_id = a.issued_question_id;
    IF j.job_kind <> 'grade_accepted_submission' OR j.job_target_kind <> 'question_submission'
       OR j.state <> 'leased' OR j.lease_token <> p_lease_token OR j.lease_expires_at <= p_committed_at
       OR j.lease_expires_at <= pg_catalog.clock_timestamp() OR g.grading_state <> 'pending' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'WeBWorK grading commit does not own the Job lease';
    END IF;
    v_possible := i.point_value::double precision;
    v_earned := CASE i.scoring_rule WHEN 'normal' THEN v_possible * p_normalized_credit
      WHEN 'full_credit' THEN v_possible WHEN 'extra_credit' THEN v_possible * p_normalized_credit
      WHEN 'excluded' THEN 0 ELSE NULL END;
    IF v_possible IS NULL OR v_earned IS NULL OR v_possible < 0 OR v_earned < 0 OR v_earned > v_possible
       OR v_possible IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
       OR v_earned IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'WeBWorK grading result is invalid';
    END IF;
    INSERT INTO ple_private.grading_result (grading_result_id, submission_id, question_submission_grading_id, question_attempt_id, correct, points_earned, points_possible, recorded_at)
    VALUES (v_result, s.submission_id, g.question_submission_grading_id, a.question_attempt_id, p_correct, v_earned, v_possible, p_committed_at);
    INSERT INTO ple_audit.automated_grading_receipt (automated_grading_receipt_id, question_submission_grading_id, grading_result_id, committed_at, automated_grading_receipt_checksum)
    VALUES (v_receipt, g.question_submission_grading_id, v_result, p_committed_at,
      pg_catalog.sha256(pg_catalog.convert_to('ple:webwork-grading-receipt:v1', 'UTF8') || pg_catalog.uuid_send(v_receipt) || pg_catalog.uuid_send(v_result)));
    UPDATE ple_private.question_submission_grading SET grading_state = 'graded', completed_at = p_committed_at WHERE question_submission_grading_id = g.question_submission_grading_id;
    UPDATE ple_private.job SET state = 'completed', lease_token = NULL, lease_expires_at = NULL, completed_at = p_committed_at WHERE job_id = p_job_id;
    RETURN v_receipt;
END $$;

CREATE FUNCTION ple_api.fail_live_demo_webwork_grading(
    p_job_id uuid, p_lease_token uuid, p_completed_at timestamp with time zone
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
	UPDATE ple_private.question_submission_grading SET grading_state = 'instructor_attention',
		completed_at = p_completed_at
	 WHERE job_id = p_job_id AND grading_state = 'pending';
	IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'WeBWorK grading failure does not own pending grading'; END IF;
    UPDATE ple_private.job SET state = 'failed', lease_token = NULL, lease_expires_at = NULL,
        completed_at = p_completed_at, job_failure_kind = 'final'
      WHERE job_id = p_job_id AND job_kind = 'grade_accepted_submission' AND job_target_kind = 'question_submission'
        AND state = 'leased' AND lease_token = p_lease_token AND lease_expires_at > pg_catalog.clock_timestamp();
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'WeBWorK grading failure does not own the Job lease'; END IF;
END $$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.claim_live_demo_webwork_grading_job(uuid, timestamp with time zone), ple_api.commit_live_demo_webwork_grading(uuid, uuid, boolean, double precision, timestamp with time zone), ple_api.fail_live_demo_webwork_grading(uuid, uuid, timestamp with time zone) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.claim_live_demo_webwork_grading_job(uuid, timestamp with time zone), ple_api.commit_live_demo_webwork_grading(uuid, uuid, boolean, double precision, timestamp with time zone), ple_api.fail_live_demo_webwork_grading(uuid, uuid, timestamp with time zone) TO ple_webwork_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.resolve_live_demo_webwork_submission(bigint, bigint, text), ple_api.accept_live_demo_webwork_submission(uuid, jsonb, uuid, uuid, uuid) TO ple_app;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_webwork_grading_worker;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
