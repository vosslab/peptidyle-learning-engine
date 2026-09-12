-- Immutable grading evidence.  Every result is rooted at one accepted
-- Submission and retained Attempt evidence; Assignment Revision is not an
-- interpretation source.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_submission_grading (
    question_submission_grading_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_submission(submission_id)
        ON DELETE CASCADE,
    job_id uuid NOT NULL UNIQUE,
    grading_state text NOT NULL CHECK (grading_state IN (
        'pending', 'instructor_attention', 'graded', 'exempt'
    )),
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    UNIQUE (question_submission_grading_id, submission_id),
    FOREIGN KEY (job_id, submission_id)
        REFERENCES ple_private.job(job_id, question_submission_id)
        ON DELETE CASCADE,
    CHECK ((grading_state IN ('pending', 'instructor_attention') AND completed_at IS NULL)
        OR (grading_state IN ('graded', 'exempt') AND completed_at IS NOT NULL)),
    CHECK (completed_at IS NULL OR completed_at >= created_at)
);

CREATE TABLE ple_private.grading_result (
    grading_result_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_submission(submission_id)
        ON DELETE CASCADE,
    question_submission_grading_id uuid NOT NULL UNIQUE,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id)
        ON DELETE CASCADE,
    correct boolean NOT NULL,
    points_earned numeric NOT NULL CHECK (points_earned >= 0),
    points_possible numeric NOT NULL CHECK (points_possible >= 0),
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (submission_id, question_attempt_id)
        REFERENCES ple_private.question_submission(submission_id, question_attempt_id)
        ON DELETE CASCADE,
    FOREIGN KEY (question_submission_grading_id, submission_id)
        REFERENCES ple_private.question_submission_grading(question_submission_grading_id, submission_id)
        ON DELETE CASCADE,
    UNIQUE (question_submission_grading_id, grading_result_id),
    CHECK (points_earned <= points_possible)
);

CREATE FUNCTION ple_private.enforce_question_submission_grading_transition()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF ROW(NEW.question_submission_grading_id, NEW.submission_id, NEW.job_id, NEW.created_at)
       IS DISTINCT FROM ROW(OLD.question_submission_grading_id, OLD.submission_id, OLD.job_id, OLD.created_at)
       OR OLD.grading_state IN ('graded', 'exempt')
       OR (OLD.grading_state <> 'pending')
       OR NEW.grading_state NOT IN ('instructor_attention', 'graded', 'exempt') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question Submission Grading evidence is forward-only';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_grading_evidence_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Grading evidence is immutable';
END $$;

CREATE TRIGGER question_submission_grading_is_forward_only
BEFORE UPDATE ON ple_private.question_submission_grading
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_submission_grading_transition();
CREATE TRIGGER question_submission_grading_delete_is_guarded
BEFORE DELETE ON ple_private.question_submission_grading
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER grading_result_is_immutable
BEFORE UPDATE ON ple_private.grading_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_grading_evidence_change();
CREATE TRIGGER grading_result_delete_is_guarded
BEFORE DELETE ON ple_private.grading_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

-- The audit owner owns the receipt relation but its receipt is an exclusive
-- child of private Student Work.  It needs only the FK/trigger capability.
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.grading_result TO ple_audit_owner;
GRANT EXECUTE ON FUNCTION ple_private.reject_student_work_delete() TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.automated_grading_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY,
    question_submission_grading_id uuid NOT NULL,
    grading_result_id uuid NOT NULL UNIQUE,
    committed_at timestamptz NOT NULL,
    automated_grading_receipt_checksum bytea NOT NULL
        CHECK (octet_length(automated_grading_receipt_checksum) = 32),
    FOREIGN KEY (question_submission_grading_id, grading_result_id)
        REFERENCES ple_private.grading_result(question_submission_grading_id, grading_result_id)
        ON DELETE CASCADE
);
CREATE FUNCTION ple_audit.reject_automated_grading_receipt_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Automated Grading Receipt is immutable';
END $$;
CREATE TRIGGER automated_grading_receipt_is_immutable
BEFORE UPDATE ON ple_audit.automated_grading_receipt
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_automated_grading_receipt_change();
CREATE TRIGGER automated_grading_receipt_delete_is_guarded
BEFORE DELETE ON ple_audit.automated_grading_receipt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

