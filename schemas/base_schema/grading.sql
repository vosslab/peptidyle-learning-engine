-- Immutable grading evidence.  Every result is rooted at one accepted
-- Submission and retained Assessment Attempt evidence; Assessment Revision is not an
-- interpretation source.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_submission_grading (
    question_submission_grading_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_submission(submission_id)
        ON DELETE CASCADE,
    grading_state text NOT NULL DEFAULT 'graded' CHECK (grading_state = 'graded'),
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    UNIQUE (question_submission_grading_id, submission_id),
    CHECK (completed_at IS NOT NULL),
    CHECK (completed_at IS NULL OR completed_at >= created_at)
);

CREATE TABLE ple_private.grading_result (
    grading_result_id uuid PRIMARY KEY,
    submission_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_submission(submission_id)
        ON DELETE CASCADE,
    question_submission_grading_id uuid NOT NULL UNIQUE,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id)
        ON DELETE CASCADE,
    -- The Question Backend owns response interpretation.  PLE retains only
    -- its normalized immutable outcome; point values remain Assessment
    -- configuration and are applied by score readers.
    normalized_credit numeric NOT NULL CHECK (
        normalized_credit >= 0 AND normalized_credit <= 1
    ),
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (submission_id, question_attempt_id)
        REFERENCES ple_private.question_submission(submission_id, question_attempt_id)
        ON DELETE CASCADE,
    FOREIGN KEY (question_submission_grading_id, submission_id)
        REFERENCES ple_private.question_submission_grading(question_submission_grading_id, submission_id)
        ON DELETE CASCADE,
    UNIQUE (question_submission_grading_id, grading_result_id)
);

-- Applies current Assessment Entry points to retained backend credit.  The
-- stable Entry identifier remains present after a released Assessment save,
-- including when an Entry is retired, so historical Student Work continues
-- to have a current score without consulting a Question Backend.
CREATE FUNCTION ple_private.score_recorded_credit(
    p_normalized_credit numeric,
    p_scoring_rule text,
    p_point_value numeric
) RETURNS TABLE (points_earned numeric, points_possible numeric)
LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT CASE p_scoring_rule
               WHEN 'full_credit' THEN p_point_value
               WHEN 'excluded' THEN 0::numeric
               ELSE p_point_value * p_normalized_credit
           END,
           p_point_value
     WHERE p_normalized_credit >= 0 AND p_normalized_credit <= 1
       AND p_scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')
       AND p_point_value >= 0
$$;

CREATE FUNCTION ple_private.reject_grading_evidence_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Grading evidence is immutable';
END $$;

CREATE TRIGGER question_submission_grading_is_immutable
BEFORE UPDATE ON ple_private.question_submission_grading
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_grading_evidence_change();
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

SET LOCAL ROLE ple_private_owner;

-- Completion reads only the Assessment Attempt's retained policy and issued scoring
-- facts.  It holds the Assessment Attempt while it decides whether a result
-- completes it; callers already acquire the Assessment root before commit.
CREATE FUNCTION ple_private.complete_assessment_attempt_after_grading()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    target_assessment_attempt ple_private.assessment_attempt%ROWTYPE;
    required_count bigint;
    resolved_count bigint;
    all_correct boolean;
    earned numeric;
    possible numeric;
    score numeric;
    complete boolean;
    deadline_finalized boolean;
