-- Assignment Unrelease is the sole destructive Assignment lifecycle operation.
-- It is a single Assignment-first transaction: the executor locks current
-- teaching authority and the Assignment before it observes or deletes Student
-- Work, so concurrent starts, submissions, and grading commits fail closed.

SET LOCAL ROLE ple_data_owner;
GRANT REFERENCES ON TABLE ple_data.assignment TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.assignment_unrelease_event (
    event_id uuid PRIMARY KEY,
    assignment_id uuid NOT NULL REFERENCES ple_data.assignment(assignment_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    assignment_edit_number bigint NOT NULL CHECK (assignment_edit_number > 0),
    assignment_attempt_count bigint NOT NULL CHECK (assignment_attempt_count >= 0),
    question_submission_count bigint NOT NULL CHECK (question_submission_count >= 0),
    assignment_submission_count bigint NOT NULL CHECK (assignment_submission_count >= 0),
    grading_result_count bigint NOT NULL CHECK (grading_result_count >= 0),
    outcome text NOT NULL CHECK (outcome = 'completed'),
    occurred_at timestamptz NOT NULL
);

CREATE FUNCTION ple_audit.reject_assignment_unrelease_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Assignment Unrelease audit evidence is immutable';
END
$$;

CREATE TRIGGER assignment_unrelease_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.assignment_unrelease_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_assignment_unrelease_event_change();

ALTER TABLE ple_audit.assignment_unrelease_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.assignment_unrelease_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.assignment_unrelease_event FROM PUBLIC;
CREATE POLICY assignment_unrelease_event_audit_owner_access
    ON ple_audit.assignment_unrelease_event
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
CREATE POLICY assignment_unrelease_event_executor_insert
    ON ple_audit.assignment_unrelease_event
    FOR INSERT TO ple_unrelease_executor WITH CHECK (true);

GRANT USAGE ON SCHEMA ple_audit TO ple_unrelease_executor;
GRANT INSERT ON TABLE ple_audit.assignment_unrelease_event TO ple_unrelease_executor;
REVOKE ALL ON FUNCTION ple_audit.reject_assignment_unrelease_event_change() FROM PUBLIC;
COMMENT ON TABLE ple_audit.assignment_unrelease_event IS
    'Redacted completed Assignment Unrelease audit evidence: actor, Assignment, aggregate counts, and time only.';

RESET ROLE;

-- The executor has no login.  These narrowly scoped grants make its single
-- SECURITY DEFINER entry point capable of reading confirmation counts and
-- deleting only the Assignment Attempt root; dependent evidence follows the
-- owning FK cascades and remains unavailable to runtime roles.
SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_unrelease_executor;
GRANT SELECT ON TABLE ple_data.course_instance TO ple_unrelease_executor;
GRANT SELECT, UPDATE ON TABLE ple_data.assignment TO ple_unrelease_executor;
CREATE POLICY course_instance_unrelease_executor_read
    ON ple_data.course_instance
    FOR SELECT TO ple_unrelease_executor USING (true);
CREATE POLICY assignment_unrelease_executor_access
    ON ple_data.assignment
    FOR ALL TO ple_unrelease_executor USING (true) WITH CHECK (true);
GRANT EXECUTE ON FUNCTION ple_data.rebuild_question_revision_statistics(text, integer, timestamptz)
    TO ple_unrelease_executor;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_unrelease_executor;
GRANT SELECT, DELETE ON TABLE ple_private.assignment_attempt TO ple_unrelease_executor;
GRANT SELECT ON TABLE ple_private.issued_question, ple_private.question_attempt,
    ple_private.question_submission, ple_private.assignment_submission,
    ple_private.grading_result, ple_private.question_statistics_observation_receipt
    TO ple_unrelease_executor;
CREATE POLICY assignment_attempt_unrelease_executor_access
    ON ple_private.assignment_attempt
    FOR ALL TO ple_unrelease_executor USING (true) WITH CHECK (true);
CREATE POLICY issued_question_unrelease_executor_read
    ON ple_private.issued_question
    FOR SELECT TO ple_unrelease_executor USING (true);
CREATE POLICY question_attempt_unrelease_executor_read
    ON ple_private.question_attempt
    FOR SELECT TO ple_unrelease_executor USING (true);
CREATE POLICY question_submission_unrelease_executor_read
    ON ple_private.question_submission
    FOR SELECT TO ple_unrelease_executor USING (true);
CREATE POLICY assignment_submission_unrelease_executor_read
    ON ple_private.assignment_submission
    FOR SELECT TO ple_unrelease_executor USING (true);
CREATE POLICY grading_result_unrelease_executor_read
    ON ple_private.grading_result
    FOR SELECT TO ple_unrelease_executor USING (true);
CREATE POLICY question_statistics_observation_unrelease_executor_read
    ON ple_private.question_statistics_observation_receipt
    FOR SELECT TO ple_unrelease_executor USING (true);
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
GRANT USAGE, CREATE ON SCHEMA ple_api TO ple_unrelease_executor;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_id(),
    ple_api.current_session_account_is_course_instructor(uuid)
    TO ple_unrelease_executor;
RESET ROLE;

-- The platform bootstrap grants the migrator SET authority for this no-login
-- capability.  Creating the function as the capability owner means no API or
-- worker role inherits the destructive privilege.  ASVS 2.2.1/2.2.2 validates
-- confirmation inputs at the trusted boundary; ASVS 2.3.1/2.3.3 keeps the
-- transition, deletion, aggregate rebuild, and audit event indivisible.
SET LOCAL ROLE ple_unrelease_executor;

CREATE FUNCTION ple_api.read_assignment_unrelease_impact(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint
)
RETURNS TABLE (
    assignment_title text,
    assignment_edit_number bigint,
    assignment_attempt_count bigint,
    question_submission_count bigint,
    assignment_submission_count bigint,
    grading_result_count bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE;
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;

    SELECT assignment.* INTO assignment_row
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND assignment.assignment_status = 'released'
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;

    assignment_title := assignment_row.assignment_title;
    assignment_edit_number := assignment_row.assignment_edit_number;
    SELECT count(*) INTO assignment_attempt_count
      FROM ple_private.assignment_attempt
     WHERE assignment_id = assignment_row.assignment_id;
    SELECT count(*) INTO question_submission_count
      FROM ple_private.question_submission AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assignment_attempt_id IN (
         SELECT assignment_attempt_id FROM ple_private.assignment_attempt
          WHERE assignment_id = assignment_row.assignment_id
     );
    SELECT count(*) INTO assignment_submission_count
      FROM ple_private.assignment_submission AS submission
      JOIN ple_private.assignment_attempt AS attempt
        ON attempt.assignment_attempt_id = submission.assignment_attempt_id
     WHERE attempt.assignment_id = assignment_row.assignment_id;
    SELECT count(*) INTO grading_result_count
      FROM ple_private.grading_result AS result
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = result.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assignment_attempt_id IN (
         SELECT assignment_attempt_id FROM ple_private.assignment_attempt
          WHERE assignment_id = assignment_row.assignment_id
     );
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.unrelease_assignment(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_expected_edit_number bigint,
    p_confirmation_title text
)
RETURNS TABLE (
    assignment_reference_number bigint,
    assignment_title text,
    assignment_status text,
    assignment_edit_number bigint,
    assignment_attempt_count bigint,
    question_submission_count bigint,
    assignment_submission_count bigint,
    grading_result_count bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE assignment_row ple_data.assignment%ROWTYPE;
DECLARE actor_account_id uuid;
DECLARE v_statistics_targets jsonb;
DECLARE statistics_target jsonb;
DECLARE now_at timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    IF p_course_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_assignment_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_edit_number IS NULL OR p_expected_edit_number <= 0
       OR p_confirmation_title IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;

    actor_account_id := ple_api.current_session_account_id();

    -- This is the mandatory first lock shared by start, save, submission, and
    -- grading paths.  It serializes every Student Work mutation with Unrelease.
    SELECT assignment.* INTO assignment_row
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment is unavailable';
    END IF;
    IF assignment_row.assignment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assignment Edit Number is stale';
    END IF;
    IF assignment_row.assignment_status <> 'released' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assignment is not Released';
    END IF;
    IF p_confirmation_title IS DISTINCT FROM assignment_row.assignment_title THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment Unrelease confirmation title does not match';
    END IF;

    SELECT count(*) INTO assignment_attempt_count
      FROM ple_private.assignment_attempt
     WHERE assignment_id = assignment_row.assignment_id;
    SELECT count(*) INTO question_submission_count
      FROM ple_private.question_submission AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assignment_attempt_id IN (
         SELECT assignment_attempt_id FROM ple_private.assignment_attempt
          WHERE assignment_id = assignment_row.assignment_id
     );
    SELECT count(*) INTO assignment_submission_count
      FROM ple_private.assignment_submission AS submission
      JOIN ple_private.assignment_attempt AS attempt
        ON attempt.assignment_attempt_id = submission.assignment_attempt_id
     WHERE attempt.assignment_id = assignment_row.assignment_id;
    SELECT count(*) INTO grading_result_count
      FROM ple_private.grading_result AS result
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = result.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assignment_attempt_id IN (
         SELECT assignment_attempt_id FROM ple_private.assignment_attempt
          WHERE assignment_id = assignment_row.assignment_id
     );

    -- Retain only exact aggregate identities before the rooted cascade removes
    -- the observation receipts that make them necessary.
    SELECT coalesce(jsonb_agg(jsonb_build_object(
               'questionId', observation.question_id,
               'revisionNumber', observation.revision_number
           )), '[]'::jsonb)
      INTO v_statistics_targets
      FROM (
          SELECT DISTINCT observation.question_id, observation.revision_number
            FROM ple_private.question_statistics_observation_receipt AS observation
            JOIN ple_private.question_attempt AS question_attempt
              ON question_attempt.question_attempt_id = observation.question_attempt_id
            JOIN ple_private.issued_question AS issued
              ON issued.issued_question_id = question_attempt.issued_question_id
           WHERE issued.assignment_attempt_id IN (
               SELECT assignment_attempt_id FROM ple_private.assignment_attempt
                WHERE assignment_id = assignment_row.assignment_id
           )
      ) AS observation;

    UPDATE ple_data.assignment AS updated
       SET assignment_status = 'unreleased',
           assignment_edit_number = updated.assignment_edit_number + 1,
           updated_at = now_at
     WHERE updated.assignment_id = assignment_row.assignment_id
     RETURNING updated.reference_number, updated.assignment_title, updated.assignment_status,
               updated.assignment_edit_number
      INTO assignment_reference_number, assignment_title, assignment_status,
           assignment_edit_number;

    DELETE FROM ple_private.assignment_attempt
     WHERE assignment_id = assignment_row.assignment_id;

    FOR statistics_target IN
        SELECT value FROM jsonb_array_elements(v_statistics_targets)
    LOOP
        PERFORM ple_data.rebuild_question_revision_statistics(
            statistics_target ->> 'questionId',
            (statistics_target ->> 'revisionNumber')::integer,
            now_at
        );
    END LOOP;

    INSERT INTO ple_audit.assignment_unrelease_event (
        event_id, assignment_id, actor_account_id, assignment_edit_number,
        assignment_attempt_count, question_submission_count,
        assignment_submission_count, grading_result_count, outcome, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), assignment_row.assignment_id, actor_account_id,
        assignment_edit_number, assignment_attempt_count, question_submission_count,
        assignment_submission_count, grading_result_count, 'completed', now_at
    );
    RETURN NEXT;
END
$$;

REVOKE ALL ON FUNCTION ple_api.read_assignment_unrelease_impact(bigint, bigint),
    ple_api.unrelease_assignment(bigint, bigint, bigint, text)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_assignment_unrelease_impact(bigint, bigint),
    ple_api.unrelease_assignment(bigint, bigint, bigint, text)
    TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_unrelease_executor;
RESET ROLE;