ALTER TABLE ple_audit.automated_grading_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.automated_grading_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.automated_grading_receipt FROM PUBLIC;
CREATE POLICY automated_grading_receipt_audit_owner_access
    ON ple_audit.automated_grading_receipt
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
CREATE POLICY automated_grading_receipt_api_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY automated_grading_receipt_private_owner_insert
    ON ple_audit.automated_grading_receipt
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY automated_grading_receipt_private_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_private_owner USING (true);
GRANT USAGE ON SCHEMA ple_audit TO ple_api_owner;
GRANT SELECT ON ple_audit.automated_grading_receipt TO ple_api_owner;
GRANT SELECT, INSERT ON ple_audit.automated_grading_receipt TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.question_submission_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_submission_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.grading_result ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.grading_result FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.question_submission_grading, ple_private.grading_result FROM PUBLIC;
CREATE POLICY question_submission_grading_private_owner_access
    ON ple_private.question_submission_grading FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY grading_result_private_owner_access
    ON ple_private.grading_result FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

-- Grade enqueue, commit, and final-failure all take the Assignment row lock
-- before they touch Attempt-rooted Student Work.  The private owner has no
-- general Assignment mutation path; this narrow cross-domain capability is
-- only what PostgreSQL requires for that root lock inside the fixed routines.
SET LOCAL ROLE ple_data_owner;
GRANT SELECT, UPDATE (assignment_id) ON TABLE ple_data.assignment TO ple_private_owner;
CREATE POLICY assignment_private_owner_grading_root_lock
    ON ple_data.assignment FOR UPDATE TO ple_private_owner
    USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

-- Completion reads only the Attempt's retained policy and issued scoring
-- facts.  It holds the Assignment Attempt while it decides whether a result
-- completes it; callers already acquire the Assignment root before commit.
CREATE FUNCTION ple_private.complete_assignment_attempt_after_grading()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    target_attempt ple_private.assignment_attempt%ROWTYPE;
    required_count bigint;
    graded_count bigint;
    all_correct boolean;
    earned numeric;
    possible numeric;
    score numeric;
    complete boolean;
BEGIN
    SELECT assignment_attempt.* INTO target_attempt
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
     WHERE question_attempt.question_attempt_id = NEW.question_attempt_id
       AND assignment_attempt.completed_at IS NULL
     FOR UPDATE OF assignment_attempt;
    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    SELECT count(*)::bigint,
           count(result.grading_result_id)::bigint,
           coalesce(bool_and(result.correct), false),
           coalesce(sum(result.points_earned), 0),
           coalesce(sum(result.points_possible), 0)
      INTO required_count, graded_count, all_correct, earned, possible
      FROM ple_private.issued_question AS issued
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = target_attempt.assignment_attempt_id;

    score := CASE WHEN possible > 0 THEN earned / possible ELSE 0 END;
    complete := required_count > 0 AND graded_count = required_count AND CASE
        WHEN target_attempt.assignment_completion_rule = 'answer_all' THEN true
        WHEN target_attempt.assignment_completion_rule = 'all_correct' THEN all_correct
        WHEN target_attempt.assignment_completion_rule = 'score_at_least'
            THEN possible > 0 AND score >= target_attempt.assignment_completion_score_threshold
        ELSE false
    END;
    IF complete THEN
        UPDATE ple_private.assignment_attempt
           SET completed_at = greatest(target_attempt.started_at, NEW.recorded_at),
               completion_score = score
         WHERE assignment_attempt_id = target_attempt.assignment_attempt_id
           AND completed_at IS NULL;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER grading_result_may_complete_assignment_attempt
AFTER INSERT ON ple_private.grading_result
FOR EACH ROW EXECUTE FUNCTION ple_private.complete_assignment_attempt_after_grading();

