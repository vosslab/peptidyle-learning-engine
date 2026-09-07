-- M13-2 native PLE grading recovery. This is deliberately separate from the
-- iMathAS worker: a native PLE worker may claim only immutable native PLE
-- submissions and may commit only its current typed lease.

CREATE ROLE ple_native_ple_grading_worker
    NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
REVOKE ple_native_ple_grading_worker FROM ple_migrator;

SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE POLICY job_private_owner_native_ple_grading_update
    ON ple_private.job FOR UPDATE TO ple_private_owner USING (job_kind = 'grade_accepted_submission' AND job_target_kind = 'question_submission') WITH CHECK (job_kind = 'grade_accepted_submission' AND job_target_kind = 'question_submission');
CREATE POLICY job_private_owner_native_ple_grading_select
    ON ple_private.job FOR SELECT TO ple_private_owner USING (job_kind = 'grade_accepted_submission' AND job_target_kind = 'question_submission');
CREATE POLICY question_submission_grading_private_owner_native_ple_grading_update
    ON ple_private.question_submission_grading FOR UPDATE TO ple_private_owner USING (EXISTS (SELECT 1 FROM ple_private.job AS job WHERE job.job_id = question_submission_grading.job_id AND job.job_kind = 'grade_accepted_submission' AND job.job_target_kind = 'question_submission')) WITH CHECK (EXISTS (SELECT 1 FROM ple_private.job AS job WHERE job.job_id = question_submission_grading.job_id AND job.job_kind = 'grade_accepted_submission' AND job.job_target_kind = 'question_submission'));
CREATE POLICY question_submission_grading_private_owner_native_ple_grading_select
    ON ple_private.question_submission_grading FOR SELECT TO ple_private_owner USING (EXISTS (SELECT 1 FROM ple_private.job AS job WHERE job.job_id = question_submission_grading.job_id AND job.job_kind = 'grade_accepted_submission' AND job.job_target_kind = 'question_submission'));
