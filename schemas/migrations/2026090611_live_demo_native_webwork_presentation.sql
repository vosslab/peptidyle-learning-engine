-- M14 private WeBWorK issuance/replay evidence.  This extends the M11
-- write-once Question Attempt spine without granting renderer or worker access.
SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_attempt_webwork_replay (
    question_attempt_id uuid PRIMARY KEY
        REFERENCES ple_private.question_attempt (question_attempt_id),
    replay_version smallint NOT NULL CHECK (replay_version = 1),
    replay_details jsonb NOT NULL
);
ALTER TABLE ple_private.question_attempt_webwork_replay ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_webwork_replay FORCE ROW LEVEL SECURITY;
CREATE POLICY question_attempt_webwork_replay_private_owner_access
    ON ple_private.question_attempt_webwork_replay
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_webwork_replay_api_owner_issue
    ON ple_private.question_attempt_webwork_replay
    FOR INSERT TO ple_api_owner WITH CHECK (true);
GRANT INSERT ON ple_private.question_attempt_webwork_replay TO ple_api_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_attempt_webwork_replay FROM PUBLIC;

CREATE FUNCTION ple_private.reject_question_attempt_webwork_replay_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'WeBWorK replay evidence is immutable';
END
$$;
CREATE TRIGGER question_attempt_webwork_replay_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_attempt_webwork_replay
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_attempt_webwork_replay_change();
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_question_attempt_webwork_replay_change() FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- The API process receives this private source pin only after exact Student
-- authorization.  No source, PG path, or replay value crosses ple_app's HTTP
-- response boundary. ASVS 2.3.1 and 8.2.1.
CREATE FUNCTION ple_api.prepare_live_demo_native_webwork_issuance(
    p_course_reference_number bigint, p_assignment_reference_number bigint
)
RETURNS TABLE (
    assignment_entry_id uuid, issued_position integer, question_id text,
    revision_number integer, source_object_id uuid, source_object_checksum text,
    webwork_pg_path text, resumed boolean
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_assignment ple_data.assignment%ROWTYPE; v_student_record_id uuid;
BEGIN
    SELECT assignment.* INTO v_assignment FROM ple_data.course_instance course
      JOIN ple_data.assignment assignment ON assignment.course_id = course.course_id
      WHERE course.reference_number = p_course_reference_number
        AND assignment.reference_number = p_assignment_reference_number;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    SELECT student_record_id INTO v_student_record_id FROM ple_data.student_record
      WHERE course_id = v_assignment.course_id
        AND student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(v_assignment.course_id, v_student_record_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    RETURN QUERY
    SELECT entry.assignment_entry_id, entry.assignment_content_entry_index,
           fixed.question_id, fixed.revision_number, binding.source_object_id,
           binding.source_object_checksum, binding.webwork_pg_path,
           EXISTS (SELECT 1 FROM ple_private.assignment_attempt attempt
                    WHERE attempt.student_record_id = v_student_record_id
                      AND attempt.assignment_id = v_assignment.assignment_id
                      AND attempt.completed_at IS NULL)
      FROM ple_data.assignment_revision_entry entry
      JOIN ple_data.assignment_revision_fixed_question fixed
        ON fixed.assignment_revision_id = entry.assignment_revision_id
       AND fixed.assignment_entry_id = entry.assignment_entry_id
      JOIN ple_private.question_revision_source_binding binding
        ON binding.question_id = fixed.question_id AND binding.revision_number = fixed.revision_number
     WHERE entry.assignment_revision_id = v_assignment.released_assignment_revision_id
       AND binding.backend = 'webwork' AND binding.question_format = 'webworkPg'
     ORDER BY entry.assignment_content_entry_index;
END
$$;

-- M11 remains the sole locked Student-start primitive.  This wrapper adds the
-- WeBWorK-only private replay record in the same transaction, never exposes it,
-- and leaves worker/grading authority ungranted. ASVS 2.3.3 and 8.2.2.
CREATE FUNCTION ple_api.start_live_demo_native_webwork_assignment(
    p_course_reference_number bigint, p_assignment_reference_number bigint, p_presentations jsonb
)
RETURNS TABLE (
    attempt_number integer, resumed boolean, assignment_title text,
    assignment_instructions text, question_id text, revision_number integer,
    issued_position integer, question_seed numeric, presentation_nonce text,
    presentation_checksum text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_resumed boolean;
    v_issued record;
BEGIN
    -- Atomic Assignment Attempt creation returns the answer-free presentation.
    -- Capture its one result set while retaining its initial `resumed` state:
    -- calling it again would correctly resume the new attempt, but would
    -- falsely label the initial response as a resume.
    -- ASVS 2.3.1/2.3.3 and 8.2.1/8.2.2.
    FOR v_issued IN
        SELECT * FROM ple_api.start_live_demo_native_ple_assignment(
            p_course_reference_number, p_assignment_reference_number, p_presentations
        )
    LOOP
        IF v_resumed IS NULL THEN
            v_resumed := v_issued.resumed;
        ELSIF v_resumed IS DISTINCT FROM v_issued.resumed THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Native PLE issuance returned inconsistent Assignment Attempt state';
        END IF;
        attempt_number := v_issued.attempt_number;
        resumed := v_issued.resumed;
        assignment_title := v_issued.assignment_title;
        assignment_instructions := v_issued.assignment_instructions;
        question_id := v_issued.question_id;
        revision_number := v_issued.revision_number;
        issued_position := v_issued.issued_position;
        question_seed := v_issued.question_seed;
        presentation_nonce := v_issued.presentation_nonce;
        presentation_checksum := v_issued.presentation_checksum;
        RETURN NEXT;
    END LOOP;
    IF v_resumed IS NULL THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable'; END IF;
    IF NOT v_resumed THEN
        -- The generic M11 commit admits the complete released entry set.  This
        -- M14 wrapper must additionally prove that replay appears exactly on
        -- the WeBWorK subset, never merely where an untrusted JSON caller put
        -- a non-null member.  The exact issued revision/source binding is the
        -- backend authority. ASVS 8.2.1/8.2.2.
        IF EXISTS (
            SELECT 1
              FROM jsonb_to_recordset(p_presentations) AS item(
                  issued_question_id uuid, replay_details jsonb)
              JOIN ple_private.issued_question issued
                ON issued.issued_question_id = item.issued_question_id
              JOIN ple_private.question_revision_source_binding binding
                ON binding.question_id = issued.question_id
               AND binding.revision_number = issued.revision_number
             WHERE (binding.backend = 'webwork' AND binding.question_format = 'webworkPg'
                    AND item.replay_details IS NULL)
                OR ((binding.backend <> 'webwork' OR binding.question_format <> 'webworkPg')
                    AND item.replay_details IS NOT NULL)
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'WeBWorK replay must match exactly the issued WeBWorK Question Source';
        END IF;
        INSERT INTO ple_private.question_attempt_webwork_replay (
            question_attempt_id, replay_version, replay_details
        )
        SELECT attempt.question_attempt_id, 1, item.replay_details
          FROM jsonb_to_recordset(p_presentations) AS item(
             issued_question_id uuid, replay_details jsonb)
          JOIN ple_private.question_attempt attempt ON attempt.issued_question_id = item.issued_question_id
          JOIN ple_private.issued_question issued ON issued.issued_question_id = item.issued_question_id
          JOIN ple_private.question_revision_source_binding binding
            ON binding.question_id = issued.question_id AND binding.revision_number = issued.revision_number
         WHERE binding.backend = 'webwork' AND binding.question_format = 'webworkPg'
           AND item.replay_details IS NOT NULL;
    END IF;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.prepare_live_demo_native_webwork_issuance(bigint, bigint),
    ple_api.start_live_demo_native_webwork_assignment(bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.prepare_live_demo_native_webwork_issuance(bigint, bigint),
    ple_api.start_live_demo_native_webwork_assignment(bigint, bigint, jsonb) TO ple_app;
RESET ROLE;