-- Backend-specific API procedures own renderer and external-service checks.
-- They call these narrow private helpers with a constant worker kind, so a
-- worker capability cannot claim another backend's Submission.
CREATE FUNCTION ple_private.claim_grade_accepted_submission(
    p_worker_kind text,
    p_lease_token uuid,
    p_lease_expires_at timestamptz,
    p_requested_job_id uuid DEFAULT NULL
) RETURNS TABLE (job_id uuid, submission_id uuid)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE now_at timestamptz := pg_catalog.clock_timestamp();
BEGIN
    IF p_worker_kind NOT IN (
        'native_ple_grading', 'webwork_grading',
        'imathas_question_backend_grading'
    ) OR p_lease_token IS NULL OR p_lease_expires_at <= now_at
      OR p_lease_expires_at > now_at + interval '300 seconds' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Grade Job lease arguments are invalid';
    END IF;
    -- ASVS 2.3.1/2.3.2: an expired last lease reaches its terminal state;
    -- a worker cannot repeatedly reclaim it beyond its bounded budget.
    UPDATE ple_private.job AS job
       SET state = 'failed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = now_at, failure_kind = 'timed_out'
     WHERE job.job_kind = 'grade_accepted_submission'
       AND job.worker_kind = p_worker_kind
       AND job.state = 'leased'
       AND job.lease_expires_at <= now_at
       AND job.attempt_count >= job.max_attempts
       AND (p_requested_job_id IS NULL OR job.job_id = p_requested_job_id);

    RETURN QUERY
    WITH candidate AS (
        SELECT job.job_id
          FROM ple_private.job AS job
          JOIN ple_private.question_submission_grading AS grading
            ON grading.job_id = job.job_id
          JOIN ple_private.question_submission AS submission
            ON submission.submission_id = job.question_submission_id
          JOIN ple_private.question_attempt AS question_attempt
            ON question_attempt.question_attempt_id = submission.question_attempt_id
          JOIN ple_private.issued_question AS issued
            ON issued.issued_question_id = question_attempt.issued_question_id
          JOIN ple_private.question_revision_source_binding AS source
            ON source.question_id = issued.question_id
           AND source.revision_number = issued.revision_number
         WHERE job.job_kind = 'grade_accepted_submission'
           AND job.job_target_kind = 'question_submission'
           AND job.worker_kind = p_worker_kind
           -- Select a Job only when its exact Question Revision is supported
           -- by that fixed worker.  Filtering after leasing would strand a
           -- valid Job for another worker kind.
           AND (
               (p_worker_kind = 'native_ple_grading'
                AND source.backend = 'ple' AND source.question_format = 'pleQuestionJson')
            OR (p_worker_kind = 'webwork_grading'
                AND source.backend = 'webwork' AND source.question_format = 'webworkPg'
                AND EXISTS (
                    SELECT 1
                      FROM ple_private.question_attempt_webwork_replay AS replay
                     WHERE replay.question_attempt_id = question_attempt.question_attempt_id
                ))
            OR (p_worker_kind = 'imathas_question_backend_grading'
                AND source.backend = 'imathas' AND source.question_format = 'imathas')
           )
           AND grading.grading_state = 'pending'
           AND job.attempt_count < job.max_attempts
           AND (p_requested_job_id IS NULL OR job.job_id = p_requested_job_id)
           AND ((job.state = 'ready' AND job.available_at <= now_at)
             OR (job.state = 'leased' AND job.lease_expires_at <= now_at))
         ORDER BY job.available_at, job.job_id
         LIMIT 1
         FOR UPDATE OF job SKIP LOCKED
    ), claimed AS (
        UPDATE ple_private.job AS job
           SET state = 'leased', lease_token = p_lease_token,
               lease_expires_at = p_lease_expires_at, attempt_count = job.attempt_count + 1
          FROM candidate
         WHERE job.job_id = candidate.job_id
        RETURNING job.job_id, job.question_submission_id
    ) SELECT claimed.job_id, claimed.question_submission_id FROM claimed;
END $$;

