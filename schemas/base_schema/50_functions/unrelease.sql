-- Functions, triggers, and views from unrelease.sql.

SET LOCAL ROLE ple_audit_owner;

CREATE FUNCTION ple_audit.reject_assessment_unrelease_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Assessment Unrelease audit evidence is immutable';
END
$$;

CREATE TRIGGER assessment_unrelease_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.assessment_unrelease_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_assessment_unrelease_event_change();

SET LOCAL ROLE ple_unrelease_executor;




-- The platform bootstrap grants the migrator SET authority for this no-login
-- capability.  Creating the function as the capability owner means no API or
-- worker role inherits the destructive privilege.  ASVS 2.2.1/2.2.2 validates
-- confirmation inputs at the trusted boundary; ASVS 2.3.1/2.3.3 keeps the
-- transition, deletion, and audit event indivisible; anonymous totals remain.
CREATE FUNCTION ple_api.read_assessment_unrelease_impact(
    p_course_instance_id text,
    p_assessment_id text
)
RETURNS TABLE (
    assessment_title text,
    assessment_edit_number bigint,
    assessment_attempt_count bigint,
    finalized_saved_response_count bigint,
    assessment_submission_count bigint,
    grading_result_count bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    IF p_course_instance_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;

    SELECT assessment.* INTO assessment_row
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = p_course_instance_id
       AND assessment.assessment_id = p_assessment_id
       AND assessment.assessment_status = 'released'
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;

    SELECT policy.assessment_title INTO assessment_title
      FROM ple_data.assessment_policy_snapshot AS policy
     WHERE policy.assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    assessment_edit_number := assessment_row.assessment_edit_number;
    SELECT count(*) INTO assessment_attempt_count
      FROM ple_private.assessment_attempt
     WHERE assessment_id = assessment_row.assessment_id;
    SELECT count(*) INTO finalized_saved_response_count
      FROM ple_private.assessment_attempt_saved_response AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE submission.finalized_at IS NOT NULL
       AND issued.assessment_attempt_id IN (
         SELECT assessment_attempt_id FROM ple_private.assessment_attempt
          WHERE assessment_id = assessment_row.assessment_id
     );
    SELECT count(*) INTO assessment_submission_count
      FROM ple_private.assessment_submission AS submission
      JOIN ple_private.assessment_attempt AS assessment_attempt
        ON assessment_attempt.assessment_attempt_id = submission.assessment_attempt_id
     WHERE assessment_attempt.assessment_id = assessment_row.assessment_id;
    SELECT count(*) INTO grading_result_count
      FROM ple_private.grading_result AS result
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = result.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assessment_attempt_id IN (
         SELECT assessment_attempt_id FROM ple_private.assessment_attempt
          WHERE assessment_id = assessment_row.assessment_id
     );
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.unrelease_assessment(
    p_course_instance_id text,
    p_assessment_id text,
    p_expected_assessment_edit_number bigint,
    p_confirmation_title text
)
RETURNS TABLE (
    assessment_id text,
    assessment_title text,
    assessment_status text,
    assessment_edit_number bigint,
    assessment_attempt_count bigint,
    finalized_saved_response_count bigint,
    assessment_submission_count bigint,
    grading_result_count bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE actor_account_id text;
DECLARE now_at timestamptz := pg_catalog.transaction_timestamp();
BEGIN
    IF p_course_instance_id IS NULL
       OR p_expected_assessment_edit_number IS NULL OR p_expected_assessment_edit_number <= 0
       OR p_confirmation_title IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;

    actor_account_id := ple_api.current_session_account_id();

    -- This is the mandatory first lock shared by start, save, submission, and
    -- grading paths.  It serializes every Student Work mutation with Unrelease.
    SELECT assessment.* INTO assessment_row
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = p_course_instance_id
       AND assessment.assessment_id = p_assessment_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR UPDATE OF assessment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    IF assessment_row.assessment_status <> 'released' THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assessment is not Released';
    END IF;
    SELECT policy.assessment_title INTO assessment_title
      FROM ple_data.assessment_policy_snapshot AS policy
     WHERE policy.assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    IF p_confirmation_title IS DISTINCT FROM assessment_title THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment Unrelease confirmation title does not match';
    END IF;

    SELECT count(*) INTO assessment_attempt_count
      FROM ple_private.assessment_attempt
     WHERE assessment_id = assessment_row.assessment_id;
    SELECT count(*) INTO finalized_saved_response_count
      FROM ple_private.assessment_attempt_saved_response AS submission
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE submission.finalized_at IS NOT NULL
       AND issued.assessment_attempt_id IN (
         SELECT assessment_attempt_id FROM ple_private.assessment_attempt
          WHERE assessment_id = assessment_row.assessment_id
     );
    SELECT count(*) INTO assessment_submission_count
      FROM ple_private.assessment_submission AS submission
      JOIN ple_private.assessment_attempt AS assessment_attempt
        ON assessment_attempt.assessment_attempt_id = submission.assessment_attempt_id
     WHERE assessment_attempt.assessment_id = assessment_row.assessment_id;
    SELECT count(*) INTO grading_result_count
      FROM ple_private.grading_result AS result
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = result.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE issued.assessment_attempt_id IN (
         SELECT assessment_attempt_id FROM ple_private.assessment_attempt
          WHERE assessment_id = assessment_row.assessment_id
     );

    UPDATE ple_data.assessment AS updated
       SET assessment_status = 'unreleased',
           assessment_edit_number = updated.assessment_edit_number + 1,
           updated_at = now_at
     WHERE updated.assessment_id = assessment_row.assessment_id
     RETURNING updated.assessment_id, updated.assessment_status, updated.assessment_edit_number
      INTO assessment_id, assessment_status, assessment_edit_number;

    -- ASVS 14.2.4: remove Student Work, not approved identity-free totals.
    DELETE FROM ple_private.assessment_attempt
     WHERE assessment_id = assessment_row.assessment_id;

    INSERT INTO ple_audit.assessment_unrelease_event (
        event_id, assessment_id, actor_account_id, assessment_edit_number,
        assessment_attempt_count, finalized_saved_response_count,
        assessment_submission_count, grading_result_count, outcome, occurred_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), assessment_row.assessment_id, actor_account_id,
        assessment_edit_number, assessment_attempt_count, finalized_saved_response_count,
        assessment_submission_count, grading_result_count, 'completed', now_at
    );
    RETURN NEXT;
END
$$;