BEGIN
    SELECT assessment_attempt.* INTO target_assessment_attempt
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.assessment_attempt AS assessment_attempt
        ON assessment_attempt.assessment_attempt_id = issued.assessment_attempt_id
     WHERE question_attempt.question_attempt_id = NEW.question_attempt_id
       AND assessment_attempt.completed_at IS NULL
     FOR UPDATE OF assessment_attempt;
    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    SELECT count(*)::bigint,
           count(*) FILTER (
               WHERE question_attempt.question_attempt_state = 'closed_at_deadline'
                  OR result.grading_result_id IS NOT NULL
           )::bigint,
           coalesce(bool_and(CASE
               WHEN question_attempt.question_attempt_state = 'closed_at_deadline' THEN false
               ELSE result.normalized_credit = 1
           END), false),
           coalesce(sum(score.points_earned), 0),
           coalesce(sum(score.points_possible), 0)
      INTO required_count, resolved_count, all_correct, earned, possible
      FROM ple_private.issued_question AS issued
      LEFT JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.grading_result AS result
        ON result.question_attempt_id = question_attempt.question_attempt_id
      JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_entry_id = issued.assessment_entry_id
      CROSS JOIN LATERAL ple_private.score_recorded_credit(
          coalesce(result.normalized_credit, 0), issued.scoring_rule,
          CASE entry.entry_kind
              WHEN 'fixed_question' THEN entry.points_possible
              ELSE entry.points_per_item
          END
      ) AS score
     WHERE issued.assessment_attempt_id = target_assessment_attempt.assessment_attempt_id;

    SELECT EXISTS (
        SELECT 1 FROM ple_private.assessment_submission AS submission
         WHERE submission.assessment_attempt_id = target_assessment_attempt.assessment_attempt_id
           AND submission.finalization_kind = 'deadline'
    ) INTO deadline_finalized;
    score := CASE WHEN possible > 0 THEN earned / possible ELSE 0 END;
    complete := required_count > 0 AND resolved_count = required_count AND (
        deadline_finalized OR CASE
            WHEN target_assessment_attempt.assessment_completion_rule = 'answer_all' THEN true
            WHEN target_assessment_attempt.assessment_completion_rule = 'all_correct' THEN all_correct
            WHEN target_assessment_attempt.assessment_completion_rule = 'score_at_least'
                THEN possible > 0 AND score >= target_assessment_attempt.assessment_completion_score_threshold
            ELSE false
        END
    );
    IF complete THEN
        UPDATE ple_private.assessment_attempt
           SET completed_at = greatest(target_assessment_attempt.started_at, NEW.recorded_at)
         WHERE assessment_attempt_id = target_assessment_attempt.assessment_attempt_id
           AND completed_at IS NULL;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER grading_result_may_complete_assessment_attempt
AFTER INSERT ON ple_private.grading_result
FOR EACH ROW EXECUTE FUNCTION ple_private.complete_assessment_attempt_after_grading();

-- Inserts the immutable evidence for one backend result that was obtained
-- before the finalization transaction. The caller has already revalidated the
-- saved-response snapshot and locked the Assessment root.
CREATE FUNCTION ple_private.record_direct_automated_grading_result(
    p_submission_id uuid,
    p_question_attempt_id uuid,
    p_normalized_credit numeric,
    p_recorded_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_audit AS $$
DECLARE grading_id uuid := pg_catalog.gen_random_uuid();
DECLARE result_id uuid := pg_catalog.gen_random_uuid();
DECLARE receipt_id uuid := pg_catalog.gen_random_uuid();
DECLARE calculated_checksum bytea;
BEGIN
    IF p_submission_id IS NULL OR p_question_attempt_id IS NULL
       OR p_normalized_credit IS NULL OR p_normalized_credit < 0
       OR p_normalized_credit > 1 OR p_recorded_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Direct automated grading facts are invalid';
    END IF;
    PERFORM 1 FROM ple_private.question_submission AS submission
     WHERE submission.submission_id = p_submission_id
       AND submission.question_attempt_id = p_question_attempt_id
     FOR KEY SHARE;
    IF NOT FOUND OR EXISTS (
        SELECT 1 FROM ple_private.grading_result AS result
         WHERE result.question_attempt_id = p_question_attempt_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Direct automated grading target is unavailable';
    END IF;
    INSERT INTO ple_private.question_submission_grading (
        question_submission_grading_id, submission_id, grading_state, created_at, completed_at
    ) VALUES (
        grading_id, p_submission_id, 'graded', p_recorded_at, p_recorded_at
    );
    INSERT INTO ple_private.grading_result (
        grading_result_id, submission_id, question_submission_grading_id,
        question_attempt_id, normalized_credit, recorded_at
    ) VALUES (
        result_id, p_submission_id, grading_id, p_question_attempt_id,
        p_normalized_credit, p_recorded_at
    );
    calculated_checksum := pg_catalog.sha256(
        pg_catalog.convert_to('ple:automated-grading-receipt:v1', 'UTF8')
        || pg_catalog.uuid_send(receipt_id)
        || pg_catalog.uuid_send(result_id)
        || pg_catalog.uuid_send(grading_id)
        || pg_catalog.uuid_send(p_submission_id)
        || pg_catalog.uuid_send(p_question_attempt_id)
        || pg_catalog.numeric_send(p_normalized_credit)
        || pg_catalog.int8send((extract(epoch FROM p_recorded_at) * 1000)::bigint)
    );
    INSERT INTO ple_audit.automated_grading_receipt (
        automated_grading_receipt_id, question_submission_grading_id,
        grading_result_id, committed_at, automated_grading_receipt_checksum
    ) VALUES (
        receipt_id, grading_id, result_id, p_recorded_at, calculated_checksum
    );
    PERFORM ple_private.capture_question_statistics_observation(receipt_id, ARRAY[]::text[]);
END $$;