CREATE POLICY grading_result_private_owner_native_ple_grading_insert
    ON ple_private.grading_result FOR INSERT TO ple_private_owner WITH CHECK (true);
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
GRANT INSERT ON TABLE ple_audit.automated_grading_receipt TO ple_private_owner;
CREATE POLICY automated_grading_receipt_private_owner_native_ple_grading_insert
    ON ple_audit.automated_grading_receipt FOR INSERT TO ple_private_owner WITH CHECK (true);
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_api.claim_live_demo_native_ple_grading_job(
    p_lease_token uuid, p_lease_expires_at timestamp with time zone
) RETURNS TABLE (
    job_id uuid, question_attempt_id uuid, question_id text, revision_number integer,
    source_object_id uuid, source_object_checksum text, question_seed numeric,
    student_response jsonb, point_value numeric, scoring_rule text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE v_job_id uuid;
BEGIN
    IF p_lease_token IS NULL OR p_lease_expires_at <= pg_catalog.clock_timestamp()
       OR p_lease_expires_at > pg_catalog.clock_timestamp() + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Native PLE grading Job lease is invalid';
    END IF;
    SELECT job.job_id INTO v_job_id FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS attempt ON attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
     WHERE job.job_kind = 'grade_accepted_submission' AND job.job_target_kind = 'question_submission'
       AND binding.backend = 'ple' AND binding.question_format = 'pleQuestionJson'
       AND (job.state = 'ready' OR (job.state = 'leased' AND job.lease_expires_at <= pg_catalog.clock_timestamp()))
       AND job.available_at <= pg_catalog.clock_timestamp() AND job.attempt_count < job.max_attempts
     ORDER BY job.available_at, job.job_id FOR UPDATE OF job SKIP LOCKED LIMIT 1;
    IF v_job_id IS NULL THEN RETURN; END IF;
    UPDATE ple_private.job AS job SET state = 'leased', lease_token = p_lease_token,
        lease_expires_at = p_lease_expires_at, attempt_count = attempt_count + 1
      WHERE job.job_id = v_job_id;
    RETURN QUERY SELECT job.job_id, attempt.question_attempt_id, issued.question_id, issued.revision_number,
        binding.source_object_id, binding.source_object_checksum, attempt.question_seed,
        submission.student_response, issued.point_value, issued.scoring_rule
      FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS attempt ON attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
     WHERE job.job_id = v_job_id AND job.lease_token = p_lease_token;
END $$;

CREATE FUNCTION ple_api.commit_live_demo_native_ple_grading(
    p_job_id uuid, p_lease_token uuid, p_correct boolean, p_normalized_credit double precision,
    p_committed_at timestamp with time zone
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_audit AS $$
DECLARE j ple_private.job%ROWTYPE; s ple_private.question_submission%ROWTYPE;
    g ple_private.question_submission_grading%ROWTYPE; a ple_private.question_attempt%ROWTYPE;
    i ple_private.issued_question%ROWTYPE; v_result uuid := pg_catalog.gen_random_uuid();
    v_receipt uuid := pg_catalog.gen_random_uuid(); v_checksum bytea;
    v_possible double precision; v_earned double precision;
BEGIN
    IF p_job_id IS NULL OR p_lease_token IS NULL OR p_committed_at IS NULL
       OR p_normalized_credit IS NULL OR p_normalized_credit < 0 OR p_normalized_credit > 1
       OR p_normalized_credit IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Native PLE grading commit facts are invalid';
    END IF;
    SELECT * INTO j FROM ple_private.job WHERE job_id = p_job_id FOR UPDATE;
    SELECT * INTO g FROM ple_private.question_submission_grading WHERE job_id = p_job_id FOR UPDATE;
    SELECT * INTO s FROM ple_private.question_submission WHERE submission_id = j.question_submission_id;
    SELECT * INTO a FROM ple_private.question_attempt WHERE question_attempt_id = s.question_attempt_id;
    SELECT * INTO i FROM ple_private.issued_question WHERE issued_question_id = a.issued_question_id;
    IF j.job_kind <> 'grade_accepted_submission' OR j.job_target_kind <> 'question_submission'
       OR j.state <> 'leased' OR j.lease_token <> p_lease_token OR j.lease_expires_at <= p_committed_at
       OR j.lease_expires_at <= pg_catalog.clock_timestamp() OR g.grading_state <> 'pending' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Native PLE grading commit does not own the live typed Job lease';
    END IF;
    v_possible := i.point_value::double precision;
    IF v_possible IS NULL OR v_possible IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
       OR v_possible < 0 OR pg_catalog.float8send(v_possible) = pg_catalog.decode('8000000000000000', 'hex') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Issued Question point value is invalid for native PLE grading';
    END IF;
    v_earned := CASE i.scoring_rule WHEN 'normal' THEN v_possible * p_normalized_credit
      WHEN 'full_credit' THEN v_possible WHEN 'extra_credit' THEN v_possible * p_normalized_credit
      WHEN 'excluded' THEN 0 ELSE NULL END;
    IF v_earned IS NULL OR v_earned IN ('NaN'::double precision, 'Infinity'::double precision, '-Infinity'::double precision)
       OR v_earned < 0 OR v_earned > v_possible
       OR pg_catalog.float8send(v_earned) = pg_catalog.decode('8000000000000000', 'hex') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Native PLE grading result is invalid';
    END IF;
    INSERT INTO ple_private.grading_result (grading_result_id, submission_id, question_submission_grading_id, question_attempt_id, correct, points_earned, points_possible, recorded_at) VALUES (v_result, s.submission_id, g.question_submission_grading_id,
      a.question_attempt_id, p_correct, v_earned, v_possible, p_committed_at);
    v_checksum := pg_catalog.sha256(
        pg_catalog.convert_to('ple:native-ple-grading-receipt:v1', 'UTF8')
        || pg_catalog.decode('00', 'hex')
        || pg_catalog.uuid_send(v_receipt)
        || pg_catalog.uuid_send(v_result)
        || pg_catalog.uuid_send(g.question_submission_grading_id)
        || pg_catalog.uuid_send(s.submission_id)
        || pg_catalog.uuid_send(a.question_attempt_id)
        || pg_catalog.uuid_send(p_job_id)
        || CASE WHEN p_correct THEN pg_catalog.decode('01', 'hex') ELSE pg_catalog.decode('00', 'hex') END
        || pg_catalog.float8send(v_earned)
        || pg_catalog.float8send(v_possible)
        || pg_catalog.int8send((pg_catalog.date_part('epoch', p_committed_at) * 1000)::bigint)
    );
    INSERT INTO ple_audit.automated_grading_receipt (automated_grading_receipt_id, question_submission_grading_id, grading_result_id, committed_at, automated_grading_receipt_checksum) VALUES (v_receipt, g.question_submission_grading_id,
      v_result, p_committed_at, v_checksum);
    UPDATE ple_private.question_submission_grading SET grading_state = 'graded', completed_at = p_committed_at WHERE question_submission_grading_id = g.question_submission_grading_id;
    UPDATE ple_private.job SET state = 'completed', lease_token = NULL, lease_expires_at = NULL, completed_at = p_committed_at WHERE job_id = p_job_id;
    RETURN v_receipt;
END $$;

-- The same public C/A/nonce capability may read only its current grading
-- projection. It never returns the accepted response, private identities,
-- source bindings, receipt, or checksum.
CREATE FUNCTION ple_api.read_live_demo_native_ple_submission_status(
    p_course_reference_number bigint, p_assignment_reference_number bigint,
    p_presentation_nonce text
) RETURNS TABLE (
    grading_state text, correct boolean, points_earned double precision,
    points_possible double precision
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL OR p_presentation_nonce !~ '^[0-9a-f]{32}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT grading.grading_state, result.correct, result.points_earned::double precision,
           result.points_possible::double precision
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id
       AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student_record
        ON student_record.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt
        ON attempt.issued_question_id = issued.issued_question_id
       AND attempt.question_attempt_state = 'submission_accepted'
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source_binding
        ON source_binding.question_id = issued.question_id
       AND source_binding.revision_number = issued.revision_number
      JOIN ple_private.question_submission AS submission
        ON submission.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_submission_grading AS grading
        ON grading.submission_id = submission.submission_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.submission_id = submission.submission_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND student_record.course_id = assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student_record.student_record_id)
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source_binding.backend = 'ple'
       AND source_binding.question_format = 'pleQuestionJson';
END $$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.claim_live_demo_native_ple_grading_job(uuid, timestamp with time zone), ple_api.commit_live_demo_native_ple_grading(uuid, uuid, boolean, double precision, timestamp with time zone), ple_api.read_live_demo_native_ple_submission_status(bigint, bigint, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.claim_live_demo_native_ple_grading_job(uuid, timestamp with time zone), ple_api.commit_live_demo_native_ple_grading(uuid, uuid, boolean, double precision, timestamp with time zone) TO ple_native_ple_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.read_live_demo_native_ple_submission_status(bigint, bigint, text) TO ple_app;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;
GRANT USAGE ON SCHEMA ple_api TO ple_native_ple_grading_worker;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
