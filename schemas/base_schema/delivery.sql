-- Delivery procedures are the narrow bridge from authenticated Students and
-- grading workers to retained Question Attempt evidence.  They lock the
-- Assignment before its Student Work, so Unrelease wins or loses atomically.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.require_owned_open_question_attempt(
    p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid
) RETURNS ple_private.question_attempt
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE result ple_private.question_attempt%ROWTYPE;
BEGIN
    -- Assignment is intentionally the first row lock in every delivery write.
    PERFORM 1 FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = p_assignment_id AND assignment.course_id = p_course_id
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question delivery is unavailable'; END IF;
    SELECT question_attempt.* INTO result
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.student_record AS student ON student.student_record_id = assignment_attempt.student_record_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id
       AND assignment_attempt.assignment_id = p_assignment_id
       AND student.course_id = p_course_id
       AND student.student_account_id = ple_api.current_session_account_id()
       AND assignment_attempt.completed_at IS NULL
       AND ple_api.current_session_account_owns_student_record(p_course_id, student.student_record_id)
     FOR UPDATE OF question_attempt, issued, assignment_attempt;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question delivery is unavailable'; END IF;
    RETURN result;
END $$;

-- API routines are owned by the API schema owner.  Their narrow private
-- helper remains private-owner code, with an explicit execute boundary.
REVOKE ALL ON FUNCTION ple_private.require_owned_open_question_attempt(uuid, uuid, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.require_owned_open_question_attempt(uuid, uuid, uuid)
    TO ple_api_owner;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_imathas_question_backend_session(
    p_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_deployment text, p_item text, p_question_id text, p_revision_number integer,
    p_source_object_id uuid, p_source_checksum bytea, p_profile text, p_seed numeric,
    p_launch_checksum text, p_response_sha256 bytea, p_challenge bytea, p_authentication bytea,
    p_issued_at timestamptz, p_expires_at timestamptz, p_state_key_id text, p_state_nonce bytea,
    p_state_ciphertext bytea
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt ple_private.question_attempt%ROWTYPE;
BEGIN
    IF p_session_id IS NULL OR p_issued_at IS NULL OR p_expires_at IS NULL OR p_expires_at <= p_issued_at
       OR p_expires_at <= clock_timestamp() THEN RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Session expiry is invalid'; END IF;
    attempt := ple_private.require_owned_open_question_attempt(p_course_id, p_assignment_id, p_question_attempt_id);
    IF attempt.question_seed <> p_seed OR NOT EXISTS (
        SELECT 1 FROM ple_private.issued_question AS issued
        JOIN ple_private.question_revision_source_binding AS source
          ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
        WHERE issued.issued_question_id = attempt.issued_question_id
          AND issued.question_id = p_question_id AND issued.revision_number = p_revision_number
          AND source.backend = 'imathas' AND source.question_format = 'imathas'
          AND source.source_object_id = p_source_object_id
          AND source.source_object_checksum = encode(p_source_checksum, 'hex')
          AND source.imathas_deployment_reference = p_deployment
          AND source.imathas_item_reference = p_item AND source.imathas_profile = p_profile
    ) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Session is unavailable'; END IF;
    INSERT INTO ple_private.imathas_question_backend_session (
        imathas_question_backend_session_id, course_id, assignment_id, question_attempt_id, account_id,
        imathas_deployment_reference, imathas_item_reference, question_id, revision_number,
        source_object_id, source_object_checksum, imathas_profile, question_seed,
        imathas_launch_binding_checksum, imathas_response_sha256,
        imathas_question_backend_session_challenge, imathas_question_backend_session_authentication,
        issued_at, expires_at, imathas_question_backend_state_key_id,
        imathas_question_backend_state_nonce, imathas_question_backend_state_ciphertext
    ) VALUES (p_session_id, p_course_id, p_assignment_id, p_question_attempt_id,
        ple_api.current_session_account_id(), p_deployment, p_item, p_question_id, p_revision_number,
        p_source_object_id, p_source_checksum, p_profile, p_seed, p_launch_checksum, p_response_sha256,
        p_challenge, p_authentication, p_issued_at, p_expires_at, p_state_key_id, p_state_nonce, p_state_ciphertext);
    RETURN p_session_id;
END $$;

CREATE FUNCTION ple_api.load_imathas_question_backend_session(
    p_session_id uuid, p_account_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_deployment text, p_item text, p_question_id text, p_revision_number integer,
    p_source_object_id uuid, p_source_checksum bytea, p_profile text, p_seed numeric, p_launch_checksum text
) RETURNS TABLE (
    imathas_question_backend_session_id uuid, imathas_item_reference text, question_seed numeric,
    imathas_profile text, imathas_launch_binding_checksum text, imathas_response_sha256 bytea,
    imathas_question_backend_session_challenge bytea, imathas_question_backend_session_authentication bytea,
    issued_at timestamptz, expires_at timestamptz, imathas_question_backend_state_key_id text,
    imathas_question_backend_state_nonce bytea, imathas_question_backend_state_ciphertext bytea
) LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    RETURN QUERY SELECT session.imathas_question_backend_session_id, session.imathas_item_reference,
        session.question_seed, session.imathas_profile, session.imathas_launch_binding_checksum,
        session.imathas_response_sha256, session.imathas_question_backend_session_challenge,
        session.imathas_question_backend_session_authentication, session.issued_at, session.expires_at,
        session.imathas_question_backend_state_key_id, session.imathas_question_backend_state_nonce,
        session.imathas_question_backend_state_ciphertext
      FROM ple_private.imathas_question_backend_session AS session
     WHERE session.imathas_question_backend_session_id = p_session_id
       AND session.account_id = p_account_id AND session.account_id = ple_api.current_session_account_id()
       AND ROW(session.course_id, session.assignment_id, session.question_attempt_id, session.imathas_deployment_reference,
               session.imathas_item_reference, session.question_id, session.revision_number, session.source_object_id,
               session.source_object_checksum, session.imathas_profile, session.question_seed, session.imathas_launch_binding_checksum)
           IS NOT DISTINCT FROM ROW(p_course_id, p_assignment_id, p_question_attempt_id, p_deployment, p_item,
               p_question_id, p_revision_number, p_source_object_id, p_source_checksum, p_profile, p_seed, p_launch_checksum)
       AND session.revoked_at IS NULL AND session.consumed_at IS NULL
       AND session.issued_at <= clock_timestamp() AND session.expires_at > clock_timestamp();
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Session is unavailable'; END IF;
END $$;

CREATE FUNCTION ple_api.lease_imathas_question_backend_session(
    p_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_deployment text, p_item text, p_question_id text, p_revision_number integer,
    p_source_object_id uuid, p_source_checksum bytea, p_profile text, p_seed numeric,
    p_launch_checksum text, p_lease_sha256 bytea, p_lease_expires_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE session ple_private.imathas_question_backend_session%ROWTYPE;
DECLARE session_course_id uuid;
DECLARE session_assignment_id uuid;
BEGIN
    IF octet_length(p_lease_sha256) <> 32 OR p_lease_expires_at <= clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Session lease is invalid';
    END IF;
    -- Lock the Assignment before Session state so Unrelease and delivery share
    -- one root-first order (ASVS 2.3.1/2.3.3/8.3.1).
    SELECT course_id, assignment_id INTO session_course_id, session_assignment_id
      FROM ple_private.imathas_question_backend_session
     WHERE imathas_question_backend_session_id = p_session_id;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Session is unavailable'; END IF;
    PERFORM ple_private.require_owned_open_question_attempt(session_course_id, session_assignment_id, p_question_attempt_id);
    SELECT * INTO session FROM ple_private.imathas_question_backend_session
     WHERE imathas_question_backend_session_id = p_session_id FOR UPDATE;
    IF NOT FOUND OR session.account_id <> ple_api.current_session_account_id()
       OR ROW(session.course_id, session.assignment_id, session.question_attempt_id, session.imathas_deployment_reference,
              session.imathas_item_reference, session.question_id, session.revision_number, session.source_object_id,
              session.source_object_checksum, session.imathas_profile, session.question_seed, session.imathas_launch_binding_checksum)
          IS DISTINCT FROM ROW(p_course_id, p_assignment_id, p_question_attempt_id, p_deployment, p_item,
              p_question_id, p_revision_number, p_source_object_id, p_source_checksum, p_profile, p_seed, p_launch_checksum)
       OR session.revoked_at IS NOT NULL OR session.consumed_at IS NOT NULL OR session.expires_at <= clock_timestamp()
       OR session.activity_lease_expires_at > clock_timestamp() OR p_lease_expires_at > session.expires_at THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Session is unavailable';
    END IF;
    UPDATE ple_private.imathas_question_backend_session SET activity_lease_token_sha256 = p_lease_sha256,
        activity_lease_expires_at = p_lease_expires_at WHERE imathas_question_backend_session_id = p_session_id;
END $$;

CREATE FUNCTION ple_api.stage_verified_imathas_result(
    p_session_id uuid, p_course_id uuid, p_assignment_id uuid, p_question_attempt_id uuid,
    p_deployment text, p_item text, p_question_id text, p_revision_number integer,
    p_source_object_id uuid, p_source_checksum bytea, p_profile text, p_seed numeric,
    p_launch_checksum text, p_lease_sha256 bytea, p_result_token_sha256 bytea,
    p_normalized_score double precision, p_result_checksum bytea, p_submission_id uuid,
    p_job_id uuid, p_grading_id uuid, p_updated_at timestamptz
) RETURNS TABLE (submission_id uuid, question_submission_grading_id uuid, job_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE session ple_private.imathas_question_backend_session%ROWTYPE;
DECLARE exchange ple_private.imathas_result_exchange%ROWTYPE;
DECLARE session_course_id uuid;
DECLARE session_assignment_id uuid;
BEGIN
    IF p_submission_id IS NULL OR p_job_id IS NULL OR p_grading_id IS NULL
       OR octet_length(p_lease_sha256) <> 32 OR octet_length(p_result_token_sha256) <> 32
       OR octet_length(p_result_checksum) <> 32 OR p_normalized_score NOT BETWEEN 0 AND 1
       OR p_updated_at IS NULL OR p_updated_at > clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS Result facts are invalid';
    END IF;
    SELECT course_id, assignment_id INTO session_course_id, session_assignment_id
      FROM ple_private.imathas_question_backend_session
     WHERE imathas_question_backend_session_id = p_session_id;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Session cannot be consumed'; END IF;
    PERFORM ple_private.require_owned_open_question_attempt(session_course_id, session_assignment_id, p_question_attempt_id);
    SELECT * INTO session FROM ple_private.imathas_question_backend_session WHERE imathas_question_backend_session_id = p_session_id FOR UPDATE;
    IF NOT FOUND OR session.account_id <> ple_api.current_session_account_id()
       OR session.course_id <> p_course_id OR session.assignment_id <> p_assignment_id
       OR session.question_attempt_id <> p_question_attempt_id OR session.activity_lease_token_sha256 <> p_lease_sha256
       OR session.activity_lease_expires_at <= p_updated_at OR session.expires_at <= p_updated_at
       OR session.revoked_at IS NOT NULL OR session.consumed_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS Session cannot be consumed';
    END IF;
    INSERT INTO ple_private.imathas_result_exchange(imathas_question_backend_session_id, state, lease_token_sha256, lease_expires_at, created_at, updated_at)
    VALUES (p_session_id, 'verifying', p_lease_sha256, session.activity_lease_expires_at, p_updated_at, p_updated_at)
    ON CONFLICT (imathas_question_backend_session_id) DO NOTHING;
    SELECT * INTO exchange FROM ple_private.imathas_result_exchange WHERE imathas_question_backend_session_id = p_session_id FOR UPDATE;
    IF exchange.state IN ('ready_to_commit', 'committed') THEN
        IF exchange.imathas_result_token_sha256 = p_result_token_sha256 AND exchange.imathas_result_checksum = p_result_checksum THEN
            RETURN QUERY SELECT exchange.submission_id, exchange.question_submission_grading_id, grading.job_id
              FROM ple_private.question_submission_grading AS grading WHERE grading.question_submission_grading_id = exchange.question_submission_grading_id;
            RETURN;
        END IF;
        RAISE EXCEPTION USING ERRCODE = '23505', MESSAGE = 'iMathAS Result replay differs';
    END IF;
    INSERT INTO ple_private.question_submission(submission_id, question_attempt_id, submitted_at, student_response)
    VALUES (p_submission_id, p_question_attempt_id, p_updated_at, jsonb_build_object('kind', 'imathasQuestionBackend'));
    UPDATE ple_private.question_attempt SET question_attempt_state = 'submission_accepted', submitted_at = p_updated_at
     WHERE question_attempt_id = p_question_attempt_id AND question_attempt_state = 'open';
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Attempt is not open'; END IF;
    PERFORM ple_private.enqueue_grade_accepted_submission(
        p_job_id, p_grading_id, p_submission_id,
        'imathas_question_backend_grading', '{}'::jsonb, p_updated_at, 3, p_updated_at
    );
    UPDATE ple_private.imathas_result_exchange SET state = 'ready_to_commit', lease_token_sha256 = NULL, lease_expires_at = NULL,
        imathas_result_token_sha256 = p_result_token_sha256, imathas_result_normalized_score = p_normalized_score,
        imathas_result_checksum = p_result_checksum, submission_id = p_submission_id,
        question_submission_grading_id = p_grading_id, updated_at = p_updated_at WHERE imathas_question_backend_session_id = p_session_id;
    RETURN QUERY SELECT p_submission_id, p_grading_id, p_job_id;
END $$;

CREATE FUNCTION ple_api.claim_imathas_result_grading_job(p_job_id uuid, p_lease_token uuid, p_lease_expires_at timestamptz)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF p_lease_token IS NULL OR p_lease_expires_at NOT BETWEEN clock_timestamp() AND clock_timestamp() + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'iMathAS grading lease is invalid';
    END IF;
    RETURN EXISTS (
        SELECT 1 FROM ple_private.claim_grade_accepted_submission(
            'imathas_question_backend_grading', p_lease_token, p_lease_expires_at, p_job_id
        )
    );
END $$;

CREATE FUNCTION ple_api.commit_imathas_result_grading(p_job_id uuid, p_lease_token uuid, p_committed_at timestamptz)
RETURNS TABLE (automated_grading_receipt_id uuid, correct boolean, points_earned double precision, points_possible double precision, automated_grading_receipt_checksum bytea)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_data, ple_private, ple_audit AS $$
DECLARE exchange ple_private.imathas_result_exchange%ROWTYPE;
DECLARE points numeric;
DECLARE result_id uuid := gen_random_uuid();
DECLARE receipt_id uuid := gen_random_uuid();
DECLARE committed record;
DECLARE exchange_session_id uuid;
BEGIN
    SELECT exchange.* INTO exchange FROM ple_private.imathas_result_exchange AS exchange
      JOIN ple_private.question_submission_grading AS grading ON grading.question_submission_grading_id = exchange.question_submission_grading_id
     WHERE grading.job_id = p_job_id;
    IF NOT FOUND OR exchange.state <> 'ready_to_commit' THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS grading lease is unavailable'; END IF;
    exchange_session_id := exchange.imathas_question_backend_session_id;
    SELECT issued.point_value INTO points FROM ple_private.imathas_question_backend_session AS session
      JOIN ple_private.question_attempt AS attempt ON attempt.question_attempt_id = session.question_attempt_id
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = attempt.issued_question_id
     WHERE session.imathas_question_backend_session_id = exchange.imathas_question_backend_session_id;
    SELECT * INTO committed FROM ple_private.commit_grade_accepted_submission(
        'imathas_question_backend_grading', p_job_id, p_lease_token, result_id, receipt_id,
        exchange.imathas_result_normalized_score = 1,
        points * exchange.imathas_result_normalized_score, p_committed_at
    );
    SELECT * INTO exchange FROM ple_private.imathas_result_exchange
     WHERE imathas_question_backend_session_id = exchange_session_id FOR UPDATE;
    IF exchange.state <> 'ready_to_commit' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'iMathAS grading lease is unavailable';
    END IF;
    UPDATE ple_private.imathas_result_exchange SET state = 'committed', grading_result_id = result_id,
        committed_job_lease_token_sha256 = sha256(uuid_send(p_lease_token)), committed_at = p_committed_at, updated_at = p_committed_at
     WHERE imathas_question_backend_session_id = exchange_session_id;
    RETURN QUERY SELECT committed.automated_grading_receipt_id, committed.correct,
        committed.points_earned::double precision, committed.points_possible::double precision,
        committed.automated_grading_receipt_checksum;
END $$;

-- Native PLE and WeBWorK use the same submission root.  The format-specific
-- renderer/replay data stays on the already-issued Question Attempt.
CREATE FUNCTION ple_api.resolve_native_ple_submission(
    p_course_reference_number bigint, p_assignment_reference_number bigint, p_presentation_nonce text
) RETURNS TABLE (
    question_attempt_id uuid, question_id text, revision_number integer, source_object_id uuid,
    source_object_checksum text, question_seed numeric, presentation_nonce text, presentation_checksum text
) LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL OR p_presentation_nonce !~ '^[0-9a-f]{32,128}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT attempt.question_attempt_id, issued.question_id, issued.revision_number,
           source.source_object_id, source.source_object_checksum, attempt.question_seed,
           presentation.presentation_nonce, presentation.presentation_checksum
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student ON student.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
        AND attempt.question_attempt_state = 'open'
      JOIN ple_private.question_attempt_presentation_binding AS presentation ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source ON source.question_id = issued.question_id
        AND source.revision_number = issued.revision_number
     WHERE course.reference_number = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source.backend = 'ple' AND source.question_format = 'pleQuestionJson';
END $$;

CREATE FUNCTION ple_api.resolve_webwork_submission(
    p_course_reference_number bigint, p_assignment_reference_number bigint, p_presentation_nonce text
) RETURNS TABLE (
    question_attempt_id uuid, question_id text, revision_number integer, source_object_id uuid,
    source_object_checksum text, webwork_pg_path text, question_seed numeric,
    presentation_nonce text, presentation_checksum text, replay_details jsonb
) LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL OR p_presentation_nonce !~ '^[0-9a-f]{32,128}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT attempt.question_attempt_id, issued.question_id, issued.revision_number,
           source.source_object_id, source.source_object_checksum, source.webwork_pg_path,
           attempt.question_seed, presentation.presentation_nonce, presentation.presentation_checksum, replay.replay_details
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student ON student.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
        AND attempt.question_attempt_state = 'open'
      JOIN ple_private.question_attempt_presentation_binding AS presentation ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_attempt_webwork_replay AS replay ON replay.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source ON source.question_id = issued.question_id
        AND source.revision_number = issued.revision_number
     WHERE course.reference_number = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source.backend = 'webwork' AND source.question_format = 'webworkPg';
END $$;

CREATE FUNCTION ple_api.read_native_ple_submission_status(
    p_course_reference_number bigint, p_assignment_reference_number bigint, p_presentation_nonce text
) RETURNS TABLE (presentation_nonce text, grading_state text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF p_presentation_nonce IS NULL OR p_presentation_nonce !~ '^[0-9a-f]{32,128}$' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    RETURN QUERY
    SELECT presentation.presentation_nonce, grading.grading_state
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_id = assignment.assignment_id AND assignment_attempt.completed_at IS NULL
      JOIN ple_data.student_record AS student ON student.student_record_id = assignment_attempt.student_record_id
      JOIN ple_private.issued_question AS issued ON issued.assignment_attempt_id = assignment_attempt.assignment_attempt_id
      JOIN ple_private.question_attempt AS attempt ON attempt.issued_question_id = issued.issued_question_id
        AND attempt.question_attempt_state = 'submission_accepted'
      JOIN ple_private.question_attempt_presentation_binding AS presentation ON presentation.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_revision_source_binding AS source ON source.question_id = issued.question_id
        AND source.revision_number = issued.revision_number
      JOIN ple_private.question_submission AS submission ON submission.question_attempt_id = attempt.question_attempt_id
      JOIN ple_private.question_submission_grading AS grading ON grading.submission_id = submission.submission_id
     WHERE course.reference_number = p_course_reference_number AND assignment.reference_number = p_assignment_reference_number
       AND student.student_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_owns_student_record(assignment.course_id, student.student_record_id)
       AND presentation.presentation_nonce = p_presentation_nonce
       AND source.backend = 'ple' AND source.question_format = 'pleQuestionJson';
END $$;

CREATE FUNCTION ple_api.accept_native_ple_submission(p_question_attempt_id uuid, p_student_response jsonb, p_submission_id uuid, p_grading_id uuid, p_job_id uuid)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE attempt ple_private.question_attempt%ROWTYPE;
DECLARE issued ple_private.issued_question%ROWTYPE;
DECLARE course_id uuid;
DECLARE assignment_id uuid;
BEGIN
    IF p_question_attempt_id IS NULL OR p_submission_id IS NULL OR p_grading_id IS NULL OR p_job_id IS NULL
       OR jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Submission facts are invalid';
    END IF;
    SELECT assignment.course_id, assignment.assignment_id INTO course_id, assignment_id
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued_question ON issued_question.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id = issued_question.assignment_attempt_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = assignment_attempt.assignment_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable'; END IF;
    attempt := ple_private.require_owned_open_question_attempt(course_id, assignment_id, p_question_attempt_id);
    SELECT issued_question.* INTO issued FROM ple_private.question_attempt question_attempt
      JOIN ple_private.issued_question issued_question ON issued_question.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt assignment_attempt ON assignment_attempt.assignment_attempt_id = issued_question.assignment_attempt_id
      JOIN ple_data.assignment assignment ON assignment.assignment_id = assignment_attempt.assignment_id
      JOIN ple_data.student_record student ON student.student_record_id = assignment_attempt.student_record_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id AND student.student_account_id = ple_api.current_session_account_id()
       AND assignment_attempt.completed_at IS NULL;
    IF NOT FOUND OR attempt.question_attempt_state <> 'open'
       OR NOT EXISTS (SELECT 1 FROM ple_private.question_revision_source_binding AS source
                       JOIN ple_private.question_attempt_presentation_binding AS presentation
                         ON presentation.question_attempt_id = p_question_attempt_id
                      WHERE source.question_id = issued.question_id AND source.revision_number = issued.revision_number
                        AND source.backend = 'ple' AND source.question_format = 'pleQuestionJson') THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable';
    END IF;
    INSERT INTO ple_private.question_submission(submission_id, question_attempt_id, submitted_at, student_response)
    VALUES (p_submission_id, p_question_attempt_id, clock_timestamp(), p_student_response);
    UPDATE ple_private.question_attempt SET question_attempt_state='submission_accepted', submitted_at=clock_timestamp() WHERE question_attempt_id=p_question_attempt_id;
    PERFORM ple_private.enqueue_grade_accepted_submission(
        p_job_id, p_grading_id, p_submission_id,
        'native_ple_grading', '{}'::jsonb, clock_timestamp(), 3, clock_timestamp()
    );
    RETURN 'pending';
END $$;

CREATE FUNCTION ple_api.accept_webwork_submission(p_question_attempt_id uuid, p_student_response jsonb, p_submission_id uuid, p_grading_id uuid, p_job_id uuid)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE course_id uuid;
DECLARE assignment_id uuid;
BEGIN
    IF p_question_attempt_id IS NULL OR p_submission_id IS NULL OR p_grading_id IS NULL OR p_job_id IS NULL
       OR jsonb_typeof(p_student_response) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Submission facts are invalid';
    END IF;
    SELECT assignment.course_id, assignment.assignment_id INTO course_id, assignment_id
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.assignment AS assignment ON assignment.assignment_id = assignment_attempt.assignment_id
     WHERE question_attempt.question_attempt_id = p_question_attempt_id;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable'; END IF;
    PERFORM ple_private.require_owned_open_question_attempt(course_id, assignment_id, p_question_attempt_id);
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_attempt AS question_attempt
        JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
        JOIN ple_private.question_revision_source_binding AS source ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
        JOIN ple_private.question_attempt_webwork_replay AS replay ON replay.question_attempt_id = question_attempt.question_attempt_id
        WHERE question_attempt.question_attempt_id = p_question_attempt_id
          AND source.backend = 'webwork' AND source.question_format = 'webworkPg'
    ) THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Submission is unavailable'; END IF;
    -- The same atomic evidence write follows the format-specific proof above.
    INSERT INTO ple_private.question_submission(submission_id, question_attempt_id, submitted_at, student_response)
    VALUES (p_submission_id, p_question_attempt_id, clock_timestamp(), p_student_response);
    UPDATE ple_private.question_attempt SET question_attempt_state='submission_accepted', submitted_at=clock_timestamp()
     WHERE question_attempt_id=p_question_attempt_id AND question_attempt_state='open';
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Submission is unavailable'; END IF;
    PERFORM ple_private.enqueue_grade_accepted_submission(
        p_job_id, p_grading_id, p_submission_id,
        'webwork_grading', '{}'::jsonb, clock_timestamp(), 3, clock_timestamp()
    );
    RETURN 'pending';
END $$;

REVOKE ALL ON FUNCTION
    ple_api.create_imathas_question_backend_session(
        uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text,
        numeric, text, bytea, bytea, bytea, timestamptz, timestamptz, text, bytea, bytea
    ),
    ple_api.load_imathas_question_backend_session(
        uuid, uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text
    ),
    ple_api.lease_imathas_question_backend_session(
        uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, bytea, timestamptz
    ),
    ple_api.stage_verified_imathas_result(
        uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text,
        bytea, bytea, double precision, bytea, uuid, uuid, uuid, timestamptz
    ),
    ple_api.claim_imathas_result_grading_job(uuid, uuid, timestamptz),
    ple_api.commit_imathas_result_grading(uuid, uuid, timestamptz),
    ple_api.resolve_native_ple_submission(bigint, bigint, text),
    ple_api.resolve_webwork_submission(bigint, bigint, text),
    ple_api.read_native_ple_submission_status(bigint, bigint, text),
    ple_api.accept_native_ple_submission(uuid, jsonb, uuid, uuid, uuid),
    ple_api.accept_webwork_submission(uuid, jsonb, uuid, uuid, uuid)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.create_imathas_question_backend_session(
        uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text,
        numeric, text, bytea, bytea, bytea, timestamptz, timestamptz, text, bytea, bytea
    ),
    ple_api.load_imathas_question_backend_session(
        uuid, uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text
    ),
    ple_api.lease_imathas_question_backend_session(
        uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text, bytea, timestamptz
    ),
    ple_api.stage_verified_imathas_result(
        uuid, uuid, uuid, uuid, text, text, text, integer, uuid, bytea, text, numeric, text,
        bytea, bytea, double precision, bytea, uuid, uuid, uuid, timestamptz
    ),
    ple_api.resolve_native_ple_submission(bigint, bigint, text),
    ple_api.resolve_webwork_submission(bigint, bigint, text),
    ple_api.read_native_ple_submission_status(bigint, bigint, text),
    ple_api.accept_native_ple_submission(uuid, jsonb, uuid, uuid, uuid),
    ple_api.accept_webwork_submission(uuid, jsonb, uuid, uuid, uuid)
TO ple_app;
GRANT USAGE ON SCHEMA ple_api TO ple_imathas_question_backend_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.claim_imathas_result_grading_job(uuid,uuid,timestamptz), ple_api.commit_imathas_result_grading(uuid,uuid,timestamptz) TO ple_imathas_question_backend_grading_worker;

RESET ROLE;