CREATE FUNCTION ple_private.commit_grade_accepted_submission(
    p_worker_kind text,
    p_job_id uuid,
    p_lease_token uuid,
    p_grading_result_id uuid,
    p_automated_grading_receipt_id uuid,
    p_correct boolean,
    p_points_earned numeric,
    p_recorded_at timestamptz,
    p_eligible_choice_ids text[] DEFAULT ARRAY[]::text[]
) RETURNS TABLE (
    automated_grading_receipt_id uuid,
    automated_grading_receipt_checksum bytea,
    correct boolean,
    points_earned numeric,
    points_possible numeric
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private, ple_audit AS $$
DECLARE
    job_row ple_private.job%ROWTYPE;
    grading_row ple_private.question_submission_grading%ROWTYPE;
    locked_assignment_attempt_id uuid;
    question_attempt_id uuid;
    issued_point_value numeric;
    calculated_checksum bytea;
    existing_receipt_id uuid;
    existing_receipt_checksum bytea;
    existing_correct boolean;
    existing_points_earned numeric;
    existing_points_possible numeric;
BEGIN
    IF p_worker_kind NOT IN (
           'native_ple_grading', 'webwork_grading',
           'imathas_question_backend_grading'
       ) OR p_job_id IS NULL OR p_lease_token IS NULL OR p_grading_result_id IS NULL
       OR p_automated_grading_receipt_id IS NULL OR p_correct IS NULL
       OR p_points_earned IS NULL OR p_points_earned < 0 OR p_recorded_at IS NULL
       OR p_eligible_choice_ids IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Grade Job commit facts are invalid';
    END IF;

    -- Lock the Assignment root before its Attempt and child work.  If
    -- Unrelease wins this lock, the lease no longer has a commit target.
    SELECT assignment_attempt.assignment_attempt_id INTO locked_assignment_attempt_id
      FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = assignment_attempt.assignment_id
     WHERE job.job_id = p_job_id
       AND job.job_kind = 'grade_accepted_submission'
       AND job.job_target_kind = 'question_submission'
       AND job.worker_kind = p_worker_kind
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Grade Job does not have a current Assignment root';
    END IF;
    PERFORM 1 FROM ple_private.assignment_attempt
     WHERE assignment_attempt_id = locked_assignment_attempt_id FOR UPDATE;

    SELECT * INTO job_row
      FROM ple_private.job AS job
     WHERE job.job_id = p_job_id
     FOR UPDATE;
    -- ASVS 2.3.1/2.3.3: a lost worker response converges on the immutable
    -- receipt written by the first successful transaction.
    IF job_row.state = 'completed' THEN
        SELECT receipt.automated_grading_receipt_id,
               receipt.automated_grading_receipt_checksum,
               result.correct, result.points_earned, result.points_possible
          INTO existing_receipt_id, existing_receipt_checksum,
               existing_correct, existing_points_earned, existing_points_possible
          FROM ple_audit.automated_grading_receipt AS receipt
          JOIN ple_private.grading_result AS result
            ON result.grading_result_id = receipt.grading_result_id
          JOIN ple_private.question_submission_grading AS grading
            ON grading.question_submission_grading_id = result.question_submission_grading_id
         WHERE grading.job_id = p_job_id;
        IF FOUND THEN
            RETURN QUERY SELECT existing_receipt_id, existing_receipt_checksum,
                existing_correct, existing_points_earned, existing_points_possible;
            RETURN;
        END IF;
        RAISE EXCEPTION USING ERRCODE = 'XX000',
            MESSAGE = 'Completed Grade Job has no immutable receipt';
    END IF;
    SELECT question_attempt.question_attempt_id, issued.point_value
      INTO question_attempt_id, issued_point_value
      FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE job.job_id = p_job_id
     FOR UPDATE OF question_attempt;
    SELECT * INTO grading_row FROM ple_private.question_submission_grading
     WHERE job_id = p_job_id FOR UPDATE;
    IF NOT FOUND OR job_row.job_kind <> 'grade_accepted_submission'
       OR job_row.state <> 'leased' OR job_row.lease_token <> p_lease_token
       OR job_row.lease_expires_at <= pg_catalog.clock_timestamp()
       OR grading_row.grading_state <> 'pending'
       OR p_points_earned > issued_point_value THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Grade Job does not own a valid current lease';
    END IF;

    INSERT INTO ple_private.grading_result (
        grading_result_id, submission_id, question_submission_grading_id,
        question_attempt_id, correct, points_earned, points_possible, recorded_at
    ) VALUES (
        p_grading_result_id, job_row.question_submission_id,
        grading_row.question_submission_grading_id, question_attempt_id,
        p_correct, p_points_earned, issued_point_value, p_recorded_at
    );
    calculated_checksum := pg_catalog.sha256(
        pg_catalog.convert_to('ple:automated-grading-receipt:v1', 'UTF8')
        || pg_catalog.uuid_send(p_automated_grading_receipt_id)
        || pg_catalog.uuid_send(p_grading_result_id)
        || pg_catalog.uuid_send(grading_row.question_submission_grading_id)
        || pg_catalog.uuid_send(job_row.question_submission_id)
        || pg_catalog.uuid_send(question_attempt_id)
        || pg_catalog.numeric_send(p_points_earned)
        || pg_catalog.numeric_send(issued_point_value)
        || pg_catalog.int8send((extract(epoch FROM p_recorded_at) * 1000)::bigint)
    );
    INSERT INTO ple_audit.automated_grading_receipt (
        automated_grading_receipt_id, question_submission_grading_id,
        grading_result_id, committed_at, automated_grading_receipt_checksum
    ) VALUES (
        p_automated_grading_receipt_id, grading_row.question_submission_grading_id,
        p_grading_result_id, p_recorded_at, calculated_checksum
    );
    UPDATE ple_private.question_submission_grading
       SET grading_state = 'graded', completed_at = p_recorded_at
     WHERE question_submission_grading_id = grading_row.question_submission_grading_id;
    -- Statistics retain identity-free observations derived from this exact
    -- immutable receipt.  Its unique receipt key makes a commit replay count
    -- once while preserving the Unrelease rebuild source.
    PERFORM ple_private.capture_question_statistics_observation(
        p_automated_grading_receipt_id, p_eligible_choice_ids);
    UPDATE ple_private.job
       SET state = 'completed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = p_recorded_at
     WHERE job_id = p_job_id;
    RETURN QUERY SELECT p_automated_grading_receipt_id, calculated_checksum,
        p_correct, p_points_earned, issued_point_value;
END $$;

-- These fixed-kind private adapters retain all table reads under the private
-- owner.  The API wrappers below only delegate, so a worker capability cannot
-- choose another backend's Job kind (ASVS 8.2.1, 8.3.1).
CREATE FUNCTION ple_private.claim_native_ple_grading_job(
    p_lease_token uuid,
    p_lease_expires_at timestamptz
) RETURNS TABLE (
    job_id uuid, question_attempt_id uuid, question_id text, revision_number integer,
    source_object_id uuid, source_object_checksum text, question_seed numeric,
    student_response jsonb
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    WITH claimed AS (
        SELECT * FROM ple_private.claim_grade_accepted_submission(
            'native_ple_grading', p_lease_token, p_lease_expires_at, NULL)
    )
    SELECT claimed.job_id, question_attempt.question_attempt_id, issued.question_id,
           issued.revision_number, source.source_object_id,
           source.source_object_checksum, question_attempt.question_seed,
           submission.student_response
      FROM claimed
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = claimed.submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id
       AND source.revision_number = issued.revision_number
     WHERE source.backend = 'ple' AND source.question_format = 'pleQuestionJson'
$$;

CREATE FUNCTION ple_private.claim_webwork_grading_job(
    p_lease_token uuid,
    p_lease_expires_at timestamptz
) RETURNS TABLE (
    job_id uuid, question_attempt_id uuid, question_id text, revision_number integer,
    source_object_id uuid, source_object_checksum text, webwork_pg_path text,
    question_seed numeric, student_response jsonb, replay_details jsonb
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    WITH claimed AS (
        SELECT * FROM ple_private.claim_grade_accepted_submission(
            'webwork_grading', p_lease_token, p_lease_expires_at, NULL)
    )
    SELECT claimed.job_id, question_attempt.question_attempt_id, issued.question_id,
           issued.revision_number, source.source_object_id,
           source.source_object_checksum, source.webwork_pg_path,
           question_attempt.question_seed, submission.student_response,
           replay.replay_details
      FROM claimed
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = claimed.submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id
       AND source.revision_number = issued.revision_number
      JOIN ple_private.question_attempt_webwork_replay AS replay
        ON replay.question_attempt_id = question_attempt.question_attempt_id
     WHERE source.backend = 'webwork' AND source.question_format = 'webworkPg'
$$;

CREATE FUNCTION ple_private.commit_normalized_grading(
    p_worker_kind text,
    p_job_id uuid,
    p_lease_token uuid,
    p_correct boolean,
    p_normalized_credit double precision,
    p_recorded_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE point_value numeric;
BEGIN
    IF p_normalized_credit IS NULL OR p_normalized_credit < 0
       OR p_normalized_credit > 1 OR p_recorded_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Normalized Grade facts are invalid';
    END IF;
    SELECT issued.point_value INTO point_value
      FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE job.job_id = p_job_id AND job.worker_kind = p_worker_kind;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Grade Job is unavailable';
    END IF;
    PERFORM ple_private.commit_grade_accepted_submission(
        p_worker_kind, p_job_id, p_lease_token, pg_catalog.gen_random_uuid(),
        pg_catalog.gen_random_uuid(), p_correct,
        point_value * p_normalized_credit::numeric, p_recorded_at, ARRAY[]::text[]);
END $$;

CREATE FUNCTION ple_private.fail_webwork_grading(
    p_job_id uuid,
    p_lease_token uuid,
    p_completed_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE assignment_attempt_id_value uuid;
DECLARE job_row ple_private.job%ROWTYPE;
BEGIN
    IF p_job_id IS NULL OR p_lease_token IS NULL OR p_completed_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'WeBWorK Job failure facts are invalid';
    END IF;
    SELECT assignment_attempt.assignment_attempt_id INTO assignment_attempt_id_value
      FROM ple_private.job AS job
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = job.question_submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assignment_attempt AS assignment_attempt
        ON assignment_attempt.assignment_attempt_id = issued.assignment_attempt_id
      JOIN ple_data.assignment AS assignment
        ON assignment.assignment_id = assignment_attempt.assignment_id
     WHERE job.job_id = p_job_id AND job.worker_kind = 'webwork_grading'
     FOR UPDATE OF assignment;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'WeBWorK Job is unavailable';
    END IF;
    PERFORM 1 FROM ple_private.assignment_attempt
     WHERE assignment_attempt_id = assignment_attempt_id_value FOR UPDATE;
    SELECT * INTO job_row FROM ple_private.job WHERE job_id = p_job_id FOR UPDATE;
    IF job_row.state = 'failed' AND EXISTS (
        SELECT 1 FROM ple_private.question_submission_grading
         WHERE job_id = p_job_id AND grading_state = 'instructor_attention'
    ) THEN
        RETURN;
    END IF;
    IF job_row.state <> 'leased' OR job_row.lease_token <> p_lease_token
       OR job_row.lease_expires_at <= pg_catalog.clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'WeBWorK Job lease is unavailable';
    END IF;
    UPDATE ple_private.question_submission_grading
       SET grading_state = 'instructor_attention'
     WHERE job_id = p_job_id AND grading_state = 'pending';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'WeBWorK Job grading state is unavailable';
    END IF;
    UPDATE ple_private.job
       SET state = 'failed', lease_token = NULL, lease_expires_at = NULL,
           completed_at = p_completed_at, failure_kind = 'final'
     WHERE job_id = p_job_id;
END $$;

-- Gradebook aggregation reads retained Student Work rather than mutable
-- Assignment configuration.  The course-facing API below supplies current
-- released Assignments and active Students; this helper selects one Student's
-- most recent Attempt for one Assignment and exposes only answer-free facts.
CREATE FUNCTION ple_private.read_latest_assignment_attempt_gradebook_evidence(
    p_student_record_id uuid,
    p_assignment_id uuid
) RETURNS TABLE (
    assignment_attempt_id uuid,
    assignment_attempt_completion text,
    graded_question_count bigint,
    issued_question_count bigint,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    WITH latest_attempt AS (
        SELECT attempt.assignment_attempt_id, attempt.completed_at
          FROM ple_private.assignment_attempt AS attempt
         WHERE attempt.student_record_id = p_student_record_id
           AND attempt.assignment_id = p_assignment_id
         ORDER BY attempt.started_at DESC, attempt.assignment_attempt_id DESC
         LIMIT 1
    )
    SELECT latest_attempt.assignment_attempt_id,
           CASE WHEN latest_attempt.completed_at IS NULL THEN 'in_progress'
                ELSE 'completed' END,
           count(result.grading_result_id)::bigint,
           count(issued.issued_question_id)::bigint,
           coalesce(sum(result.points_earned), 0)::double precision,
           coalesce(sum(result.points_possible), 0)::double precision
      FROM latest_attempt
      LEFT JOIN ple_private.issued_question AS issued
        ON issued.assignment_attempt_id = latest_attempt.assignment_attempt_id
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
     GROUP BY latest_attempt.assignment_attempt_id, latest_attempt.completed_at
$$;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.has_automated_grading_receipt(
    p_question_submission_grading_id uuid,
    p_grading_result_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_audit AS $$
    SELECT p_question_submission_grading_id IS NOT NULL
       AND p_grading_result_id IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM ple_audit.automated_grading_receipt AS receipt
            WHERE receipt.question_submission_grading_id = p_question_submission_grading_id
              AND receipt.grading_result_id = p_grading_result_id
       )
$$;
CREATE FUNCTION ple_api.claim_native_ple_grading_job(
    p_lease_token uuid, p_lease_expires_at timestamptz
) RETURNS TABLE (
    job_id uuid, question_attempt_id uuid, question_id text, revision_number integer,
    source_object_id uuid, source_object_checksum text, question_seed numeric,
    student_response jsonb
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.claim_native_ple_grading_job(p_lease_token, p_lease_expires_at)
$$;
CREATE FUNCTION ple_api.commit_native_ple_grading(
    p_job_id uuid, p_lease_token uuid, p_correct boolean,
    p_normalized_credit double precision, p_committed_at timestamptz
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.commit_normalized_grading('native_ple_grading', p_job_id,
        p_lease_token, p_correct, p_normalized_credit, p_committed_at)
$$;
CREATE FUNCTION ple_api.claim_webwork_grading_job(
    p_lease_token uuid, p_lease_expires_at timestamptz
) RETURNS TABLE (
    job_id uuid, question_attempt_id uuid, question_id text, revision_number integer,
    source_object_id uuid, source_object_checksum text, webwork_pg_path text,
    question_seed numeric, student_response jsonb, replay_details jsonb
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT * FROM ple_private.claim_webwork_grading_job(p_lease_token, p_lease_expires_at)
$$;
CREATE FUNCTION ple_api.commit_webwork_grading(
    p_job_id uuid, p_lease_token uuid, p_correct boolean,
    p_normalized_credit double precision, p_committed_at timestamptz
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.commit_normalized_grading('webwork_grading', p_job_id,
        p_lease_token, p_correct, p_normalized_credit, p_committed_at)
$$;
CREATE FUNCTION ple_api.fail_webwork_grading(
    p_job_id uuid, p_lease_token uuid, p_completed_at timestamptz
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT ple_private.fail_webwork_grading(p_job_id, p_lease_token, p_completed_at)
$$;

-- An Instructor's course gradebook combines current released Assignment
-- aggregates with the chosen Attempt's immutable issue/submission/grading
-- evidence.  It contains neither responses nor Question content.
CREATE FUNCTION ple_api.read_course_gradebook(p_course_reference_number bigint)
RETURNS TABLE (
    course_reference_number bigint,
    roster_id text,
    assignment_reference_number bigint,
    assignment_attempt_completion text,
    graded_question_count bigint,
    question_count bigint,
    points_earned double precision,
    points_possible double precision
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    WITH course AS (
        SELECT instance.course_id, instance.reference_number
          FROM ple_data.course_instance AS instance
         WHERE instance.reference_number = p_course_reference_number
           AND ple_api.current_session_account_is_course_instructor(instance.course_id)
    ), active_student AS (
        SELECT record.student_record_id, profile.roster_id
          FROM course
          JOIN ple_data.student_record AS record
            ON record.course_id = course.course_id
          JOIN ple_data.course_membership AS membership
            ON membership.course_id = course.course_id
           AND membership.student_record_id = record.student_record_id
           AND membership.account_id = record.student_account_id
           AND membership.role = 'student'
           AND ple_data.course_membership_is_active(membership.membership_id)
          JOIN ple_private.course_roster_profile AS profile
            ON profile.course_id = course.course_id
           AND profile.student_account_id = record.student_account_id
    ), released_assignment AS (
        SELECT assignment.assignment_id, assignment.reference_number,
               coalesce(sum(CASE entry.entry_kind
                   WHEN 'fixed_question' THEN 1
                   ELSE entry.selection_count
               END) FILTER (WHERE entry.availability = 'available'), 0)::bigint
                   AS current_question_count
          FROM course
          JOIN ple_data.assignment AS assignment
            ON assignment.course_id = course.course_id
           AND assignment.assignment_status = 'released'
          LEFT JOIN ple_data.assignment_entry AS entry
            ON entry.assignment_id = assignment.assignment_id
         GROUP BY assignment.assignment_id, assignment.reference_number
    ), gradebook AS (
        SELECT course.reference_number AS course_reference_number,
               student.roster_id,
               assignment.reference_number AS assignment_reference_number,
               evidence.assignment_attempt_completion,
               coalesce(evidence.graded_question_count, 0)::bigint AS graded_question_count,
               coalesce(evidence.issued_question_count, assignment.current_question_count)::bigint
                   AS question_count,
               coalesce(evidence.points_earned, 0)::double precision AS points_earned,
               coalesce(evidence.points_possible, 0)::double precision AS points_possible
          FROM course
          CROSS JOIN active_student AS student
          CROSS JOIN released_assignment AS assignment
          LEFT JOIN LATERAL ple_private.read_latest_assignment_attempt_gradebook_evidence(
              student.student_record_id, assignment.assignment_id
          ) AS evidence ON true
    )
    SELECT course_reference_number, roster_id, assignment_reference_number,
           assignment_attempt_completion, graded_question_count, question_count,
           points_earned, points_possible
      FROM gradebook
    UNION ALL
    SELECT course.reference_number, NULL::text, NULL::bigint, NULL::text,
           NULL::bigint, NULL::bigint, NULL::double precision, NULL::double precision
      FROM course
     WHERE NOT EXISTS (SELECT 1 FROM gradebook)
     ORDER BY roster_id NULLS FIRST, assignment_reference_number NULLS FIRST
$$;

REVOKE ALL ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid),
    ple_api.claim_native_ple_grading_job(uuid, timestamptz),
    ple_api.commit_native_ple_grading(uuid, uuid, boolean, double precision, timestamptz),
    ple_api.claim_webwork_grading_job(uuid, timestamptz),
    ple_api.commit_webwork_grading(uuid, uuid, boolean, double precision, timestamptz),
    ple_api.fail_webwork_grading(uuid, uuid, timestamptz),
    ple_api.read_course_gradebook(bigint)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid)
    TO ple_private_owner;
GRANT USAGE ON SCHEMA ple_api
    TO ple_native_ple_grading_worker, ple_webwork_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.claim_native_ple_grading_job(uuid, timestamptz),
    ple_api.commit_native_ple_grading(uuid, uuid, boolean, double precision, timestamptz)
    TO ple_native_ple_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.claim_webwork_grading_job(uuid, timestamptz),
    ple_api.commit_webwork_grading(uuid, uuid, boolean, double precision, timestamptz),
    ple_api.fail_webwork_grading(uuid, uuid, timestamptz)
    TO ple_webwork_grading_worker;
GRANT EXECUTE ON FUNCTION ple_api.read_course_gradebook(bigint) TO ple_app;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
REVOKE ALL ON FUNCTION ple_private.enforce_question_submission_grading_transition(),
    ple_private.reject_grading_evidence_change(),
    ple_private.complete_assignment_attempt_after_grading(),
    ple_private.claim_grade_accepted_submission(text, uuid, timestamptz, uuid),
    ple_private.commit_grade_accepted_submission(text, uuid, uuid, uuid, uuid, boolean, numeric, timestamptz, text[]),
    ple_private.claim_native_ple_grading_job(uuid, timestamptz),
    ple_private.claim_webwork_grading_job(uuid, timestamptz),
    ple_private.commit_normalized_grading(text, uuid, uuid, boolean, double precision, timestamptz),
    ple_private.fail_webwork_grading(uuid, uuid, timestamptz),
    ple_private.read_latest_assignment_attempt_gradebook_evidence(uuid, uuid)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.claim_grade_accepted_submission(text, uuid, timestamptz, uuid),
    ple_private.commit_grade_accepted_submission(text, uuid, uuid, uuid, uuid, boolean, numeric, timestamptz, text[]),
    ple_private.claim_native_ple_grading_job(uuid, timestamptz),
    ple_private.claim_webwork_grading_job(uuid, timestamptz),
    ple_private.commit_normalized_grading(text, uuid, uuid, boolean, double precision, timestamptz),
    ple_private.fail_webwork_grading(uuid, uuid, timestamptz)
    TO ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_private.read_latest_assignment_attempt_gradebook_evidence(uuid, uuid)
    TO ple_api_owner;

COMMENT ON TABLE ple_private.grading_result IS
    'One immutable scored result for an accepted Submission, interpreted through retained Attempt and Issued Question evidence.';
SET LOCAL ROLE ple_audit_owner;
REVOKE ALL ON FUNCTION ple_audit.reject_automated_grading_receipt_change() FROM PUBLIC;
COMMENT ON TABLE ple_audit.automated_grading_receipt IS
    'Immutable receipt for one automated grading commit; deleted only with its exclusive Student Work root.';

RESET ROLE;
