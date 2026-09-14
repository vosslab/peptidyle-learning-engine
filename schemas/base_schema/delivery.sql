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
    )
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
    )
TO ple_app;

RESET ROLE;
